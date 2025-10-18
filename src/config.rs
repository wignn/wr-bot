use std::env;
use std::fs;

#[derive(Clone, Debug)]
pub struct Config {
    pub token: String,
    pub client_id: String,
    pub api_key: String,
    pub model_ai: String,
    pub base_url: String,
    pub prompt: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let prompt_file = "system-prompt.txt";
        let prompt = fs::read_to_string(&prompt_file)
            .map_err(|e| format!("Failed to read prompt file '{}': {}", prompt_file, e))?;

        Ok(Self {
            api_key: env::var("API_KEY").expect("API_KEY not configured"),
            token: env::var("TOKEN").expect("TOKEN not configured"),
            client_id: env::var("CLIENT_ID").expect("CLIENT_ID not configured"),
            model_ai: env::var("MODEL_AI").expect("MODEL_AI not configured"),
            base_url: env::var("BASE_URL").expect("BASE_URL not configured"),
            prompt,
        })
    }
}
