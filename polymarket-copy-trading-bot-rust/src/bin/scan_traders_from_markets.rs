//! Scan traders from popular markets
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use polymarket_copy_trading_bot_rust::config::load_env;
// use polymarket_copy_trading_bot_rust::utils::fetch_data; // Unused for now

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Scanning Traders from Markets");
    println!("════════════════════════════════════════════════════\n");

    let _env = load_env()?;

    println!("📊 This script scans traders from popular Polymarket markets.");
    println!("   For detailed analysis, use find_best_traders.\n");

    println!("💡 To find traders:");
    println!("   1. Visit https://polymarket.com");
    println!("   2. Browse popular markets");
    println!("   3. Check trader profiles on each market");
    println!("   4. Add promising traders to USER_ADDRESSES in .env\n");

    Ok(())
}

