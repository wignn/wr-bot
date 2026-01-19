use crate::config::Config;
use crate::error::BotError;
use crate::services::ai::Ai;
use crate::services::gemini::GeminiService;
use poise::serenity_prelude::{CreateEmbed, CreateEmbedFooter};
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

async fn send_ai_response(ctx: Context<'_>, content: String) -> Result<(), Error> {
    const DISCORD_MAX_LEN: usize = 2000;
    const CHUNK_MAX: usize = 1900;

    if content.len() <= DISCORD_MAX_LEN {
        ctx.say(&content).await?;
    } else {
        ctx.say("Response terlalu panjang, mengirim dalam beberapa pesan...").await?;
        let chunks = split_into_chunks(&content, CHUNK_MAX);
        for chunk in chunks {
            ctx.say(chunk).await?;
        }
    }
    Ok(())
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


///Gemini AI
#[poise::command(prefix_command, slash_command, aliases("gem", "gm"))]
pub async fn gemini(
    ctx: Context<'_>,
    #[rest]
    #[description = "Pertanyaan untuk Gemini AI"]
    text: String,
) -> Result<(), Error> {
    let config = Config::from_env()
        .map_err(|e| BotError::Config(format!("Failed to load config: {}", e)))?;

    if config.gemini_api_key == "api_key" {
        ctx.say("Fitur Gemini AI belum dikonfigurasi. Harap set `GEMINI_API_KEY` di environment.")
            .await?;
        return Ok(());
    }

    let gemini = GeminiService::new(
        config.gemini_api_key,
        None,
        config.prompt,
    );

    ctx.defer().await?;

    match gemini.generate(&text).await {
        Ok(response) => {
            send_ai_response(ctx, response).await?;
        }
        Err(e) => {
            ctx.say(format!("❌ Error: {}", e)).await?;
        }
    }

    Ok(())
}

/// Chat dengan Gemini dengan memory (ingat percakapan sebelumnya)
#[poise::command(prefix_command, slash_command, aliases("gchat", "gc"))]
pub async fn gemini_chat(
    ctx: Context<'_>,
    #[rest]
    #[description = "Pesan untuk Gemini AI"]
    text: String,
) -> Result<(), Error> {
    let config = Config::from_env()
        .map_err(|e| BotError::Config(format!("Failed to load config: {}", e)))?;

    if config.gemini_api_key == "api_key" {
        ctx.say("❌ Fitur Gemini AI belum dikonfigurasi. Harap set `GEMINI_API_KEY` di environment.")
            .await?;
        return Ok(());
    }

    let gemini = GeminiService::new(
        config.gemini_api_key,
        None,
        config.prompt,
    );

    ctx.defer().await?;

    let user_id = ctx.author().id.to_string();
    
    match gemini.chat(&user_id, &text).await {
        Ok(response) => {
            send_ai_response(ctx, response).await?;
        }
        Err(e) => {
            ctx.say(format!("❌ Error: {}", e)).await?;
        }
    }

    Ok(())
}

/// Hapus history chat Gemini
#[poise::command(prefix_command, slash_command, aliases("gclear"))]
pub async fn gemini_clear(ctx: Context<'_>) -> Result<(), Error> {
    let config = Config::from_env()
        .map_err(|e| BotError::Config(format!("Failed to load config: {}", e)))?;

    if config.gemini_api_key == "api_key" {
        ctx.say("❌ Fitur Gemini AI belum dikonfigurasi.").await?;
        return Ok(());
    }

    let gemini = GeminiService::new(
        config.gemini_api_key,
        None,
        config.prompt,
    );

    let user_id = ctx.author().id.to_string();
    gemini.clear_history(&user_id).await;

    ctx.say("✅ History chat kamu telah dihapus!").await?;
    Ok(())
}

/// Analisis gambar dengan Gemini Vision
#[poise::command(prefix_command, slash_command, aliases("gvision", "gv"))]
pub async fn gemini_vision(
    ctx: Context<'_>,
    #[description = "URL gambar untuk dianalisis"]
    image_url: String,
    #[rest]
    #[description = "Pertanyaan tentang gambar (opsional)"]
    prompt: Option<String>,
) -> Result<(), Error> {
    let config = Config::from_env()
        .map_err(|e| BotError::Config(format!("Failed to load config: {}", e)))?;

    if config.gemini_api_key == "api_key" {
        ctx.say("❌ Fitur Gemini AI belum dikonfigurasi. Harap set `GEMINI_API_KEY` di environment.")
            .await?;
        return Ok(());
    }

    let gemini = GeminiService::new(
        config.gemini_api_key,
        None,
        config.prompt,
    );

    ctx.defer().await?;

    match gemini.analyze_image(&image_url, prompt.as_deref()).await {
        Ok(response) => {
            let embed = CreateEmbed::default()
                .title("🖼️ Analisis Gambar")
                .thumbnail(&image_url)
                .description(&response)
                .color(0x4285F4)
                .footer(CreateEmbedFooter::new("Powered by Gemini Vision"));

            if response.len() > 4000 {
                send_ai_response(ctx, response).await?;
            } else {
                ctx.send(CreateReply::default().embed(embed)).await?;
            }
        }
        Err(e) => {
            ctx.say(format!("Error: {}", e)).await?;
        }
    }

    Ok(())
}

/// Analisis chart trading dengan Gemini Vision (attach gambar langsung atau reply ke gambar)
#[poise::command(prefix_command, aliases("market", "chart", "ta"))]
pub async fn analisa(
    ctx: Context<'_>,
    #[description = "Symbol/Pair (contoh: BTCUSDT, EURUSD, XAUUSD)"]
    symbol: Option<String>,
    #[description = "Timeframe (contoh: 1H, 4H, 1D, 1W)"]
    timeframe: Option<String>,
    #[rest]
    #[description = "Konteks tambahan (opsional)"]
    context: Option<String>,
) -> Result<(), Error> {
    let config = Config::from_env()
        .map_err(|e| BotError::Config(format!("Failed to load config: {}", e)))?;

    if config.gemini_api_key == "api_key" {
        ctx.say("❌ Fitur Gemini AI belum dikonfigurasi. Harap set `GEMINI_API_KEY` di environment.")
            .await?;
        return Ok(());
    }

    // Get image URL from attachment or replied message
    let image_url = match ctx {
        poise::Context::Prefix(prefix_ctx) => {
            // Check attachments in current message
            if let Some(attachment) = prefix_ctx.msg.attachments.first() {
                if attachment.content_type.as_ref().map(|ct| ct.starts_with("image/")).unwrap_or(false) {
                    Some(attachment.url.clone())
                } else {
                    None
                }
            }
            // Check if replying to a message with image
            else if let Some(ref replied) = prefix_ctx.msg.referenced_message {
                replied.attachments.first()
                    .filter(|a| a.content_type.as_ref().map(|ct| ct.starts_with("image/")).unwrap_or(false))
                    .map(|a| a.url.clone())
                    .or_else(|| {
                        // Check embeds for image
                        replied.embeds.first()
                            .and_then(|e| e.image.as_ref().map(|i| i.url.clone()))
                    })
            } else {
                None
            }
        }
        poise::Context::Application(_) => None,
    };

    let image_url = match image_url {
        Some(url) => url,
        None => {
            ctx.say("❌ Tidak ada gambar ditemukan!\n\n**Cara pakai:**\n• Attach gambar + `!analisa [symbol] [timeframe]`\n• Reply ke pesan dengan gambar + `!analisa [symbol] [timeframe]`").await?;
            return Ok(());
        }
    };

    let gemini = GeminiService::new(
        config.gemini_api_key,
        None,
        config.gemini_prompt,
    );

    let loading_msg = ctx.say("📊 Menganalisis chart... Mohon tunggu sebentar.").await?;

    match gemini.analyze_market_image(
        &image_url, 
        symbol.as_deref(), 
        timeframe.as_deref(),
        context.as_deref()
    ).await {
        Ok(response) => {
            loading_msg.delete(ctx).await.ok();
            
            let title = format!(
                "📊 Market Analysis{}{}",
                symbol.as_ref().map(|s| format!(" - {}", s)).unwrap_or_default(),
                timeframe.as_ref().map(|t| format!(" ({})", t)).unwrap_or_default()
            );
            
            // Response biasanya panjang, kirim sebagai text biasa
            if response.len() > 4000 {
                send_ai_response(ctx, format!("**{}**\n\n{}", title, response)).await?;
            } else {
                let embed = CreateEmbed::default()
                    .title(&title)
                    .thumbnail(&image_url)
                    .description(&response)
                    .color(0x00C853)
                    .footer(CreateEmbedFooter::new("⚠️ Bukan financial advice - DYOR"));

                ctx.send(CreateReply::default().embed(embed)).await?;
            }
        }
        Err(e) => {
            loading_msg.delete(ctx).await.ok();
            ctx.say(format!("❌ Error menganalisis chart: {}", e)).await?;
        }
    }

    Ok(())
}

/// Ringkas teks dengan Gemini
#[poise::command(prefix_command, slash_command, aliases("gsum", "gs"))]
pub async fn gemini_summarize(
    ctx: Context<'_>,
    #[rest]
    #[description = "Teks yang ingin diringkas"]
    text: String,
) -> Result<(), Error> {
    let config = Config::from_env()
        .map_err(|e| BotError::Config(format!("Failed to load config: {}", e)))?;

    if config.gemini_api_key == "api_key" {
        ctx.say("❌ Fitur Gemini AI belum dikonfigurasi. Harap set `GEMINI_API_KEY` di environment.")
            .await?;
        return Ok(());
    }

    let gemini = GeminiService::new(
        config.gemini_api_key,
        None,
        String::new(),
    );

    ctx.defer().await?;

    match gemini.summarize(&text).await {
        Ok(response) => {
            let embed = CreateEmbed::default()
                .title("📝 Ringkasan")
                .description(&response)
                .color(0x34A853)
                .footer(CreateEmbedFooter::new("Powered by Gemini AI"));

            if response.len() > 4000 {
                send_ai_response(ctx, response).await?;
            } else {
                ctx.send(CreateReply::default().embed(embed)).await?;
            }
        }
        Err(e) => {
            ctx.say(format!("❌ Error: {}", e)).await?;
        }
    }

    Ok(())
}

/// Terjemahkan teks dengan Gemini
#[poise::command(prefix_command, slash_command, aliases("gtrans", "gt"))]
pub async fn gemini_translate(
    ctx: Context<'_>,
    #[description = "Bahasa tujuan (contoh: Indonesia, English, Japanese)"]
    target_language: String,
    #[rest]
    #[description = "Teks yang ingin diterjemahkan"]
    text: String,
) -> Result<(), Error> {
    let config = Config::from_env()
        .map_err(|e| BotError::Config(format!("Failed to load config: {}", e)))?;

    if config.gemini_api_key == "api_key" {
        ctx.say("❌ Fitur Gemini AI belum dikonfigurasi. Harap set `GEMINI_API_KEY` di environment.")
            .await?;
        return Ok(());
    }

    let gemini = GeminiService::new(
        config.gemini_api_key,
        None,
        String::new(),
    );

    ctx.defer().await?;

    match gemini.translate(&text, &target_language).await {
        Ok(response) => {
            let embed = CreateEmbed::default()
                .title(format!("🌐 Terjemahan ke {}", target_language))
                .field("Original", &text, false)
                .field("Terjemahan", &response, false)
                .color(0xFBBC04)
                .footer(CreateEmbedFooter::new("Powered by Gemini AI"));

            if response.len() > 1000 || text.len() > 1000 {
                ctx.say(format!("**🌐 Terjemahan ke {}:**\n\n{}", target_language, response)).await?;
            } else {
                ctx.send(CreateReply::default().embed(embed)).await?;
            }
        }
        Err(e) => {
            ctx.say(format!("❌ Error: {}", e)).await?;
        }
    }

    Ok(())
}

/// Generate code dengan Gemini
#[poise::command(prefix_command, slash_command, aliases("gcode"))]
pub async fn gemini_code(
    ctx: Context<'_>,
    #[description = "Bahasa pemrograman (contoh: Python, Rust, JavaScript)"]
    language: String,
    #[rest]
    #[description = "Deskripsi kode yang ingin dibuat"]
    description: String,
) -> Result<(), Error> {
    let config = Config::from_env()
        .map_err(|e| BotError::Config(format!("Failed to load config: {}", e)))?;

    if config.gemini_api_key == "api_key" {
        ctx.say("❌ Fitur Gemini AI belum dikonfigurasi. Harap set `GEMINI_API_KEY` di environment.")
            .await?;
        return Ok(());
    }

    let gemini = GeminiService::new(
        config.gemini_api_key,
        None,
        String::new(),
    );

    ctx.defer().await?;

    match gemini.generate_code(&description, &language).await {
        Ok(response) => {
            send_ai_response(ctx, format!("**💻 Code Generation ({}):**\n\n{}", language, response)).await?;
        }
        Err(e) => {
            ctx.say(format!("❌ Error: {}", e)).await?;
        }
    }

    Ok(())
}

/// Jelaskan code dengan Gemini
#[poise::command(prefix_command, slash_command, aliases("gexplain", "gexp"))]
pub async fn gemini_explain(
    ctx: Context<'_>,
    #[rest]
    #[description = "Code yang ingin dijelaskan"]
    code: String,
) -> Result<(), Error> {
    let config = Config::from_env()
        .map_err(|e| BotError::Config(format!("Failed to load config: {}", e)))?;

    if config.gemini_api_key == "api_key" {
        ctx.say("❌ Fitur Gemini AI belum dikonfigurasi. Harap set `GEMINI_API_KEY` di environment.")
            .await?;
        return Ok(());
    }

    let gemini = GeminiService::new(
        config.gemini_api_key,
        None,
        String::new(),
    );

    ctx.defer().await?;

    match gemini.explain_code(&code).await {
        Ok(response) => {
            send_ai_response(ctx, format!("**📖 Code Explanation:**\n\n{}", response)).await?;
        }
        Err(e) => {
            ctx.say(format!("❌ Error: {}", e)).await?;
        }
    }

    Ok(())
}

