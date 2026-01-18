use dotenvy::dotenv;
use lavalink_rs::client::LavalinkClient;
use lavalink_rs::model::events::{Events, TrackEnd};
use lavalink_rs::node::NodeBuilder;
use poise::serenity_prelude::UserId;
use serenity::all::{ActivityData, GatewayIntents, OnlineStatus};
use songbird::SerenityInit;
use std::collections::HashSet;
use std::env;
use worm::commands::{admin, ai, general, music, ping, qr, redeem, reminder, sys, Data};
use worm::config::Config;
use worm::error::BotError;
use worm::repository::create_pool;
use worm::services::genshin_redeem_checker::start_code_checker;
use worm::services::music::MusicPlayer;

#[tokio::main]
async fn main() -> Result<(), BotError> {
    dotenv().ok();

    println!("Starting WR Bot...");

    let config = Config::from_env()
        .map_err(|e| BotError::Config(format!("Failed to load config: {}", e)))?;

    // Add voice intents for music
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_VOICE_STATES;

    let owner_id = env::var("CLIENT_ID")
        .unwrap_or_else(|_| "0".to_string())
        .parse::<u64>()
        .expect("CLIENT_ID must be a valid u64");

    let mut owners = HashSet::new();
    owners.insert(UserId::new(owner_id));

    let db = create_pool("redeem_bot.db")
        .map_err(|e| BotError::Config(format!("Failed to initialize database: {}", e)))?;

    println!("[OK] Database initialized successfully");

    // Check if AI is enabled
    if config.is_ai_enabled() {
        println!("[OK] AI features enabled");
    } else {
        println!("[WARN] AI features disabled (no API_KEY configured)");
    }

    // Lavalink configuration
    let lavalink_host = env::var("LAVALINK_HOST").unwrap_or_else(|_| "localhost".to_string());
    let lavalink_port = env::var("LAVALINK_PORT")
        .unwrap_or_else(|_| "2333".to_string())
        .parse::<u16>()
        .unwrap_or(2333);
    let lavalink_password =
        env::var("LAVALINK_PASSWORD").unwrap_or_else(|_| "youshallnotpass".to_string());

    let owners_clone = owners.clone();
    let db_for_checker = db.clone();
    let db_for_setup = db.clone();

    // Create Songbird voice manager BEFORE client creation
    // This must be done before framework setup so it can be passed in
    let songbird = songbird::Songbird::serenity();
    let songbird_for_data = songbird.clone();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                // General commands
                ping::ping(),
                general::ping(),
                general::say(),
                // Admin commands
                admin::everyone(),
                // AI commands
                ai::worm(),
                // System commands
                sys::sys(),
                qr::qr(),
                // Redeem commands
                redeem::redeem_setup(),
                redeem::redeem_codes(),
                redeem::redeem_disable(),
                redeem::redeem_enable(),
                // Reminder commands
                reminder::reminder_setup(),
                reminder::reminder_list(),
                reminder::reminder_delete(),
                // Music commands
                music::join(),
                music::leave(),
                music::play(),
                music::pause(),
                music::resume(),
                music::skip(),
                music::stop(),
                music::queue(),
                music::nowplaying(),
                music::volume(),
                music::loop_track(),
                music::shuffle(),
                music::remove(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".into()),
                ..Default::default()
            },
            on_error: |error| Box::pin(on_error(error)),
            event_handler: |ctx, event, _framework, data| {
                Box::pin(handle_event(ctx, event, data))
            },
            ..Default::default()
        })
        .setup(move |ctx, ready, framework| {
            let inner_db = db_for_setup.clone();
            let owners_inner = owners_clone.clone();
            let user_id = ready.user.id;
            let songbird_clone = songbird_for_data.clone();
            let http_clone = ctx.http.clone();

            let lavalink_host = lavalink_host.clone();
            let lavalink_password = lavalink_password.clone();

            Box::pin(async move {
                println!("[OK] Logged in as {}", ready.user.name);

                // Register slash commands globally
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                println!("[OK] Slash commands registered globally");

                // Initialize global HTTP client for track_end handler
                worm::services::music::player::init_global_http(http_clone);
                
                // Initialize bot user ID for voice state checks
                worm::services::music::player::init_bot_user_id(ready.user.id);

                // Try to initialize Lavalink
                let music_player = match initialize_lavalink(
                    &lavalink_host,
                    lavalink_port,
                    &lavalink_password,
                    user_id.get(),
                )
                .await
                {
                    Ok(lavalink) => {
                        println!("[OK] Lavalink connected successfully");
                        let player = MusicPlayer::new(lavalink);
                        // Initialize global player for track_end handler access
                        worm::services::music::player::init_global_player(player.clone());
                        Some(player)
                    }
                    Err(e) => {
                        println!(
                            "[WARN] Lavalink not available: {} - Music features disabled",
                            e
                        );
                        None
                    }
                };

                // Initialize YouTube search service
                let youtube_search = worm::services::youtube::YouTubeSearch::new();
                if youtube_search.is_some() {
                    println!("[OK] YouTube search service initialized");
                } else {
                    println!("[WARN] YouTube search not available (no YOUTUBE_API_KEY)");
                }

                Ok(Data {
                    owners: owners_inner,
                    db: inner_db,
                    music_player,
                    songbird: songbird_clone,
                    youtube_search,
                })
            })
        })
        .build();

    // Register Songbird with the client - THIS IS CRITICAL!
    // Songbird must be registered so it receives voice state updates from Discord
    let mut client = serenity::Client::builder(&config.token, intents)
        .framework(framework)
        .register_songbird_with(songbird.clone())
        .await
        .map_err(|e| BotError::Client(format!("Failed to create client: {}", e)))?;

    let shard_manager = client.shard_manager.clone();
    let http = client.http.clone();

    // Activity rotation task
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        let activities = vec![
            ActivityData::custom("/redeem_codes"),
            ActivityData::custom("!worm untuk chat AI"),
            ActivityData::custom("/reminder_setup"),
            ActivityData::custom("/play untuk musik"),
        ];
        let mut idx = 0;
        loop {
            interval.tick().await;
            let runners = shard_manager.runners.lock().await;
            for (_, runner) in runners.iter() {
                runner
                    .runner_tx
                    .set_presence(Some(activities[idx].clone()), OnlineStatus::Online);
            }
            idx = (idx + 1) % activities.len();
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    start_code_checker(db_for_checker, http).await;
    println!("[OK] Code checker service started!");

    client
        .start()
        .await
        .map_err(|e| BotError::Client(format!("Failed to initialize client: {}", e)))?;

    Ok(())
}

