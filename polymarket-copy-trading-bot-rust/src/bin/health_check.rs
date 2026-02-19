//! Standalone health check utility
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use colored::*;
use polymarket_copy_trading_bot_rust::config::{load_env, connect_db};
use polymarket_copy_trading_bot_rust::utils::{perform_health_check, log_health_check, health_check::HealthCheckResult};

fn print_header() {
    println!("\n{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan().bold());
    println!("{}", "     🏥 POLYMARKET BOT - HEALTH CHECK".cyan().bold());
    println!("{}\n", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan().bold());
}

fn print_recommendations(result: &HealthCheckResult) {
    let mut issues = Vec::new();

    if result.checks.database.status == "error" {
        issues.push("❌ Database Connection Failed");
        println!("{}\n", "📋 Database Issue:".red().bold());
        println!("   • Check your MONGO_URI in .env file");
        println!("   • Verify MongoDB Atlas IP whitelist (allow 0.0.0.0/0)");
        println!("   • Ensure database user has correct permissions");
        println!("   • Test connection: https://www.mongodb.com/docs/atlas/troubleshoot-connection\n");
    }

    if result.checks.rpc.status == "error" {
        issues.push("❌ RPC Endpoint Failed");
        println!("{}\n", "📋 RPC Issue:".red().bold());
        println!("   • Check your RPC_URL in .env file");
        println!("   • Verify your API key is valid");
        println!("   • Try alternative providers:");
        println!("     - Infura: https://infura.io");
        println!("     - Alchemy: https://www.alchemy.com\n");
    }

    if result.checks.balance.status == "error" {
        issues.push("❌ Zero USDC Balance");
        println!("{}\n", "📋 Balance Issue:".red().bold());
        println!("   • Your wallet has no USDC to trade with");
        println!("   • Bridge USDC to Polygon: https://wallet.polygon.technology/polygon/bridge/deposit");
        println!("   • Or buy USDC on an exchange and withdraw to Polygon network");
        println!("   • Also get POL (MATIC) for gas fees (~$5-10 worth)\n");
    } else if result.checks.balance.status == "warning" {
        println!("{}\n", "⚠️  Low Balance Warning:".yellow().bold());
        if let Some(bal) = result.checks.balance.balance {
            println!("   • Balance: ${:.2}", bal);
        }
        println!("   • Consider adding more USDC to avoid missing trades");
        println!("   • Recommended minimum: $50-100 for active trading\n");
    }

    if result.checks.polymarket_api.status == "error" {
        issues.push("❌ Polymarket API Failed");
        println!("{}\n", "📋 API Issue:".red().bold());
        println!("   • Polymarket API is not responding");
        println!("   • Check your internet connection");
        println!("   • Polymarket may be experiencing downtime");
        println!("   • Check status: https://polymarket.com\n");
    }

    if issues.is_empty() {
        println!("{}\n", "🎉 All Systems Operational!".green().bold());
        println!("{}", "You're ready to start trading:".cyan());
        println!("   {}\n", "cargo run --release".green());
    } else {
        println!("{}\n", format!("⚠️  {} Issue(s) Found", issues.len()).red().bold());
        println!("{}\n", "Fix the issues above before starting the bot.".yellow());
    }
}

fn print_configuration(env: &polymarket_copy_trading_bot_rust::config::Env) {
    println!("{}", "📊 Configuration Summary:".cyan());
    println!();
    println!("   Trading Wallet: {}...{}", &env.proxy_wallet[..6], &env.proxy_wallet[env.proxy_wallet.len()-4..]);
    println!("   Tracking {} trader(s):", env.user_addresses.len());
    for (idx, addr) in env.user_addresses.iter().enumerate() {
        println!("      {}. {}...{}", idx + 1, &addr[..6], &addr[addr.len()-4..]);
    }
    println!("   Check Interval: {}s", env.fetch_interval);
    println!("   Trade Multiplier: {}x", env.trade_multiplier);
    println!();
}

#[tokio::main]
async fn main() -> Result<()> {
    print_header();
    println!("{}\n", "⏳ Running diagnostic checks...".yellow());

    let env = load_env()?;
    let db = connect_db(&env.mongo_uri).await?;
    let result = perform_health_check(&db, &env).await?;

    log_health_check(&result);
    print_configuration(&env);
    print_recommendations(&result);

    if result.healthy {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}

