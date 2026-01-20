use crate::commands::Data;
use crate::repository::ModerationRepository;
use crate::services::link::Downloader;
use crate::services::music::player::get_bot_user_id;
use crate::utils::embed;
use serenity::all::{ChannelId, Context, CreateAttachment, CreateMessage, FullEvent, GuildId, Member, RoleId, User};

/// Main event handler for Discord events
pub async fn handle_event(
    ctx: &Context,
    event: &FullEvent,
    data: &Data,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match event {
        FullEvent::Message { new_message } => {
            handle_video_link(ctx, new_message).await?;
        }
        FullEvent::VoiceStateUpdate { old, new } => {
            handle_voice_state_update(ctx, old, new, data).await?;
        }
        FullEvent::GuildMemberAddition { new_member } => {
            handle_member_join(ctx, new_member, data).await?;
        }
        FullEvent::GuildMemberRemoval {
            guild_id,
            user,
            member_data_if_available,
        } => {
            handle_member_leave(
                ctx,
                *guild_id,
                user,
                member_data_if_available.as_ref(),
                data,
            )
            .await?;
        }
        _ => {}
    }

    Ok(())
}

async fn handle_video_link(
    ctx: &Context,
    message: &serenity::all::Message,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if message.author.bot {
        return Ok(());
    }

    let url = extract_video_url(&message.content);
    if url.is_none() {
        return Ok(());
    }
    let url = url.unwrap();

    let platform = Downloader::detect_platform(&url);
    if !platform.is_supported() {
        return Ok(());
    }

    let _ = message.channel_id.broadcast_typing(&ctx.http).await;

    let downloader = match Downloader::new().await {
        Ok(dl) => dl,
        Err(e) => {
            println!("[VIDEO] Failed to initialize downloader: {}", e);
            return Ok(());
        }
    };

    let video_path = match downloader.download(&url).await {
        Ok(path) => path,
        Err(e) => {
            println!("[VIDEO] Failed to download video: {}", e);
            return Ok(());
        }
    };

    let file_size = std::fs::metadata(&video_path)?.len();
    let max_size: u64 = 25 * 1024 * 1024;

    if file_size > max_size {
        let _ = Downloader::delete_video(&video_path);
        println!("[VIDEO] Video too large: {:.2} MB", file_size as f64 / 1024.0 / 1024.0);
        return Ok(());
    }

    let file_data = std::fs::read(&video_path)?;
    let attachment = CreateAttachment::bytes(file_data, "video.mp4");

    let _ = message.channel_id.send_message(
        &ctx.http,
        CreateMessage::new().add_file(attachment)
    ).await;

    let _ = Downloader::delete_video(&video_path);

    Ok(())
}

/// Extract video URL from message content
fn extract_video_url(content: &str) -> Option<String> {
    // Simple URL extraction - look for http/https links
    for word in content.split_whitespace() {
        if word.starts_with("http://") || word.starts_with("https://") {
            let platform = Downloader::detect_platform(word);
            if platform.is_supported() {
                return Some(word.to_string());
            }
        }
    }
    None
}

/// Handle voice state updates (join/leave voice channels)
async fn handle_voice_state_update(
    ctx: &Context,
    old: &Option<serenity::all::VoiceState>,
    new: &serenity::all::VoiceState,
    data: &Data,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Ok(user) = ctx.http.get_user(new.user_id).await {
        if user.bot {
            return Ok(());
        }
    }

    let old_channel = old.as_ref().and_then(|vs| vs.channel_id);
    let new_channel = new.channel_id;

    if old_channel.is_some() && old_channel != new_channel {
        if let Some(guild_id) = new.guild_id {
            if let Some(left_channel_id) = old_channel {
                handle_auto_disconnect(ctx, data, guild_id, left_channel_id).await;
            }
        }
    }

    if let Some(guild_id) = new.guild_id {
        handle_voice_logging(ctx, data, guild_id, old_channel, new_channel, new.user_id).await?;
    }

    Ok(())
}

/// Auto-disconnect bot when all users leave the voice channel
async fn handle_auto_disconnect(
    ctx: &Context,
    data: &Data,
    guild_id: GuildId,
    left_channel_id: ChannelId,
) {
    let bot_user_id = match get_bot_user_id() {
        Some(id) => id,
        None => return,
    };

    let should_disconnect = {
        if let Some(guild) = ctx.cache.guild(guild_id) {
            let bot_in_channel = guild
                .voice_states
                .get(&bot_user_id)
                .and_then(|vs| vs.channel_id)
                .map(|ch| ch == left_channel_id)
                .unwrap_or(false);

            if bot_in_channel {
                let users_in_channel = guild
                    .voice_states
                    .iter()
                    .filter(|(user_id, vs)| {
                        vs.channel_id == Some(left_channel_id) && **user_id != bot_user_id
                    })
                    .count();

                println!("[MUSIC] Users remaining in channel: {}", users_in_channel);
                users_in_channel == 0
            } else {
                false
            }
        } else {
            false
        }
    };

    if should_disconnect {
        println!("[MUSIC] No users in voice channel, auto-disconnecting...");

        if let Some(player) = &data.music_player {
            if let Some(player_ctx) = player.get_player_context(guild_id) {
                let _ = player_ctx.close();
            }
            player.clear_queue(guild_id);
        }

        let _ = data.songbird.leave(guild_id).await;

        if let Some(player) = &data.music_player {
            if let Some(channel_id) = player.get_text_channel(guild_id) {
                let embed_msg = embed::info("Disconnect", "Left voice channel.");
                let message = CreateMessage::new().embed(embed_msg);
                let _ = channel_id.send_message(&ctx.http, message).await;
            }
        }
    }
}

