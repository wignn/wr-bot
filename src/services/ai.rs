use std::collections::HashMap;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone)]
pub struct Ai {
    base_url: String,
    api_key: String,
    model: String,
    prompt: String,
    history: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

impl Ai {
    pub fn new(base_url: String, api_key: String, model: String, prompt: String) -> Self {
        Self {
            base_url,
            api_key,
            model,
            prompt,
            history: HashMap::new(),
        }
    }

    pub async fn call_api(
        &mut self,
        user_input: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let url = format!("{}/chat/completions", self.base_url);

        self.history.insert("user".to_string(), user_input.clone());

        let mut messages = vec![
            json!({"role": "system", "content": self.prompt})
        ];

        for (role, content) in &self.history {
            messages.push(json!({
                "role": role,
                "content": content
            }));
        }

        let body = json!({
            "model": self.model,
            "max_tokens": 2000,
            "temperature": 0.7,
            "messages": messages
        });

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!("API request failed with status: {}", status).into());
        }

        let api_response: ApiResponse = response.json().await?;
        let reply = api_response.choices[0].message.content.clone();

        self.history.insert("assistant".to_string(), reply.clone());

        Ok(reply)
    }
}