async fn initialize_lavalink(
    host: &str,
    port: u16,
    password: &str,
    user_id: u64,
) -> Result<LavalinkClient, String> {
    let events = Events {
        track_end: Some(|client, _session_id, event| Box::pin(handle_track_end(client, event))),
        ..Default::default()
    };

    let node = NodeBuilder {
        hostname: format!("{}:{}", host, port),
        password: password.to_string(),
        user_id: user_id.into(),
        session_id: None,
        is_ssl: false,
        events: events.clone().into(),
    };

    Ok(LavalinkClient::new(
        events,
        vec![node],
        lavalink_rs::model::client::NodeDistributionStrategy::round_robin(),
    )
    .await)
}

/// Handle track end events - plays next track in queue
async fn handle_track_end(_client: LavalinkClient, event: &TrackEnd) {
    use lavalink_rs::model::events::TrackEndReason;
    use serenity::all::CreateMessage;
    use worm::utils::embed;
    
    let should_continue: bool = event.reason.clone().into();
    // Convert lavalink GuildId to serenity GuildId
    let guild_id = serenity::all::GuildId::new(event.guild_id.0);

    // Only process Finished events - ignore Stopped/Replaced/LoadFailed/Cleanup
    let is_finished = matches!(event.reason, TrackEndReason::Finished);
    
    println!(
        "[MUSIC] Track ended in guild {}: {:?} - is_finished: {}, should_continue: {}",
        guild_id.get(), event.reason, is_finished, should_continue
    );

    // Only advance queue when track naturally finishes
    if !is_finished {
        println!("[MUSIC] Track was stopped/replaced/failed, not advancing queue");
        return;
    }

    // Get the global music player to access the queue
    let player = match worm::services::music::player::get_global_player() {
        Some(p) => p,
        None => {
            eprintln!("[MUSIC] Global music player not initialized");
            return;
        }
    };

    // Get the player context
    let player_ctx = match player.get_player_context(guild_id) {
        Some(ctx) => ctx,
        None => {
            println!("[MUSIC] No player context found for guild {}", guild_id.get());
            return;
        }
    };

    // Get text channel for sending embed
    let text_channel = player.get_text_channel(guild_id);
    
    // Get queue info for embed
    let queue = player.get_queue(guild_id);
    let volume = queue.volume;
    let is_looping = queue.is_looping;

    // Get next track from our custom queue
    let next_track = player.next_track(guild_id);

    match next_track {
        Some(track) => {
            println!("[MUSIC] Playing next track: {}", track.track.info.title);
            
            // Update current track in our queue
            player.set_current(guild_id, Some(track.clone()));
            
            // Small delay to prevent race conditions
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            // Play the track
            match player_ctx.play(&track.track).await {
                Ok(info) => {
                    println!("[MUSIC] Successfully started next track, state: {:?}", info.state);
                    
                    // Send Now Playing embed to the text channel
                    if let Some(channel_id) = text_channel {
                        if let Some(http) = worm::services::music::player::get_global_http() {
                            let track_info = &track.track.info;
                            let duration_ms = track_info.length;
                            let duration = format!(
                                "{}:{:02}",
                                duration_ms / 60000,
                                (duration_ms % 60000) / 1000
                            );
                            
                            let now_playing_embed = embed::now_playing(
                                &track_info.title,
                                &track_info.uri.clone().unwrap_or_default(),
                                &track_info.author,
                                &duration,
                                &track.requester_name,
                                volume,
                                is_looping,
                                track_info.artwork_url.as_deref(),
                            );
                            
                            let message = CreateMessage::new().embed(now_playing_embed);
                            if let Err(e) = channel_id.send_message(http.as_ref(), message).await {
                                eprintln!("[MUSIC] Failed to send Now Playing embed: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[MUSIC] Failed to play next track: {}", e);
                    // Clear current since playback failed
                    player.set_current(guild_id, None);
                }
            }
        }
        None => {
            println!("[MUSIC] Queue is empty, no more tracks to play");
            // Clear current track since nothing is playing
            player.set_current(guild_id, None);
        }
    }
}

async fn handle_event(
    ctx: &serenity::all::Context,
    event: &serenity::all::FullEvent,
    data: &Data,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use serenity::all::FullEvent;
    
    if let FullEvent::VoiceStateUpdate { old, new } = event {
        // Only process when someone leaves a channel (old has channel, new doesn't or different channel)
        let old_channel = old.as_ref().and_then(|vs| vs.channel_id);
        let new_channel = new.channel_id;
        
        if old_channel.is_some() && old_channel != new_channel {
            if let Some(guild_id) = new.guild_id {
                if let Some(left_channel_id) = old_channel {
                    let bot_user_id = match worm::services::music::player::get_bot_user_id() {
                        Some(id) => id,
                        None => return Ok(()),
                    };
                    
                    let should_disconnect = {
                        if let Some(guild) = ctx.cache.guild(guild_id) {
                            let bot_in_channel = guild.voice_states.get(&bot_user_id)
                                .and_then(|vs| vs.channel_id)
                                .map(|ch| ch == left_channel_id)
                                .unwrap_or(false);
                            
                            if bot_in_channel {
                                let users_in_channel = guild.voice_states.iter()
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
                                let embed = worm::utils::embed::info(
                                    "Auto Disconnect", 
                                    "Left voice channel because everyone left. 👋"
                                );
                                let message = serenity::all::CreateMessage::new().embed(embed);
                                let _ = channel_id.send_message(&ctx.http, message).await;
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

async fn on_error(
    error: poise::FrameworkError<'_, Data, Box<dyn std::error::Error + Send + Sync>>,
) {
    use poise::serenity_prelude::CreateEmbed;

    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            eprintln!("Error in command '{}': {:?}", ctx.command().name, error);
            let embed = CreateEmbed::new()
                .title("[ERROR] Command Failed")
                .description(format!("{}", error))
                .color(0xE74C3C);
            let _ = ctx.send(poise::CreateReply::default().embed(embed)).await;
        }
        poise::FrameworkError::CommandPanic { payload, ctx, .. } => {
            eprintln!("Command '{}' panicked: {:?}", ctx.command().name, payload);
            let embed = CreateEmbed::new()
                .title("[ERROR] Internal Error")
                .description("An unexpected error occurred. Please try again later.")
                .color(0xE74C3C);
            let _ = ctx.send(poise::CreateReply::default().embed(embed)).await;
        }
        error => {
            eprintln!("Other error: {:?}", error);
        }
    }
}
