use crate::config::Config;
use crate::repository::{DbPool, ForexRepository};
use chrono::{DateTime, Utc};
use chrono_tz::Asia::Jakarta;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serenity::all::{ChannelId, Color, CreateEmbed, CreateEmbedFooter, CreateMessage, Http};
use std::sync::Arc;
use tokio::time::{Duration, interval};

const FXSTREET_RSS: &str = "https://www.fxstreet-id.com/rss/news";
const FXSTREET_ANALYSIS_RSS: &str = "https://www.fxstreet-id.com/rss/analysis";
const DAILY_FOREX: &str = "https://www.dailyforex.com/rss/technicalanalysis.xml";
const WSJ_WORLD_NEWS_RSS: &str = "https://feeds.content.dowjones.io/public/rss/RSSWorldNews";
const WSJ_MARKETS_RSS: &str = "https://feeds.content.dowjones.io/public/rss/RSSMarketsMain";

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiCandidateContent,
}

#[derive(Deserialize)]
struct GeminiCandidateContent {
    parts: Vec<GeminiResponsePart>,
}

#[derive(Deserialize)]
struct GeminiResponsePart {
    text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ForexNews {
    pub title: String,
    pub description: String,
    pub currency: String,
    pub impact: Impact,
    pub time: Option<DateTime<Utc>>,
    pub link: Option<String>,
    pub id: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Impact {
    High,
    Medium,
    Low,
}

impl Impact {
    pub fn color(&self) -> Color {
        match self {
            Impact::High => Color::from_rgb(220, 53, 69),   // Red
            Impact::Medium => Color::from_rgb(255, 153, 0), // Orange
            Impact::Low => Color::from_rgb(40, 167, 69),    // Green
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Impact::High => "HIGH IMPACT",
            Impact::Medium => "MEDIUM IMPACT",
            Impact::Low => "LOW IMPACT",
        }
    }

    pub fn bar(&self) -> &str {
        match self {
            Impact::High => "▰▰▰",
            Impact::Medium => "▰▰▱",
            Impact::Low => "▰▱▱",
        }
    }
}

pub struct ForexService {
    client: Client,
    db: DbPool,
    http: Arc<Http>,
    check_interval_secs: u64,
    gemini_api_key: Option<String>,
}

impl ForexService {
    pub fn new(db: DbPool, http: Arc<Http>) -> Self {
        let gemini_api_key = Config::from_env().ok().and_then(|c| {
            if c.gemini_api_key != "api_key" {
                Some(c.gemini_api_key)
            } else {
                None
            }
        });

        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .unwrap_or_default(),
            db,
            http,
            check_interval_secs: 30,
            gemini_api_key,
        }
    }

    pub async fn start_monitoring(self: Arc<Self>) {
        let mut check_interval = interval(Duration::from_secs(self.check_interval_secs));

        println!("[FOREX] Starting forex news monitor...");

        loop {
            check_interval.tick().await;

            if let Err(e) = self.check_for_news().await {
                eprintln!("[FOREX] Error checking news: {}", e);
            }
        }
    }

    async fn check_for_news(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut all_news = Vec::new();

        match self.fetch_fxstreet().await {
            Ok(news) => all_news.extend(news),
            Err(e) => eprintln!("[FOREX] Error fetching FXStreet News: {}", e),
        }

        match self.fetch_fxstreet_analysis().await {
            Ok(news) => all_news.extend(news),
            Err(e) => eprintln!("[FOREX] Error fetching FXStreet Analysis: {}", e),
        }

        match self.fetch_dailyforex().await {
            Ok(news) => all_news.extend(news),
            Err(e) => eprintln!("[FOREX] Error fetching DailyForex: {}", e),
        }

        match self.fetch_wsj_world_news().await {
            Ok(news) => all_news.extend(news),
            Err(e) => eprintln!("[FOREX] Error fetching WSJ World News: {}", e),
        }

        match self.fetch_wsj_markets().await {
            Ok(news) => all_news.extend(news),
            Err(e) => eprintln!("[FOREX] Error fetching WSJ Markets: {}", e),
        }

        if all_news.is_empty() {
            return Ok(());
        }

        let pool = self.db.as_ref();

        let mut new_items = Vec::new();
        for item in &all_news {
            if !ForexRepository::is_news_sent(pool, &item.id).await? {
                new_items.push(item.clone());
            }
        }

        if !new_items.is_empty() {
            println!("[FOREX] Found {} new item(s)", new_items.len());

            self.notify_news(&new_items).await?;

            for item in &new_items {
                let source = if item.id.starts_with("wsj_world") {
                    "WSJ World News"
                } else if item.id.starts_with("wsj_markets") {
                    "WSJ Markets"
                } else if item.id.starts_with("dailyforex") {
                    "DailyForex"
                } else if item.id.starts_with("fxstreet_analysis") {
                    "FXStreet Analysis"
                } else {
                    "FXStreet"
                };
                ForexRepository::insert_news(pool, &item.id, source).await?;
            }
        }

        Ok(())
    }

    async fn fetch_fxstreet(
        &self,
    ) -> Result<Vec<ForexNews>, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client.get(FXSTREET_RSS).send().await?;
        let body = response.text().await?;

        let channel = rss::Channel::read_from(body.as_bytes())?;
        let mut news = Vec::new();

        for item in channel.items().iter().take(15) {
            let title = item.title().unwrap_or_default().to_string();
            let description = item.description().unwrap_or_default().to_string();
            let link = item.link().map(|s| s.to_string());
            let guid = item
                .guid()
                .map(|g| g.value().to_string())
                .unwrap_or_else(|| title.clone());

            let currency = Self::extract_currency(&title);
            let impact = Self::determine_impact(&title, &description);

            let time = item
                .pub_date()
                .and_then(|d| DateTime::parse_from_rfc2822(d).ok())
                .map(|dt| dt.with_timezone(&Utc));

            news.push(ForexNews {
                title: Self::clean_html(&title),
                description: Self::clean_html(&description),
                currency,
                impact,
                time,
                link,
                id: format!("fxstreet_{}", Self::hash_string(&guid)),
            });
        }

        println!("[FOREX] Fetched {} news from FXStreet ID", news.len());
        Ok(news)
    }

