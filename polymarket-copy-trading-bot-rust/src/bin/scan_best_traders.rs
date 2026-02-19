//! Scan and analyze top traders
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use polymarket_copy_trading_bot_rust::config::load_env;
// use polymarket_copy_trading_bot_rust::utils::fetch_data; // Unused for now

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Scanning Best Traders");
    println!("════════════════════════════════════════════════════\n");

    let _env = load_env()?;

    println!("📊 This script analyzes top traders from Polymarket leaderboard.");
    println!("   For detailed analysis, use find_best_traders or find_low_risk_traders.\n");

    println!("💡 Available trader analysis scripts:");
    println!("   • cargo run --bin find_best_traders");
    println!("   • cargo run --bin find_low_risk_traders");
    println!("   • cargo run --bin scan_traders_from_markets\n");

    Ok(())
}

