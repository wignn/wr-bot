type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, super::Data, Error>;

#[poise::command(prefix_command, guild_only, owners_only)]
pub async fn everyone(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("@everyone").await?;
    Ok(())
}