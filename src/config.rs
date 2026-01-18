use std::env;
use std::fs;

#[derive(Clone, Debug)]
pub struct Config {
    pub token: String,
    pub client_id: String,
    pub api_key: Option<String>,
    pub model_ai: String,
    pub base_url: String,
    pub prompt: String,
    pub scraper_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let prompt_file = "system-prompt.txt";
        let prompt = fs::read_to_string(&prompt_file)
            .map_err(|e| format!("Failed to read prompt file '{}': {}", prompt_file, e))?;

        let token = env::var("TOKEN").map_err(|_| "TOKEN not configured in .env")?;
        let client_id = env::var("CLIENT_ID").map_err(|_| "CLIENT_ID not configured in .env")?;

        let api_key = env::var("API_KEY").ok();
        let model_ai = env::var("MODEL_AI")
            .unwrap_or_else(|_| "tngtech/deepseek-r1t2-chimera:free".to_string());
        let base_url =
            env::var("BASE_URL").unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());
        let scraper_url =
            env::var("SCRAPER_URL").unwrap_or_else(|_| "https://api.ennead.cc/mihoyo".to_string());

        Ok(Self {
            token,
            client_id,
            api_key,
            model_ai,
            base_url,
            prompt,
            scraper_url,
        })
    }

    pub fn is_ai_enabled(&self) -> bool {
        self.api_key.is_some()
    }
}
