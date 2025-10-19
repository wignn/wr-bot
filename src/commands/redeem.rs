use poise::serenity_prelude as serenity;
use crate::repository::{DbPool, RedeemRepository};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, super::Data, Error>;

pub struct Data {
    pub db: DbPool,
}

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "ADMINISTRATOR"
)]
pub async fn redeem_setup(
    ctx: Context<'_>,
    #[description = "Channel for notifications"] channel: serenity::GuildChannel,
    #[description = "Game (wuwa/genshin/hsr/zzz)"] game: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?.get();
    let channel_id = channel.id.get();

    let db = ctx.data().db.lock().await;
    let conn = db.get_connection();
    RedeemRepository::insert_server(conn, guild_id, channel_id, &game)?;

    let embed = serenity::CreateEmbed::default()
        .title("Redeem Setup Successful")
        .description(format!(
            "Redeem code notifications for **{}** will be sent to <#{}>",
            game.to_uppercase(),
            channel_id
        ))
        .color(serenity::Colour::DARK_GREEN)
        .timestamp(serenity::Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    required_permissions = "ADMINISTRATOR"
)]
pub async fn redeem_disable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?.get();

    let db = ctx.data().db.lock().await;
    let conn = db.get_connection();
    RedeemRepository::disable_server(conn, guild_id)?;

    let embed = serenity::CreateEmbed::default()
        .title("Notifications Disabled")
        .description("Redeem code notifications have been disabled for this server")
        .color(serenity::Colour::RED)
        .timestamp(serenity::Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "ADMINISTRATOR"
)]
pub async fn redeem_enable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?.get();

    let db = ctx.data().db.lock().await;
    let conn = db.get_connection();
    RedeemRepository::enable_server(conn, guild_id)?;

    let embed = serenity::CreateEmbed::default()
        .title("Notifications Enabled")
        .description("Redeem code notifications have been enabled for this server")
        .color(serenity::Colour::DARK_GREEN)
        .timestamp(serenity::Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn redeem_codes(
    ctx: Context<'_>,
    #[description = "Game (wuwa/genshin/hsr/zzz)"] game: String,
) -> Result<(), Error> {
    let db = ctx.data().db.lock().await;
    let conn = db.get_connection();
    let codes = RedeemRepository::get_codes_by_game(conn, &game)?;

    if codes.is_empty() {
        ctx.say(format!("No redeem codes available for **{}**", game.to_uppercase()))
            .await?;
        return Ok(());
    }

    let codes_list = codes
        .iter()
        .map(|code_data| {
            let desc = code_data.description.as_ref()
                .map(|d| format!(" - {}", d))
                .unwrap_or_default();
            format!("`{}`{}", code_data.code, desc)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let embed = serenity::CreateEmbed::default()
        .title(format!("{} Redeem Codes", game.to_uppercase()))
        .description(codes_list)
        .color(serenity::Colour::BLUE)
        .footer(serenity::CreateEmbedFooter::new(format!("Total: {} codes", codes.len())))
        .timestamp(serenity::Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
