use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use serde::Serialize;
use serenity::all::{ChannelId, CreateEmbed, CreateMessage, Http};
use std::collections::HashMap;
use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

const TIINGO_WS_URL: &str = "wss://api.tiingo.com/fx";

#[derive(Debug, Clone)]
pub struct ForexPrice {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub mid: f64,
    pub timestamp: DateTime<Utc>,
}

impl ForexPrice {
    pub fn spread(&self) -> f64 {
        self.ask - self.bid
    }

    pub fn spread_pips(&self) -> f64 {
        let multiplier = if self.symbol.to_uppercase().contains("JPY") {
            100.0
        } else if self.symbol.to_uppercase().contains("XAU") {
            10.0
        } else {
            10000.0
        };
        self.spread() * multiplier
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlertCondition {
    Above,
    Below,
}

impl std::fmt::Display for AlertCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertCondition::Above => write!(f, "above"),
            AlertCondition::Below => write!(f, "below"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PriceAlert {
    pub id: i64,
    pub guild_id: u64,
    pub user_id: u64,
    pub channel_id: u64,
    pub symbol: String,
    pub condition: AlertCondition,
    pub target_price: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct TiingoService {
    api_key: String,
    prices: Arc<RwLock<HashMap<String, ForexPrice>>>,
    alerts: Arc<RwLock<Vec<PriceAlert>>>,
}

#[derive(Serialize)]
struct SubscribeMessage {
    #[serde(rename = "eventName")]
    event_name: String,
    authorization: String,
    #[serde(rename = "eventData")]
    event_data: SubscribeEventData,
}

#[derive(Serialize)]
struct SubscribeEventData {
    #[serde(rename = "thresholdLevel")]
    threshold_level: u32,
}

impl TiingoService {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            prices: Arc::new(RwLock::new(HashMap::new())),
            alerts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn get_price(&self, symbol: &str) -> Option<ForexPrice> {
        self.prices.read().get(&symbol.to_lowercase()).cloned()
    }

    pub fn get_all_prices(&self) -> HashMap<String, ForexPrice> {
        self.prices.read().clone()
    }

    pub fn add_alert(&self, alert: PriceAlert) {
        self.alerts.write().push(alert);
    }

    pub fn remove_alert(&self, alert_id: i64) -> bool {
        let mut alerts = self.alerts.write();
        if let Some(pos) = alerts.iter().position(|a| a.id == alert_id) {
            alerts.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn get_user_alerts(&self, user_id: u64) -> Vec<PriceAlert> {
        self.alerts
            .read()
            .iter()
            .filter(|a| a.user_id == user_id)
            .cloned()
            .collect()
    }

    fn update_price(&self, symbol: String, bid: f64, ask: f64) {
        let mid = (bid + ask) / 2.0;
        let price = ForexPrice {
            symbol: symbol.clone(),
            bid,
            ask,
            mid,
            timestamp: Utc::now(),
        };
        self.prices.write().insert(symbol.to_lowercase(), price);
    }

    fn check_alerts(&self, symbol: &str, price: f64) -> Vec<PriceAlert> {
        let alerts = self.alerts.read();
        alerts
            .iter()
            .filter(|a| {
                a.symbol.to_lowercase() == symbol.to_lowercase()
                    && match a.condition {
                        AlertCondition::Above => price >= a.target_price,
                        AlertCondition::Below => price <= a.target_price,
                    }
            })
            .cloned()
            .collect()
    }

    fn remove_triggered_alerts(&self, triggered: &[PriceAlert]) {
        let mut alerts = self.alerts.write();
        alerts.retain(|a| !triggered.iter().any(|t| t.id == a.id));
    }

    pub async fn start_price_polling(self: Arc<Self>, http: Arc<Http>) {
        loop {
            println!("[TIINGO] Connecting to WebSocket...");
            match self.connect_and_run(http.clone()).await {
                Ok(_) => println!("[TIINGO] WebSocket closed normally"),
                Err(e) => eprintln!("[TIINGO] WebSocket error: {}", e),
            }
            println!("[TIINGO] Reconnecting in 5 seconds...");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    async fn connect_and_run(
        &self,
        http: Arc<Http>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (ws_stream, _) = connect_async(TIINGO_WS_URL).await?;
        println!("[TIINGO] Connected to WebSocket");

        let (mut write, mut read) = ws_stream.split();

        let subscribe_msg = SubscribeMessage {
            event_name: "subscribe".to_string(),
            authorization: self.api_key.clone(),
            event_data: SubscribeEventData { threshold_level: 5 },
        };

        let msg_json = serde_json::to_string(&subscribe_msg)?;
        write.send(WsMessage::Text(msg_json)).await?;
        println!("[TIINGO] Sent subscription message");

        let mut log_count = 0u64;

        while let Some(msg) = read.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    self.handle_message(&text, &http, &mut log_count).await;
                }
                Ok(WsMessage::Ping(data)) => {
                    let _ = write.send(WsMessage::Pong(data)).await;
                }
                Ok(WsMessage::Close(_)) => {
                    println!("[TIINGO] WebSocket closed by server");
                    break;
                }
                Err(e) => {
                    eprintln!("[TIINGO] WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_message(&self, text: &str, http: &Arc<Http>, log_count: &mut u64) {
        let json: serde_json::Value = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(_) => return,
        };

        let message_type = json
            .get("messageType")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match message_type {
            "A" => {
                if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                    let update_type = data.get(0).and_then(|v| v.as_str()).unwrap_or("");

                    if update_type == "Q" && data.len() >= 8 {
                        let symbol = data
                            .get(1)
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let bid = data.get(4).and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let ask = data.get(7).and_then(|v| v.as_f64()).unwrap_or(0.0);

                        if symbol.is_empty() || bid <= 0.0 || ask <= 0.0 {
                            return;
                        }

                        let spread_pct = (ask - bid).abs() / bid * 100.0;
                        if spread_pct > 1.0 {
                            return;
                        }

                        *log_count += 1;
                        if *log_count <= 15 {
                            println!("[TIINGO] {} bid={:.5} ask={:.5}", symbol, bid, ask);
                        }

                        self.update_price(symbol.clone(), bid, ask);

                        let mid = (bid + ask) / 2.0;
                        let triggered = self.check_alerts(&symbol, mid);
                        if !triggered.is_empty() {
                            self.send_alert_notifications(&triggered, mid, http).await;
                            self.remove_triggered_alerts(&triggered);
                        }
                    }
                }
            }
            "I" => {
                println!("[TIINGO] Info: {}", text);
            }
            "E" => {
                eprintln!("[TIINGO] Error: {}", text);
            }
            _ => {}
        }
    }

    async fn send_alert_notifications(
        &self,
        alerts: &[PriceAlert],
        current_price: f64,
        http: &Arc<Http>,
    ) {
        for alert in alerts {
            let embed = CreateEmbed::new()
                .title("Price Alert Triggered!")
                .description(format!(
                    "**{}** is now {} **{:.5}**\n\nTarget: {:.5}\nCurrent: {:.5}",
                    alert.symbol.to_uppercase(),
                    alert.condition,
                    alert.target_price,
                    alert.target_price,
                    current_price
                ))
                .color(0x00ff00);

            let channel_id = ChannelId::new(alert.channel_id);
            let message = CreateMessage::new()
                .content(format!("<@{}>", alert.user_id))
                .embed(embed);

            let _ = channel_id.send_message(http, message).await;
        }
    }
}

use once_cell::sync::OnceCell;
static GLOBAL_TIINGO: OnceCell<Arc<TiingoService>> = OnceCell::new();

pub fn init_global_tiingo(service: Arc<TiingoService>) {
    let _ = GLOBAL_TIINGO.set(service);
}

pub fn get_global_tiingo() -> Option<&'static Arc<TiingoService>> {
    GLOBAL_TIINGO.get()
}
