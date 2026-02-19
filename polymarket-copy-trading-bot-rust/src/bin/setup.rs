//! Interactive setup wizard for creating .env file
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use std::fs;
use std::io::{self, Write};
use colored::*;

fn print_header() {
    println!("{}", "━".repeat(65).cyan().bold());
    println!("{}", "     🤖 POLYMARKET COPY TRADING BOT - SETUP WIZARD".cyan().bold());
    println!("{}\n", "━".repeat(65).cyan().bold());
    println!("{}", "This wizard will help you create your .env configuration file.".yellow());
    println!("{}", "Press Ctrl+C at any time to cancel.\n".yellow());
}

fn question(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn is_valid_ethereum_address(address: &str) -> bool {
    address.starts_with("0x") && address.len() == 42 && address[2..].chars().all(|c| c.is_ascii_hexdigit())
}

fn is_valid_private_key(key: &str) -> bool {
    let key = key.trim_start_matches("0x");
    key.len() == 64 && key.chars().all(|c| c.is_ascii_hexdigit())
}

#[tokio::main]
async fn main() -> Result<()> {
    print_header();

    println!("{}", "\n━━━ STEP 1: TRADERS TO COPY ━━━\n".blue().bold());
    println!("{}", "Find top traders on:".cyan());
    println!("  • https://polymarket.com/leaderboard");
    println!("  • https://predictfolio.com\n");
    println!("{}", "Tip: Look for traders with:".yellow());
    println!("  • Positive P&L (green numbers)");
    println!("  • Win rate above 55%");
    println!("  • Recent trading activity\n");

    let mut addresses = Vec::new();
    loop {
        let address = question(&format!("{}Enter trader wallet address {} (or press Enter to finish): ", "".green(), addresses.len() + 1))?;
        if address.is_empty() {
            break;
        }
        if is_valid_ethereum_address(&address) {
            addresses.push(address);
            println!("{}", "  ✅ Address added\n".green());
        } else {
            println!("{}", "  ❌ Invalid address format. Must be 0x followed by 40 hex characters.\n".red());
        }
    }

    if addresses.is_empty() {
        println!("{}", "❌ At least one trader address is required!".red());
        return Ok(());
    }

    println!("{}", "\n━━━ STEP 2: WALLET CONFIGURATION ━━━\n".blue().bold());
    
    let proxy_wallet = loop {
        let addr = question("Enter your PROXY_WALLET address: ")?;
        if is_valid_ethereum_address(&addr) {
            break addr;
        }
        println!("{}", "  ❌ Invalid address format.\n".red());
    };

    let private_key = loop {
        let key = question("Enter your PRIVATE_KEY (without 0x prefix): ")?;
        if is_valid_private_key(&key) {
            break if key.starts_with("0x") { key } else { format!("0x{}", key) };
        }
        println!("{}", "  ❌ Invalid private key format. Must be 64 hex characters.\n".red());
    };

    println!("{}", "\n━━━ STEP 3: DATABASE CONFIGURATION ━━━\n".blue().bold());
    let mongo_uri = question("Enter MongoDB URI (or press Enter for default): ")?;
    let mongo_uri = if mongo_uri.is_empty() {
        "mongodb://localhost:27017/polymarket_bot".to_string()
    } else {
        mongo_uri
    };

    println!("{}", "\n━━━ STEP 4: NETWORK CONFIGURATION ━━━\n".blue().bold());
    let rpc_url = question("Enter Polygon RPC URL (or press Enter for default): ")?;
    let rpc_url = if rpc_url.is_empty() {
        "https://polygon-rpc.com".to_string()
    } else {
        rpc_url
    };

    let clob_http_url = question("Enter CLOB HTTP URL (or press Enter for default): ")?;
    let clob_http_url = if clob_http_url.is_empty() {
        "https://clob.polymarket.com".to_string()
    } else {
        clob_http_url
    };

    let clob_ws_url = question("Enter CLOB WebSocket URL (or press Enter for default): ")?;
    let clob_ws_url = if clob_ws_url.is_empty() {
        "wss://clob-ws.polymarket.com".to_string()
    } else {
        clob_ws_url
    };

    println!("{}", "\n━━━ STEP 5: TRADING CONFIGURATION ━━━\n".blue().bold());
    let copy_strategy = question("Copy strategy (PERCENTAGE/FIXED/ADAPTIVE, default: PERCENTAGE): ")?;
    let copy_strategy = if copy_strategy.is_empty() { "PERCENTAGE".to_string() } else { copy_strategy };

    let copy_size = question("Copy size/percentage (default: 0.1 for 10%): ")?;
    let copy_size = if copy_size.is_empty() { "0.1".to_string() } else { copy_size };

    println!("{}", "\n━━━ STEP 6: TAKE PROFIT / STOP LOSS (OPTIONAL) ━━━\n".blue().bold());
    println!("{}", "Configure automatic position closing based on profit/loss thresholds.".cyan());
    println!("{}", "Leave empty to disable TP/SL.\n".yellow());
    
    let take_profit = question("Take Profit % (e.g., 10.0 for 10% profit, or press Enter to skip): ")?;
    let stop_loss = question("Stop Loss % (e.g., 10.0 for 10% loss, or press Enter to skip): ")?;

    // Generate .env file
    let env_content = format!(r#"# Polymarket Copy Trading Bot Configuration
# Generated by setup wizard

# Traders to copy (comma-separated)
USER_ADDRESSES={}

# Wallet Configuration
PROXY_WALLET={}
PRIVATE_KEY={}

# Database
MONGO_URI={}

# Network
RPC_URL={}
CLOB_HTTP_URL={}
CLOB_WS_URL={}
USDC_CONTRACT_ADDRESS=0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174

# Trading Strategy
COPY_STRATEGY={}
COPY_SIZE={}
TRADE_MULTIPLIER=1.0

# Take Profit / Stop Loss (optional - leave unset to disable)
{}
{}

# Advanced (optional - defaults will be used if not set)
# FETCH_INTERVAL=1
# RETRY_LIMIT=3
# TRADE_AGGREGATION_ENABLED=true
# PREVIEW_MODE=false
"#,
        addresses.join(","),
        proxy_wallet,
        private_key,
        mongo_uri,
        rpc_url,
        clob_http_url,
        clob_ws_url,
        copy_strategy,
        copy_size,
        if take_profit.is_empty() {
            "# TAKE_PROFIT_PERCENT=".to_string()
        } else {
            format!("TAKE_PROFIT_PERCENT={}", take_profit)
        },
        if stop_loss.is_empty() {
            "# STOP_LOSS_PERCENT=".to_string()
        } else {
            format!("STOP_LOSS_PERCENT={}", stop_loss)
        }
    );

    fs::write(".env", env_content)?;

    println!("\n{}", "━".repeat(65).green());
    println!("{}", "✅ Configuration saved to .env file!".green().bold());
    println!("{}\n", "━".repeat(65).green());
    println!("{}", "Next steps:".yellow());
    println!("  1. Review the .env file");
    println!("  2. Run: cargo run --bin health_check");
    println!("  3. Start the bot: cargo run --release\n");

    Ok(())
}

