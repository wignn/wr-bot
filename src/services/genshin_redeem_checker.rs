use crate::repository::{DbPool, RedeemRepository};
use crate::scraper::genshin::{GenshinCodeData, GenshinCodeScraper};
use serenity::all::{ChannelId, Color, CreateEmbed, CreateMessage, Http};
use std::sync::Arc;
use tokio::time::{Duration, interval};

pub struct CodeCheckerService {
    scraper: GenshinCodeScraper,
    db: DbPool,
    http: Arc<Http>,
    check_interval_secs: u64,
}

impl CodeCheckerService {
    pub fn new(db: DbPool, http: Arc<Http>) -> Self {
        Self {
            scraper: GenshinCodeScraper::new(),
            db,
            http,
            check_interval_secs: 300,
        }
    }

    pub async fn start_monitoring(self: Arc<Self>) {
        let mut check_interval = interval(Duration::from_secs(self.check_interval_secs));

        loop {
            check_interval.tick().await;

            if let Err(e) = self.check_for_new_codes().await {
                eprintln!("Error checking for new codes: {}", e);
            }
        }
    }

    async fn check_for_new_codes(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("Checking for new Genshin codes...");

        let current_codes = self.scraper.fetch_codes().await?;

        if current_codes.is_empty() {
            println!("No active codes found from API");
            return Ok(());
        }

        let pool = self.db.as_ref();

        let mut new_codes = Vec::new();
        for code_data in &current_codes {
            if !RedeemRepository::is_code_sent(pool, &code_data.code).await? {
                new_codes.push(code_data);
            }
        }

        if !new_codes.is_empty() {
            println!("Found {} new code(s)!", new_codes.len());

            self.notify_new_codes(&new_codes).await?;

            for code in &new_codes {
                RedeemRepository::insert_code(
                    pool,
                    "genshin",
                    &code.code,
                    Some(&code.rewards),
                    None,
                )
                .await?;
                println!("Saved code to database: {}", code.code);
            }
        } else {
            println!("No new codes found.");
        }

        Ok(())
    }

    async fn notify_new_codes(
        &self,
        new_codes: &[&GenshinCodeData],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db.as_ref();
        let servers = RedeemRepository::get_active_servers(pool, "genshin").await?;

        if servers.is_empty() {
            println!("No active servers configured for notifications");
            return Ok(());
        }

        println!("Sending notifications to {} server(s)", servers.len());

        for server in servers {
            if let Err(e) = self
                .send_notification(server.channel_id as u64, new_codes)
                .await
            {
                eprintln!(
                    "Failed to send notification to channel {} (guild {}): {}",
                    server.channel_id, server.guild_id, e
                );
            } else {
                println!(
                    "Successfully sent notification to guild {} (channel {})",
                    server.guild_id, server.channel_id
                );
            }
        }

        Ok(())
    }

    async fn send_notification(
        &self,
        channel_id: u64,
        codes: &[&GenshinCodeData],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let channel = ChannelId::new(channel_id);

        for code in codes {
            let embed = CreateEmbed::new()
                .title("Kode Redeem Genshin Impact Baru!")
                .description(format!(
                    "Kode baru telah ditemukan! Segera redeem sebelum kadaluarsa.\n\n\
                    **Kode:** `{}`\n\n\
                    **Cara Redeem:**\n\
                    1. Buka [Genshin Impact Redeem](https://genshin.hoyoverse.com/en/gift)\n\
                    2. Login dengan akun Anda\n\
                    3. Masukkan kode di atas\n\
                    4. Klaim reward di in-game mail",
                    code.code
                ))
                .color(Color::from_rgb(91, 206, 250))
                .field("Rewards", &code.rewards, false)
                .field("Status", &code.status, true)
                .footer(serenity::all::CreateEmbedFooter::new(
                    "Auto-detected by Redeem Bot",
                ))
                .timestamp(serenity::model::Timestamp::now());

            let message = CreateMessage::new().content("@here").embed(embed);

            channel.send_message(&self.http, message).await?;

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Ok(())
    }
}

pub async fn start_code_checker(db: DbPool, http: Arc<Http>) {
    let checker = Arc::new(CodeCheckerService::new(db, http));

    tokio::spawn(async move {
        println!("Code checker service started - monitoring every 5 minutes");
        checker.start_monitoring().await;
    });
}
