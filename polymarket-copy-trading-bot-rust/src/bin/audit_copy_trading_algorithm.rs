//! Audit copy trading algorithm
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use polymarket_copy_trading_bot_rust::config::load_env;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Audit Copy Trading Algorithm");
    println!("════════════════════════════════════════════════════\n");

    let env = load_env()?;

    println!("📊 Current Configuration:\n");
    println!("   Copy Strategy: {:?}", env.copy_strategy_config.strategy);
    println!("   Copy Size: {}", env.copy_percentage);
    println!("   Trade Multiplier: {}", env.trade_multiplier);
    println!("   Retry Limit: {}", env.retry_limit);
    println!("\n💡 This script audits the copy trading algorithm logic.");
    println!("   For detailed auditing, review the source code in:");
    println!("   • src/services/trade_executor.rs");
    println!("   • src/config/copy_strategy.rs\n");

    Ok(())
}

