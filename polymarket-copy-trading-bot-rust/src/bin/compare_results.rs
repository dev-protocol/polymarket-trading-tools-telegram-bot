//! Compare simulation results
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use colored::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct SimulationResult {
    id: Option<String>,
    name: String,
    logic: String,
    timestamp: i64,
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
    positions: Vec<serde_json::Value>,
}

fn load_simulation_results() -> Vec<SimulationResult> {
    let results_dir = Path::new("simulation_results");

    if !results_dir.exists() {
        println!("{}", "No simulation results found. Run simulations first.".red());
        return Vec::new();
    }

    let files: Vec<_> = fs::read_dir(results_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|s| s.to_str())
                        == Some("json")
                })
                .collect()
        })
        .unwrap_or_default();

    if files.is_empty() {
        println!("{}", "No result files found in simulation_results/".yellow());
        return Vec::new();
    }

    let mut results = Vec::new();

    for file_entry in files {
        let file_path = file_entry.path();
        if let Ok(content) = fs::read_to_string(&file_path) {
            if let Ok(data) = serde_json::from_str::<SimulationResult>(&content) {
                results.push(data);
            } else {
                println!("{}", format!("  Skipped {} (invalid JSON)", file_path.display()).bright_black());
            }
        }
    }

    results
}

fn group_by_trader(results: &[SimulationResult]) -> HashMap<String, Vec<&SimulationResult>> {
    let mut grouped = HashMap::new();

    for result in results {
        let trader = result.trader_address.to_lowercase();
        grouped.entry(trader).or_insert_with(Vec::new).push(result);
    }

    grouped
}

fn print_comparison_table(results: &[SimulationResult]) {
    println!("\n{}", "════════════════════════════════════════════════════════════════════════════".bold().cyan());
    println!("{}", "  📊 SIMULATION RESULTS COMPARISON".bold().cyan());
    println!("{}\n", "════════════════════════════════════════════════════════════════════════════".bold().cyan());

    println!("{}", format!("Total results found: {}\n", results.len()).bright_black());

    let grouped = group_by_trader(results);

    for (trader, trader_results) in grouped.iter() {
        let trader_short = if trader.len() > 18 {
            format!("{}...{}", &trader[..10], &trader[trader.len() - 8..])
        } else {
            trader.clone()
        };
        println!("{}", format!("\n▶ Trader: {}", trader_short).bold().blue());
        println!("{}\n", "─".repeat(80).bright_black());

        let mut sorted = trader_results.clone();
        sorted.sort_by(|a, b| b.roi.partial_cmp(&a.roi).unwrap_or(std::cmp::Ordering::Equal));

        println!("{}", format!("{:<30} | {:<10} | {:<12} | {:<10} | {:<10}", "Name", "ROI", "P&L", "Trades", "Positions").bold());
        println!("{}", "─".repeat(80).bright_black());

        for result in sorted {
            let roi_str = if result.roi >= 0.0 {
                format!("+{:.2}%", result.roi).green()
            } else {
                format!("{:.2}%", result.roi).red()
            };
            let pnl_str = if result.total_pnl >= 0.0 {
                format!("+${:.2}", result.total_pnl).green()
            } else {
                format!("-${:.2}", result.total_pnl.abs()).red()
            };
            let trades_str = format!("{}/{}", result.copied_trades, result.total_trades);
            let open_positions = result
                .positions
                .iter()
                .filter_map(|p| p.get("closed"))
                .filter(|closed| !closed.as_bool().unwrap_or(false))
                .count();

            let name_display = if result.name.len() > 30 {
                result.name.chars().take(27).collect::<String>() + "..."
            } else {
                result.name.clone()
            };

            println!(
                "{:<30} | {:<10} | {:<12} | {:<10} | {:<10}",
                name_display,
                roi_str,
                pnl_str,
                trades_str,
                open_positions
            );
        }
    }

    println!("\n{}\n", "════════════════════════════════════════════════════════════════════════════".bold().cyan());
}

