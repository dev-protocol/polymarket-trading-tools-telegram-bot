//! Post order execution.
//! Handles 'buy', 'sell', and 'merge' order strategies.

use anyhow::Result;
use polymarket_client_sdk::clob::Client as ClobClient;
use polymarket_client_sdk::auth::state::Authenticated;
use polymarket_client_sdk::auth::Normal;
use alloy::signers::local::PrivateKeySigner;

use crate::interfaces::{UserActivity, UserPosition};
use crate::config::{CopyStrategyConfig, calculate_order_size, get_trade_multiplier};
use crate::utils::{logger::Logger, fetch_data};
use crate::config::Env;
use mongodb::Database;

const MIN_ORDER_SIZE_TOKENS: f64 = 1.0; // Minimum order size in tokens for SELL/MERGE orders

/// Extract error message from order response 
#[allow(dead_code)]
fn extract_order_error(response: &serde_json::Value) -> Option<String> {
    if response.is_null() {
        return None;
    }

    if let Some(s) = response.as_str() {
        return Some(s.to_string());
    }

    if let Some(obj) = response.as_object() {
        // Check direct error field
        if let Some(error_val) = obj.get("error") {
            if let Some(s) = error_val.as_str() {
                return Some(s.to_string());
            }
            if let Some(nested) = error_val.as_object() {
                if let Some(err) = nested.get("error").and_then(|v| v.as_str()) {
                    return Some(err.to_string());
                }
                if let Some(msg) = nested.get("message").and_then(|v| v.as_str()) {
                    return Some(msg.to_string());
                }
            }
        }

        // Check errorMsg field
        if let Some(s) = obj.get("errorMsg").and_then(|v| v.as_str()) {
            return Some(s.to_string());
        }

        // Check message field
        if let Some(s) = obj.get("message").and_then(|v| v.as_str()) {
            return Some(s.to_string());
        }
    }

    None
}

/// Check if error is insufficient balance or allowance
#[allow(dead_code)]
fn is_insufficient_balance_or_allowance_error(message: Option<&str>) -> bool {
    let Some(msg) = message else {
        return false;
    };
    let lower = msg.to_lowercase();
    lower.contains("not enough balance") || lower.contains("allowance")
}

/// Get order book from CLOB API
async fn get_order_book(env: &Env, asset: &str) -> Result<serde_json::Value> {
    let book_url = format!(
        "{}/book?token_id={}",
        env.clob_http_url.trim_end_matches('/'),
        asset
    );
    fetch_data(&book_url, env).await
}

/// Post order execution
pub async fn post_order(
    _clob_client: &ClobClient<Authenticated<Normal>>,
    condition: &str,
    my_position: Option<&UserPosition>,
    user_position: Option<&UserPosition>,
    trade: &UserActivity,
    my_balance: f64,
    user_balance: f64,
    user_address: &str,
    config: &CopyStrategyConfig,
    env: &Env,
    _db: &Database,
    _signer: &PrivateKeySigner,
) -> Result<()> {
    // Preview mode: simulate execution without actually placing orders
    Logger::info("🔍 PREVIEW MODE: Simulating order execution (no actual trades will be placed)");

    match condition {
        "merge" => {
            execute_merge_strategy(_clob_client, trade, my_position, user_address, env, _signer).await?;
        }
        "buy" => {
            execute_buy_strategy(
                _clob_client,
                trade,
                my_position,
                my_balance,
                user_balance,
                user_address,
                config,
                env,
                _signer,
            )
            .await?;
        }
        "sell" => {
            execute_sell_strategy(
                _clob_client,
                trade,
                my_position,
                user_position,
                user_address,
                config,
                env,
                _signer,
            )
            .await?;
        }
        _ => {
            Logger::error(&format!("Unknown condition: {}", condition));
        }
    }
    Ok(())
}