    async fn fetch_fxstreet_analysis(
        &self,
    ) -> Result<Vec<ForexNews>, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client.get(FXSTREET_ANALYSIS_RSS).send().await?;
        let body = response.text().await?;

        let channel = rss::Channel::read_from(body.as_bytes())?;
        let mut news = Vec::new();

        for item in channel.items().iter().take(15) {
            let title = item.title().unwrap_or_default().to_string();
            let description = item.description().unwrap_or_default().to_string();
            let link = item.link().map(|s| s.to_string());
            let guid = item
                .guid()
                .map(|g| g.value().to_string())
                .unwrap_or_else(|| title.clone());

            let currency = Self::extract_currency(&title);
            let impact = Self::determine_impact_analysis(&title, &description);

            let time = item
                .pub_date()
                .and_then(|d| DateTime::parse_from_rfc2822(d).ok())
                .map(|dt| dt.with_timezone(&Utc));

            news.push(ForexNews {
                title: Self::clean_html(&title),
                description: Self::clean_html(&description),
                currency,
                impact,
                time,
                link,
                id: format!("fxstreet_analysis_{}", Self::hash_string(&guid)),
            });
        }

        println!("[FOREX] Fetched {} analysis from FXStreet ID", news.len());
        Ok(news)
    }

    async fn fetch_dailyforex(
        &self,
    ) -> Result<Vec<ForexNews>, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client.get(DAILY_FOREX).send().await?;
        let body = response.text().await?;

        let channel = rss::Channel::read_from(body.as_bytes())?;
        let mut news = Vec::new();

        for item in channel.items().iter().take(15) {
            let title = item.title().unwrap_or_default().to_string();
            let description = item.description().unwrap_or_default().to_string();
            let link = item.link().map(|s| s.to_string());
            let guid = link.clone().unwrap_or_else(|| title.clone());

            let currency = Self::extract_currency(&title);
            let impact = Self::determine_impact_analysis(&title, &description);

            let time = item
                .pub_date()
                .and_then(|d| DateTime::parse_from_rfc2822(d).ok())
                .map(|dt| dt.with_timezone(&Utc));

            let (translated_title, translated_desc) = if self.gemini_api_key.is_some() {
                let t_title = self
                    .translate_to_indonesian(&title)
                    .await
                    .unwrap_or_else(|_| Self::clean_html(&title));
                let t_desc = self
                    .translate_to_indonesian(&description)
                    .await
                    .unwrap_or_else(|_| Self::clean_html(&description));
                (t_title, t_desc)
            } else {
                (Self::clean_html(&title), Self::clean_html(&description))
            };

            news.push(ForexNews {
                title: translated_title,
                description: translated_desc,
                currency,
                impact,
                time,
                link,
                id: format!("dailyforex_{}", Self::hash_string(&guid)),
            });
        }

        println!("[FOREX] Fetched {} analysis from DailyForex", news.len());
        Ok(news)
    }

    async fn fetch_wsj_world_news(
        &self,
    ) -> Result<Vec<ForexNews>, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client.get(WSJ_WORLD_NEWS_RSS).send().await?;
        let body = response.text().await?;

        let channel = rss::Channel::read_from(body.as_bytes())?;
        let mut news = Vec::new();

        for item in channel.items().iter().take(10) {
            let title = item.title().unwrap_or_default().to_string();
            let description = item.description().unwrap_or_default().to_string();
            let link = item.link().map(|s| s.to_string());
            let guid = item
                .guid()
                .map(|g| g.value().to_string())
                .unwrap_or_else(|| title.clone());

            let currency = Self::extract_currency(&title);
            let impact = Self::determine_impact(&title, &description);

            let time = item
                .pub_date()
                .and_then(|d| DateTime::parse_from_rfc2822(d).ok())
                .map(|dt| dt.with_timezone(&Utc));

            news.push(ForexNews {
                title: Self::clean_html(&title),
                description: Self::clean_html(&description),
                currency,
                impact,
                time,
                link,
                id: format!("wsj_world_{}", Self::hash_string(&guid)),
            });
        }

        println!("[FOREX] Fetched {} news from WSJ World News", news.len());
        Ok(news)
    }

    async fn fetch_wsj_markets(
        &self,
    ) -> Result<Vec<ForexNews>, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client.get(WSJ_MARKETS_RSS).send().await?;
        let body = response.text().await?;

        let channel = rss::Channel::read_from(body.as_bytes())?;
        let mut news = Vec::new();

        for item in channel.items().iter().take(10) {
            let title = item.title().unwrap_or_default().to_string();
            let description = item.description().unwrap_or_default().to_string();
            let link = item.link().map(|s| s.to_string());
            let guid = item
                .guid()
                .map(|g| g.value().to_string())
                .unwrap_or_else(|| title.clone());

            let currency = Self::extract_currency(&title);
            let impact = Self::determine_impact(&title, &description);

            let time = item
                .pub_date()
                .and_then(|d| DateTime::parse_from_rfc2822(d).ok())
                .map(|dt| dt.with_timezone(&Utc));

            news.push(ForexNews {
                title: Self::clean_html(&title),
                description: Self::clean_html(&description),
                currency,
                impact,
                time,
                link,
                id: format!("wsj_markets_{}", Self::hash_string(&guid)),
            });
        }

        println!("[FOREX] Fetched {} news from WSJ Markets", news.len());
        Ok(news)
    }

    async fn translate_to_indonesian(
        &self,
        text: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let api_key = self
            .gemini_api_key
            .as_ref()
            .ok_or("Gemini API key not configured")?;

        let prompt = format!(
            "Terjemahkan teks berikut ke Bahasa Indonesia. Hanya berikan hasil terjemahan, tanpa penjelasan tambahan:\n\n{}",
            text
        );

        let request = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart { text: prompt }],
            }],
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
            api_key
        );

        let response = self.client.post(&url).json(&request).send().await?;

        let gemini_response: GeminiResponse = response.json().await?;

        let translated = gemini_response
            .candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.content.parts.into_iter().next())
            .and_then(|p| p.text)
            .ok_or("No translation received")?;

        Ok(Self::clean_html(&translated))
    }

    async fn notify_news(
        &self,
        news: &[ForexNews],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db.as_ref();
        let channels = ForexRepository::get_active_channels(pool).await?;

        if channels.is_empty() {
            println!("[FOREX] No active channels configured");
            return Ok(());
        }

        println!("[FOREX] Sending to {} channel(s)", channels.len());

        for channel in channels {
            for item in news {
                if let Err(e) = self
                    .send_notification(channel.channel_id as u64, item)
                    .await
                {
                    eprintln!("[FOREX] Failed to send to {}: {}", channel.channel_id, e);
                }
                tokio::time::sleep(Duration::from_millis(800)).await;
            }
        }

        Ok(())
    }

    async fn send_notification(
        &self,
        channel_id: u64,
        news: &ForexNews,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let channel = ChannelId::new(channel_id);

        let time_str = news
            .time
            .map(|t| {
                let jakarta_time = t.with_timezone(&Jakarta);
                jakarta_time.format("%H:%M WIB").to_string()
            })
            .unwrap_or_else(|| "—".to_string());

        let desc = if news.description.len() > 350 {
            format!("{}...", &news.description[..350])
        } else {
            news.description.clone()
        };

        let is_dailyforex = news.id.starts_with("dailyforex");
        let is_fxstreet_analysis = news.id.starts_with("fxstreet_analysis");
        let is_wsj_world = news.id.starts_with("wsj_world");
        let is_wsj_markets = news.id.starts_with("wsj_markets");
        let source_name = if is_wsj_world {
            "WSJ World News"
        } else if is_wsj_markets {
            "WSJ Markets"
        } else if is_dailyforex {
            "DailyForex"
        } else if is_fxstreet_analysis {
            "FXStreet"
        } else {
            "FXStreet"
        };

        let source_link = news
            .link
            .as_ref()
            .map(|l| format!("[Baca Selengkapnya]({})", l))
            .unwrap_or_else(|| source_name.to_string());

        let embed = CreateEmbed::new()
            .title(&news.title)
            .color(news.impact.color())
            .field(&news.currency, &news.title, false)
            .field("", &desc, false)
            .field("Time", &time_str, true)
            .field("Impact", news.impact.bar(), true)
            .field("Source", &source_link, false)
            .footer(CreateEmbedFooter::new(format!(
                "Forex Alert • {}",
                source_name
            )))
            .timestamp(serenity::all::Timestamp::now());

        let message = CreateMessage::new().embed(embed);
        channel.send_message(&self.http, message).await?;

        Ok(())
    }

    fn extract_currency(text: &str) -> String {
        let upper = text.to_uppercase();

        let pairs = [
            ("EUR/USD", "EUR/USD"),
            ("EURUSD", "EUR/USD"),
            ("GBP/USD", "GBP/USD"),
            ("GBPUSD", "GBP/USD"),
            ("USD/JPY", "USD/JPY"),
            ("USDJPY", "USD/JPY"),
            ("USD/CHF", "USD/CHF"),
            ("USDCHF", "USD/CHF"),
            ("AUD/USD", "AUD/USD"),
            ("AUDUSD", "AUD/USD"),
            ("NZD/USD", "NZD/USD"),
            ("NZDUSD", "NZD/USD"),
            ("USD/CAD", "USD/CAD"),
            ("USDCAD", "USD/CAD"),
            ("EUR/GBP", "EUR/GBP"),
            ("EURGBP", "EUR/GBP"),
            ("EUR/JPY", "EUR/JPY"),
            ("EURJPY", "EUR/JPY"),
            ("GBP/JPY", "GBP/JPY"),
            ("GBPJPY", "GBP/JPY"),
            ("GBP/CHF", "GBP/CHF"),
            ("GBPCHF", "GBP/CHF"),
            ("USD/MXN", "USD/MXN"),
            ("USDMXN", "USD/MXN"),
            ("USD/ZAR", "USD/ZAR"),
            ("USDZAR", "USD/ZAR"),
            ("XAU/USD", "XAU/USD"),
            ("XAUUSD", "XAU/USD"),
            ("XAG/USD", "XAG/USD"),
            ("XAGUSD", "XAG/USD"),
            ("BTC/USD", "BTC/USD"),
            ("BTCUSD", "BTC/USD"),
            ("ETH/USD", "ETH/USD"),
            ("ETHUSD", "ETH/USD"),
        ];

        for (pair, label) in pairs {
            if upper.contains(pair) {
                return label.to_string();
            }
        }

        let currencies = [
            ("JAPANESE YEN", "JPY"),
            ("YEN", "JPY"),
            ("JPY", "JPY"),
            ("US DOLLAR", "USD"),
            ("DOLLAR", "USD"),
            ("USD", "USD"),
            ("EURO", "EUR"),
            ("EUR", "EUR"),
            ("BRITISH POUND", "GBP"),
            ("POUND", "GBP"),
            ("STERLING", "GBP"),
            ("GBP", "GBP"),
            ("SWISS FRANC", "CHF"),
            ("CHF", "CHF"),
            ("AUSTRALIAN", "AUD"),
            ("AUD", "AUD"),
            ("NEW ZEALAND", "NZD"),
            ("KIWI", "NZD"),
            ("NZD", "NZD"),
            ("CANADIAN", "CAD"),
            ("LOONIE", "CAD"),
            ("CAD", "CAD"),
            ("MEXICAN PESO", "MXN"),
            ("MXN", "MXN"),
            ("SOUTH AFRICAN RAND", "ZAR"),
            ("RAND", "ZAR"),
            ("ZAR", "ZAR"),
            ("GOLD", "XAU"),
            ("XAU", "XAU"),
            ("SILVER", "XAG"),
            ("XAG", "XAG"),
            ("OIL", "OIL"),
            ("CRUDE", "OIL"),
            ("WTI", "OIL"),
            ("BRENT", "OIL"),
            ("BITCOIN", "BTC"),
            ("BTC", "BTC"),
            ("ETHEREUM", "ETH"),
            ("ETH", "ETH"),
            ("S&P 500", "S&P500"),
            ("S&P500", "S&P500"),
            ("SP500", "S&P500"),
            ("NASDAQ", "NASDAQ"),
            ("DAX", "DAX"),
            ("TESLA", "TSLA"),
            ("TSLA", "TSLA"),
            ("COCA-COLA", "KO"),
            ("COCACOLA", "KO"),
        ];

        for (keyword, curr) in currencies {
            if upper.contains(keyword) {
                return curr.to_string();
            }
        }

        "MARKET".to_string()
    }

    fn determine_impact(title: &str, description: &str) -> Impact {
        let text = format!("{} {}", title, description).to_lowercase();

        let high_keywords = [
            "interest rate",
            "rate decision",
            "nfp",
            "non-farm",
            "payroll",
            "cpi",
            "inflation",
            "gdp",
            "fomc",
            "fed",
            "ecb",
            "boe",
            "boj",
            "employment",
            "unemployment",
            "central bank",
            "monetary policy",
            "recession",
            "crisis",
            "all-time high",
            "record high",
            "crash",
            "tariff",
            "trade war",
            "breaking",
            "urgent",
        ];

        let medium_keywords = [
            "trade balance",
            "housing",
            "manufacturing",
            "industrial",
            "consumer confidence",
            "business confidence",
            "pmi",
            "retail sales",
            "durable goods",
            "import",
            "export",
            "wage",
            "earnings",
            "rally",
            "surge",
            "plunge",
            "drop",
            "bullish",
            "bearish",
        ];

        for keyword in high_keywords {
            if text.contains(keyword) {
                return Impact::High;
            }
        }

        for keyword in medium_keywords {
            if text.contains(keyword) {
                return Impact::Medium;
            }
        }

        Impact::Low
    }

    /// Determine impact for technical analysis (DailyForex)
    fn determine_impact_analysis(title: &str, description: &str) -> Impact {
        let text = format!("{} {}", title, description).to_lowercase();

        let high_keywords = [
            "breakout",
            "breakdown",
            "all-time high",
            "record high",
            "crash",
            "surge",
            "plunge",
            "major support",
            "major resistance",
            "key level",
            "critical",
            "urgent",
            "bitcoin",
            "btc",
            "ethereum",
            "eth",
            "gold",
            "xau",
            "s&p 500",
            "nasdaq",
        ];

        let medium_keywords = [
            "bullish",
            "bearish",
            "signal",
            "forecast",
            "analysis",
            "support",
            "resistance",
            "pattern",
            "trend",
            "momentum",
            "target",
            "rally",
            "drop",
            "dip",
            "bounce",
        ];

        for keyword in high_keywords {
            if text.contains(keyword) {
                return Impact::High;
            }
        }

        for keyword in medium_keywords {
            if text.contains(keyword) {
                return Impact::Medium;
            }
        }

        Impact::Low
    }

    fn clean_html(text: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;

        for c in text.chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => result.push(c),
                _ => {}
            }
        }

        result
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&nbsp;", " ")
            .trim()
            .to_string()
    }

    fn hash_string(s: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }
}

/// Start the forex news service
pub async fn start_forex_service(db: DbPool, http: Arc<Http>) {
    let service = Arc::new(ForexService::new(db, http));
    tokio::spawn(async move {
        service.start_monitoring().await;
    });
}
