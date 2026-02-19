mod config;
mod interfaces;
mod services;
mod utils;

use anyhow::Result;
use crate::config::{load_env, connect_db, cleanup_database};
use crate::services::{start_trade_monitor, start_trade_executor, start_auto_claim, start_take_profit_stop_loss};
use crate::utils::{create_clob_client, perform_health_check, log_health_check, Logger};
use tokio::signal;
use std::sync::Arc;
use colored::Colorize;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    let env = Arc::new(load_env()?);
    
    // Welcome message
    println!("\n{} First time running the bot?", "💡".yellow());
    println!("   Read the guide: {}", "GETTING_STARTED.md".cyan());
    println!("   Run health check: {}\n", "cargo run --bin health_check".cyan());
    
    // Connect to database
    let db = Arc::new(connect_db(&env.mongo_uri).await?);
    
    // Clean up old database entries if enabled
    if env.db_cleanup_enabled {
        cleanup_database(&db, &env.proxy_wallet, &env.user_addresses).await?;
    }
    
    Logger::startup(&env.user_addresses, &env.proxy_wallet);
    
    // Check if user tried to set PREVIEW_MODE=false (live trading is premium only)
    if !env.preview_mode {
        println!("\n{} {} {}", 
            "💎".yellow(), 
            "LIVE TRADING IS AVAILABLE IN PREMIUM VERSION".yellow().bold(),
            "".yellow()
        );
        println!("   {}", "To contact the developer for this premium version, please contact \"https://t.me/soulcrancerdev.\"".bright_cyan().bold());
        println!();
        eprintln!("{}", "Exiting: Live trading requires premium version".red().bold());
        std::process::exit(1);
    }
    
    // Show preview mode status (always enabled in free version)
    println!("\n{} {} {}", 
        "🔍".yellow(), 
        "PREVIEW MODE ENABLED".yellow().bold(),
        "- No actual trades will be executed".yellow()
    );
    println!("   {}", "Live trading is available in premium version".bright_yellow().bold());
    println!("   {}", "To contact the developer for this premium version, please contact \"https://t.me/soulcrancerdev.\"".bright_cyan().bold());
    println!();
    
    // Perform initial health check
    Logger::info("Performing initial health check...");
    let health_result = perform_health_check(&db, &env).await?;
    log_health_check(&health_result);
    
    if !health_result.healthy {
        Logger::warning("Health check failed, but continuing startup...");
    }
    
    // Initialize CLOB client
    Logger::info("Initializing CLOB client...");
    let (clob_client, signer) = create_clob_client(&env).await?;
    let clob_client = Arc::new(clob_client);
    let signer = Arc::new(signer);
    Logger::success("CLOB client ready");
    
    Logger::separator();
    
    // Start services
    Logger::info("Starting trade executor...");
    let _executor_handle = {
        let clob_client = clob_client.clone();
        let env = env.clone();
        let db = db.clone();
        tokio::spawn(async move {
            start_trade_executor(clob_client, env, db).await
        })
    };
    
    Logger::info("Starting trade monitor...");
    let _monitor_handle = {
        let env = env.clone();
        let db = db.clone();
        let clob_client = clob_client.clone();
        let signer = signer.clone();
        tokio::spawn(async move {
            start_trade_monitor(env, db, clob_client, signer).await
        })
    };
    
    // Start auto-claim service if enabled
    if env.auto_claim_enabled {
        Logger::info("Starting auto-claim service...");
        let _claim_handle = {
            let env = env.clone();
            tokio::spawn(async move {
                start_auto_claim(env).await
            })
        };
    } else {
        Logger::info("Auto-claim service is disabled (set AUTO_CLAIM_ENABLED=true to enable)");
    }
    
    // Start Take Profit / Stop Loss monitor (if configured)
    if env.take_profit_percent.is_some() || env.stop_loss_percent.is_some() {
        Logger::info("Starting Take Profit / Stop Loss monitor...");
        let _tp_sl_handle = {
            let clob_client = clob_client.clone();
            let env = env.clone();
            tokio::spawn(async move {
                start_take_profit_stop_loss(clob_client, env).await
            })
        };
    } else {
        Logger::info("Take Profit / Stop Loss monitor disabled (set TAKE_PROFIT_PERCENT and/or STOP_LOSS_PERCENT in .env to enable)");
    }
    
    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            Logger::separator();
            Logger::info("Received SIGINT, initiating graceful shutdown...");
        }
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
        }
    }
    
    // Graceful shutdown
    Logger::info("Waiting for services to finish current operations...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    Logger::success("Graceful shutdown completed");
    Ok(())
}
