use crate::repository::{DbPool, ForexRepository};
use chrono::{DateTime, Utc};
use chrono_tz::Asia::Jakarta;
use reqwest::Client;
use serenity::all::{ChannelId, Color, CreateEmbed, CreateEmbedFooter, CreateMessage, Http};
use std::sync::Arc;
use tokio::time::{interval, Duration};

const FXSTREET_RSS: &str = "https://www.fxstreet-id.com/rss/news";

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
}

impl ForexService {
    pub fn new(db: DbPool, http: Arc<Http>) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .unwrap_or_default(),
            db,
            http,
            check_interval_secs: 30,
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
        let news_items = self.fetch_fxstreet().await?;

        if news_items.is_empty() {
            return Ok(());
        }

        let db_lock = self.db.lock().await;
        let conn = db_lock.get_connection();

        let mut new_items = Vec::new();
        for item in &news_items {
            if !ForexRepository::is_news_sent(conn, &item.id)? {
                new_items.push(item.clone());
            }
        }

        drop(db_lock);

        if !new_items.is_empty() {
            println!("[FOREX] Found {} new item(s)", new_items.len());

            self.notify_news(&new_items).await?;

            let db_lock = self.db.lock().await;
            let conn = db_lock.get_connection();

            for item in &new_items {
                ForexRepository::insert_news(conn, &item.id, "FXStreet")?;
            }
        }

        Ok(())
    }

    async fn fetch_fxstreet(&self) -> Result<Vec<ForexNews>, Box<dyn std::error::Error + Send + Sync>> {
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

    async fn notify_news(&self, news: &[ForexNews]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let db_lock = self.db.lock().await;
        let conn = db_lock.get_connection();
        let channels = ForexRepository::get_active_channels(conn)?;
        drop(db_lock);

        if channels.is_empty() {
            println!("[FOREX] No active channels configured");
            return Ok(());
        }

        println!("[FOREX] Sending to {} channel(s)", channels.len());

        for channel in channels {
            for item in news {
                if let Err(e) = self.send_notification(channel.channel_id, item).await {
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

        let source_link = news
            .link
            .as_ref()
            .map(|l| format!("[📖 Baca Selengkapnya]({})", l))
            .unwrap_or_else(|| "FXStreet".to_string());

        let embed = CreateEmbed::new()
            .title(format!("{} FOREX NEWS", news.impact.label()))
            .color(news.impact.color())
            .field(&news.currency, &news.title, false)
            .field("", &desc, false)
            .field("Time", &time_str, true)
            .field("Impact", news.impact.bar(), true)
            .field("Source", &source_link, false)
            .footer(CreateEmbedFooter::new("Forex News Alert"))
            .timestamp(serenity::all::Timestamp::now());

        let message = CreateMessage::new().embed(embed);
        channel.send_message(&self.http, message).await?;

        Ok(())
    }

    fn extract_currency(text: &str) -> String {
        let upper = text.to_uppercase();

        // Check for currency pairs first
        let pairs = [
            ("EUR/USD", "EUR/USD"),
            ("GBP/USD", "GBP/USD"),
            ("USD/JPY", "USD/JPY"),
            ("USD/CHF", "USD/CHF"),
            ("AUD/USD", "AUD/USD"),
            ("NZD/USD", "NZD/USD"),
            ("USD/CAD", "USD/CAD"),
            ("EUR/GBP", "EUR/GBP"),
            ("EUR/JPY", "EUR/JPY"),
            ("GBP/JPY", "GBP/JPY"),
            ("XAU/USD", "XAU/USD"),
            ("XAG/USD", "XAG/USD"),
        ];

        for (pair, label) in pairs {
            if upper.contains(pair) {
                return label.to_string();
            }
        }

        // Check for individual currencies/assets
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
            ("GOLD", "XAU"),
            ("XAU", "XAU"),
            ("SILVER", "XAG"),
            ("XAG", "XAG"),
            ("OIL", "OIL"),
            ("CRUDE", "OIL"),
            ("WTI", "OIL"),
            ("BRENT", "OIL"),
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
    {
        let db_lock = db.lock().await;
        let conn = db_lock.get_connection();
        if let Err(e) = ForexRepository::init_tables(conn) {
            eprintln!("[FOREX] Failed to initialize tables: {}", e);
            return;
        }
    }

    let service = Arc::new(ForexService::new(db, http));
    tokio::spawn(async move {
        service.start_monitoring().await;
    });
}
