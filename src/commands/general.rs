type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, super::Data, Error>;

#[poise::command(prefix_command, guild_only)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong?").await?;
    Ok(())
}

#[poise::command(prefix_command, aliases("repeat", "echo"))]
pub async fn say(
    ctx: Context<'_>,
    #[rest] text: String,
) -> Result<(), Error> {
    ctx.say(text).await?;
    Ok(())
}


