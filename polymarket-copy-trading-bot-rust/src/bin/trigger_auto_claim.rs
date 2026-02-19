//! Manually trigger auto-claim check
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use polymarket_copy_trading_bot_rust::config::load_env;
use polymarket_copy_trading_bot_rust::services::auto_claim::trigger_auto_claim;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Manually triggering auto-claim check...\n");
    
    let env = load_env()?;
    
    match trigger_auto_claim(&env).await {
        Ok(_) => {
            println!("\n✅ Auto-claim check completed");
            Ok(())
        }
        Err(e) => {
            eprintln!("\n❌ Auto-claim check failed: {}", e);
            std::process::exit(1);
        }
    }
}

