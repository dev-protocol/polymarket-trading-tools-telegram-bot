use anyhow::Result;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use crate::config::Env;
use crate::interfaces::UserActivity;
use crate::utils::{fetch_data, get_my_balance, post_order, logger::{Logger, TradeDetails}};
use mongodb::Database;
use polymarket_client_sdk::clob::Client as ClobClient;
use polymarket_client_sdk::auth::state::Authenticated;
use polymarket_client_sdk::auth::Normal;

pub async fn start_trade_executor(
    _clob_client: Arc<ClobClient<Authenticated<Normal>>>,
    env: Arc<Env>,
    _db: Arc<Database>,
) -> Result<()> {
    Logger::success(&format!(
        "Trade executor ready for {} trader(s)",
        env.user_addresses.len()
    ));

    if env.trade_aggregation_enabled {
        Logger::info(&format!(
            "Trade aggregation enabled: {}s window, ${} minimum",
            env.trade_aggregation_window_seconds,
            1.0 // TRADE_AGGREGATION_MIN_TOTAL_USD
        ));
    }

    let mut check_interval = interval(Duration::from_millis(300));
    let mut last_check = std::time::Instant::now();

    loop {
        check_interval.tick().await;

        // Check for ready aggregated trades if enabled
        if env.trade_aggregation_enabled {
            // TODO: Implement trade aggregation logic
        }

        // Update waiting message
        if last_check.elapsed().as_millis() > 300 {
            if env.trade_aggregation_enabled {
                // TODO: Show buffered count
                Logger::waiting(env.user_addresses.len(), None);
            } else {
                Logger::waiting(env.user_addresses.len(), None);
            }
            last_check = std::time::Instant::now();
        }
    }
}

pub async fn execute_trade_directly(
    trade: &UserActivity,
    user_address: &str,
    clob_client: &ClobClient<Authenticated<Normal>>,
    env: &Env,
    _db: &Database,
    signer: &alloy::signers::local::PrivateKeySigner,
) -> Result<()> {
    Logger::clear_line();
    Logger::header("⚡ NEW TRADE TO COPY");

    // Log trade details first
    let trade_details = TradeDetails {
        asset: Some(trade.asset.clone()),
        side: Some(trade.side.clone()),
        amount: Some(trade.usdc_size),
        price: Some(trade.price),
        slug: Some(trade.slug.clone()),
        event_slug: Some(trade.event_slug.clone()),
        transaction_hash: Some(trade.transaction_hash.clone()),
        title: Some(trade.title.clone()),
    };
    Logger::trade(user_address, &trade.side, &trade_details);

    // Fetch positions
    let my_positions_url = format!("https://data-api.polymarket.com/positions?user={}", env.proxy_wallet);
    let my_positions: Vec<serde_json::Value> = fetch_data(&my_positions_url, env).await?
        .as_array()
        .cloned()
        .unwrap_or_default();

    let user_positions_url = format!("https://data-api.polymarket.com/positions?user={}", user_address);
    let user_positions: Vec<serde_json::Value> = fetch_data(&user_positions_url, env).await?
        .as_array()
        .cloned()
        .unwrap_or_default();

    let _my_position = my_positions
        .iter()
        .find(|p| p.get("conditionId").and_then(|v| v.as_str()) == Some(&trade.condition_id));

    let _user_position = user_positions
        .iter()
        .find(|p| p.get("conditionId").and_then(|v| v.as_str()) == Some(&trade.condition_id));

    // Get balances
    let my_balance = get_my_balance(&env.proxy_wallet, env).await?;
    let user_balance: f64 = user_positions
        .iter()
        .map(|p| p.get("currentValue").and_then(|v| v.as_f64()).unwrap_or(0.0))
        .sum();

    Logger::balance(my_balance, user_balance, user_address);

    // Convert positions to proper types if needed
    // For now, pass None and let post_order handle it
    post_order(
        clob_client,
        if trade.side == "BUY" { "buy" } else { "sell" },
        None, // my_position
        None, // user_position
        trade,
        my_balance,
        user_balance,
        user_address,
        &env.copy_strategy_config,
        env,
        _db,
        signer,
    )
    .await?;

    Logger::separator();
    Ok(())
}