fn print_best_results(results: &[SimulationResult], limit: usize) {
    println!("{}", format!("\n🏆 TOP {} BEST PERFORMING CONFIGURATIONS\n", limit).bold().green());

    let mut sorted: Vec<&SimulationResult> = results.iter().collect();
    sorted.sort_by(|a, b| b.roi.partial_cmp(&a.roi).unwrap_or(std::cmp::Ordering::Equal));

    for (i, result) in sorted.iter().take(limit).enumerate() {
        let rank = i + 1;
        let medal = match rank {
            1 => "🥇",
            2 => "🥈",
            3 => "🥉",
            _ => "",
        };

        println!("{}", format!("{}{} {}", medal, rank, result.name).bold());
        println!("{}", format!("   Trader: {}...", &result.trader_address[..10.min(result.trader_address.len())]).bright_black());
        println!(
            "   ROI: {}",
            if result.roi >= 0.0 {
                format!("+{:.2}%", result.roi).green()
            } else {
                format!("{:.2}%", result.roi).red()
            }
        );
        println!(
            "   P&L: {}",
            if result.total_pnl >= 0.0 {
                format!("+${:.2}", result.total_pnl).green()
            } else {
                format!("-${:.2}", result.total_pnl.abs()).red()
            }
        );
        println!(
            "   Trades: {} copied, {} skipped",
            result.copied_trades, result.skipped_trades
        );
        println!(
            "   Capital: ${:.2} → ${:.2}",
            result.starting_capital, result.current_capital
        );
        println!();
    }
}

fn print_worst_results(results: &[SimulationResult], limit: usize) {
    println!("{}", format!("\n⚠️  WORST {} PERFORMING CONFIGURATIONS\n", limit).bold().red());

    let mut sorted: Vec<&SimulationResult> = results.iter().collect();
    sorted.sort_by(|a, b| a.roi.partial_cmp(&b.roi).unwrap_or(std::cmp::Ordering::Equal));

    for (i, result) in sorted.iter().take(limit).enumerate() {
        println!("{}", format!("{}. {}", i + 1, result.name).bold());
        println!("{}", format!("   Trader: {}...", &result.trader_address[..10.min(result.trader_address.len())]).bright_black());
        println!(
            "   ROI: {}",
            if result.roi >= 0.0 {
                format!("+{:.2}%", result.roi).green()
            } else {
                format!("{:.2}%", result.roi).red()
            }
        );
        println!(
            "   P&L: {}",
            if result.total_pnl >= 0.0 {
                format!("+${:.2}", result.total_pnl).green()
            } else {
                format!("-${:.2}", result.total_pnl.abs()).red()
            }
        );
        println!(
            "   Trades: {} copied, {} skipped",
            result.copied_trades, result.skipped_trades
        );
        println!();
    }
}

fn print_statistics(results: &[SimulationResult]) {
    println!("{}", "\n📈 AGGREGATE STATISTICS\n".bold().cyan());

    if results.is_empty() {
        println!("{}", "No results to analyze".yellow());
        return;
    }

    let avg_roi = results.iter().map(|r| r.roi).sum::<f64>() / results.len() as f64;
    let avg_pnl = results.iter().map(|r| r.total_pnl).sum::<f64>() / results.len() as f64;
    let total_trades_copied: usize = results.iter().map(|r| r.copied_trades).sum();
    let total_trades_skipped: usize = results.iter().map(|r| r.skipped_trades).sum();
    let positive_results = results.iter().filter(|r| r.roi > 0.0).count();
    let negative_results = results.iter().filter(|r| r.roi < 0.0).count();

    println!("Total simulations: {}", results.len().to_string().yellow());
    println!(
        "Profitable: {} ({:.1}%)",
        positive_results.to_string().green(),
        (positive_results as f64 / results.len() as f64) * 100.0
    );
    println!(
        "Unprofitable: {} ({:.1}%)",
        negative_results.to_string().red(),
        (negative_results as f64 / results.len() as f64) * 100.0
    );
    println!();
    println!(
        "Average ROI: {}",
        if avg_roi >= 0.0 {
            format!("+{:.2}%", avg_roi).green()
        } else {
            format!("{:.2}%", avg_roi).red()
        }
    );
    println!(
        "Average P&L: {}",
        if avg_pnl >= 0.0 {
            format!("+${:.2}", avg_pnl).green()
        } else {
            format!("-${:.2}", avg_pnl.abs()).red()
        }
    );
    println!();
    println!("Total trades copied: {}", total_trades_copied.to_string().cyan());
    println!("Total trades skipped: {}", total_trades_skipped.to_string().yellow());
    println!();
}

