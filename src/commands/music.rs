use crate::commands::Data;
use crate::services::music::queue::QueuedTrack;
use crate::utils::embed;
use poise::serenity_prelude::{CreateEmbed, Mentionable};
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Helper to send embed response
async fn send_embed(ctx: Context<'_>, embed: CreateEmbed) -> Result<(), Error> {
    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Extract YouTube video ID from URL
fn extract_video_id(url: &str) -> Option<String> {
    if url.contains("youtu.be/") {
        return url
            .split("youtu.be/")
            .nth(1)?
            .split('?')
            .next()
            .map(|s| s.to_string());
    }
    if url.contains("youtube.com") {
        if let Some(v_param) = url.split("v=").nth(1) {
            return Some(v_param.split('&').next()?.to_string());
        }
    }
    None
}

/// Bergabung ke voice channel (Lavalink mode)
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn join(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;
    let guild = ctx.guild().ok_or("Cannot get server info")?.clone();

    let channel_id = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|vs| vs.channel_id);

    let channel_id = match channel_id {
        Some(id) => id,
        None => {
            send_embed(
                ctx,
                embed::error(
                    "Voice Channel Required",
                    "You must be in a voice channel first!",
                ),
            )
            .await?;
            return Ok(());
        }
    };

    let player = ctx
        .data()
        .music_player
        .as_ref()
        .ok_or("Music player not available. Make sure Lavalink server is running.")?;

    let songbird = ctx.data().songbird.clone();

    let _ = songbird.leave(guild_id).await;

    let (connection_info, handle) = match songbird.join_gateway(guild_id, channel_id).await {
        Ok(result) => result,
        Err(e) => {
            send_embed(
                ctx,
                embed::error(
                    "Connection Failed",
                    &format!("Failed to join voice channel: {:?}", e),
                ),
            )
            .await?;
            return Ok(());
        }
    };

    let _handler = handle;

    use lavalink_rs::model::player::ConnectionInfo as LavalinkConnectionInfo;
    let lavalink_connection_info = LavalinkConnectionInfo {
        endpoint: connection_info.endpoint,
        token: connection_info.token,
        session_id: connection_info.session_id,
    };

    match player
        .create_player_with_connection(guild_id, lavalink_connection_info)
        .await
    {
        Ok(_) => {
            player.ensure_queue(guild_id);

            send_embed(
                ctx,
                embed::success(
                    "Connected",
                    &format!("Joined {}! Use /play to add songs.", channel_id.mention()),
                ),
            )
            .await?;
        }
        Err(e) => {
            let _ = songbird.leave(guild_id).await;
            send_embed(
                ctx,
                embed::error("Player Error", &format!("Failed to create player: {}", e)),
            )
            .await?;
        }
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;

    let player = ctx
        .data()
        .music_player
        .as_ref()
        .ok_or("Music player not available")?;

    if let Some(player_ctx) = player.get_player_context(guild_id) {
        let _ = player_ctx.close();
    }
    player.clear_queue(guild_id);

    let songbird = ctx.data().songbird.clone();
    let _ = songbird.leave(guild_id).await;

    send_embed(ctx, embed::info("Goodbye", "See you next time!")).await?;

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "URL or song title"]
    #[rest]
    query: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;
    let guild = ctx.guild().ok_or("Cannot get server info")?.clone();

    let player = ctx
        .data()
        .music_player
        .as_ref()
        .ok_or("Music player not available. Make sure Lavalink server is running.")?;

    let channel_id = match guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|vs| vs.channel_id)
    {
        Some(id) => id,
        None => {
            send_embed(
                ctx,
                embed::error(
                    "Voice Channel Required",
                    "You must be in a voice channel first!",
                ),
            )
            .await?;
            return Ok(());
        }
    };

    ctx.defer().await?;

    let songbird = ctx.data().songbird.clone();
    let needs_join = player.get_player_context(guild_id).is_none();

    if needs_join {
        let (connection_info, _handle) = match songbird.join_gateway(guild_id, channel_id).await {
            Ok(result) => result,
            Err(e) => {
                send_embed(
                    ctx,
                    embed::error(
                        "Connection Failed",
                        &format!("Failed to join voice channel: {:?}", e),
                    ),
                )
                .await?;
                return Ok(());
            }
        };

        use lavalink_rs::model::player::ConnectionInfo as LavalinkConnectionInfo;
        let lavalink_connection_info = LavalinkConnectionInfo {
            endpoint: connection_info.endpoint,
            token: connection_info.token,
            session_id: connection_info.session_id,
        };

        if let Err(e) = player
            .create_player_with_connection(guild_id, lavalink_connection_info)
            .await
        {
            let _ = songbird.leave(guild_id).await;
            send_embed(
                ctx,
                embed::error("Player Error", &format!("Failed to create player: {}", e)),
            )
            .await?;
            return Ok(());
        }

        player.ensure_queue(guild_id);
    }

    let is_url = query.starts_with("http://") || query.starts_with("https://");

    if is_url {
        let tracks = player.search_tracks(guild_id, &query).await?;
        if tracks.is_empty() {
            send_embed(ctx, embed::error("Not Found", "Could not load this URL")).await?;
            return Ok(());
        }

        // Check if it's a playlist (more than one track)
        if tracks.len() > 1 {
            return play_playlist(ctx, player, guild_id, tracks).await;
        }

        return play_track(ctx, player, guild_id, &tracks[0]).await;
    }

    if let Some(youtube) = &ctx.data().youtube_search {
        match youtube.search(&query, 10).await {
            Ok(videos) if !videos.is_empty() => {
                // Show dropdown with search results
                return show_search_results(ctx, player, guild_id, videos, &query).await;
            }
            Ok(_) => {
                send_embed(ctx, embed::error("Not Found", "No YouTube videos found")).await?;
                return Ok(());
            }
            Err(e) => {
                println!("[WARN] YouTube API search failed: {}", e);
            }
        }
    }

    let tracks = player.search_tracks(guild_id, &query).await?;
    if tracks.is_empty() {
        send_embed(ctx, embed::error("Not Found", "No songs found")).await?;
        return Ok(());
    }
    play_track(ctx, player, guild_id, &tracks[0]).await
}

