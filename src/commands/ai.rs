use crate::config::Config;
use crate::error::BotError;
use crate::utils::ai::Ai;
use poise::CreateReply;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, super::Data, Error>;

#[poise::command(prefix_command, aliases("worm", "wr"))]
pub async fn worm(
    ctx: Context<'_>,
    #[rest] text: String
) -> Result<(), Error> {
    let config = Config::from_env()
        .map_err(|e| BotError::Config(format!("Failed to load config: {}", e)))?;

    let mut ai = Ai::new(config.base_url, config.api_key, config.model_ai, config.prompt);

    let loading_msg = ctx.say("Loading...").await?;

    let response = ai.call_api(text).await.map_err(|e| e.to_string());

    let content = response.unwrap_or_else(|e| format!("Error: {}", e));

    loading_msg.edit(ctx, CreateReply::default().content(content)).await?;

    Ok(())
}