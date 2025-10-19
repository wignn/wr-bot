use reqwest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenshinCodeResponse {
    pub codes: Vec<GenshinCodeData>,
    pub game: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenshinCodeData {
    pub id: u64,
    pub code: String,
    pub status: String,
    pub game: String,
    pub rewards: String,
}

pub struct GenshinCodeScraper {
    client: reqwest::Client,
    api_url: String,
}

impl GenshinCodeScraper {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .unwrap(),
            api_url: "https://hoyo-codes.seria.moe/codes?game=genshin".to_string(),
        }
    }

    pub async fn fetch_codes(&self) -> Result<Vec<GenshinCodeData>, Box<dyn std::error::Error>> {
        let response = self.client
            .get(&self.api_url)
            .send()
            .await?;

        let data: GenshinCodeResponse = response.json().await?;

        let active_codes: Vec<GenshinCodeData> = data.codes
            .into_iter()
            .filter(|code| code.status == "OK")
            .collect();

        Ok(active_codes)
    }
}

