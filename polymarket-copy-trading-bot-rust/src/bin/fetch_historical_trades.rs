//! Fetch historical trades for traders
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use polymarket_copy_trading_bot_rust::config::load_env;
use polymarket_copy_trading_bot_rust::utils::fetch_data;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Trade {
    id: String,
    timestamp: i64,
    slug: Option<String>,
    market: Option<String>,
    asset: String,
    side: String,
    price: f64,
    usdc_size: f64,
    size: f64,
    outcome: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CachedTrades {
    name: String,
    trader_address: String,
    fetched_at: String,
    period: String,
    history_days: i32,
    total_trades: usize,
    trades: Vec<Trade>,
}

const HISTORY_DAYS: i32 = 30;
const MAX_TRADES_PER_TRADER: usize = 20000;
const BATCH_SIZE: usize = 100;
const MAX_PARALLEL: usize = 4;

async fn fetch_batch(
    address: &str,
    offset: usize,
    limit: usize,
    env: &polymarket_copy_trading_bot_rust::config::Env,
) -> Result<Vec<Trade>> {
    let url = format!(
        "https://data-api.polymarket.com/activity?user={}&type=TRADE&limit={}&offset={}",
        address, limit, offset
    );
    let json: serde_json::Value = fetch_data(&url, env).await?;
    let trades: Vec<Trade> = json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();
    Ok(trades)
}

async fn fetch_trades_for_trader(
    address: &str,
    env: &polymarket_copy_trading_bot_rust::config::Env,
) -> Result<Vec<Trade>> {
    println!("\n🚀 Loading history for {} (last {} days)", address, HISTORY_DAYS);
    let since_timestamp = (chrono::Utc::now().timestamp() - (HISTORY_DAYS as i64 * 24 * 60 * 60)) as i64;

    let mut offset = 0;
    let mut all_trades = Vec::new();
    let mut has_more = true;

    while has_more && all_trades.len() < MAX_TRADES_PER_TRADER {
        let batch_limit = (MAX_TRADES_PER_TRADER - all_trades.len()).min(BATCH_SIZE);
        let batch = fetch_batch(address, offset, batch_limit, env).await?;
        let batch_len = batch.len();

        if batch.is_empty() {
            break;
        }

        let filtered: Vec<_> = batch
            .into_iter()
            .filter(|trade| trade.timestamp >= since_timestamp)
            .collect();
        all_trades.extend(filtered);

        if batch_len < batch_limit {
            has_more = false;
        }

        offset += batch_limit;

        if all_trades.len() % (BATCH_SIZE * MAX_PARALLEL) == 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        }
    }

    all_trades.sort_by_key(|t| t.timestamp);
    println!("✓ Retrieved {} trades", all_trades.len());
    Ok(all_trades)
}

fn save_trades_to_cache(address: &str, trades: &[Trade]) -> Result<()> {
    let cache_dir = Path::new("trader_data_cache");
    if !cache_dir.exists() {
        fs::create_dir_all(cache_dir)?;
    }

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let cache_file = cache_dir.join(format!("{}_{}d_{}.json", address, HISTORY_DAYS, today));

    let payload = CachedTrades {
        name: format!("trader_{}_{}d_{}", &address[..6.min(address.len())], HISTORY_DAYS, today),
        trader_address: address.to_string(),
        fetched_at: chrono::Utc::now().to_rfc3339(),
        period: format!("{}_days", HISTORY_DAYS),
        history_days: HISTORY_DAYS,
        total_trades: trades.len(),
        trades: trades.to_vec(),
    };

    let json = serde_json::to_string_pretty(&payload)?;
    fs::write(&cache_file, json)?;
    println!("💾 Saved to {}", cache_file.display());
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let env = load_env()?;

    if env.user_addresses.is_empty() {
        println!("USER_ADDRESSES is empty. Check .env");
        return Ok(());
    }

    println!("📥 Starting trade history export");
    println!("Traders: {}", env.user_addresses.len());
    println!(
        "Period: {} days, maximum {} trades per trader",
        HISTORY_DAYS, MAX_TRADES_PER_TRADER
    );

    // Process in chunks
    for chunk in env.user_addresses.chunks(MAX_PARALLEL) {
        let mut handles = Vec::new();
        for address in chunk {
            let address = address.clone();
            let env_clone = env.clone();
            let handle = tokio::spawn(async move {
                match fetch_trades_for_trader(&address, &env_clone).await {
                    Ok(trades) => {
                        if let Err(e) = save_trades_to_cache(&address, &trades) {
                            eprintln!("✗ Error saving for {}: {}", address, e);
                        }
                    }
                    Err(e) => {
                        eprintln!("✗ Error loading for {}: {}", address, e);
                    }
                }
            });
            handles.push(handle);
        }
        for handle in handles {
            let _ = handle.await;
        }
    }

    println!("\n✅ Export completed");
    Ok(())
}

