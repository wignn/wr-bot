use chrono::{Duration, Utc};
use poise::serenity_prelude::{self as serenity, GetMessages};

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

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    required_permissions = "MANAGE_MESSAGES"
)]
pub async fn purge(
    ctx: Context<'_>,
    #[description = "Jumlah pesan yang akan dihapus (1-100)"]
    #[min = 1]
    #[max = 100]
    amount: u8,
) -> Result<(), Error> {
    let channel_id = ctx.channel_id();
    
    let messages = channel_id
        .messages(&ctx.http(), GetMessages::new().limit(amount))
        .await?;

    if messages.is_empty() {
        ctx.say("Tidak ada pesan untuk dihapus.").await?;
        return Ok(());
    }

    let fourteen_days_ago = Utc::now() - Duration::days(14);
    let mut recent_messages: Vec<serenity::MessageId> = Vec::new();
    let mut old_messages: Vec<serenity::MessageId> = Vec::new();

    for msg in &messages {
        let msg_time = msg.timestamp.to_utc();
        if msg_time > fourteen_days_ago {
            recent_messages.push(msg.id);
        } else {
            old_messages.push(msg.id);
        }
    }

    let total_count = recent_messages.len() + old_messages.len();

    if recent_messages.len() > 1 {
        channel_id
            .delete_messages(&ctx.http(), recent_messages.iter().copied())
            .await?;
    } else if recent_messages.len() == 1 {
        channel_id
            .delete_message(&ctx.http(), recent_messages[0])
            .await?;
    }

    for msg_id in old_messages {
        if let Err(e) = channel_id.delete_message(&ctx.http(), msg_id).await {
            eprintln!("Gagal menghapus pesan lama {}: {}", msg_id, e);
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }

    let embed_msg = ctx
        .send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::default()
                    .title("Berhasil")
                    .description(format!("✅ Berhasil menghapus {} pesan.", total_count))
                    .color(0x57F287),
            ),
        )
        .await?;

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let _ = embed_msg.delete(ctx).await;

    Ok(())
}