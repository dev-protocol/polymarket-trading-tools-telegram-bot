//! Check recent trading activity
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use polymarket_copy_trading_bot_rust::config::load_env;
use polymarket_copy_trading_bot_rust::utils::fetch_data;

#[derive(Debug, serde::Deserialize)]
struct Activity {
    proxy_wallet: String,
    timestamp: i64,
    condition_id: String,
    #[serde(rename = "type")]
    activity_type: String,
    size: f64,
    usdc_size: f64,
    transaction_hash: String,
    price: f64,
    asset: String,
    side: String,
    title: Option<String>,
    market: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let env = load_env()?;
    let url = format!("https://data-api.polymarket.com/activity?user={}&type=TRADE", env.proxy_wallet);
    let activities_json: serde_json::Value = fetch_data(&url, &env).await?;
    let activities: Vec<Activity> = activities_json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();

    if activities.is_empty() {
        println!("No trade data available");
        return Ok(());
    }

    // Redemption ended at 18:14:16 UTC (October 31, 2025)
    let redemption_end_time = chrono::DateTime::parse_from_rfc3339("2025-10-31T18:14:16Z")
        .unwrap()
        .timestamp();

    println!("{}", "═".repeat(65));
    println!("📋 CLOSED POSITIONS (Redeemed October 31, 2025 at 18:00-18:14)");
    println!("{}\n", "═".repeat(65));
    println!("💰 TOTAL RECEIVED FROM REDEMPTION: $66.37 USDC\n");

    println!("{}", "═".repeat(65));
    println!("🛒 PURCHASES AFTER REDEMPTION (after 18:14 UTC October 31)");
    println!("{}\n", "═".repeat(65));

    let trades_after_redemption: Vec<_> = activities
        .iter()
        .filter(|t| t.timestamp > redemption_end_time && t.side == "BUY")
        .collect();

    if trades_after_redemption.is_empty() {
        println!("✅ No purchases after redemption!\n");
        println!("This means funds should be in the balance.");
        return Ok(());
    }

    let mut total_spent = 0.0;
    for (i, trade) in trades_after_redemption.iter().enumerate() {
        let date = chrono::DateTime::from_timestamp(trade.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let value = trade.usdc_size;
        total_spent += value;

        println!("{}. 🟢 BOUGHT: {}", i + 1, trade.title.as_deref().or(trade.market.as_deref()).unwrap_or("Unknown"));
        println!("   💸 Spent: ${:.2}", value);
        println!("   📊 Size: {:.2} tokens @ ${:.4}", trade.size, trade.price);
        println!("   📅 Date: {}", date);
        if trade.transaction_hash.len() >= 20 {
            println!("   🔗 TX: https://polygonscan.com/tx/{}...\n", &trade.transaction_hash[..20]);
        }
    }

    println!("{}", "═".repeat(65));
    println!("📊 TOTAL PURCHASES AFTER REDEMPTION:");
    println!("   Number of trades: {}", trades_after_redemption.len());
    println!("   💸 SPENT: ${:.2} USDC", total_spent);
    println!("{}\n", "═".repeat(65));

    println!("💡 EXPLANATION OF WHERE THE MONEY WENT:\n");
    println!("   ✅ Received from redemption: +$66.37");
    println!("   ❌ Spent on new purchases: -${:.2}", total_spent);
    println!("   📊 Balance change: ${:.2}", 66.37 - total_spent);
    println!("\n{}\n", "═".repeat(65));

    // Show recent sales too
    println!("💵 RECENT SALES:\n");
    let recent_sells: Vec<_> = activities.iter().filter(|t| t.side == "SELL").take(10).collect();

    let mut total_sold = 0.0;
    for (i, trade) in recent_sells.iter().enumerate() {
        let date = chrono::DateTime::from_timestamp(trade.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let value = trade.usdc_size;
        total_sold += value;

        println!("{}. 🔴 SOLD: {}", i + 1, trade.title.as_deref().or(trade.market.as_deref()).unwrap_or("Unknown"));
        println!("   💰 Received: ${:.2}", value);
        println!("   📅 Date: {}\n", date);
    }

    println!("{}", "═".repeat(65));
    println!("💵 Sold in recent trades: ${:.2}", total_sold);
    println!("{}", "═".repeat(65));

    Ok(())
}

