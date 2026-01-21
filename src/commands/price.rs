use crate::commands::Data;
use crate::services::tiingo::{AlertCondition, PriceAlert, get_global_tiingo};
use chrono::Utc;
use poise::serenity_prelude::CreateEmbed;
use std::sync::atomic::{AtomicI64, Ordering};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

static ALERT_ID_COUNTER: AtomicI64 = AtomicI64::new(1);

fn next_alert_id() -> i64 {
    ALERT_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

async fn send_embed(ctx: Context<'_>, embed: CreateEmbed) -> Result<(), Error> {
    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Get live forex price
#[poise::command(slash_command, prefix_command)]
pub async fn price(
    ctx: Context<'_>,
    #[description = "Symbol (e.g., xauusd, eurusd, gbpusd)"] symbol: String,
) -> Result<(), Error> {
    let tiingo = match get_global_tiingo() {
        Some(t) => t,
        None => {
            send_embed(
                ctx,
                CreateEmbed::new()
                    .title("Error")
                    .description("Price service not available")
                    .color(0xff0000),
            )
            .await?;
            return Ok(());
        }
    };

    let symbol_lower = symbol.to_lowercase();

    match tiingo.get_price(&symbol_lower) {
        Some(price) => {
            let spread_pips = price.spread_pips();
            let time_ago = Utc::now().signed_duration_since(price.timestamp);
            let time_str = if time_ago.num_seconds() < 60 {
                format!("{}s ago", time_ago.num_seconds())
            } else {
                format!("{}m ago", time_ago.num_minutes())
            };

            let embed = CreateEmbed::new()
                .title(format!("💱 {}", symbol.to_uppercase()))
                .field("Bid", format!("{:.5}", price.bid), true)
                .field("Ask", format!("{:.5}", price.ask), true)
                .field("Spread", format!("{:.1} pips", spread_pips), true)
                .field("Mid", format!("{:.5}", price.mid), false)
                .footer(poise::serenity_prelude::CreateEmbedFooter::new(format!(
                    "Updated: {}",
                    time_str
                )))
                .color(0x1DB954);

            send_embed(ctx, embed).await?;
        }
        None => {
            let available = tiingo
                .get_all_prices()
                .keys()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");

            let desc = if available.is_empty() {
                format!(
                    "No data for **{}**. Price service may still be connecting.\n\nTry again in a few seconds.",
                    symbol.to_uppercase()
                )
            } else {
                format!(
                    "No data for **{}**.\n\nAvailable: {}",
                    symbol.to_uppercase(),
                    available.to_uppercase()
                )
            };

            send_embed(
                ctx,
                CreateEmbed::new()
                    .title("Symbol Not Found")
                    .description(desc)
                    .color(0xff0000),
            )
            .await?;
        }
    }

    Ok(())
}

/// Set a price alert
#[poise::command(slash_command, prefix_command)]
pub async fn alert(
    ctx: Context<'_>,
    #[description = "Symbol (e.g., xauusd)"] symbol: String,
    #[description = "Condition: above or below"] condition: String,
    #[description = "Target price"] target: f64,
) -> Result<(), Error> {
    let tiingo = match get_global_tiingo() {
        Some(t) => t,
        None => {
            send_embed(
                ctx,
                CreateEmbed::new()
                    .title("Error")
                    .description("Price service not available")
                    .color(0xff0000),
            )
            .await?;
            return Ok(());
        }
    };

    let condition_parsed = match condition.to_lowercase().as_str() {
        "above" | ">" | "up" => AlertCondition::Above,
        "below" | "<" | "down" => AlertCondition::Below,
        _ => {
            send_embed(
                ctx,
                CreateEmbed::new()
                    .title("Invalid Condition")
                    .description("Use `above` or `below`")
                    .color(0xff0000),
            )
            .await?;
            return Ok(());
        }
    };

    let guild_id = ctx.guild_id().map(|g| g.get()).unwrap_or(0);

    let alert = PriceAlert {
        id: next_alert_id(),
        guild_id,
        user_id: ctx.author().id.get(),
        channel_id: ctx.channel_id().get(),
        symbol: symbol.to_lowercase(),
        condition: condition_parsed.clone(),
        target_price: target,
        created_at: Utc::now(),
    };

    let alert_id = alert.id;
    tiingo.add_alert(alert);

    let current_price = tiingo
        .get_price(&symbol.to_lowercase())
        .map(|p| format!("{:.5}", p.mid))
        .unwrap_or_else(|| "N/A".to_string());

    let embed = CreateEmbed::new()
        .title("Alert Created")
        .description(format!(
            "Alert **#{}** set!\n\n**{}** {} **{:.5}**\n\nCurrent: {}",
            alert_id,
            symbol.to_uppercase(),
            condition_parsed,
            target,
            current_price
        ))
        .color(0x00ff00)
        .footer(poise::serenity_prelude::CreateEmbedFooter::new(
            "You'll be notified when the price is reached",
        ));

    send_embed(ctx, embed).await?;

    Ok(())
}

/// List your active alerts
#[poise::command(slash_command, prefix_command)]
pub async fn alerts(ctx: Context<'_>) -> Result<(), Error> {
    let tiingo = match get_global_tiingo() {
        Some(t) => t,
        None => {
            send_embed(
                ctx,
                CreateEmbed::new()
                    .title("Error")
                    .description("Price service not available")
                    .color(0xff0000),
            )
            .await?;
            return Ok(());
        }
    };

    let user_alerts = tiingo.get_user_alerts(ctx.author().id.get());

    if user_alerts.is_empty() {
        send_embed(
            ctx,
            CreateEmbed::new()
                .title("Your Alerts")
                .description("No active alerts.\n\nUse `/alert <symbol> <above/below> <price>` to create one.")
                .color(0x808080),
        )
        .await?;
        return Ok(());
    }

    let mut description = String::new();
    for alert in &user_alerts {
        description.push_str(&format!(
            "**#{}** {} {} {:.5}\n",
            alert.id,
            alert.symbol.to_uppercase(),
            alert.condition,
            alert.target_price
        ));
    }

    let embed = CreateEmbed::new()
        .title("Your Alerts")
        .description(description)
        .footer(poise::serenity_prelude::CreateEmbedFooter::new(
            "Use /alertremove <id> to remove",
        ))
        .color(0x1DB954);

    send_embed(ctx, embed).await?;

    Ok(())
}

/// Remove a price alert
#[poise::command(slash_command, prefix_command)]
pub async fn alertremove(
    ctx: Context<'_>,
    #[description = "Alert ID to remove"] id: i64,
) -> Result<(), Error> {
    let tiingo = match get_global_tiingo() {
        Some(t) => t,
        None => {
            send_embed(
                ctx,
                CreateEmbed::new()
                    .title("Error")
                    .description("Price service not available")
                    .color(0xff0000),
            )
            .await?;
            return Ok(());
        }
    };

    // Check if alert belongs to user
    let user_alerts = tiingo.get_user_alerts(ctx.author().id.get());
    if !user_alerts.iter().any(|a| a.id == id) {
        send_embed(
            ctx,
            CreateEmbed::new()
                .title("Not Found")
                .description(format!("Alert #{} not found or doesn't belong to you", id))
                .color(0xff0000),
        )
        .await?;
        return Ok(());
    }

    tiingo.remove_alert(id);

    send_embed(
        ctx,
        CreateEmbed::new()
            .title("Alert Removed")
            .description(format!("Alert **#{}** has been removed", id))
            .color(0x00ff00),
    )
    .await?;

    Ok(())
}