/// Play a playlist - adds all tracks to queue
async fn play_playlist(
    ctx: Context<'_>,
    player: &crate::services::music::MusicPlayer,
    guild_id: poise::serenity_prelude::GuildId,
    tracks: Vec<lavalink_rs::model::track::TrackData>,
) -> Result<(), Error> {
    let track_count = tracks.len();

    player.set_text_channel(guild_id, ctx.channel_id());

    // Get current queue state before adding
    let queue_before = player.get_queue(guild_id);
    let was_empty = queue_before.current.is_none() && queue_before.is_empty();

    // Add all tracks to queue
    for track in &tracks {
        let queued_track = QueuedTrack {
            track: track.clone(),
            requester_id: ctx.author().id.get(),
            requester_name: ctx.author().name.clone(),
        };
        player.add_to_queue(guild_id, queued_track);
    }

    // If queue was empty, start playing the first track
    if was_empty {
        if let Some(player_ctx) = player.get_player_context(guild_id) {
            if let Some(first_track) = player.next_track(guild_id) {
                println!(
                    "[MUSIC] Playing first track from playlist: {}",
                    first_track.track.info.title
                );

                match player_ctx.play(&first_track.track).await {
                    Ok(player_info) => {
                        println!(
                            "[MUSIC] Playlist playback started, player state: {:?}",
                            player_info.state
                        );
                        player.set_current(guild_id, Some(first_track.clone()));

                        // Send now playing embed for first track
                        let first_info = &first_track.track.info;
                        send_embed(
                            ctx,
                            embed::playlist_added(
                                &first_info.title,
                                &first_info.uri.clone().unwrap_or_default(),
                                track_count,
                                &ctx.author().name,
                                first_info.artwork_url.as_deref(),
                            ),
                        )
                        .await?;
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("[MUSIC] Failed to play playlist: {}", e);
                        send_embed(ctx, embed::error("Playback Error", &format!("{}", e))).await?;
                        return Ok(());
                    }
                }
            }
        }
    }

    // If something was already playing, just show added message
    let first_track = tracks.first().map(|t| &t.info);
    send_embed(
        ctx,
        embed::playlist_added(
            first_track.map(|i| i.title.as_str()).unwrap_or("Unknown"),
            first_track.and_then(|i| i.uri.as_deref()).unwrap_or(""),
            track_count,
            &ctx.author().name,
            first_track.and_then(|i| i.artwork_url.as_deref()),
        ),
    )
    .await?;

    Ok(())
}

