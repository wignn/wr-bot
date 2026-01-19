use crate::repository::ModerationRepository;
use crate::utils::embed;
use poise::serenity_prelude as serenity;
use serenity::{Colour, CreateEmbed, CreateEmbedFooter, Member, Mentionable, Timestamp};
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, super::Data, Error>;

fn parse_duration(input: &str) -> Option<Duration> {
    let input = input.trim().to_lowercase();
    let (num_str, unit) = input.split_at(input.len().saturating_sub(1));
    let num: u64 = num_str.parse().ok()?;

    match unit {
        "s" => Some(Duration::from_secs(num)),
        "m" => Some(Duration::from_secs(num * 60)),
        "h" => Some(Duration::from_secs(num * 3600)),
        "d" => Some(Duration::from_secs(num * 86400)),
        _ => None,
    }
}

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "MODERATE_MEMBERS"
)]
pub async fn warn(
    ctx: Context<'_>,
    #[description = "User to warn"] user: Member,
    #[description = "Reason for warning"] reason: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?;
    let moderator = ctx.author();

    if user.user.id == moderator.id {
        let embed_err = embed::error("Cannot Warn", "You cannot warn yourself!");
        ctx.send(poise::CreateReply::default().embed(embed_err))
            .await?;
        return Ok(());
    }
    if user.user.bot {
        let embed_err = embed::error("Cannot Warn", "You cannot warn bots!");
        ctx.send(poise::CreateReply::default().embed(embed_err))
            .await?;
        return Ok(());
    }

    let db = ctx.data().db.lock().await;
    let conn = db.get_connection();
    ModerationRepository::add_warning(
        conn,
        guild_id.get(),
        user.user.id.get(),
        moderator.id.get(),
        &reason,
    )?;
    let warn_count =
        ModerationRepository::get_warning_count(conn, guild_id.get(), user.user.id.get())?;
    drop(db);

    let embed = CreateEmbed::new()
        .title("⚠️ User Warned")
        .description(format!(
            "**User:** {}\n**Reason:** {}\n**Total Warnings:** {}",
            user.user.mention(),
            reason,
            warn_count
        ))
        .color(Colour::ORANGE)
        .footer(CreateEmbedFooter::new(format!(
            "Warned by {}",
            moderator.name
        )))
        .timestamp(Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "MODERATE_MEMBERS"
)]
pub async fn warnings(
    ctx: Context<'_>,
    #[description = "User to check"] user: Member,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?;

    let db = ctx.data().db.lock().await;
    let conn = db.get_connection();
    let warns = ModerationRepository::get_warnings(conn, guild_id.get(), user.user.id.get())?;
    drop(db);

    if warns.is_empty() {
        let embed = CreateEmbed::new()
            .title("No Warnings")
            .description(format!("{} has no warnings.", user.user.mention()))
            .color(Colour::DARK_GREEN)
            .timestamp(Timestamp::now());
        ctx.send(poise::CreateReply::default().embed(embed)).await?;
        return Ok(());
    }

    let warnings_list: String = warns
        .iter()
        .enumerate()
        .map(|(i, w)| {
            format!(
                "**{}. ID #{}** - {}\n└ <t:{}:R>",
                i + 1,
                w.id,
                w.reason,
                chrono::DateTime::parse_from_rfc3339(&w.created_at)
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let embed = CreateEmbed::new()
        .title(format!("⚠️ Warnings for {}", user.user.name))
        .description(warnings_list)
        .color(Colour::ORANGE)
        .footer(CreateEmbedFooter::new(format!(
            "Total: {} warnings",
            warns.len()
        )))
        .timestamp(Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "MODERATE_MEMBERS"
)]
pub async fn clearwarnings(
    ctx: Context<'_>,
    #[description = "User to clear warnings"] user: Member,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?;

    let db = ctx.data().db.lock().await;
    let conn = db.get_connection();
    let cleared = ModerationRepository::clear_warnings(conn, guild_id.get(), user.user.id.get())?;
    drop(db);

    let embed = CreateEmbed::new()
        .title("Warnings Cleared")
        .description(format!(
            "Cleared **{}** warning(s) for {}",
            cleared,
            user.user.mention()
        ))
        .color(Colour::DARK_GREEN)
        .footer(CreateEmbedFooter::new(format!(
            "Cleared by {}",
            ctx.author().name
        )))
        .timestamp(Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Timeout (mute) a user
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "MODERATE_MEMBERS"
)]
pub async fn mute(
    ctx: Context<'_>,
    #[description = "User to mute"] mut user: Member,
    #[description = "Duration (e.g. 5m, 1h, 7d)"] duration: String,
    #[description = "Reason"] reason: Option<String>,
) -> Result<(), Error> {
    let reason_text = reason.unwrap_or_else(|| "No reason provided".to_string());

    let dur = parse_duration(&duration).ok_or("Invalid duration format. Use: 5m, 1h, 7d")?;

    if dur.as_secs() > 28 * 24 * 3600 {
        let embed_err = embed::error("Invalid Duration", "Maximum timeout duration is 28 days.");
        ctx.send(poise::CreateReply::default().embed(embed_err))
            .await?;
        return Ok(());
    }
    let timeout_until = serenity::Timestamp::from_unix_timestamp(
        chrono::Utc::now().timestamp() + dur.as_secs() as i64,
    )?;

    user.disable_communication_until_datetime(&ctx.http(), timeout_until)
        .await?;

    let embed = CreateEmbed::new()
        .title("User Muted")
        .description(format!(
            "**User:** {}\n**Duration:** {}\n**Reason:** {}",
            user.user.mention(),
            duration,
            reason_text
        ))
        .color(Colour::RED)
        .footer(CreateEmbedFooter::new(format!(
            "Muted by {}",
            ctx.author().name
        )))
        .timestamp(Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "MODERATE_MEMBERS"
)]
pub async fn unmute(
    ctx: Context<'_>,
    #[description = "User to unmute"] mut user: Member,
) -> Result<(), Error> {
    user.enable_communication(&ctx.http()).await?;

    let embed = CreateEmbed::new()
        .title("User Unmuted")
        .description(format!("{} can now speak again.", user.user.mention()))
        .color(Colour::DARK_GREEN)
        .footer(CreateEmbedFooter::new(format!(
            "Unmuted by {}",
            ctx.author().name
        )))
        .timestamp(Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "KICK_MEMBERS"
)]
pub async fn kick(
    ctx: Context<'_>,
    #[description = "User to kick"] user: Member,
    #[description = "Reason"] reason: Option<String>,
) -> Result<(), Error> {
    let reason_text = reason.unwrap_or_else(|| "No reason provided".to_string());

    user.kick_with_reason(&ctx.http(), &reason_text).await?;

    let embed = CreateEmbed::new()
        .title("User Kicked")
        .description(format!(
            "**User:** {} ({})\n**Reason:** {}",
            user.user.name, user.user.id, reason_text
        ))
        .color(Colour::ORANGE)
        .footer(CreateEmbedFooter::new(format!(
            "Kicked by {}",
            ctx.author().name
        )))
        .timestamp(Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "BAN_MEMBERS"
)]
pub async fn ban(
    ctx: Context<'_>,
    #[description = "User to ban"] user: Member,
    #[description = "Reason"] reason: Option<String>,
    #[description = "Delete message days (0-7)"] delete_days: Option<u8>,
) -> Result<(), Error> {
    let reason_text = reason.unwrap_or_else(|| "No reason provided".to_string());
    let del_days = delete_days.unwrap_or(0).min(7);

    user.ban_with_reason(&ctx.http(), del_days, &reason_text)
        .await?;

    let embed = CreateEmbed::new()
        .title("🔨 User Banned")
        .description(format!(
            "**User:** {} ({})\n**Reason:** {}\n**Messages deleted:** {} days",
            user.user.name, user.user.id, reason_text, del_days
        ))
        .color(Colour::DARK_RED)
        .footer(CreateEmbedFooter::new(format!(
            "Banned by {}",
            ctx.author().name
        )))
        .timestamp(Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "BAN_MEMBERS"
)]
pub async fn unban(
    ctx: Context<'_>,
    #[description = "User ID to unban"] user_id: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?;

    let uid: u64 = user_id.parse().map_err(|_| "Invalid user ID")?;
    let user_id_parsed = serenity::UserId::new(uid);

    guild_id.unban(&ctx.http(), user_id_parsed).await?;

    let embed = CreateEmbed::new()
        .title("User Unbanned")
        .description(format!("User ID `{}` has been unbanned.", uid))
        .color(Colour::DARK_GREEN)
        .footer(CreateEmbedFooter::new(format!(
            "Unbanned by {}",
            ctx.author().name
        )))
        .timestamp(Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "ADMINISTRATOR"
)]
pub async fn autorole_set(
    ctx: Context<'_>,
    #[description = "Role to assign to new members"] role: serenity::Role,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?;

    let db = ctx.data().db.lock().await;
    let conn = db.get_connection();
    ModerationRepository::set_auto_role(conn, guild_id.get(), role.id.get())?;
    drop(db);

    let embed = CreateEmbed::new()
        .title("Auto-Role Set")
        .description(format!(
            "New members will automatically receive the {} role.",
            role.mention()
        ))
        .color(Colour::DARK_GREEN)
        .timestamp(Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "ADMINISTRATOR"
)]
pub async fn autorole_disable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?;

    let db = ctx.data().db.lock().await;
    let conn = db.get_connection();
    ModerationRepository::disable_auto_role(conn, guild_id.get())?;
    drop(db);

    let embed = CreateEmbed::new()
        .title("Auto-Role Disabled")
        .description("New members will no longer receive an automatic role.")
        .color(Colour::RED)
        .timestamp(Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "ADMINISTRATOR"
)]
pub async fn log_setup(
    ctx: Context<'_>,
    #[description = "Channel for logging"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?;

    let db = ctx.data().db.lock().await;
    let conn = db.get_connection();
    ModerationRepository::set_log_channel(conn, guild_id.get(), channel.id.get())?;
    drop(db);

    let embed = CreateEmbed::new()
        .title("Logging Enabled")
        .description(format!(
            "Member join/leave events will be logged to {}.",
            channel.mention()
        ))
        .color(Colour::DARK_GREEN)
        .timestamp(Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "ADMINISTRATOR"
)]
pub async fn log_disable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?;

    let db = ctx.data().db.lock().await;
    let conn = db.get_connection();
    ModerationRepository::disable_logging(conn, guild_id.get())?;
    drop(db);

    let embed = CreateEmbed::new()
        .title("Logging Disabled")
        .description("Member join/leave logging has been disabled.")
        .color(Colour::RED)
        .timestamp(Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
