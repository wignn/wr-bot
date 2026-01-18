use crate::repository::ReminderRepository;
use poise::serenity_prelude as serenity;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, super::Data, Error>;

/// Setup reminder untuk server
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "ADMINISTRATOR"
)]
pub async fn reminder_setup(
    ctx: Context<'_>,
    #[description = "Channel untuk notifikasi"] channel: serenity::GuildChannel,
    #[description = "Waktu dalam format HHMM UTC (contoh: 1700)"] time: i64,
    #[description = "Pesan reminder"] message: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let guild_id = ctx.guild_id().ok_or("Must be used in a guild")?.get();
    let channel_id = channel.id.get();
    let user_id: u64 = ctx.author().id.get();

    let db = ctx.data().db.lock().await;
    let conn = db.get_connection();

    ReminderRepository::insert_reminder(
        conn,
        user_id,
        guild_id,
        channel_id,
        message.as_str(),
        time,
    )?;

    let embed = serenity::CreateEmbed::default()
        .title("⏰ Reminder Berhasil Diset")
        .description(format!(
            "Reminder telah diaktifkan!\n\n\
            📢 Channel: <#{}>\n\
            🕐 Waktu: {} UTC\n\
            📝 Pesan: {}",
            channel_id, time, message
        ))
        .color(serenity::Colour::DARK_GREEN)
        .timestamp(serenity::Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Lihat daftar reminder kamu
#[poise::command(slash_command, prefix_command)]
pub async fn reminder_list(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();

    let db = ctx.data().db.lock().await;
    let conn = db.get_connection();
    let reminders = ReminderRepository::get_user_reminders(conn, user_id)?;
    drop(db);

    if reminders.is_empty() {
        let embed = serenity::CreateEmbed::default()
            .title("📭 Tidak Ada Reminder")
            .description("Kamu belum memiliki reminder aktif.")
            .color(serenity::Colour::ORANGE)
            .timestamp(serenity::Timestamp::now());

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
        return Ok(());
    }

    let reminder_list = reminders
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "{}. **ID {}** - {} UTC\n   └ {}",
                i + 1,
                r.id,
                r.remind_at,
                r.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let embed = serenity::CreateEmbed::default()
        .title("⏰ Reminder Aktif")
        .description(reminder_list)
        .color(serenity::Colour::BLUE)
        .footer(serenity::CreateEmbedFooter::new(format!(
            "Total: {} reminder",
            reminders.len()
        )))
        .timestamp(serenity::Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Hapus reminder berdasarkan ID
#[poise::command(slash_command, prefix_command)]
pub async fn reminder_delete(
    ctx: Context<'_>,
    #[description = "ID reminder yang akan dihapus"] reminder_id: u64,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get();

    let db = ctx.data().db.lock().await;
    let conn = db.get_connection();
    let deleted = ReminderRepository::delete_reminder(conn, reminder_id, user_id)?;
    drop(db);

    if deleted {
        let embed = serenity::CreateEmbed::default()
            .title("✅ Reminder Dihapus")
            .description(format!(
                "Reminder dengan ID {} berhasil dihapus.",
                reminder_id
            ))
            .color(serenity::Colour::DARK_GREEN)
            .timestamp(serenity::Timestamp::now());

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    } else {
        let embed = serenity::CreateEmbed::default()
            .title("❌ Gagal Menghapus")
            .description("Reminder tidak ditemukan atau bukan milik kamu.")
            .color(serenity::Colour::RED)
            .timestamp(serenity::Timestamp::now());

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    }

    Ok(())
}
