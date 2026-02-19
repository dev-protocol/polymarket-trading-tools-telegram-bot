//! Check both wallet addresses
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use polymarket_copy_trading_bot_rust::config::load_env;
use polymarket_copy_trading_bot_rust::utils::{fetch_data, get_my_balance};

#[derive(Debug, serde::Deserialize)]
struct Activity {
    proxy_wallet: Option<String>,
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
    slug: Option<String>,
    outcome: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct Position {
    asset: String,
    condition_id: String,
    size: f64,
    avg_price: f64,
    initial_value: f64,
    current_value: f64,
    cash_pnl: f64,
    percent_pnl: f64,
    total_bought: f64,
    realized_pnl: f64,
    percent_realized_pnl: f64,
    cur_price: f64,
    title: Option<String>,
    slug: Option<String>,
    outcome: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 CHECKING BOTH ADDRESSES\n");
    println!("{}\n", "━".repeat(65));

    let env = load_env()?;
    
    // Addresses to compare (can be customized via env or args)
    let address_1 = env.proxy_wallet.clone();
    let address_2 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "0xd62531bc536bff72394fc5ef715525575787e809".to_string());

    // Check first address (from .env)
    println!("📊 ADDRESS 1 (from .env - PROXY_WALLET):\n");
    println!("   {}", address_1);
    println!("   Profile: https://polymarket.com/profile/{}\n", address_1);

    let addr1_activities_url = format!("https://data-api.polymarket.com/activity?user={}&type=TRADE", address_1);
    let addr1_activities_json: serde_json::Value = fetch_data(&addr1_activities_url, &env).await?;
    let addr1_activities: Vec<Activity> = addr1_activities_json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();

    let addr1_positions_url = format!("https://data-api.polymarket.com/positions?user={}", address_1);
    let addr1_positions_json: serde_json::Value = fetch_data(&addr1_positions_url, &env).await?;
    let addr1_positions: Vec<Position> = addr1_positions_json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();

    println!("   • Trades in API: {}", addr1_activities.len());
    println!("   • Positions in API: {}", addr1_positions.len());

    if !addr1_activities.is_empty() {
        let buy_trades: Vec<_> = addr1_activities.iter().filter(|a| a.side == "BUY").collect();
        let sell_trades: Vec<_> = addr1_activities.iter().filter(|a| a.side == "SELL").collect();
        let total_volume: f64 = buy_trades.iter().map(|t| t.usdc_size).sum::<f64>() +
            sell_trades.iter().map(|t| t.usdc_size).sum::<f64>();

        println!("   • Buys: {}", buy_trades.len());
        println!("   • Sells: {}", sell_trades.len());
        println!("   • Volume: ${:.2}", total_volume);

        if let Some(proxy_wallet) = &addr1_activities[0].proxy_wallet {
            println!("   • proxyWallet in trades: {}", proxy_wallet);
        }
    }

    // Balance
    match get_my_balance(&address_1, &env).await {
        Ok(balance) => println!("   • USDC Balance: ${:.2}", balance),
        Err(_) => println!("   • USDC Balance: failed to get"),
    }

    println!("\n{}\n", "━".repeat(65));

    // Check second address
    println!("📊 ADDRESS 2:\n");
    println!("   {}", address_2);
    println!("   Profile: https://polymarket.com/profile/{}\n", address_2);

    let addr2_activities_url = format!("https://data-api.polymarket.com/activity?user={}&type=TRADE", address_2);
    let addr2_activities_json: serde_json::Value = fetch_data(&addr2_activities_url, &env).await?;
    let addr2_activities: Vec<Activity> = addr2_activities_json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();

    let addr2_positions_url = format!("https://data-api.polymarket.com/positions?user={}", address_2);
    let addr2_positions_json: serde_json::Value = fetch_data(&addr2_positions_url, &env).await?;
    let addr2_positions: Vec<Position> = addr2_positions_json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();

    println!("   • Trades in API: {}", addr2_activities.len());
    println!("   • Positions in API: {}", addr2_positions.len());

