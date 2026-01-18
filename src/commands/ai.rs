use crate::config::Config;
use crate::error::BotError;
use crate::services::ai::Ai;
use poise::CreateReply;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, super::Data, Error>;

fn split_into_chunks(s: &str, max: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let len = s.len();
    while start < len {
        let mut end = usize::min(start + max, len);
        while end > start && !s.is_char_boundary(end) {
            end -= 1;
        }
        if end == start {
            end = usize::min(start + max, len);
        }
        chunks.push(s[start..end].to_string());
        start = end;
    }
    chunks
}

/// Chat dengan AI WormGPT
#[poise::command(prefix_command, slash_command, aliases("worm", "wr"))]
pub async fn worm(
    ctx: Context<'_>,
    #[rest]
    #[description = "Pertanyaan untuk AI"]
    text: String,
) -> Result<(), Error> {
    let config = Config::from_env()
        .map_err(|e| BotError::Config(format!("Failed to load config: {}", e)))?;

    // Check if AI is enabled
    let api_key = match &config.api_key {
        Some(key) => key.clone(),
        None => {
            ctx.say("❌ Fitur AI belum dikonfigurasi. Harap set `API_KEY` di environment.")
                .await?;
            return Ok(());
        }
    };

    let mut ai = Ai::new(config.base_url, api_key, config.model_ai, config.prompt);

    let loading_msg = ctx.say("⏳ Memproses...").await?;

    let response = ai.call_api(text).await.map_err(|e| e.to_string());

    let content = response.unwrap_or_else(|e| format!("❌ Error: {}", e));

    const DISCORD_MAX_LEN: usize = 2000;
    const CHUNK_MAX: usize = 1900;

    if content.len() <= DISCORD_MAX_LEN {
        loading_msg
            .edit(ctx, CreateReply::default().content(content))
            .await?;
    } else {
        loading_msg
            .edit(
                ctx,
                CreateReply::default()
                    .content("📜 Response terlalu panjang, mengirim dalam beberapa pesan..."),
            )
            .await?;
        let chunks = split_into_chunks(&content, CHUNK_MAX);
        for chunk in chunks {
            ctx.say(chunk).await?;
        }
    }

    Ok(())
}