/// Log voice channel join/leave events
async fn handle_voice_logging(
    ctx: &Context,
    data: &Data,
    guild_id: GuildId,
    old_channel: Option<ChannelId>,
    new_channel: Option<ChannelId>,
    user_id: serenity::all::UserId,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db = data.db.lock().await;
    let conn = db.get_connection();
    let config = ModerationRepository::get_config(conn, guild_id.get());
    drop(db);

    if let Ok(Some(config)) = config {
        if let Some(log_channel_id) = config.log_channel_id {
            let log_channel = ChannelId::new(log_channel_id);
            let user = ctx.http.get_user(user_id).await?;
            let avatar = user.avatar_url();

            // User joined a voice channel
            if new_channel.is_some() && old_channel != new_channel {
                if let Some(joined_channel_id) = new_channel {
                    let channel_name = get_channel_name(ctx, guild_id, joined_channel_id);
                    let embed_msg = embed::voice_join(
                        &user.name,
                        user.id.get(),
                        &channel_name,
                        avatar.as_deref(),
                    );
                    let message = CreateMessage::new().embed(embed_msg);
                    let _ = log_channel.send_message(&ctx.http, message).await;
                }
            }

            // User left a voice channel
            if old_channel.is_some() && old_channel != new_channel {
                if let Some(left_channel_id) = old_channel {
                    let channel_name = get_channel_name(ctx, guild_id, left_channel_id);
                    let embed_msg = embed::voice_leave(
                        &user.name,
                        user.id.get(),
                        &channel_name,
                        avatar.as_deref(),
                    );
                    let message = CreateMessage::new().embed(embed_msg);
                    let _ = log_channel.send_message(&ctx.http, message).await;
                }
            }
        }
    }

    Ok(())
}

/// Get channel name from cache or guild
fn get_channel_name(ctx: &Context, guild_id: GuildId, channel_id: ChannelId) -> String {
    ctx.cache
        .guild(guild_id)
        .and_then(|g| g.channels.get(&channel_id).map(|c| c.name.clone()))
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Handle new member joining the server
async fn handle_member_join(
    ctx: &Context,
    new_member: &Member,
    data: &Data,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let guild_id = new_member.guild_id;

    let db = data.db.lock().await;
    let conn = db.get_connection();
    let config = ModerationRepository::get_config(conn, guild_id.get());
    drop(db);

    if let Ok(Some(config)) = config {
        if let Some(role_id) = config.auto_role_id {
            let role = RoleId::new(role_id);
            let member = new_member.clone();
            if let Err(e) = member.add_role(&ctx.http, role).await {
                eprintln!("[MOD] Failed to assign auto-role: {}", e);
            }
        }

        if let Some(log_channel_id) = config.log_channel_id {
            let channel = ChannelId::new(log_channel_id);
            let member_count = ctx
                .cache
                .guild(guild_id)
                .map(|g| g.member_count)
                .unwrap_or(0);

            let guild_name = ctx
                .cache
                .guild(guild_id)
                .map(|g| g.name.clone())
                .unwrap_or_else(|| "Server".to_string());

            let account_created = new_member
                .user
                .created_at()
                .format("%Y-%m-%d %H:%M UTC")
                .to_string();
            let avatar = new_member.user.avatar_url();

            let embed_msg = embed::member_join(
                &new_member.user.name,
                new_member.user.id.get(),
                &account_created,
                member_count,
                avatar.as_deref(),
                &guild_name,
            );

            let message = CreateMessage::new().embed(embed_msg);
            if let Err(e) = channel.send_message(&ctx.http, message).await {
                eprintln!("[MOD] Failed to send join log: {}", e);
            }
        }
    }

    Ok(())
}

/// Handle member leaving the server
async fn handle_member_leave(
    ctx: &Context,
    guild_id: GuildId,
    user: &User,
    member_data: Option<&Member>,
    data: &Data,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db = data.db.lock().await;
    let conn = db.get_connection();
    let config = ModerationRepository::get_config(conn, guild_id.get());
    drop(db);

    if let Ok(Some(config)) = config {
        if let Some(log_channel_id) = config.log_channel_id {
            let channel = ChannelId::new(log_channel_id);
            let member_count = ctx
                .cache
                .guild(guild_id)
                .map(|g| g.member_count)
                .unwrap_or(0);

            let guild_name = ctx
                .cache
                .guild(guild_id)
                .map(|g| g.name.clone())
                .unwrap_or_else(|| "Server".to_string());

            let joined_at = member_data
                .and_then(|m| m.joined_at)
                .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string());

            let avatar = user.avatar_url();

            let embed_msg = embed::member_leave(
                &user.name,
                user.id.get(),
                joined_at.as_deref(),
                member_count,
                avatar.as_deref(),
                &guild_name,
            );

            let message = CreateMessage::new().embed(embed_msg);
            if let Err(e) = channel.send_message(&ctx.http, message).await {
                eprintln!("[MOD] Failed to send leave log: {}", e);
            }
        }
    }

    Ok(())
}
