use crate::commands::Data;
use poise::serenity_prelude::CreateEmbed;

type Error = Box<dyn std::error::Error + Send + Sync>;

/// Handle framework errors
pub async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            eprintln!("Error in command '{}': {:?}", ctx.command().name, error);
            let embed = CreateEmbed::new()
                .title("[ERROR] Command Failed")
                .description(format!("{}", error))
                .color(0xE74C3C);
            let _ = ctx.send(poise::CreateReply::default().embed(embed)).await;
        }
        poise::FrameworkError::CommandPanic { payload, ctx, .. } => {
            eprintln!("Command '{}' panicked: {:?}", ctx.command().name, payload);
            let embed = CreateEmbed::new()
                .title("[ERROR] Internal Error")
                .description("An unexpected error occurred. Please try again later.")
                .color(0xE74C3C);
            let _ = ctx.send(poise::CreateReply::default().embed(embed)).await;
        }
        error => {
            eprintln!("Other error: {:?}", error);
        }
    }
}
