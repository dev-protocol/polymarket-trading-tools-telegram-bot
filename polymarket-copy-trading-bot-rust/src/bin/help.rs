//! Help command - displays all available bot commands
#![allow(dead_code)] // Struct fields used for JSON deserialization

use colored::*;

fn main() {
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan().bold());
    println!("{}", "     🤖 POLYMARKET COPY TRADING BOT - COMMANDS".cyan().bold());
    println!("{}\n", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan().bold());

    println!("{}\n", "📖 GETTING STARTED".yellow().bold());
    println!("  {}          Interactive configuration wizard", "cargo run --bin setup".green());
    println!("  {}   Verify everything is working", "cargo run --bin health_check".green());
    println!("  {}          Compile Rust project", "cargo build --release".green());
    println!("  {}              Start the trading bot", "cargo run --release".green());
    println!();

    println!("{}\n", "💰 WALLET & BALANCE".yellow().bold());
    println!("  {}       Check your wallet balance and positions", "cargo run --bin check_proxy_wallet".green());
    println!("  {}        Check both your wallet and EOA", "cargo run --bin check_both_wallets".green());
    println!("  {}   Verify USDC token allowance", "cargo run --bin check_allowance".green());
    println!("  {}  Set USDC spending approval", "cargo run --bin set_token_allowance".green());
    println!("  {}         Get instructions for swapping native USDC to USDC.e", "cargo run --bin swap_native_to_bridged_usdc".green());
    println!("  {}     Transfer USDC from proxy wallet to private key wallet", "cargo run --bin transfer_usdc_from_proxy".green());
    println!();

    println!("{}\n", "📊 MONITORING & STATS".yellow().bold());
    println!("  {}       View your trading statistics", "cargo run --bin check_my_stats".green());
    println!("  {}    See recent trading activity", "cargo run --bin check_recent_activity".green());
    println!("  {}         Check profit & loss discrepancies", "cargo run --bin check_pnl_discrepancy".green());
    println!("  {}  Check positions with detailed information", "cargo run --bin check_positions_detailed".green());
    println!();

    println!("{}\n", "🎯 POSITION MANAGEMENT".yellow().bold());
    println!("  {}       Manually sell a specific position", "cargo run --bin manual_sell".green());
    println!("  {}        Sell large positions (bulk action)", "cargo run --bin sell_large_positions".green());
    println!("  {}       Close stale/old positions", "cargo run --bin close_stale_positions".green());
    println!("  {}    Close resolved market positions", "cargo run --bin close_resolved_positions".green());
    println!("  {}   Redeem resolved positions for USDC", "cargo run --bin redeem_resolved_positions".green());
    println!("  {}        Manually trigger auto-claim check", "cargo run --bin trigger_auto_claim".green());
    println!("  {}    Test auto-claim configuration & diagnostics", "cargo run --bin auto_claim_test".green());
    println!();

    println!("{}\n", "🔍 TRADER RESEARCH".yellow().bold());
    println!("  {}      Find best performing traders", "cargo run --bin find_best_traders".green());
    println!("  {}     Find low-risk traders", "cargo run --bin find_low_risk_traders".green());
    println!("  {}      Scan and analyze top traders", "cargo run --bin scan_best_traders".green());
    println!("  {}      Scan traders from popular markets", "cargo run --bin scan_traders_from_markets".green());
    println!("  {}     Fetch historical trade data", "cargo run --bin fetch_historical_trades".green());
    println!();

    println!("{}\n", "🧪 SIMULATION & TESTING".yellow().bold());
    println!("  {}          Simulate profitability with current logic", "cargo run --bin simulate_profitability".green());
    println!("  {}      Simulate with old algorithm", "cargo run --bin simulate_profitability_old_logic".green());
    println!("  {}               Run comprehensive simulations", "cargo run --bin run_simulations".green());
    println!("  {}           Compare simulation results", "cargo run --bin compare_results".green());
    println!();

    println!("{}\n", "🔧 ADVANCED & UTILITIES".yellow().bold());
    println!("  {}             Audit copy trading algorithm", "cargo run --bin audit_copy_trading_algorithm".green());
    println!("  {}     Fetch historical trade data", "cargo run --bin fetch_historical_trades".green());
    println!("  {}         Aggregate trading results", "cargo run --bin aggregate_results".green());
    println!("  {}         Find EOA address from private key", "cargo run --bin find_my_eoa".green());
    println!("  {}         Find real proxy wallet", "cargo run --bin find_real_proxy_wallet".green());
    println!("  {}         Compute Gnosis Safe address", "cargo run --bin compute_gnosis_safe_address".green());
    println!("  {}         Find Gnosis Safe proxy", "cargo run --bin find_gnosis_safe_proxy".green());
    println!();

    println!("{}\n", "📚 DOCUMENTATION".yellow().bold());
    println!("  {}        Complete beginner's guide", "README.md".cyan());
    println!("  {}                 Full documentation", "README.md".cyan());
    println!("  {}        Next steps guide", "NEXT_STEPS.md".cyan());
    println!("  {}        Utility scripts documentation", "UTILITY_SCRIPTS.md".cyan());
    println!();

    println!("{}\n", "━".repeat(65).blue());
    println!("{}\n", "💡 Quick Tips:".yellow());
    println!("  • New user? Start with: cargo run --bin setup");
    println!("  • Before trading: cargo run --bin health_check");
    println!("  • Test strategies: cargo run --bin simulate_profitability");
    println!("  • Find traders: cargo run --bin find_best_traders");
    println!("  • Emergency stop: Press Ctrl+C");
    println!();
    println!("{}\n", "⚠️  Always start with small amounts and monitor regularly!".yellow());
}

