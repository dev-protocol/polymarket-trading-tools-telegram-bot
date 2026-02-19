use anyhow::Result;
use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use crate::config::Env;
use crate::interfaces::{RtdsActivity, UserActivity};
use crate::utils::{fetch_data, logger::Logger, get_my_balance};
use mongodb::Database;
use polymarket_client_sdk::clob::Client as ClobClient;
use polymarket_client_sdk::auth::state::Authenticated;
use polymarket_client_sdk::auth::Normal;
use serde_json::Value;

const RTDS_URL: &str = "wss://ws-live-data.polymarket.com";
const MAX_RECONNECT_ATTEMPTS: u32 = 10;
const RECONNECT_DELAY_SECS: u64 = 5;

pub async fn start_trade_monitor(
    env: Arc<Env>,
    db: Arc<Database>,
    clob_client: Arc<ClobClient<Authenticated<Normal>>>,
    signer: Arc<alloy::signers::local::PrivateKeySigner>,
) -> Result<()> {
    Logger::clear_line();
    Logger::info(&format!(
        "Initializing trade monitor for {} trader(s)...",
        env.user_addresses.len()
    ));

    // Show your own positions first
    show_my_positions(&env).await?;

    // Show current positions for traders
    show_traders_positions(&env, &db).await?;

    Logger::success(&format!(
        "Monitoring {} trader(s) using RTDS (Real-Time Data Stream)",
        env.user_addresses.len()
    ));
    Logger::separator();

    // Connect to RTDS
    let mut reconnect_attempts = 0;
    loop {
        match connect_rtds(&env, &db, &clob_client, &signer).await {
            Ok(_) => {
                reconnect_attempts = 0;
                Logger::success("RTDS WebSocket connected");
            }
            Err(e) => {
                reconnect_attempts += 1;
                if reconnect_attempts >= MAX_RECONNECT_ATTEMPTS {
                    Logger::error(&format!(
                        "Max reconnection attempts ({}) reached. Please restart the bot.",
                        MAX_RECONNECT_ATTEMPTS
                    ));
                    return Err(e);
                }
                let delay = RECONNECT_DELAY_SECS * reconnect_attempts.min(5) as u64;
                Logger::info(&format!(
                    "Reconnecting to RTDS in {}s (attempt {}/{})...",
                    delay,
                    reconnect_attempts,
                    MAX_RECONNECT_ATTEMPTS
                ));
                tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
            }
        }
    }
}