async fn play_track(
    ctx: Context<'_>,
    player: &crate::services::music::MusicPlayer,
    guild_id: poise::serenity_prelude::GuildId,
    track: &lavalink_rs::model::track::TrackData,
) -> Result<(), Error> {
    let track_info = track.info.clone();

    let queued_track = QueuedTrack {
        track: track.clone(),
        requester_id: ctx.author().id.get(),
        requester_name: ctx.author().name.clone(),
    };

    player.set_text_channel(guild_id, ctx.channel_id());
    player.add_to_queue(guild_id, queued_track.clone());

    if let Some(player_ctx) = player.get_player_context(guild_id) {
        let queue = player.get_queue(guild_id);
        let queue_position = queue.len();
        let is_first_track = queue.current.is_none() && queue_position == 1;

        if is_first_track {
            println!("[MUSIC] Playing first track: {}", track.info.title);

            match player_ctx.play(track).await {
                Ok(player_info) => {
                    println!(
                        "[MUSIC] Play request successful, player state: {:?}",
                        player_info.state
                    );

                    // Reset idle timer since we're playing music
                    player.touch_activity(guild_id);

                    if let Some(next_track) = player.next_track(guild_id) {
                        // Save track title for autoplay
                        player.set_last_track_title(
                            guild_id,
                            Some(next_track.track.info.title.clone()),
                        );
                        // Save video ID for YouTube Mix
                        if let Some(ref uri) = next_track.track.info.uri {
                            if let Some(vid) = extract_video_id(uri) {
                                player.set_last_video_id(guild_id, Some(vid));
                            }
                        }
                        player.set_current(guild_id, Some(next_track));
                    }
                }
                Err(e) => {
                    eprintln!("[MUSIC] Failed to play track: {}", e);
                    send_embed(ctx, embed::error("Playback Error", &format!("{}", e))).await?;
                    return Ok(());
                }
            }
        } else {
            println!(
                "[MUSIC] Added to queue: {} (position #{})",
                track.info.title, queue_position
            );
        }

        send_embed(
            ctx,
            if is_first_track {
                embed::now_playing(
                    &track_info.title,
                    &track_info.uri.clone().unwrap_or_default(),
                    &track_info.author,
                    &format_duration(track_info.length),
                    &ctx.author().name,
                    100,   // default volume
                    false, // not looping
                    track_info.artwork_url.as_deref(),
                )
            } else {
                embed::added_to_queue(
                    &track_info.title,
                    &track_info.uri.unwrap_or_default(),
                    &format_duration(track_info.length),
                    queue_position,
                    &ctx.author().name,
                    track_info.artwork_url.as_deref(),
                )
            },
        )
        .await?;
    } else {
        send_embed(ctx, embed::error("Error", "Player not connected")).await?;
    }

    Ok(())
}

