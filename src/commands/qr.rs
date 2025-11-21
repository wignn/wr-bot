type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, super::Data, Error>;
use poise::serenity_prelude::*;

#[poise::command(prefix_command, guild_only)]
pub async fn qr(
    ctx: Context<'_>,
    #[rest] text: String
) -> Result<(), Error> {

    if text.trim().is_empty() {
        ctx.say("Masukkan teks atau link untuk dijadikan QR!").await?;
        return Ok(());
    }

    let url = format!(
        "https://api.qrserver.com/v1/create-qr-code/?size=500x500&data={}",
        urlencoding::encode(&text)
    );

    let client = reqwest::Client::new();
    let res = client.get(&url).send().await?;

    if !res.status().is_success() {
        ctx.say("Gagal membuat QR code.").await?;
        return Ok(());
    }

    let bytes = res.bytes().await?;

    ctx.send(
        poise::CreateReply::default()
            .content(format!("QR Code untuk: `{}`", text))
            .attachment(CreateAttachment::bytes(bytes.to_vec(), "qrcode.png"))
    )
        .await?;

    Ok(())
}