/// Execute MERGE strategy
async fn execute_merge_strategy(
    _clob_client: &ClobClient<Authenticated<Normal>>,
    trade: &UserActivity,
    my_position: Option<&UserPosition>,
    _user_address: &str,
    env: &Env,
    _signer: &PrivateKeySigner,
) -> Result<()> {
    Logger::info("Executing MERGE strategy...");

    let my_position = match my_position {
        Some(p) => p,
        None => {
            Logger::warning("No position to merge");
            return Ok(());
        }
    };

    let asset = &trade.asset;
    if asset.is_empty() {
        Logger::warning("No asset specified");
        return Ok(());
    }

    let mut remaining = my_position.size;

    // Check minimum order size
    if remaining < MIN_ORDER_SIZE_TOKENS {
        Logger::warning(&format!(
            "Position size ({:.2} tokens) too small to merge - skipping",
            remaining
        ));
        return Ok(());
    }

    let mut retry = 0u32;

    while remaining > 0.0 && retry < env.retry_limit {
        let order_book = get_order_book(env, asset).await?;
        let bids = order_book
            .get("bids")
            .and_then(|b| b.as_array())
            .ok_or_else(|| anyhow::anyhow!("No bids in order book"))?;

        if bids.is_empty() {
            Logger::warning("No bids available in order book");
            break;
        }

        // Find best bid (maximum price)
        let best_bid = bids
            .iter()
            .filter_map(|b| {
                let price: f64 = b.get("price").and_then(|p| p.as_str()).and_then(|s| s.parse().ok())?;
                let size: f64 = b.get("size").and_then(|s| s.as_str()).and_then(|s| s.parse().ok())?;
                Some((price, size))
            })
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let (best_price, best_size) = match best_bid {
            Some(bid) => bid,
            None => {
                Logger::warning("No valid bids found");
                break;
            }
        };

        Logger::info(&format!("Best bid: {} @ ${}", best_size, best_price));

        let sell_amount = remaining.min(best_size);
        if sell_amount < MIN_ORDER_SIZE_TOKENS {
            Logger::info("Remaining amount below minimum - completing sell");
            break;
        }

        // Preview mode: simulate order without actually placing it
        Logger::order_result(
            true,
            &format!("[PREVIEW] Would sell {:.2} tokens at ${} (${:.2} total)", 
                sell_amount, best_price, sell_amount * best_price),
        );
        remaining -= sell_amount;
        retry = 0;
        continue;
    }

    Ok(())
}

/// Execute BUY strategy
async fn execute_buy_strategy(
    _clob_client: &ClobClient<Authenticated<Normal>>,
    trade: &UserActivity,
    my_position: Option<&UserPosition>,
    my_balance: f64,
    _user_balance: f64,
    _user_address: &str,
    config: &CopyStrategyConfig,
    env: &Env,
    _signer: &PrivateKeySigner,
) -> Result<()> {
    Logger::info("Executing BUY strategy...");

    Logger::info(&format!("Your balance: ${:.2}", my_balance));
    Logger::info(&format!("Trader bought: ${:.2}", trade.usdc_size));

    // Get current position size for position limit checks
    let current_position_value = my_position
        .map(|p| p.size * p.avg_price)
        .unwrap_or(0.0);

    // Calculate order size using copy strategy
    let order_calc = calculate_order_size(
        config,
        trade.usdc_size,
        my_balance,
        current_position_value,
    );

    Logger::info(&format!("📊 {}", order_calc.reasoning));

    // Check if order should be executed
    if order_calc.final_amount == 0.0 {
        Logger::warning(&format!("❌ Cannot execute: {}", order_calc.reasoning));
        if order_calc.below_minimum {
            Logger::warning("💡 Increase COPY_SIZE or wait for larger trades");
        }
        return Ok(());
    }

    let mut remaining = order_calc.final_amount;
    let mut available_balance = my_balance;
    let mut retry = 0u32;
    let mut total_bought_tokens = 0.0;

    while remaining > 0.0 && retry < env.retry_limit {
        let order_book = get_order_book(env, &trade.asset).await?;
        let asks = order_book
            .get("asks")
            .and_then(|a| a.as_array())
            .ok_or_else(|| anyhow::anyhow!("No asks in order book"))?;

        if asks.is_empty() {
            Logger::warning("No asks available in order book");
            break;
        }

        // Find best ask (minimum price)
        let best_ask = asks
            .iter()
            .filter_map(|a| {
                let price: f64 = a.get("price").and_then(|p| p.as_str()).and_then(|s| s.parse().ok())?;
                let size: f64 = a.get("size").and_then(|s| s.as_str()).and_then(|s| s.parse().ok())?;
                Some((price, size))
            })
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let (best_price, best_size) = match best_ask {
            Some(ask) => ask,
            None => {
                Logger::warning("No valid asks found");
                break;
            }
        };

        Logger::info(&format!("Best ask: {} @ ${}", best_size, best_price));

        // Check if remaining amount is below minimum
        if remaining < config.min_order_size_usd {
            Logger::info(&format!(
                "Remaining amount (${:.2}) below minimum - completing trade",
                remaining
            ));
            break;
        }

        let max_order_size_from_orderbook = best_size * best_price;
        let max_order_size = remaining.min(max_order_size_from_orderbook).min(config.max_order_size_usd);
        let order_size = max_order_size.max(config.min_order_size_usd);

        if order_size < config.min_order_size_usd {
            Logger::info("Order size below minimum - completing trade");
            break;
        }

        // Check if balance is sufficient
        if available_balance < order_size {
            Logger::warning(&format!(
                "Insufficient balance: Need ${:.2} but only have ${:.2}",
                order_size, available_balance
            ));
            break;
        }

        // Preview mode: simulate order without actually placing it
        let tokens_bought = order_size / best_price;
        total_bought_tokens += tokens_bought;
        Logger::order_result(
            true,
            &format!(
                "[PREVIEW] Would buy ${:.2} at ${} ({:.2} tokens)",
                order_size, best_price, tokens_bought
            ),
        );
        remaining -= order_size;
        available_balance -= order_size;
        retry = 0;
        continue;
    }

    if total_bought_tokens > 0.0 {
        Logger::info(&format!(
            "📝 Tracked purchase: {:.2} tokens for future sell calculations",
            total_bought_tokens
        ));
    }

    Ok(())
}

