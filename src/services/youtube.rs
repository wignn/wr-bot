use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouTubeVideo {
    pub video_id: String,
    pub title: String,
    pub channel: String,
    pub thumbnail: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct YouTubeSearchResponse {
    items: Vec<YouTubeSearchItem>,
}

#[derive(Debug, Deserialize)]
struct YouTubeSearchItem {
    id: YouTubeVideoId,
    snippet: YouTubeSnippet,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct YouTubeVideoId {
    video_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct YouTubeSnippet {
    title: String,
    channel_title: String,
    thumbnails: YouTubeThumbnails,
}

#[derive(Debug, Deserialize)]
struct YouTubeThumbnails {
    default: YouTubeThumbnail,
}

#[derive(Debug, Deserialize)]
struct YouTubeThumbnail {
    url: String,
}

#[derive(Clone)]
pub struct YouTubeSearch {
    client: Client,
    api_key: String,
}

impl YouTubeSearch {
    pub fn new() -> Option<Self> {
        let api_key = env::var("YOUTUBE_API_KEY").ok()?;
        if api_key.is_empty() {
            return None;
        }

        Some(Self {
            client: Client::new(),
            api_key,
        })
    }

    pub async fn search(&self, query: &str, max_results: u32) -> Result<Vec<YouTubeVideo>, String> {
        let max_results = max_results.min(10);

        let url = format!(
            "https://www.googleapis.com/youtube/v3/search?part=snippet&type=video&maxResults={}&q={}&key={}",
            max_results,
            urlencoding::encode(query),
            self.api_key
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("YouTube API error {}: {}", status, body));
        }

        let data: YouTubeSearchResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let videos = data
            .items
            .into_iter()
            .filter_map(|item| {
                let video_id = item.id.video_id?;
                Some(YouTubeVideo {
                    url: format!("https://www.youtube.com/watch?v={}", video_id),
                    video_id,
                    title: item.snippet.title,
                    channel: item.snippet.channel_title,
                    thumbnail: item.snippet.thumbnails.default.url,
                })
            })
            .collect();

        Ok(videos)
    }
}

use std::sync::OnceLock;

static GLOBAL_YOUTUBE: OnceLock<YouTubeSearch> = OnceLock::new();

pub fn init_global_youtube(youtube: YouTubeSearch) {
    let _ = GLOBAL_YOUTUBE.set(youtube);
}

pub fn get_global_youtube() -> Option<&'static YouTubeSearch> {
    GLOBAL_YOUTUBE.get()
}
