use dotenvy::dotenv;

use worm::commands::{ping, general, admin, ai, Data};
use std::env;
use std::collections::HashSet;
use serenity::all::GatewayIntents;
use worm::config::Config;
use worm::error::BotError;
use poise::serenity_prelude::UserId;

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

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                ping::ping(),
                admin::everyone(),
                general::ping(),
                general::say(),
                ai::worm()
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".into()),
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", _ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data { owners })
            })
        })
        .build();

    let mut client = serenity::Client::builder(&config.token, intents)
        .framework(framework)
        .await
        .map_err(|e| BotError::Client(format!("Failed to create client: {}", e)))?;

    client
        .start()
        .await
        .map_err(|e| BotError::Client(format!("Failed to initialize client: {}", e)))?;

    Ok(())
}