/// Execute SELL strategy
async fn execute_sell_strategy(
    _clob_client: &ClobClient<Authenticated<Normal>>,
    trade: &UserActivity,
    my_position: Option<&UserPosition>,
    user_position: Option<&UserPosition>,
    _user_address: &str,
    config: &CopyStrategyConfig,
    env: &Env,
    _signer: &PrivateKeySigner,
) -> Result<()> {
    Logger::info("Executing SELL strategy...");

    let my_position = match my_position {
        Some(p) => p,
        None => {
            Logger::warning("No position to sell");
            return Ok(());
        }
    };

    let remaining = if user_position.is_none() {
        // Trader sold entire position - we sell entire position too
        let remaining = my_position.size;
        Logger::info(&format!(
            "Trader closed entire position → Selling all your {:.2} tokens",
            remaining
        ));
        remaining
    } else {
        let user_pos = user_position.unwrap();
        // Calculate the % of position the trader is selling
        let trader_sell_percent = trade.size / (user_pos.size + trade.size);
        let trader_position_before = user_pos.size + trade.size;

        Logger::info(&format!(
            "Position comparison: Trader has {:.2} tokens, You have {:.2} tokens",
            trader_position_before, my_position.size
        ));
        Logger::info(&format!(
            "Trader selling: {:.2} tokens ({:.2}% of their position)",
            trade.size,
            trader_sell_percent * 100.0
        ));

        // Calculate sell size based on trader's percentage
        let base_sell_size = my_position.size * trader_sell_percent;

        // Apply tiered or single multiplier
        let multiplier = get_trade_multiplier(config, trade.usdc_size);
        let remaining = base_sell_size * multiplier;

        if multiplier != 1.0 {
            Logger::info(&format!(
                "Applying {}x multiplier (based on trader's ${:.2} order): {:.2} → {:.2} tokens",
                multiplier, trade.usdc_size, base_sell_size, remaining
            ));
        }
        remaining
    };

    // Check minimum order size
    if remaining < MIN_ORDER_SIZE_TOKENS {
        Logger::warning(&format!(
            "❌ Cannot execute: Sell amount {:.2} tokens below minimum ({} token)",
            remaining, MIN_ORDER_SIZE_TOKENS
        ));
        return Ok(());
    }

    // Cap sell amount to available position size
    let mut remaining = if remaining > my_position.size {
        Logger::warning(&format!(
            "⚠️  Calculated sell {:.2} tokens > Your position {:.2} tokens",
            remaining, my_position.size
        ));
        Logger::warning(&format!(
            "Capping to maximum available: {:.2} tokens",
            my_position.size
        ));
        my_position.size
    } else {
        remaining
    };

    let mut retry = 0u32;

    while remaining > 0.0 && retry < env.retry_limit {
        let order_book = get_order_book(env, &trade.asset).await?;
        let bids = order_book
            .get("bids")
            .and_then(|b| b.as_array())
            .ok_or_else(|| anyhow::anyhow!("No bids in order book"))?;

        if bids.is_empty() {
            Logger::warning("No bids available in order book");
            break;
        }

        // Find best bid (maximum price)
        let best_bid = bids
            .iter()
            .filter_map(|b| {
                let price: f64 = b.get("price").and_then(|p| p.as_str()).and_then(|s| s.parse().ok())?;
                let size: f64 = b.get("size").and_then(|s| s.as_str()).and_then(|s| s.parse().ok())?;
                Some((price, size))
            })
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let (best_price, best_size) = match best_bid {
            Some(bid) => bid,
            None => {
                Logger::warning("No valid bids found");
                break;
            }
        };

        Logger::info(&format!("Best bid: {} @ ${}", best_size, best_price));

        // Check if remaining amount is below minimum
        if remaining < MIN_ORDER_SIZE_TOKENS {
            Logger::info("Remaining amount below minimum - completing trade");
            break;
        }

        let sell_amount = remaining.min(best_size);

        // Final check: don't create orders below minimum
        if sell_amount < MIN_ORDER_SIZE_TOKENS {
            Logger::info("Order amount below minimum - completing trade");
            break;
        }

        // Preview mode: simulate order without actually placing it
        Logger::order_result(
            true,
            &format!(
                "[PREVIEW] Would sell {:.2} tokens at ${} (${:.2} total)",
                sell_amount, best_price, sell_amount * best_price
            ),
        );
        remaining -= sell_amount;
        retry = 0;
        continue;
    }

    Ok(())
}
