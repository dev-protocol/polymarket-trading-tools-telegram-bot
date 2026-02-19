//! Find low-risk traders
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
    println!("🔍 Finding Low-Risk Traders");
    println!("════════════════════════════════════════════════════\n");

    let env = load_env()?;

    println!("📊 Fetching trader leaderboard...\n");

    let leaderboard_url = "https://data-api.polymarket.com/leaderboard?category=OVERALL&timePeriod=MONTH&orderBy=PNL&limit=100";
    match fetch_data(leaderboard_url, &env).await {
        Ok(leaderboard_json) => {
            let entries: Vec<LeaderboardEntry> = leaderboard_json
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| serde_json::from_value(v).ok())
                .collect();

            // Filter for low-risk traders (high win rate, positive P&L, reasonable volume)
            let low_risk: Vec<_> = entries
                .iter()
                .filter(|e| {
                    let win_rate = e.win_rate.unwrap_or(0.0);
                    let pnl = e.pnl.unwrap_or(0.0);
                    let trades = e.total_trades.unwrap_or(0);
                    win_rate >= 0.55 && pnl > 0.0 && trades >= 50
                })
                .take(20)
                .collect();

            if low_risk.is_empty() {
                println!("❌ No low-risk traders found");
                return Ok(());
            }

            println!("✅ Found {} low-risk traders\n", low_risk.len());
            println!("{}", "━".repeat(65));
            println!("📊 LOW-RISK TRADERS (Win Rate >= 55%, Positive P&L, 50+ trades):\n");

            for (i, entry) in low_risk.iter().enumerate() {
                let pnl = entry.pnl.unwrap_or(0.0);
                let win_rate = entry.win_rate.unwrap_or(0.0);
                let trades = entry.total_trades.unwrap_or(0);

                println!("{}. {}", i + 1, &entry.address[..10]);
                println!("   P&L: ${:.2}", pnl);
                println!("   Win Rate: {:.1}%", win_rate * 100.0);
                println!("   Trades: {}\n", trades);
            }

            println!("{}", "━".repeat(65));
        }
        Err(e) => {
            println!("⚠️  Failed to fetch leaderboard: {}", e);
        }
    }

    Ok(())
}

