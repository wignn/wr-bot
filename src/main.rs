use dotenvy::dotenv;
use lavalink_rs::client::LavalinkClient;
use lavalink_rs::model::events::Events;
use lavalink_rs::node::NodeBuilder;
use poise::serenity_prelude::UserId;
use serenity::all::{ActivityData, GatewayIntents, OnlineStatus};
use songbird::SerenityInit;
use std::collections::HashSet;
use std::env;
use worm::commands::{admin, ai, general, moderation, music, ping, redeem, sys, Data};
use worm::config::Config;
use worm::error::BotError;
use worm::handlers::{handle_event, handle_track_end, on_error};
use worm::repository::create_pool;
use worm::services::genshin_redeem_checker::start_code_checker;
use worm::services::music::MusicPlayer;

#[tokio::main]
async fn main() -> Result<(), BotError> {
    dotenv().ok();

    println!("Starting WR Bot...");

    let config = Config::from_env()
        .map_err(|e| BotError::Config(format!("Failed to load config: {}", e)))?;

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_MEMBERS;

    let owner_id = env::var("CLIENT_ID")
        .unwrap_or_else(|_| "0".to_string())
        .parse::<u64>()
        .expect("CLIENT_ID must be a valid u64");

    let mut owners = HashSet::new();
    owners.insert(UserId::new(owner_id));

    let db = create_pool("redeem_bot.db")
        .map_err(|e| BotError::Config(format!("Failed to initialize database: {}", e)))?;

    println!("[OK] Database initialized successfully");

    if config.is_ai_enabled() {
        println!("[OK] AI features enabled");
    } else {
        println!("[WARN] AI features disabled (no API_KEY configured)");
    }

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

    let songbird = songbird::Songbird::serenity();
    let songbird_for_data = songbird.clone();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                // General commands
                ping::ping(),
                general::ping(),
                general::say(),
                general::purge(),
                // Admin commands
                admin::everyone(),
                // AI commands
                ai::worm(),
                // System commands
                sys::sys(),
                // Redeem commands
                redeem::redeem_setup(),
                redeem::redeem_codes(),
                redeem::redeem_disable(),
                redeem::redeem_enable(),
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
                music::autoplay(),
                // Moderation commands
                moderation::warn(),
                moderation::warnings(),
                moderation::clearwarnings(),
                moderation::mute(),
                moderation::unmute(),
                moderation::kick(),
                moderation::ban(),
                moderation::unban(),
                // Auto-role commands
                moderation::autorole_set(),
                moderation::autorole_disable(),
                // Logging commands
                moderation::log_setup(),
                moderation::log_disable(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".into()),
                ..Default::default()
            },
            on_error: |error| Box::pin(on_error(error)),
            event_handler: |ctx, event, _framework, data| Box::pin(handle_event(ctx, event, data)),
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

                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                println!("[OK] Slash commands registered globally");

                worm::services::music::player::init_global_http(http_clone);
                worm::services::music::player::init_bot_user_id(ready.user.id);

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

                let youtube_search = worm::services::youtube::YouTubeSearch::new();
                if let Some(ref yt) = youtube_search {
                    worm::services::youtube::init_global_youtube(yt.clone());
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

    let mut client = serenity::Client::builder(&config.token, intents)
        .framework(framework)
        .register_songbird_with(songbird.clone())
        .await
        .map_err(|e| BotError::Client(format!("Failed to create client: {}", e)))?;

    let shard_manager = client.shard_manager.clone();
    let http = client.http.clone();
    let cache = client.cache.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        let mut idx = 0;
        loop {
            interval.tick().await;

            let total_users: u64 = cache
                .guilds()
                .iter()
                .filter_map(|guild_id| cache.guild(*guild_id))
                .map(|g| g.member_count)
                .sum();

            let activities = vec![ActivityData::custom(format!("With {} users", total_users))];

            let runners = shard_manager.runners.lock().await;
            for (_, runner) in runners.iter() {
                runner.runner_tx.set_presence(
                    Some(activities[idx % activities.len()].clone()),
                    OnlineStatus::Online,
                );
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
