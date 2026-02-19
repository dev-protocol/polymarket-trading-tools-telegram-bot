//! Check wallet statistics on Polymarket
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use polymarket_copy_trading_bot_rust::config::load_env;
use polymarket_copy_trading_bot_rust::utils::{fetch_data, get_my_balance};

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
    slug: Option<String>,
    outcome: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
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
    println!("🔍 Checking your wallet statistics on Polymarket\n");
    
    let env = load_env()?;
    println!("Wallet: {}\n", env.proxy_wallet);
    println!("{}\n", "━".repeat(65));

    // 1. USDC Balance
    println!("💰 USDC BALANCE");
    let balance = get_my_balance(&env.proxy_wallet, &env).await?;
    println!("   Available: ${:.2}\n", balance);

    // 2. Open Positions
    println!("📊 OPEN POSITIONS");
    let positions_url = format!("https://data-api.polymarket.com/positions?user={}", env.proxy_wallet);
    let positions_json: serde_json::Value = fetch_data(&positions_url, &env).await?;
    let positions: Vec<Position> = positions_json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();

    if !positions.is_empty() {
        println!("   Total positions: {}\n", positions.len());

        let total_value: f64 = positions.iter().map(|p| p.current_value).sum();
        let total_initial_value: f64 = positions.iter().map(|p| p.initial_value).sum();
        let total_unrealized_pnl: f64 = positions.iter().map(|p| p.cash_pnl).sum();
        let total_realized_pnl: f64 = positions.iter().map(|p| p.realized_pnl).sum();

        println!("   💵 Current value: ${:.2}", total_value);
        println!("   💵 Initial value: ${:.2}", total_initial_value);
        if total_initial_value > 0.0 {
            println!(
                "   📈 Unrealized P&L: ${:.2} ({:.2}%)",
                total_unrealized_pnl,
                (total_unrealized_pnl / total_initial_value) * 100.0
            );
        }
        println!("   ✅ Realized P&L: ${:.2}\n", total_realized_pnl);

        // Top 5 positions by profit
        println!("   🏆 Top-5 positions by profit:\n");
        let mut top_positions = positions.clone();
        top_positions.sort_by(|a, b| b.percent_pnl.partial_cmp(&a.percent_pnl).unwrap_or(std::cmp::Ordering::Equal));
        top_positions.truncate(5);

        for (idx, pos) in top_positions.iter().enumerate() {
            let pnl_icon = if pos.percent_pnl >= 0.0 { "📈" } else { "📉" };
            println!("   {}. {} {}", idx + 1, pnl_icon, pos.title.as_deref().unwrap_or("Unknown"));
            println!("      {}", pos.outcome.as_deref().unwrap_or("N/A"));
            println!(
                "      Size: {:.2} tokens @ ${:.3}",
                pos.size, pos.avg_price
            );
            println!(
                "      P&L: ${:.2} ({:.2}%)",
                pos.cash_pnl, pos.percent_pnl
            );
            println!("      Current price: ${:.3}", pos.cur_price);
            if let Some(slug) = &pos.slug {
                println!("      📍 https://polymarket.com/event/{}", slug);
            }
            println!();
        }
    } else {
        println!("   ❌ No open positions found\n");
    }

    // 3. Trade History (last 20)
    println!("{}\n", "━".repeat(65));
    println!("📜 TRADE HISTORY (last 20)\n");
    let activity_url = format!(
        "https://data-api.polymarket.com/activity?user={}&type=TRADE",
        env.proxy_wallet
    );
    let activities_json: serde_json::Value = fetch_data(&activity_url, &env).await?;
    let activities: Vec<Activity> = activities_json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();

    if !activities.is_empty() {
        println!("   Total trades in API: {}\n", activities.len());

        // Trade statistics
        let buy_trades: Vec<&Activity> = activities.iter().filter(|a| a.side == "BUY").collect();
        let sell_trades: Vec<&Activity> = activities.iter().filter(|a| a.side == "SELL").collect();
        let total_buy_volume: f64 = buy_trades.iter().map(|t| t.usdc_size).sum();
        let total_sell_volume: f64 = sell_trades.iter().map(|t| t.usdc_size).sum();

        println!("   📊 Trade statistics:");
        println!(
            "      • Buys: {} (volume: ${:.2})",
            buy_trades.len(), total_buy_volume
        );
        println!(
            "      • Sells: {} (volume: ${:.2})",
            sell_trades.len(), total_sell_volume
        );
        println!(
            "      • Total volume: ${:.2}\n",
            total_buy_volume + total_sell_volume
        );

        // Last 20 trades
        let recent_trades: Vec<&Activity> = activities.iter().take(20).collect();
        println!("   📝 Last 20 trades:\n");

        for (idx, trade) in recent_trades.iter().enumerate() {
            let date = chrono::DateTime::from_timestamp(trade.timestamp, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            let side_icon = if trade.side == "BUY" { "🟢" } else { "🔴" };
            println!("   {}. {} {} - {}", idx + 1, side_icon, trade.side, date);
            println!("      {}", trade.title.as_deref().unwrap_or("Unknown Market"));
            println!("      {}", trade.outcome.as_deref().unwrap_or("N/A"));
            println!(
                "      Volume: ${:.2} @ ${:.3}",
                trade.usdc_size, trade.price
            );
            if trade.transaction_hash.len() >= 18 {
                println!(
                    "      TX: {}...{}",
                    &trade.transaction_hash[..10],
                    &trade.transaction_hash[trade.transaction_hash.len() - 8..]
                );
            }
            println!("      🔗 https://polygonscan.com/tx/{}", trade.transaction_hash);
            println!();
        }
    } else {
        println!("   ❌ Trade history not found\n");
    }

    // 4. Why no P&L charts
    println!("{}\n", "━".repeat(65));
    println!("❓ WHY NO P&L CHARTS ON POLYMARKET?\n");
    println!("   Profit/Loss charts on Polymarket only show REALIZED");
    println!("   profit (closed positions). This is why it shows $0.00:\n");

    if !positions.is_empty() {
        let total_realized_pnl: f64 = positions.iter().map(|p| p.realized_pnl).sum();
        let total_unrealized_pnl: f64 = positions.iter().map(|p| p.cash_pnl).sum();

        println!("   ✅ Realized P&L (closed positions):");
        println!("      → ${:.2} ← THIS is displayed on the chart\n", total_realized_pnl);

        println!("   📊 Unrealized P&L (open positions):");
        println!(
            "      → ${:.2} ← THIS is NOT displayed on the chart\n",
            total_unrealized_pnl
        );

        if total_realized_pnl == 0.0 {
            println!("   💡 Solution: To see charts, you need to:");
            println!("      1. Close several positions with profit");
            println!("      2. Wait 5-10 minutes for Polymarket API to update");
            println!("      3. P&L chart will start displaying data\n");
        }
    }

    println!("{}\n", "━".repeat(65));
    println!("✅ Check completed!\n");
    println!("📱 Your profile: https://polymarket.com/profile/{}\n", env.proxy_wallet);

    Ok(())
}

