//! Check positions with detailed information
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use polymarket_copy_trading_bot_rust::config::load_env;
use polymarket_copy_trading_bot_rust::utils::fetch_data;

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
    println!("\n📊 CURRENT POSITIONS:\n");

    let env = load_env()?;
    let positions_url = format!("https://data-api.polymarket.com/positions?user={}", env.proxy_wallet);
    let positions_json: serde_json::Value = fetch_data(&positions_url, &env).await?;
    let positions: Vec<Position> = positions_json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();

    if positions.is_empty() {
        println!("❌ No open positions");
        return Ok(());
    }

    println!("✅ Found positions: {}\n", positions.len());

    // Sort by current value
    let mut sorted = positions.clone();
    sorted.sort_by(|a, b| b.current_value.partial_cmp(&a.current_value).unwrap_or(std::cmp::Ordering::Equal));

    let mut total_value = 0.0;
    for pos in &sorted {
        total_value += pos.current_value;
        println!("{}", "━".repeat(50));
        println!("Market: {}", pos.title.as_deref().unwrap_or("Unknown"));
        println!("Outcome: {}", pos.outcome.as_deref().unwrap_or("Unknown"));
        if pos.asset.len() >= 10 {
            println!("Asset ID: {}...", &pos.asset[..10]);
        }
        println!("Size: {:.2} shares", pos.size);
        println!("Avg Price: ${:.4}", pos.avg_price);
        println!("Current Price: ${:.4}", pos.cur_price);
        println!("Initial Value: ${:.2}", pos.initial_value);
        println!("Current Value: ${:.2}", pos.current_value);
        println!("PnL: ${:.2} ({:.2}%)", pos.cash_pnl, pos.percent_pnl);
        if let Some(slug) = &pos.slug {
            println!("URL: https://polymarket.com/event/{}", slug);
        }
    }

    println!("\n{}", "━".repeat(50));
    println!("💰 TOTAL CURRENT VALUE: ${:.2}", total_value);
    println!("{}\n", "━".repeat(50));

    // Identify large positions (greater than $5)
    let large_positions: Vec<_> = sorted.iter().filter(|p| p.current_value > 5.0).collect();

    if !large_positions.is_empty() {
        println!("\n🎯 LARGE POSITIONS (> $5): {}\n", large_positions.len());
        for pos in &large_positions {
            println!(
                "• {} [{}]: ${:.2} ({:.2} shares @ ${:.4})",
                pos.title.as_deref().unwrap_or("Unknown"),
                pos.outcome.as_deref().unwrap_or("Unknown"),
                pos.current_value,
                pos.size,
                pos.cur_price
            );
        }

        println!("\n💡 To sell 80% of these positions, use:\n");
        println!("   cargo run --bin manual_sell\n");

        println!("📋 Data for selling:\n");
        for pos in &large_positions {
            let sell_size = (pos.size * 0.8).floor();
            println!("   Asset ID: {}", pos.asset);
            println!("   Size to sell: {} (80% of {:.2})", sell_size, pos.size);
            println!("   Market: {} [{}]", pos.title.as_deref().unwrap_or("Unknown"), pos.outcome.as_deref().unwrap_or("Unknown"));
            println!();
        }
    } else {
        println!("\n✅ No large positions (> $5)");
    }

    Ok(())
}

