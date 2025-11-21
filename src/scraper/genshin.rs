use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenshinCodeData {
    pub code: String,
    pub rewards: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    active: Vec<CodeInfo>,
    #[allow(dead_code)]
    inactive: Vec<CodeInfo>,
}

#[derive(Debug, Deserialize)]
struct CodeInfo {
    code: String,
    rewards: Vec<String>,
}

pub struct GenshinCodeScraper {
    api_url: String,
    client: reqwest::Client,
}

impl GenshinCodeScraper {
    pub fn new() -> Self {
        Self {
            api_url: "https://api.ennead.cc/mihoyo/genshin/codes".to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn fetch_codes(&self) -> Result<Vec<GenshinCodeData>, Box<dyn std::error::Error>> {
        println!("Fetching codes from API: {}", self.api_url);

        let response = self.client
            .get(&self.api_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("API request failed with status: {}", response.status()).into());
        }

        let api_response: ApiResponse = response.json().await?;

        let codes: Vec<GenshinCodeData> = api_response
            .active
            .into_iter()
            .map(|code_info| {
                let rewards = if code_info.rewards.is_empty() {
                    "Unknown rewards".to_string()
                } else {
                    code_info.rewards.join(", ")
                };

                GenshinCodeData {
                    code: code_info.code,
                    rewards,
                    status: "Active".to_string(),
                }
            })
            .collect();

        println!("Successfully fetched {} active codes", codes.len());

        Ok(codes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_codes() {
        let scraper = GenshinCodeScraper::new();
        match scraper.fetch_codes().await {
            Ok(codes) => {
                println!("Fetched {} codes", codes.len());
                for code in codes.iter().take(3) {
                    println!("Code: {} - Rewards: {}", code.code, code.rewards);
                }
                assert!(!codes.is_empty(), "Should fetch at least one code");
            }
            Err(e) => {
                eprintln!("Failed to fetch codes: {}", e);
            }
        }
    }
}