async fn show_search_results(
    ctx: Context<'_>,
    player: &crate::services::music::MusicPlayer,
    guild_id: poise::serenity_prelude::GuildId,
    videos: Vec<crate::services::youtube::YouTubeVideo>,
    query: &str,
) -> Result<(), Error> {
    use poise::serenity_prelude::{
        ComponentInteractionCollector, CreateActionRow, CreateInteractionResponse,
        CreateInteractionResponseMessage, CreateSelectMenu, CreateSelectMenuKind,
        CreateSelectMenuOption,
    };
    use std::time::Duration;

    let options: Vec<CreateSelectMenuOption> = videos
        .iter()
        .enumerate()
        .map(|(i, video)| {
            let label = if video.title.len() > 95 {
                format!("{}...", &video.title[..92])
            } else {
                video.title.clone()
            };
            CreateSelectMenuOption::new(label, i.to_string())
                .description(format!("by {}", &video.channel))
        })
        .collect();

    let select_menu =
        CreateSelectMenu::new("song_select", CreateSelectMenuKind::String { options })
            .placeholder("🎵 Select a song to play");

    let action_row = CreateActionRow::SelectMenu(select_menu);

    let description = videos
        .iter()
        .enumerate()
        .take(10)
        .map(|(i, v)| format!("**{}. {}**\n└ {}", i + 1, v.title, v.channel))
        .collect::<Vec<_>>()
        .join("\n\n");

    let search_embed = CreateEmbed::new()
        .title(format!("🔍 Search: {}", query))
        .description(description)
        .footer(poise::serenity_prelude::CreateEmbedFooter::new(
            "Select a song from the dropdown below • Expires in 60s",
        ))
        .color(embed::COLOR_MUSIC);

    let reply = ctx
        .send(
            poise::CreateReply::default()
                .embed(search_embed)
                .components(vec![action_row]),
        )
        .await?;

    let msg = reply.message().await?;

    let interaction = ComponentInteractionCollector::new(ctx.serenity_context().shard.clone())
        .message_id(msg.id)
        .author_id(ctx.author().id)
        .timeout(Duration::from_secs(60))
        .await;

    match interaction {
        Some(interaction) => {
            use poise::serenity_prelude::ComponentInteractionDataKind;
            let selected_idx: usize = match &interaction.data.kind {
                ComponentInteractionDataKind::StringSelect { values } => values
                    .first()
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(0),
                _ => 0,
            };

            if let Some(video) = videos.get(selected_idx) {
                interaction
                    .create_response(
                        ctx.http(),
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::new()
                                .content(format!("Loading **{}**...", video.title))
                                .embeds(vec![])
                                .components(vec![]),
                        ),
                    )
                    .await?;

                let tracks = player.search_tracks(guild_id, &video.url).await?;
                if let Some(track) = tracks.first() {
                    play_track(ctx, player, guild_id, track).await?;
                } else {
                    send_embed(
                        ctx,
                        embed::error("Error", "Failed to load the selected video"),
                    )
                    .await?;
                }
            }
        }
        None => {
            // Timeout - remove dropdown
            let _ = reply
                .edit(
                    ctx,
                    poise::CreateReply::default()
                        .embed(
                            CreateEmbed::new()
                                .title("Selection Expired")
                                .description("No song was selected. Use `/play` again to search.")
                                .color(0x95a5a6),
                        )
                        .components(vec![]),
                )
                .await;
        }
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn pause(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;
    let player = ctx
        .data()
        .music_player
        .as_ref()
        .ok_or("Music player not available")?;

    if let Some(player_ctx) = player.get_player_context(guild_id) {
        player_ctx.set_pause(true).await?;
        player.set_paused(guild_id, true);
        send_embed(ctx, embed::music("Paused", "Playback has been paused")).await?;
    } else {
        send_embed(
            ctx,
            embed::error("Not Playing", "The bot is not playing music"),
        )
        .await?;
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn resume(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;
    let player = ctx
        .data()
        .music_player
        .as_ref()
        .ok_or("Music player not available")?;

    if let Some(player_ctx) = player.get_player_context(guild_id) {
        player_ctx.set_pause(false).await?;
        player.set_paused(guild_id, false);
        send_embed(ctx, embed::music("Resumed", "Playback has been resumed")).await?;
    } else {
        send_embed(
            ctx,
            embed::error("Not Playing", "The bot is not playing music"),
        )
        .await?;
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;
    let player = ctx
        .data()
        .music_player
        .as_ref()
        .ok_or("Music player not available")?;

    if let Some(player_ctx) = player.get_player_context(guild_id) {
        if let Some(next_track) = player.next_track(guild_id) {
            // Save track title for autoplay
            player.set_last_track_title(guild_id, Some(next_track.track.info.title.clone()));
            player.set_current(guild_id, Some(next_track.clone()));
            let _ = player_ctx.play(&next_track.track).await;
            send_embed(
                ctx,
                embed::music(
                    "Skipped",
                    &format!("Now playing: **{}**", next_track.track.info.title),
                ),
            )
            .await?;
        } else {
            // Queue is empty - check autoplay
            let is_autoplay = player.is_autoplay(guild_id);

            if is_autoplay {
                // Trigger autoplay directly instead of relying on track_end
                send_embed(
                    ctx,
                    embed::music("Skipped", "Queue empty - finding a new song..."),
                )
                .await?;

                // Do autoplay search and play
                if let Some(track) = search_autoplay_track(player, guild_id).await {
                    // Stop current track first
                    let _ = player_ctx.stop_now().await;

                    player.set_last_track_title(guild_id, Some(track.info.title.clone()));
                    let queued = crate::services::music::queue::QueuedTrack {
                        track: track.clone(),
                        requester_id: 0,
                        requester_name: "Autoplay".to_string(),
                    };
                    player.set_current(guild_id, Some(queued));

                    if let Err(e) = player_ctx.play(&track).await {
                        eprintln!("[MUSIC] Skip autoplay failed: {}", e);
                        send_embed(ctx, embed::error("Autoplay Error", "Failed to play track"))
                            .await?;
                    } else {
                        let channel_id = ctx.channel_id();
                        let mut embed_msg = CreateEmbed::new()
                            .title("🔄 Autoplay")
                            .description(format!(
                                "**[{}]({})**\nby {}",
                                track.info.title,
                                track.info.uri.clone().unwrap_or_default(),
                                track.info.author
                            ))
                            .color(0x1DB954)
                            .footer(serenity::all::CreateEmbedFooter::new(
                                "Use /autoplay to disable",
                            ));

                        // Add thumbnail if available
                        if let Some(ref artwork) = track.info.artwork_url {
                            embed_msg = embed_msg.thumbnail(artwork);
                        }

                        let message = serenity::all::CreateMessage::new().embed(embed_msg);
                        let _ = channel_id.send_message(&ctx.http(), message).await;
                    }
                } else {
                    player_ctx.stop_now().await?;
                    send_embed(ctx, embed::info("Autoplay", "No related songs found")).await?;
                }
            } else {
                player_ctx.stop_now().await?;
                send_embed(
                    ctx,
                    embed::info("Queue Empty", "No more songs in queue, playback stopped"),
                )
                .await?;
            }
        }
    } else {
        send_embed(
            ctx,
            embed::error("Not Playing", "The bot is not playing music"),
        )
        .await?;
    }

    Ok(())
}

/// Search for autoplay track using YouTube API
async fn search_autoplay_track(
    player: &crate::services::music::MusicPlayer,
    guild_id: serenity::all::GuildId,
) -> Option<lavalink_rs::model::track::TrackData> {
    use crate::services::youtube::get_global_youtube;

    let last_title = player.get_last_track_title(guild_id)?;
    let youtube = get_global_youtube()?;

    // Simplify search query - take first 2 words + "mix"
    let simplified: String = last_title
        .split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ");

    let search_query = format!("{} mix", simplified);
    println!("[MUSIC] Skip autoplay searching: {}", search_query);

    let videos = youtube.search(&search_query, 10).await.ok()?;
    if videos.is_empty() {
        return None;
    }

    // Filter out similar titles
    let last_title_lower = last_title.to_lowercase();
    let simplified_lower = simplified.to_lowercase();

    let filtered: Vec<_> = videos
        .iter()
        .filter(|v| {
            let title_lower = v.title.to_lowercase();
            !title_lower.contains(&simplified_lower) && !last_title_lower.contains(&title_lower)
        })
        .collect();

    let selected = if !filtered.is_empty() {
        &filtered[0]
    } else if videos.len() > 1 {
        &videos[1]
    } else {
        return None;
    };

    println!("[MUSIC] Skip autoplay found: {}", selected.title);

    // Load track via Lavalink
    let lavalink_guild_id = lavalink_rs::model::GuildId(guild_id.get());
    let load_result = player
        .lavalink
        .load_tracks(lavalink_guild_id, &selected.url)
        .await
        .ok()?;

    use lavalink_rs::model::track::TrackLoadData;
    match load_result.data {
        Some(TrackLoadData::Track(t)) => Some(t),
        Some(TrackLoadData::Search(mut t)) if !t.is_empty() => Some(t.remove(0)),
        Some(TrackLoadData::Playlist(mut p)) if !p.tracks.is_empty() => Some(p.tracks.remove(0)),
        _ => None,
    }
}

#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;
    let player = ctx
        .data()
        .music_player
        .as_ref()
        .ok_or("Music player not available")?;

    if let Some(player_ctx) = player.get_player_context(guild_id) {
        player_ctx.stop_now().await?;
        player.clear_queue(guild_id);
        send_embed(
            ctx,
            embed::info("Stopped", "Music stopped and queue cleared"),
        )
        .await?;
    } else {
        player.clear_queue(guild_id);
        send_embed(ctx, embed::info("Queue Cleared", "Queue has been cleared")).await?;
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn queue(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;
    let player = ctx
        .data()
        .music_player
        .as_ref()
        .ok_or("Music player not available")?;

    let queue = player.get_queue(guild_id);

    if queue.current.is_none() && queue.is_empty() {
        send_embed(ctx, embed::info("Queue Empty", "No songs in queue")).await?;
        return Ok(());
    }

    let mut description = String::new();

    if let Some(current) = &queue.current {
        description.push_str(&format!(
            "**Now Playing:**\n[{}]({}) - Requested by {}\n\n",
            current.track.info.title,
            current.track.info.uri.clone().unwrap_or_default(),
            current.requester_name
        ));
    }

    if !queue.is_empty() {
        description.push_str("**Queue:**\n");
        for (i, track) in queue.tracks.iter().take(10).enumerate() {
            description.push_str(&format!(
                "{}. [{}]({}) - {}\n",
                i + 1,
                track.track.info.title,
                track.track.info.uri.clone().unwrap_or_default(),
                format_duration(track.track.info.length)
            ));
        }
        if queue.len() > 10 {
            description.push_str(&format!("\n... and {} more songs", queue.len() - 10));
        }
    }

    let embed = CreateEmbed::new()
        .title("Music Queue")
        .description(description)
        .field(
            "Total Songs",
            format!(
                "{}",
                queue.len() + if queue.current.is_some() { 1 } else { 0 }
            ),
            true,
        )
        .field(
            "Loop",
            {
                use crate::services::music::queue::LoopMode;
                match queue.loop_mode {
                    LoopMode::Off => "Off",
                    LoopMode::Track => "🔂 Track",
                    LoopMode::Queue => "🔁 Queue",
                }
            },
            true,
        )
        .field("Volume", format!("{}%", queue.volume), true)
        .color(embed::COLOR_MUSIC);

    send_embed(ctx, embed).await?;

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only, aliases("np"))]
pub async fn nowplaying(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;
    let player = ctx
        .data()
        .music_player
        .as_ref()
        .ok_or("Music player not available")?;

    let queue = player.get_queue(guild_id);

    match &queue.current {
        Some(current) => {
            let track_info = &current.track.info;
            send_embed(
                ctx,
                embed::now_playing(
                    &track_info.title,
                    &track_info.uri.clone().unwrap_or_default(),
                    &track_info.author,
                    &format_duration(track_info.length),
                    &current.requester_name,
                    queue.volume,
                    queue.is_looping,
                    track_info.artwork_url.as_deref(),
                ),
            )
            .await?;
        }
        None => {
            send_embed(
                ctx,
                embed::error("Not Playing", "No song is currently playing"),
            )
            .await?;
        }
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn volume(
    ctx: Context<'_>,
    #[description = "Volume (0-150)"]
    #[min = 0]
    #[max = 150]
    level: u8,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;
    let player = ctx
        .data()
        .music_player
        .as_ref()
        .ok_or("Music player not available")?;

    player.set_volume(guild_id, level);

    if let Some(player_ctx) = player.get_player_context(guild_id) {
        // Lavalink volume: 0-1000, where 100 = 100% normal volume
        // User input is 0-150, so we can use it directly
        let lavalink_volume = level as u16;
        player_ctx.set_volume(lavalink_volume).await?;
    }

    let icon = match level {
        0 => "Muted",
        1..=30 => "Low",
        31..=70 => "Medium",
        _ => "High",
    };

    send_embed(
        ctx,
        embed::music(
            "Volume Changed",
            &format!("Volume set to **{}%** ({})", level, icon),
        ),
    )
    .await?;

    Ok(())
}

/// Toggle repeat mode (use 'q' for queue repeat)
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    rename = "repeat",
    aliases("r")
)]
pub async fn repeat(
    ctx: Context<'_>,
    #[description = "'q' for queue repeat, empty for track"] mode: Option<String>,
) -> Result<(), Error> {
    use crate::services::music::queue::LoopMode;

    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;
    let player = ctx
        .data()
        .music_player
        .as_ref()
        .ok_or("Music player not available")?;

    let current_mode = player.get_loop_mode(guild_id);

    // Determine target mode based on argument
    let is_queue_mode = mode
        .as_ref()
        .map(|m| {
            let m = m.to_lowercase();
            m == "q" || m == "queue"
        })
        .unwrap_or(false);

    let new_mode = if is_queue_mode {
        // Toggle queue repeat
        if current_mode == LoopMode::Queue {
            LoopMode::Off
        } else {
            LoopMode::Queue
        }
    } else {
        // Toggle track repeat
        if current_mode == LoopMode::Track {
            LoopMode::Off
        } else {
            LoopMode::Track
        }
    };

    player.set_loop_mode(guild_id, new_mode.clone());

    let (title, description) = match new_mode {
        LoopMode::Off => ("🔁 Repeat Disabled", "Playback will continue normally"),
        LoopMode::Track => ("🔂 Repeat Track", "Current track will repeat"),
        LoopMode::Queue => ("🔁 Repeat Queue", "Entire queue will repeat when finished"),
    };

    send_embed(ctx, embed::music(title, description)).await?;

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn shuffle(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;
    let player = ctx
        .data()
        .music_player
        .as_ref()
        .ok_or("Music player not available")?;

    player.shuffle_queue(guild_id);
    send_embed(
        ctx,
        embed::music("Queue Shuffled", "The queue has been shuffled"),
    )
    .await?;

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Position in queue (1, 2, 3, ...)"] position: usize,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;
    let player = ctx
        .data()
        .music_player
        .as_ref()
        .ok_or("Music player not available")?;

    if position == 0 {
        send_embed(
            ctx,
            embed::error("Invalid Position", "Position must start from 1"),
        )
        .await?;
        return Ok(());
    }

    match player.remove_from_queue(guild_id, position - 1) {
        Some(removed) => {
            send_embed(
                ctx,
                embed::success(
                    "Removed",
                    &format!("Removed **{}** from queue", removed.track.info.title),
                ),
            )
            .await?;
        }
        None => {
            send_embed(ctx, embed::error("Not Found", "No song at that position")).await?;
        }
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn autoplay(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a server")?;
    let player = ctx
        .data()
        .music_player
        .as_ref()
        .ok_or("Music player not available")?;

    let current_state = player.is_autoplay(guild_id);
    let new_state = !current_state;
    player.set_autoplay(guild_id, new_state);

    let (title, description, color) = if new_state {
        (
            "Autoplay Enabled",
            "When the queue ends, related songs will be automatically added from YouTube.",
            embed::COLOR_SUCCESS,
        )
    } else {
        (
            "Autoplay Disabled",
            "Automatic song recommendations have been turned off.",
            embed::COLOR_WARNING,
        )
    };

    let embed = CreateEmbed::new()
        .title(title)
        .description(description)
        .color(color);
    send_embed(ctx, embed).await?;

    Ok(())
}

fn format_duration(ms: u64) -> String {
    let duration = Duration::from_millis(ms);
    let secs = duration.as_secs();
    let mins = secs / 60;
    let secs = secs % 60;
    let hours = mins / 60;
    let mins = mins % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{:02}:{:02}", mins, secs)
    }
}