    if !addr2_activities.is_empty() {
        let buy_trades: Vec<_> = addr2_activities.iter().filter(|a| a.side == "BUY").collect();
        let sell_trades: Vec<_> = addr2_activities.iter().filter(|a| a.side == "SELL").collect();
        let total_volume: f64 = buy_trades.iter().map(|t| t.usdc_size).sum::<f64>() +
            sell_trades.iter().map(|t| t.usdc_size).sum::<f64>();

        println!("   • Buys: {}", buy_trades.len());
        println!("   • Sells: {}", sell_trades.len());
        println!("   • Volume: ${:.2}", total_volume);

        if let Some(proxy_wallet) = &addr2_activities[0].proxy_wallet {
            println!("   • proxyWallet in trades: {}", proxy_wallet);
        }

        println!("\n   📝 Last 5 trades:");
        for (idx, trade) in addr2_activities.iter().take(5).enumerate() {
            let date = chrono::DateTime::from_timestamp(trade.timestamp, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            println!("      {}. {} - {}", idx + 1, trade.side, trade.title.as_deref().unwrap_or("Unknown"));
            println!("         ${:.2} @ {}", trade.usdc_size, date);
            if trade.transaction_hash.len() >= 18 {
                println!("         TX: {}...{}", &trade.transaction_hash[..10], &trade.transaction_hash[trade.transaction_hash.len()-6..]);
            }
        }
    }

    // Balance
    match get_my_balance(&address_2, &env).await {
        Ok(balance) => println!("\n   • USDC Balance: ${:.2}", balance),
        Err(_) => println!("\n   • USDC Balance: failed to get"),
    }

    println!("\n{}\n", "━".repeat(65));

    // Comparison
    println!("🔍 ADDRESS COMPARISON:\n");

    let addr1_has_data = !addr1_activities.is_empty() || !addr1_positions.is_empty();
    let addr2_has_data = !addr2_activities.is_empty() || !addr2_positions.is_empty();

    println!("   Address 1 ({}...):", &address_1[..8.min(address_1.len())]);
    println!("   {}", if addr1_has_data { "✅ Has data" } else { "❌ No data" });
    println!("   • Trades: {}", addr1_activities.len());
    println!("   • Positions: {}\n", addr1_positions.len());

    println!("   Address 2 ({}...):", &address_2[..8.min(address_2.len())]);
    println!("   {}", if addr2_has_data { "✅ Has data" } else { "❌ No data" });
    println!("   • Trades: {}", addr2_activities.len());
    println!("   • Positions: {}\n", addr2_positions.len());

    // Check connection through proxyWallet field
    println!("{}\n", "━".repeat(65));
    println!("🔗 CONNECTION BETWEEN ADDRESSES:\n");

    if let (Some(proxy1), Some(proxy2)) = (
        addr1_activities.first().and_then(|a| a.proxy_wallet.as_ref()),
        addr2_activities.first().and_then(|a| a.proxy_wallet.as_ref()),
    ) {
        let proxy1_lower = proxy1.to_lowercase();
        let proxy2_lower = proxy2.to_lowercase();

        println!("   Address 1 uses proxyWallet: {}", proxy1);
        println!("   Address 2 uses proxyWallet: {}\n", proxy2);

        if proxy1_lower == proxy2_lower {
            println!("   ✅ BOTH ADDRESSES LINKED TO ONE PROXY WALLET!\n");
            println!("   This explains why profiles show the same data.\n");
        } else if proxy1_lower == address_2.to_lowercase() {
            println!("   🎯 CONNECTION FOUND!\n");
            println!("   Address 1 ({}...) uses", &address_1[..8.min(address_1.len())]);
            println!("   Address 2 ({}...) as proxy wallet!\n", &address_2[..8.min(address_2.len())]);
        } else if proxy2_lower == address_1.to_lowercase() {
            println!("   🎯 CONNECTION FOUND!\n");
            println!("   Address 2 ({}...) uses", &address_2[..8.min(address_2.len())]);
            println!("   Address 1 ({}...) as proxy wallet!\n", &address_1[..8.min(address_1.len())]);
        } else {
            println!("   ⚠️  Addresses use different proxy wallets\n");
        }
    }

    println!("{}\n", "━".repeat(65));

    // Summary
    println!("✅ SUMMARY AND SOLUTION:\n");

    if addr2_has_data && !addr1_has_data {
        println!("   🎯 YOUR BOT IS USING THE WRONG ADDRESS!\n");
        println!("   All trading goes through address:");
        println!("   {}\n", address_2);
        println!("   But .env specifies:");
        println!("   {}\n", address_1);
        println!("   🔧 SOLUTION: Update .env file:\n");
        println!("   PROXY_WALLET={}\n", address_2);
    } else if addr1_has_data && !addr2_has_data {
        println!("   ✅ Bot is working correctly!");
        println!("   Trading goes through address from .env\n");
    } else if addr1_has_data && addr2_has_data {
        println!("   ⚠️  Activity on BOTH addresses!\n");
        println!("   Possible reasons:");
        println!("   1. You switched wallets");
        println!("   2. Traded manually from one, with bot from another");
        println!("   3. Both addresses linked through Polymarket proxy system\n");
    } else {
        println!("   ❌ No data on any address!\n");
        println!("   Check address correctness.\n");
    }

    println!("{}\n", "━".repeat(65));

    Ok(())
}

