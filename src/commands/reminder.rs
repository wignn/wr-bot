use crate::repository::{DbPool, ReminderRepository};
use poise::serenity_prelude as serenity;
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, super::Data, Error>;


pub struct Data {
    pub db: DbPool
}

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "ADMINISTRATOR"
)]
pub async fn reminder_setup(
    ctx: Context<'_>,
    #[description="Channel for notification"] channel: serenity::GuildChannel,
    #[description="Time 1700 utc"] time: i64,
    #[description="message"] message: String
)->Result<(), Error> {
    ctx.defer().await?;

    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?.get();
    let channel_id = channel.id.get();
    let user_id: u64 = ctx.author().id.get();


    let db = ctx.data().db.lock().await;

    let conn = db.get_connection();

    ReminderRepository::insert_reminder(conn, user_id, guild_id, channel_id, message.as_str(), time)?;

    let embed = serenity::CreateEmbed::default()
        .title("Reminder Enable")
        .description("Reminder notifications have been enabled for this server")
        .color(serenity::Colour::DARK_GREEN)
        .timestamp(serenity::Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

