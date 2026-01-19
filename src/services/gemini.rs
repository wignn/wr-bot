use gemini_rust::Gemini;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<SystemInstruction>,
}

#[derive(Serialize)]
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Part {
    Text { text: String },
    InlineData { inline_data: InlineData },
}

#[derive(Serialize)]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
    error: Option<GeminiError>,
}

#[derive(Deserialize)]
struct GeminiError {
    message: String,
}

#[derive(Deserialize)]
struct Candidate {
    content: CandidateContent,
}

#[derive(Deserialize)]
struct CandidateContent {
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: Option<String>,
}

#[derive(Clone)]
pub struct GeminiService {
    api_key: String,
    model: String,
    system_prompt: String,
    http_client: Client,
    // Conversation history per user (user_id -> Vec<(role, message)>)
    history: Arc<RwLock<HashMap<String, Vec<(String, String)>>>>,
}

impl GeminiService {
    pub fn new(api_key: String, model: Option<String>, system_prompt: String) -> Self {
        let model = model.unwrap_or_else(|| "gemini-3-flash-preview".to_string());

        Self {
            api_key,
            model,
            system_prompt,
            http_client: Client::new(),
            history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn create_client(&self) -> Result<Gemini, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Gemini::new(&self.api_key)?)
    }
    
    fn get_api_url(&self) -> String {
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        )
    }

    pub async fn generate(
        &self,
        prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.create_client()?;

        let response = client
            .generate_content()
            .with_system_prompt(&self.system_prompt)
            .with_user_message(prompt)
            .execute()
            .await?;

        let text = response.text();
        if text.is_empty() {
            return Err("No response text from Gemini".into());
        }

        Ok(text)
    }

    pub async fn analyze_market(
        &self,
        symbol: &str,
        timeframe: &str,
        context: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let prompt = format!(
            "Analisa market {} timeframe {}.\n\nKonteks market:\n{}",
            symbol, timeframe, context
        );

        self.generate(&prompt).await
    }

    pub async fn chat(
        &self,
        user_id: &str,
        message: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.create_client()?;

        let mut history = self.history.write().await;
        let user_history = history.entry(user_id.to_string()).or_insert_with(Vec::new);

        user_history.push(("user".to_string(), message.to_string()));

        let mut builder = client
            .generate_content()
            .with_system_prompt(&self.system_prompt);

        for (role, content) in user_history.iter() {
            if role == "user" {
                builder = builder.with_user_message(content);
            } else {
                builder = builder.with_model_message(content);
            }
        }

        let response = builder.execute().await?;

        let text = response.text();
        if text.is_empty() {
            return Err("No response text from Gemini".into());
        }

        user_history.push(("model".to_string(), text.clone()));

        if user_history.len() > 20 {
            *user_history = user_history.split_off(user_history.len() - 20);
        }

        Ok(text)
    }

    pub async fn clear_history(&self, user_id: &str) {
        let mut history = self.history.write().await;
        history.remove(user_id);
    }

    pub async fn clear_all_history(&self) {
        let mut history = self.history.write().await;
        history.clear();
    }

    async fn download_image_as_base64(&self, url: &str) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
        let response = self.http_client.get(url).send().await?;
        
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/png")
            .to_string();
        
        let mime_type = if content_type.contains("jpeg") || content_type.contains("jpg") {
            "image/jpeg".to_string()
        } else if content_type.contains("png") {
            "image/png".to_string()
        } else if content_type.contains("gif") {
            "image/gif".to_string()
        } else if content_type.contains("webp") {
            "image/webp".to_string()
        } else {
            "image/png".to_string()
        };
        
        let bytes = response.bytes().await?;
        let base64_data = BASE64.encode(&bytes);
        
        Ok((base64_data, mime_type))
    }

    pub async fn analyze_image(
        &self,
        image_url: &str,
        prompt: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let (base64_data, mime_type) = self.download_image_as_base64(image_url).await?;
        
        let prompt_text = prompt.unwrap_or("Describe this image in detail.");
        
        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![
                    Part::Text { text: prompt_text.to_string() },
                    Part::InlineData {
                        inline_data: InlineData {
                            mime_type,
                            data: base64_data,
                        },
                    },
                ],
                role: Some("user".to_string()),
            }],
            system_instruction: if !self.system_prompt.is_empty() {
                Some(SystemInstruction {
                    parts: vec![Part::Text { text: self.system_prompt.clone() }],
                })
            } else {
                None
            },
        };

        let response = self.http_client
            .post(&self.get_api_url())
            .json(&request)
            .send()
            .await?;

        let gemini_response: GeminiResponse = response.json().await?;
        
        if let Some(error) = gemini_response.error {
            return Err(format!("Gemini API Error: {}", error.message).into());
        }

        let text = gemini_response
            .candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.content.parts.into_iter().next())
            .and_then(|p| p.text)
            .ok_or("No response text from Gemini Vision")?;

        Ok(text)
    }

    pub async fn analyze_market_image(
        &self,
        image_url: &str,
        symbol: Option<&str>,
        timeframe: Option<&str>,
        additional_context: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let symbol_str = symbol.unwrap_or("Unknown");
        let timeframe_str = timeframe.unwrap_or("Unknown");
        let context = additional_context.unwrap_or("");
        
        let base_prompt = if !self.system_prompt.is_empty() {
            self.system_prompt.clone()
        } else {
            r#"You are a professional trading assistant specialized in Forex and XAUUSD (Gold).
                Your role:
                - Act as a trading analyst, NOT a signal seller.
                - Provide analysis based on price action, market structure, supply & demand,
                liquidity, trend, and momentum.
                - Always include risk management (RR, SL, TP).
                - Avoid emotional language.
                - Do NOT guarantee profit.
                - If data is insufficient, say "data tidak cukup".

                Trading rules:
                - Focus on H1, H4, and Daily timeframe.
                - XAUUSD volatility awareness is mandatory.
                - Prefer high-probability setups only.
                - Always mention trading bias: Bullish / Bearish / Neutral.

                Output format:
                - Market Bias
                - Key Levels
                - Possible Scenario
                - Risk Management
                - Notes

                Language:
                - Use Bahasa Indonesia."#.to_string()
        };
        
        let market_prompt = format!(
            r#"{}

Analisis chart trading ini:
- Symbol: {}
- Timeframe: {}
{}

Berikan analisis lengkap berdasarkan instruksi di atas."#,
            base_prompt,
            symbol_str,
            timeframe_str,
            if !context.is_empty() { format!("\nKonteks tambahan: {}", context) } else { String::new() }
        );

        self.analyze_image(image_url, Some(&market_prompt)).await
    }

    pub async fn summarize(
        &self,
        text: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let prompt = format!(
            "Please provide a clear and concise summary of the following text. \
            Focus on the main points and key information:\n\n{}",
            text
        );

        self.generate(&prompt).await
    }

    /// Translate text
    pub async fn translate(
        &self,
        text: &str,
        target_language: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let prompt = format!(
            "Translate the following text to {}. Only provide the translation, no explanations:\n\n{}",
            target_language, text
        );

        self.generate(&prompt).await
    }

    /// Generate code
    pub async fn generate_code(
        &self,
        description: &str,
        language: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let prompt = format!(
            "Write {} code for the following task. Include comments explaining the code:\n\n{}",
            language, description
        );

        self.generate(&prompt).await
    }

    /// Explain code
    pub async fn explain_code(
        &self,
        code: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let prompt = format!(
            "Explain the following code in detail. Describe what it does, how it works, \
            and any important concepts used:\n\n```\n{}\n```",
            code
        );

        self.generate(&prompt).await
    }
}
