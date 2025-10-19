type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, super::Data, Error>;

#[poise::command(slash_command, prefix_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
    Ok(())
}

