//! Find best performing traders
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use polymarket_copy_trading_bot_rust::config::load_env;
use polymarket_copy_trading_bot_rust::utils::fetch_data;

#[derive(Debug, serde::Deserialize)]
struct LeaderboardEntry {
    address: String,
    pnl: Option<f64>,
    win_rate: Option<f64>,
    total_trades: Option<u64>,
    volume: Option<f64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Finding Best Traders");
    println!("════════════════════════════════════════════════════\n");

    let env = load_env()?;

    // Fetch leaderboard from Polymarket
    println!("📊 Fetching trader leaderboard from Polymarket...\n");

    let leaderboard_url = "https://data-api.polymarket.com/leaderboard?category=OVERALL&timePeriod=MONTH&orderBy=PNL&limit=50";
    match fetch_data(leaderboard_url, &env).await {
        Ok(leaderboard_json) => {
            let entries: Vec<LeaderboardEntry> = leaderboard_json
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| serde_json::from_value(v).ok())
                .collect();

            if entries.is_empty() {
                println!("❌ No traders found in leaderboard");
                return Ok(());
            }

            println!("✅ Found {} traders\n", entries.len());
            println!("{}", "━".repeat(65));
            println!("📊 TOP TRADERS:\n");

            for (i, entry) in entries.iter().take(20).enumerate() {
                let pnl = entry.pnl.unwrap_or(0.0);
                let win_rate = entry.win_rate.unwrap_or(0.0);
                let trades = entry.total_trades.unwrap_or(0);
                let volume = entry.volume.unwrap_or(0.0);

                println!("{}. {}", i + 1, &entry.address[..10]);
                println!("   P&L: ${:.2}", pnl);
                println!("   Win Rate: {:.1}%", win_rate * 100.0);
                println!("   Trades: {}", trades);
                println!("   Volume: ${:.2}\n", volume);
            }

            println!("{}", "━".repeat(65));
            println!("\n💡 To copy these traders, add their addresses to USER_ADDRESSES in .env");
            println!("   Example: USER_ADDRESSES={}", entries.iter().take(3).map(|e| e.address.clone()).collect::<Vec<_>>().join(","));
        }
        Err(e) => {
            println!("⚠️  Failed to fetch leaderboard: {}", e);
            println!("\n💡 Alternative: Check https://polymarket.com/leaderboard manually");
        }
    }

    Ok(())
}