async fn connect_rtds(
    env: &Env,
    db: &Database,
    clob_client: &ClobClient<Authenticated<Normal>>,
    signer: &alloy::signers::local::PrivateKeySigner,
) -> Result<()> {
    Logger::info(&format!("Connecting to RTDS at {}...", RTDS_URL));

    let (mut ws_stream, _) = connect_async(RTDS_URL).await?;
    Logger::success("RTDS WebSocket connected");

    // Subscribe to activity/trades for each trader address
    let subscriptions: Vec<Value> = env
        .user_addresses
        .iter()
        .map(|_| {
            serde_json::json!({
                "topic": "activity",
                "type": "trades"
            })
        })
        .collect();

    let subscribe_message = serde_json::json!({
        "action": "subscribe",
        "subscriptions": subscriptions
    });

    ws_stream
        .send(Message::Text(subscribe_message.to_string()))
        .await?;

    Logger::success(&format!(
        "Subscribed to RTDS for {} trader(s) - monitoring in real-time",
        env.user_addresses.len()
    ));

    // Update positions periodically (every 30 seconds)
    let env_clone = env.clone();
    let db_clone = db.clone();
    let position_update_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(e) = update_positions(&env_clone, &db_clone).await {
                Logger::error(&format!("Error updating positions: {}", e));
            }
        }
    });

    // Process messages
    while let Some(msg) = ws_stream.next().await {
        match msg? {
            Message::Text(text) => {
                if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                    // Handle subscription confirmation
                    if parsed.get("action").and_then(|a| a.as_str()) == Some("subscribed")
                        || parsed.get("status").and_then(|s| s.as_str()) == Some("subscribed")
                    {
                        Logger::info("RTDS subscription confirmed");
                        continue;
                    }

                    // Handle trade activity messages
                    // Check: topic === 'activity' && type === 'trades' && payload exists
                    if parsed.get("topic").and_then(|t| t.as_str()) == Some("activity")
                        && parsed.get("type").and_then(|t| t.as_str()) == Some("trades")
                    {
                        if let Some(payload) = parsed.get("payload") {
                            if let Ok(rtds_activity) = serde_json::from_value::<RtdsActivity>(payload.clone()) {
                                // Extract proxy wallet from RTDS activity
                                let proxy = rtds_activity
                                    .proxy_wallet
                                    .as_deref()
                                    .unwrap_or("")
                                    .to_lowercase();
                                
                                if env.user_addresses.iter().any(|a| a.to_lowercase() == proxy) {
                                    // Convert RtdsActivity to UserActivity for processing
                                    let activity = UserActivity {
                                        id: None,
                                        proxy_wallet: rtds_activity.proxy_wallet.clone().unwrap_or_default(),
                                        timestamp: rtds_activity.timestamp.unwrap_or(0),
                                        condition_id: rtds_activity.condition_id.clone().unwrap_or_default(),
                                        r#type: rtds_activity.activity_type.clone().unwrap_or_default(),
                                        size: rtds_activity.size.unwrap_or(0.0),
                                        usdc_size: rtds_activity.usdc_size(),
                                        transaction_hash: rtds_activity.transaction_hash.clone().unwrap_or_default(),
                                        price: rtds_activity.price.unwrap_or(0.0),
                                        asset: rtds_activity.asset.clone().unwrap_or_default(),
                                        side: rtds_activity.side.clone().unwrap_or_default(),
                                        outcome_index: rtds_activity.outcome_index.unwrap_or(0),
                                        title: rtds_activity.title.clone().unwrap_or_default(),
                                        slug: rtds_activity.slug.clone().unwrap_or_default(),
                                        icon: rtds_activity.icon.clone().unwrap_or_default(),
                                        event_slug: rtds_activity.event_slug.clone().unwrap_or_default(),
                                        outcome: rtds_activity.outcome.clone().unwrap_or_default(),
                                        name: rtds_activity.name.clone().unwrap_or_default(),
                                        pseudonym: String::new(),
                                        bio: String::new(),
                                        profile_image: String::new(),
                                        profile_image_optimized: String::new(),
                                        bot: false,
                                        bot_executed_time: 0,
                                        my_bought_size: None,
                                    };

                                    Logger::info(&format!(
                                        "📊 Trade detected from {}",
                                        crate::utils::logger::Logger::format_address(&proxy)
                                    ));

                                    let env_arc = Arc::new(env.clone());
                                    let db_arc = Arc::new(db.clone());
                                    if let Err(e) = process_trade_activity(&activity, &proxy, clob_client, env_arc, db_arc, signer).await {
                                        Logger::error(&format!("Error processing trade: {}", e));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Message::Close(_) => {
                Logger::warning("RTDS WebSocket closed");
                break;
            }
            _ => {}
        }
    }

    position_update_handle.abort();
    Ok(())
}

async fn process_trade_activity(
    activity: &UserActivity,
    address: &str,
    clob_client: &ClobClient<Authenticated<Normal>>,
    env: Arc<Env>,
    db: Arc<Database>,
    signer: &alloy::signers::local::PrivateKeySigner,
) -> Result<()> {
    // Skip if too old
    let activity_timestamp = if activity.timestamp > 1000000000000 {
        activity.timestamp
    } else {
        activity.timestamp * 1000
    };
    let hours_ago = (chrono::Utc::now().timestamp_millis() - activity_timestamp) as f64 / (1000.0 * 60.0 * 60.0);
    if hours_ago > env.too_old_timestamp as f64 {
        return Ok(());
    }

    // Execute trade directly
    crate::services::trade_executor::execute_trade_directly(activity, address, clob_client, &env, &db, signer).await?;
    Logger::info(&format!(
        "Trade executed for {}...{}",
        &address[..6.min(address.len())],
        &address[address.len().saturating_sub(4)..]
    ));

    Ok(())
}

async fn update_positions(env: &Env, db: &Database) -> Result<()> {
    for address in &env.user_addresses {
        let positions_url = format!("https://data-api.polymarket.com/positions?user={}", address);
        let positions: Vec<Value> = fetch_data(&positions_url, env).await?
            .as_array()
            .cloned()
            .unwrap_or_default();

        if !positions.is_empty() {
            let collection = crate::config::get_user_position_collection(db, address);
            for position in positions {
                let filter = mongodb::bson::doc! {
                    "asset": position.get("asset").and_then(|v| v.as_str()),
                    "conditionId": position.get("conditionId").and_then(|v| v.as_str())
                };
                let update = mongodb::bson::to_document(&position)?;
                collection.update_one(filter, mongodb::bson::doc! { "$set": update }, None).await?;
            }
        }
    }
    Ok(())
}

async fn show_my_positions(env: &Env) -> Result<()> {
    let my_positions_url = format!("https://data-api.polymarket.com/positions?user={}", env.proxy_wallet);
    let my_positions: Vec<Value> = fetch_data(&my_positions_url, env).await?
        .as_array()
        .cloned()
        .unwrap_or_default();

    let current_balance = get_my_balance(&env.proxy_wallet, env).await.unwrap_or(0.0);

    if my_positions.is_empty() {
        Logger::clear_line();
        Logger::my_positions(&env.proxy_wallet, 0, &[], 0.0, 0.0, 0.0, current_balance);
    } else {
        // Calculate overall profitability
        let mut total_value = 0.0;
        let mut initial_value = 0.0;
        let mut weighted_pnl = 0.0;
        let mut top_positions: Vec<crate::utils::logger::PositionDisplay> = Vec::new();
        
        for pos in &my_positions {
            let value = pos.get("currentValue").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let initial = pos.get("initialValue").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let pnl = pos.get("percentPnl").and_then(|v| v.as_f64()).unwrap_or(0.0);
            total_value += value;
            initial_value += initial;
            weighted_pnl += value * pnl;
            
            // Collect position for display
            top_positions.push(crate::utils::logger::PositionDisplay::from_json_value(pos));
        }
        
        // Sort by current value (descending) and take top 5
        top_positions.sort_by(|a, b| b.current_value.partial_cmp(&a.current_value).unwrap_or(std::cmp::Ordering::Equal));
        top_positions.truncate(5);
        
        let my_overall_pnl = if total_value > 0.0 { weighted_pnl / total_value } else { 0.0 };
        Logger::my_positions(
            &env.proxy_wallet,
            my_positions.len(),
            &top_positions,
            my_overall_pnl,
            total_value,
            initial_value,
            current_balance,
        );
    }
    Ok(())
}

async fn show_traders_positions(env: &Env, db: &Database) -> Result<()> {
    let mut position_counts = Vec::new();
    for address in &env.user_addresses {
        let collection = crate::config::get_user_position_collection(db, address);
        let count = collection.count_documents(mongodb::bson::doc! {}, None).await?;
        position_counts.push(count as usize);
    }
    Logger::traders_positions(&env.user_addresses, &position_counts, None, None);
    Ok(())
}