fn print_detailed_result(result: &SimulationResult) {
    println!("\n{}", "════════════════════════════════════════════════════════════════════════════".bold().cyan());
    println!("{}", "  📋 DETAILED RESULT".bold().cyan());
    println!("{}\n", "════════════════════════════════════════════════════════════════════════════".bold().cyan());

    println!("{}", "Configuration:".bold());
    println!("  Name: {}", result.name.yellow());
    println!("  Trader: {}", result.trader_address.blue());
    println!("  Logic: {}", result.logic);
    println!("  Date: {}", chrono::DateTime::from_timestamp(result.timestamp, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "Unknown".to_string()));
    println!();

    println!("{}", "Capital:".bold());
    println!("  Starting: {}", format!("${:.2}", result.starting_capital).cyan());
    println!("  Current:  {}", format!("${:.2}", result.current_capital).cyan());
    println!("  Invested: {}", format!("${:.2}", result.total_invested).cyan());
    println!("  Value:    {}", format!("${:.2}", result.current_value).cyan());
    println!();

    println!("{}", "Performance:".bold());
    let pnl_color = if result.total_pnl >= 0.0 { "green" } else { "red" };
    let roi_color = if result.roi >= 0.0 { "green" } else { "red" };
    println!(
        "  Total P&L:     {}",
        format!("{}{:.2}", if result.total_pnl >= 0.0 { "+" } else { "" }, result.total_pnl).color(pnl_color)
    );
    println!(
        "  ROI:           {}",
        format!("{}{:.2}%", if result.roi >= 0.0 { "+" } else { "" }, result.roi).color(roi_color)
    );
    println!("  Realized:      ${:.2}", result.realized_pnl);
    println!("  Unrealized:    ${:.2}", result.unrealized_pnl);
    println!();

    println!("{}", "Trading Activity:".bold());
    println!("  Total trades:    {}", result.total_trades.to_string().cyan());
    println!("  Copied:          {}", result.copied_trades.to_string().green());
    println!("  Skipped:         {}", result.skipped_trades.to_string().yellow());
    println!(
        "  Copy rate:       {:.1}%",
        (result.copied_trades as f64 / result.total_trades as f64) * 100.0
    );
    println!();

    let open_positions = result
        .positions
        .iter()
        .filter_map(|p| p.get("closed"))
        .filter(|closed| !closed.as_bool().unwrap_or(false))
        .count();
    let closed_positions = result
        .positions
        .iter()
        .filter_map(|p| p.get("closed"))
        .filter(|closed| closed.as_bool().unwrap_or(false))
        .count();

    println!("{}", "Positions:".bold());
    println!("  Open:   {}", open_positions.to_string().cyan());
    println!("  Closed: {}", closed_positions.to_string().bright_black());
    println!();
}

fn print_help() {
    println!("{}", "\n📊 Simulation Results Comparison - Usage\n".cyan());

    println!("Commands:");
    println!("{}", "  cargo run --bin compare_results              # Show all results".yellow());
    println!("{}", "  cargo run --bin compare_results best [N]     # Show top N results (default: 10)".yellow());
    println!("{}", "  cargo run --bin compare_results worst [N]    # Show worst N results (default: 5)".yellow());
    println!("{}", "  cargo run --bin compare_results stats        # Show aggregate statistics".yellow());
    println!("{}", "  cargo run --bin compare_results detail <name> # Show detailed info for a result\n".yellow());

    println!("Examples:");
    println!("{}", "  cargo run --bin compare_results best 5".bright_black());
    println!("{}", "  cargo run --bin compare_results detail std_m2p0\n".bright_black());
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let results = load_simulation_results();

    if results.is_empty() {
        println!("{}", "\nNo simulation results to compare. Run simulations first with:".yellow());
        println!("{}", "  cargo run --bin simulate_profitability\n".cyan());
        return Ok(());
    }

    let command = args.get(0).map(|s| s.as_str()).unwrap_or("all");

    match command {
        "all" => {
            print_comparison_table(&results);
            print_best_results(&results, 5);
            print_worst_results(&results, 3);
            print_statistics(&results);
        }
        "best" => {
            let limit = args.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(10);
            print_best_results(&results, limit);
        }
        "worst" => {
            let limit = args.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(5);
            print_worst_results(&results, limit);
        }
        "stats" => {
            print_statistics(&results);
        }
        "detail" => {
            let search_name = args.get(1);
            if search_name.is_none() {
                println!("{}", "Please provide a result name to view details".red());
                println!("{}", "Usage: cargo run --bin compare_results detail <name>".yellow());
                return Ok(());
            }

            let search_name = search_name.unwrap();
            if let Some(found) = results.iter().find(|r| r.name.contains(search_name)) {
                print_detailed_result(found);
            } else {
                println!("{}", format!("No result found matching: {}", search_name).red());
            }
        }
        "help" | "--help" | "-h" => {
            print_help();
        }
        _ => {
            println!("{}", format!("Unknown command: {}\n", command).red());
            print_help();
        }
    }

    Ok(())
}
