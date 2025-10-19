use crate::utils::sys::SysInfo;
use poise::serenity_prelude as serenity;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, super::Data, Error>;

#[poise::command(slash_command, prefix_command, owners_only)]
pub async fn sys(ctx: Context<'_>) -> Result<(), Error> {
    let sistem = SysInfo::new();


    ctx.defer_ephemeral().await?;


    let embed = serenity::CreateEmbed::default()
        .title("Sys")
        .field("OS", &sistem.os, false)
        .field("CPU", &sistem.cpu, false)
        .field("Memory", &sistem.memory, false)
        .color(serenity::Colour::BLUE)
        .timestamp(serenity::Timestamp::now());

    ctx.send(
        poise::CreateReply::default()
            .embed(embed)
            .ephemeral(true)
    ).await?;

    Ok(())
}