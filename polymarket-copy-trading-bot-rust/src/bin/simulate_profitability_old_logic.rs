//! Simulate with old algorithm (ratio-based)
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use colored::*;
use polymarket_copy_trading_bot_rust::config::load_env;
use polymarket_copy_trading_bot_rust::utils::fetch_data;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Trade {
    id: String,
    timestamp: i64,
    market: Option<String>,
    asset: String,
    side: String,
    price: f64,
    #[serde(rename = "usdcSize")]
    usdc_size: f64,
    size: f64,
    outcome: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Position {
    #[serde(rename = "conditionId")]
    condition_id: Option<String>,
    market: Option<String>,
    outcome: Option<String>,
    #[serde(rename = "outcomeIndex")]
    outcome_index: Option<i32>,
    asset: String,
    size: f64,
    cost: Option<f64>,
    #[serde(rename = "avgEntryPrice")]
    avg_entry_price: Option<f64>,
    #[serde(rename = "currentValue")]
    current_value: f64,
    #[serde(rename = "realizedPnl")]
    realized_pnl: Option<f64>,
    #[serde(rename = "unrealizedPnl")]
    unrealized_pnl: Option<f64>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SimulatedPosition {
    market: String,
    outcome: String,
    #[serde(rename = "entryPrice")]
    entry_price: f64,
    #[serde(rename = "exitPrice")]
    exit_price: Option<f64>,
    invested: f64,
    #[serde(rename = "currentValue")]
    current_value: f64,
    pnl: f64,
    closed: bool,
    trades: Vec<PositionTrade>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PositionTrade {
    timestamp: i64,
    side: String,
    price: f64,
    size: f64,
    #[serde(rename = "usdcSize")]
    usdc_size: f64,
    #[serde(rename = "traderBalance")]
    trader_balance: f64,
    #[serde(rename = "yourBalance")]
    your_balance: f64,
    ratio: f64,
    #[serde(rename = "yourSize")]
    your_size: f64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SimulationResult {
    #[serde(rename = "traderAddress")]
    trader_address: String,
    #[serde(rename = "startingCapital")]
    starting_capital: f64,
    #[serde(rename = "currentCapital")]
    current_capital: f64,
    #[serde(rename = "totalTrades")]
    total_trades: usize,
    #[serde(rename = "copiedTrades")]
    copied_trades: usize,
    #[serde(rename = "skippedTrades")]
    skipped_trades: usize,
    #[serde(rename = "totalInvested")]
    total_invested: f64,
    #[serde(rename = "currentValue")]
    current_value: f64,
    #[serde(rename = "realizedPnl")]
    realized_pnl: f64,
    #[serde(rename = "unrealizedPnl")]
    unrealized_pnl: f64,
    #[serde(rename = "totalPnl")]
    total_pnl: f64,
    roi: f64,
    positions: Vec<SimulatedPosition>,
}

#[derive(Debug, serde::Deserialize)]
struct CachedTrades {
    trades: Vec<Trade>,
}

const DEFAULT_TRADER_ADDRESS: &str = "0x7c3db723f1d4d8cb9c550095203b686cb11e5c6b";
const STARTING_CAPITAL: f64 = 1000.0;

fn get_env_var_or_default(key: &str, default: f64) -> f64 {
    env::var(key)
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|&v| v > 0.0)
        .unwrap_or(default)
}

fn get_env_var_int_or_default(key: &str, default: i32) -> i32 {
    env::var(key)
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(default)
}

async fn fetch_batch(
    trader_address: &str,
    offset: usize,
    limit: usize,
    since_timestamp: i64,
    env: &polymarket_copy_trading_bot_rust::config::Env,
) -> Result<Vec<Trade>> {
    let url = format!(
        "https://data-api.polymarket.com/activity?user={}&type=TRADE&limit={}&offset={}",
        trader_address, limit, offset
    );
    let json: serde_json::Value = fetch_data(&url, env).await?;
    let trades: Vec<Trade> = json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .filter(|t: &Trade| t.timestamp >= since_timestamp)
        .collect();
    Ok(trades)
}

async fn fetch_trader_activity(
    trader_address: &str,
    history_days: i32,
    max_trades_limit: usize,
    env: &polymarket_copy_trading_bot_rust::config::Env,
) -> Result<Vec<Trade>> {
    let cache_dir = Path::new("trader_data_cache");
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let cache_file = cache_dir.join(format!("{}_{}d_{}.json", trader_address, history_days, today));

    if cache_file.exists() {
        println!("{}", "📦 Loading cached trader activity...".cyan());
        let content = fs::read_to_string(&cache_file)?;
        let cached: CachedTrades = serde_json::from_str(&content)?;
        println!(
            "{}",
            format!("✓ Loaded {} trades from cache", cached.trades.len()).green()
        );
        return Ok(cached.trades);
    }

    println!(
        "{}",
        format!(
            "📊 Fetching trader activity from last {} days (with parallel requests)...",
            history_days
        )
        .cyan()
    );

    let since_timestamp =
        (chrono::Utc::now().timestamp() - (history_days as i64 * 24 * 60 * 60)) as i64;

    let first_batch = fetch_batch(trader_address, 0, 100, since_timestamp, env).await?;
    let mut all_trades = first_batch.clone();

    if first_batch.len() == 100 {
        let batch_size = 100;
        let max_parallel = 5;
        let mut offset = 100;
        let mut has_more = true;

        while has_more && all_trades.len() < max_trades_limit {
            let mut promises = Vec::new();
            for i in 0..max_parallel {
                let addr = trader_address.to_string();
                let env_clone = env.clone();
                let offset_val = offset + i * batch_size;
                promises.push(tokio::spawn(async move {
                    fetch_batch(&addr, offset_val, batch_size, since_timestamp, &env_clone).await
                }));
            }

            let mut added_count = 0;
            for result in futures_util::future::join_all(promises).await {
                match result {
                    Ok(Ok(batch)) => {
                        if !batch.is_empty() {
                            all_trades.extend(batch.clone());
                            added_count += batch.len();
                            if batch.len() < batch_size {
                                has_more = false;
                            }
                        }
                    }
                    _ => {}
                }
            }

            if added_count == 0 {
                has_more = false;
            }

            if all_trades.len() >= max_trades_limit {
                println!(
                    "{}",
                    format!(
                        "⚠️  Reached trade limit ({}), stopping fetch...",
                        max_trades_limit
                    )
                    .yellow()
                );
                all_trades.truncate(max_trades_limit);
                has_more = false;
            }

            offset += max_parallel * batch_size;
            println!("{}", format!("  Fetched {} trades so far...", all_trades.len()).bright_black());
        }
    }

    all_trades.sort_by_key(|t| t.timestamp);
    println!("{}", format!("✓ Fetched {} trades from last {} days", all_trades.len(), history_days).green());

    // Save to cache
    if !cache_dir.exists() {
        fs::create_dir_all(cache_dir)?;
    }

    let cache_data = serde_json::json!({
        "name": format!("trader_{}_{}d_{}", &trader_address[..6.min(trader_address.len())], history_days, today),
        "traderAddress": trader_address,
        "fetchedAt": chrono::Utc::now().to_rfc3339(),
        "period": format!("{}_days", history_days),
        "totalTrades": all_trades.len(),
        "trades": all_trades
    });

    fs::write(&cache_file, serde_json::to_string_pretty(&cache_data)?)?;
    println!("{}", format!("✓ Cached trades to: {}", cache_file.display()).green());
    println!();

    Ok(all_trades)
}

async fn fetch_trader_positions(
    trader_address: &str,
    env: &polymarket_copy_trading_bot_rust::config::Env,
) -> Result<Vec<Position>> {
    println!("{}", "📈 Fetching trader positions...".cyan());
    let url = format!("https://data-api.polymarket.com/positions?user={}", trader_address);
    let json: serde_json::Value = fetch_data(&url, env).await?;
    
    // Handle both array and object responses
    let positions: Vec<Position> = if json.is_array() {
        serde_json::from_value(json)?
    } else {
        // If it's an object, try to extract positions array
        json.get("positions")
            .or_else(|| json.get("data"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value::<Position>(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default()
    };
    
    println!("{}", format!("✓ Fetched {} positions", positions.len()).green());
    Ok(positions)
}

fn get_trader_positions_value_at_time(timestamp: i64, trades: &[Trade]) -> f64 {
    let past_trades: Vec<_> = trades.iter().filter(|t| t.timestamp <= timestamp).collect();
    let mut positions_value = 0.0;

    for trade in past_trades {
        if trade.side == "BUY" {
            positions_value += trade.usdc_size;
        } else {
            positions_value -= trade.usdc_size;
        }
    }

    positions_value.max(0.0)
}

async fn simulate_copy_trading_old_logic(
    trades: Vec<Trade>,
    trader_address: &str,
    multiplier: f64,
    min_order_size: f64,
    env: &polymarket_copy_trading_bot_rust::config::Env,
) -> Result<SimulationResult> {
    println!("{}", "\n🎮 Starting simulation with OLD LOGIC...\n".cyan());
    println!(
        "{}",
        "OLD LOGIC: ratio = my_balance / (trader_positions_value + trade.usdcSize)".yellow()
    );
    println!("{}", "           multiplier only applied to trades < $1\n".yellow());

    let mut your_balance = STARTING_CAPITAL;
    let mut total_invested = 0.0;
    let mut copied_trades = 0;
    let mut skipped_trades = 0;

    let mut positions: HashMap<String, SimulatedPosition> = HashMap::new();

    for trade in &trades {
        // OLD LOGIC: Get trader's position value (not including USDC balance)
        let trader_positions_value = get_trader_positions_value_at_time(trade.timestamp, &trades);

        // OLD LOGIC: Calculate ratio = my_balance / (trader_positions + trade.usdcSize)
        let ratio = your_balance / (trader_positions_value + trade.usdc_size);
        let mut order_size = trade.usdc_size * ratio;

        // OLD LOGIC: Only apply multiplier if below minimum
        if order_size < min_order_size {
            order_size = order_size * multiplier;
        }

        // Check if order meets minimum after multiplier
        if order_size < min_order_size {
            skipped_trades += 1;
            continue;
        }

        // Check if we have enough balance
        if order_size > your_balance * 0.95 {
            order_size = your_balance * 0.95;
            if order_size < min_order_size {
                skipped_trades += 1;
                continue;
            }
        }

        let position_key = format!("{}:{}", trade.asset, trade.outcome.as_deref().unwrap_or("Unknown"));

        if trade.side == "BUY" {
            let shares_received = order_size / trade.price;

            let pos = positions.entry(position_key.clone()).or_insert_with(|| {
                SimulatedPosition {
                    market: trade.market.clone().unwrap_or_else(|| trade.asset.clone()),
                    outcome: trade.outcome.clone().unwrap_or_else(|| "Unknown".to_string()),
                    entry_price: trade.price,
                    exit_price: None,
                    invested: 0.0,
                    current_value: 0.0,
                    pnl: 0.0,
                    closed: false,
                    trades: Vec::new(),
                }
            });

            pos.trades.push(PositionTrade {
                timestamp: trade.timestamp,
                side: "BUY".to_string(),
                price: trade.price,
                size: shares_received,
                usdc_size: order_size,
                trader_balance: trader_positions_value,
                your_balance,
                ratio,
                your_size: order_size,
            });

            pos.invested += order_size;
            pos.current_value += order_size;
            your_balance -= order_size;
            total_invested += order_size;
            copied_trades += 1;
        } else if trade.side == "SELL" {
            if let Some(pos) = positions.get_mut(&position_key) {
                let sell_amount = order_size.min(pos.current_value);

                pos.trades.push(PositionTrade {
                    timestamp: trade.timestamp,
                    side: "SELL".to_string(),
                    price: trade.price,
                    size: sell_amount / trade.price,
                    usdc_size: sell_amount,
                    trader_balance: trader_positions_value,
                    your_balance,
                    ratio,
                    your_size: sell_amount,
                });

                pos.current_value -= sell_amount;
                pos.exit_price = Some(trade.price);
                your_balance += sell_amount;

                if pos.current_value < 0.01 {
                    pos.closed = true;
                    pos.pnl = your_balance + pos.current_value - pos.invested;
                }

                copied_trades += 1;
            } else {
                skipped_trades += 1;
            }
        }
    }

    // Calculate current values based on trader's current positions
    let trader_positions = fetch_trader_positions(trader_address, env).await?;
    let mut total_current_value = your_balance;
    let mut unrealized_pnl = 0.0;
    let mut realized_pnl = 0.0;

    for (key, sim_pos) in positions.iter_mut() {
        if !sim_pos.closed {
            // Extract asset ID from position key (format: "asset:outcome")
            let asset_id = key.split(':').next().unwrap_or("");
            
            if let Some(trader_pos) = trader_positions.iter().find(|tp| tp.asset == asset_id) {
                if trader_pos.size > 0.0 {
                    let current_price = trader_pos.current_value / trader_pos.size;
                let total_shares: f64 = sim_pos
                    .trades
                    .iter()
                    .filter(|t| t.side == "BUY")
                    .map(|t| t.size)
                    .sum();
                let sold_shares: f64 = sim_pos
                    .trades
                    .iter()
                    .filter(|t| t.side == "SELL")
                    .map(|t| t.size)
                    .sum();
                let remaining_shares = total_shares - sold_shares;
                sim_pos.current_value = remaining_shares * current_price;
                }
            }

            sim_pos.pnl = sim_pos.current_value - sim_pos.invested;
            unrealized_pnl += sim_pos.pnl;
            total_current_value += sim_pos.current_value;
        } else {
            let total_bought: f64 = sim_pos
                .trades
                .iter()
                .filter(|t| t.side == "BUY")
                .map(|t| t.usdc_size)
                .sum();
            let total_sold: f64 = sim_pos
                .trades
                .iter()
                .filter(|t| t.side == "SELL")
                .map(|t| t.usdc_size)
                .sum();
            sim_pos.pnl = total_sold - total_bought;
            realized_pnl += sim_pos.pnl;
        }
    }

    let current_capital = your_balance
        + positions
            .values()
            .filter(|p| !p.closed)
            .map(|p| p.current_value)
            .sum::<f64>();

    let total_pnl = current_capital - STARTING_CAPITAL;
    let roi = (total_pnl / STARTING_CAPITAL) * 100.0;

    Ok(SimulationResult {
        trader_address: trader_address.to_string(),
        starting_capital: STARTING_CAPITAL,
        current_capital,
        total_trades: trades.len(),
        copied_trades,
        skipped_trades,
        total_invested,
        current_value: total_current_value,
        realized_pnl,
        unrealized_pnl,
        total_pnl,
        roi,
        positions: positions.into_values().collect(),
    })
}

fn print_report(result: &SimulationResult, multiplier: f64) {
    println!("\n{}", "═".repeat(80).cyan());
    println!("{}", "  📊 COPY TRADING SIMULATION REPORT (OLD LOGIC)".cyan());
    println!("{}\n", "═".repeat(80).cyan());

    println!("Trader: {}", result.trader_address.blue());
    println!("Multiplier: {}", format!("{}x", multiplier).yellow());
    println!("{}", "Logic: OLD (ratio = my_balance / trader_positions)".yellow());
    println!();

    println!("{}", "Capital:".bold());
    println!("  Starting: {}", format!("${:.2}", result.starting_capital).green());
    println!("  Current:  {}", format!("${:.2}", result.current_capital).green());
    println!();

    println!("{}", "Performance:".bold());
    let pnl_color = if result.total_pnl >= 0.0 { "green" } else { "red" };
    let roi_color = if result.roi >= 0.0 { "green" } else { "red" };
    let pnl_sign = if result.total_pnl >= 0.0 { "+" } else { "" };
    let roi_sign = if result.roi >= 0.0 { "+" } else { "" };
    println!(
        "  Total P&L:     {}",
        format!("{}{:.2}", pnl_sign, result.total_pnl).color(pnl_color)
    );
    println!(
        "  ROI:           {}",
        format!("{}{:.2}%", roi_sign, result.roi).color(roi_color)
    );
    println!(
        "  Realized:      {}{:.2}",
        if result.realized_pnl >= 0.0 { "+" } else { "" },
        result.realized_pnl
    );
    println!(
        "  Unrealized:    {}{:.2}",
        if result.unrealized_pnl >= 0.0 { "+" } else { "" },
        result.unrealized_pnl
    );
    println!();

    println!("{}", "Trades:".bold());
    println!("  Total trades:  {}", result.total_trades.to_string().cyan());
    println!("  Copied:        {}", result.copied_trades.to_string().green());
    println!(
        "  Skipped:       {} (below ${:.2} minimum)",
        result.skipped_trades.to_string().yellow(),
        get_env_var_or_default("SIM_MIN_ORDER_USD", 1.0)
    );
    println!();

    let open_positions: Vec<_> = result.positions.iter().filter(|p| !p.closed).collect();
    let closed_positions: Vec<_> = result.positions.iter().filter(|p| p.closed).collect();

    println!("{}", "Open Positions:".bold());
    println!("  Count: {}\n", open_positions.len());

    for (i, pos) in open_positions.iter().take(10).enumerate() {
        let pnl_str = if pos.pnl >= 0.0 {
            format!("+${:.2}", pos.pnl).green()
        } else {
            format!("-${:.2}", pos.pnl.abs()).red()
        };
        let market_label = pos.market.chars().take(50).collect::<String>();
        println!("  {}. {}", i + 1, market_label);
        println!(
            "     Outcome: {} | Invested: ${:.2} | Value: ${:.2} | P&L: {}",
            pos.outcome, pos.invested, pos.current_value, pnl_str
        );
    }

    if open_positions.len() > 10 {
        println!(
            "{}",
            format!("\n  ... and {} more positions", open_positions.len() - 10).bright_black()
        );
    }

    if !closed_positions.is_empty() {
        println!("\n{}", "Closed Positions:".bold());
        println!("  Count: {}\n", closed_positions.len());

        for (i, pos) in closed_positions.iter().take(5).enumerate() {
            let pnl_str = if pos.pnl >= 0.0 {
                format!("+${:.2}", pos.pnl).green()
            } else {
                format!("-${:.2}", pos.pnl.abs()).red()
            };
            let market_label = pos.market.chars().take(50).collect::<String>();
            println!("  {}. {}", i + 1, market_label);
            println!("     Outcome: {} | P&L: {}", pos.outcome, pnl_str);
        }

        if closed_positions.len() > 5 {
            println!(
                "{}",
                format!("\n  ... and {} more closed positions", closed_positions.len() - 5)
                    .bright_black()
            );
        }
    }

    println!("\n{}\n", "═".repeat(80).cyan());
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("{}", "\n🚀 POLYMARKET COPY TRADING PROFITABILITY SIMULATOR (OLD LOGIC)\n".cyan());

    let trader_address = env::var("SIM_TRADER_ADDRESS")
        .unwrap_or_else(|_| DEFAULT_TRADER_ADDRESS.to_string())
        .to_lowercase();
    let history_days = get_env_var_int_or_default("SIM_HISTORY_DAYS", 7);
    let multiplier = get_env_var_or_default("TRADE_MULTIPLIER", 1.0);
    let min_order_size = get_env_var_or_default("SIM_MIN_ORDER_USD", 1.0);
    let max_trades_limit = get_env_var_int_or_default("SIM_MAX_TRADES", 5000) as usize;

    println!("{}", format!("Trader: {}", trader_address).bright_black());
    println!("{}", format!("Starting Capital: ${:.2}", STARTING_CAPITAL).bright_black());
    println!("{}", format!("Multiplier: {}x", multiplier).bright_black());
    println!(
        "{}",
        format!(
            "History window: {} day(s), max trades: {}\n",
            history_days, max_trades_limit
        )
        .bright_black()
    );

    let env = load_env()?;

    let trades = fetch_trader_activity(&trader_address, history_days, max_trades_limit, &env).await?;
    let result = simulate_copy_trading_old_logic(
        trades,
        &trader_address,
        multiplier,
        min_order_size,
        &env,
    )
    .await?;
    print_report(&result, multiplier);

    // Save to JSON file
    let results_dir = Path::new("simulation_results");
    if !results_dir.exists() {
        fs::create_dir_all(results_dir)?;
    }

    let tag = env::var("SIM_RESULT_TAG")
        .map(|t| format!("_{}", t.trim().replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "-")))
        .unwrap_or_default();
    let filename = format!(
        "old_logic_{}_{}d{}_{}.json",
        trader_address,
        history_days,
        tag,
        chrono::Utc::now().format("%Y-%m-%d")
    );
    let filepath = results_dir.join(&filename);

    fs::write(&filepath, serde_json::to_string_pretty(&result)?)?;
    println!("{}", format!("✓ Results saved to: {}\n", filepath.display()).green());

    println!("{}", "✓ Simulation completed successfully!\n".green());
    Ok(())
}
