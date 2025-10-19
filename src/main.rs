use dotenvy::dotenv;
use worm::commands::{ping, general, admin, ai, redeem, Data, sys};
use worm::repository::{create_pool};
use std::env;
use std::collections::HashSet;
use serenity::all::{ActivityData, GatewayIntents, OnlineStatus};
use worm::error::BotError;
use poise::serenity_prelude::UserId;
use worm::config::Config;
use worm::services::genshin_redeem_checker::start_code_checker;

#[tokio::main]
async fn main() -> Result<(), BotError> {
    dotenv().ok();

    let config = Config::from_env()
        .map_err(|e| BotError::Config(format!("Failed to load config: {}", e)))?;

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS;

    let owner_id = env::var("CLIENT_ID")
        .unwrap_or_else(|_| "YOUR_DISCORD_USER_ID".to_string())
        .parse::<u64>()
        .expect("OWNER_ID must be a valid u64");

    let mut owners = HashSet::new();
    owners.insert(UserId::new(owner_id));

    let db = create_pool("redeem_bot.db")
        .map_err(|e| BotError::Config(format!("Failed to initialize database: {}", e)))?;

    let owners_clone = owners.clone();
    let db_for_checker = db.clone();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                ping::ping(),
                admin::everyone(),
                general::ping(),
                general::say(),
                ai::worm(),
                sys::sys(),
                redeem::redeem_setup(),
                redeem::redeem_codes(),
                redeem::redeem_disable(),
                redeem::redeem_enable(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".into()),
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(move |_ctx, _ready, _framework| {
            let inner_db = db.clone();
            let owners_inner = owners_clone.clone();
            Box::pin(async move {
                println!("Logged in as {}", _ready.user.name);
                Ok(Data { owners: owners_inner, db: inner_db})
            })
        })
        .build();

    let mut client = serenity::Client::builder(&config.token, intents)
        .framework(framework)
        .await
        .map_err(|e| BotError::Client(format!("Failed to create client: {}", e)))?;

    let shard_manager = client.shard_manager.clone();
    let http = client.http.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        let activities = vec![
            ActivityData::playing("YouTube"),
            ActivityData::watching("Discord"),
            ActivityData::listening("Music"),
        ];
        let mut idx = 0;
        loop {
            interval.tick().await;
            let runners = shard_manager.runners.lock().await;
            for (_, runner) in runners.iter() {
                runner.runner_tx.set_presence(
                    Some(activities[idx].clone()),
                    OnlineStatus::DoNotDisturb,
                );
            }
            idx = (idx + 1) % activities.len();
        }
    });

    start_code_checker(db_for_checker, http).await;
    println!("Code checker service started!");

    client
        .start()
        .await
        .map_err(|e| BotError::Client(format!("Failed to initialize client: {}", e)))?;

    Ok(())
}
