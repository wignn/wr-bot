use crate::repository::RedeemRepository;
use poise::serenity_prelude as serenity;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, super::Data, Error>;

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

    let game_lower = game.to_lowercase();
    if !["wuwa", "genshin", "hsr", "zzz"].contains(&game_lower.as_str()) {
        ctx.say("Invalid game! Available games: `wuwa`, `genshin`, `hsr`, `zzz`")
            .await?;
        return Ok(());
    }

    let pool = ctx.data().db.as_ref();
    RedeemRepository::insert_server(pool, guild_id, channel_id, &game_lower).await?;

    let embed = serenity::CreateEmbed::default()
        .title("✅ Redeem Setup Successful")
        .description(format!(
            "Redeem code notifications for **{}** will be sent to <#{}>\n\n\
            The bot will automatically notify this channel when new codes are detected.",
            game_lower.to_uppercase(),
            channel_id
        ))
        .color(serenity::Colour::DARK_GREEN)
        .footer(serenity::CreateEmbedFooter::new(
            "Notifications are now active",
        ))
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
pub async fn redeem_disable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?.get();

    let pool = ctx.data().db.as_ref();
    RedeemRepository::disable_server(pool, guild_id).await?;

    let embed = serenity::CreateEmbed::default()
        .title("🔕 Notifications Disabled")
        .description(
            "Redeem code notifications have been disabled for this server.\n\n\
                     Use `/redeem_enable` to turn them back on.",
        )
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

    let pool = ctx.data().db.as_ref();
    RedeemRepository::enable_server(pool, guild_id).await?;

    let embed = serenity::CreateEmbed::default()
        .title("🔔 Notifications Enabled")
        .description(
            "Redeem code notifications have been enabled for this server.\n\n\
                     You will receive alerts when new codes are detected.",
        )
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
    let game_lower = game.to_lowercase();

    if !["wuwa", "genshin", "hsr", "zzz"].contains(&game_lower.as_str()) {
        ctx.say("Invalid game! Available games: `wuwa`, `genshin`, `hsr`, `zzz`")
            .await?;
        return Ok(());
    }

    let pool = ctx.data().db.as_ref();
    let codes = RedeemRepository::get_codes_by_game(pool, &game_lower).await?;

    if codes.is_empty() {
        let embed = serenity::CreateEmbed::default()
            .title(format!("📭 No Codes Available"))
            .description(format!(
                "No redeem codes found for **{}**.\n\n\
                Codes will appear here once they are detected by the bot.",
                game_lower.to_uppercase()
            ))
            .color(serenity::Colour::ORANGE)
            .timestamp(serenity::Timestamp::now());

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
        return Ok(());
    }

    let codes_list = codes
        .iter()
        .enumerate()
        .map(|(i, code_data)| {
            let rewards = code_data
                .rewards
                .as_ref()
                .map(|r| format!("\n└ 🎁 {}", r))
                .unwrap_or_default();
            format!("{}. `{}`{}", i + 1, code_data.code, rewards)
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let redeem_link = match game_lower.as_str() {
        "genshin" => "https://genshin.hoyoverse.com/en/gift",
        "hsr" => "https://hsr.hoyoverse.com/gift",
        "zzz" => "https://zenless.hoyoverse.com/redemption",
        "wuwa" => "https://wutheringwaves.kurogames.com/en/main/gift",
        _ => "",
    };

    let embed = serenity::CreateEmbed::default()
        .title(format!("🎮 {} Redeem Codes", game_lower.to_uppercase()))
        .description(format!(
            "{}\n\n\
            **How to Redeem:**\n\
            Visit [Redemption Page]({}) and enter the codes above.",
            codes_list, redeem_link
        ))
        .color(serenity::Colour::BLUE)
        .footer(serenity::CreateEmbedFooter::new(format!(
            "Total: {} codes | Last 10 codes shown",
            codes.len()
        )))
        .timestamp(serenity::Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
