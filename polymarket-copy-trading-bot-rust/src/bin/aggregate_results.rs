//! Aggregate trading results
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use colored::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct TraderResult {
    address: Option<String>,
    roi: f64,
    #[serde(rename = "totalPnl")]
    total_pnl: Option<f64>,
    #[serde(rename = "winRate")]
    win_rate: Option<f64>,
    #[serde(rename = "copiedTrades")]
    copied_trades: Option<usize>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ScanResult {
    #[serde(rename = "scanDate")]
    scan_date: Option<String>,
    config: Config,
    summary: Option<Summary>,
    traders: Vec<TraderResult>,
}

#[derive(Debug, Deserialize)]
struct AnalysisResult {
    timestamp: Option<i64>,
    #[serde(rename = "traderAddress")]
    trader_address: Option<String>,
    config: Config,
    results: Vec<TraderResult>,
}

#[derive(Debug, Deserialize, Clone)]
struct Config {
    #[serde(rename = "historyDays")]
    history_days: i32,
    multiplier: Option<f64>,
    #[serde(rename = "minOrderSize")]
    min_order_size: Option<f64>,
    #[serde(rename = "startingCapital")]
    starting_capital: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct Summary {
    #[serde(rename = "totalAnalyzed")]
    total_analyzed: Option<usize>,
    profitable: Option<usize>,
    #[serde(rename = "avgROI")]
    avg_roi: Option<f64>,
    #[serde(rename = "avgWinRate")]
    avg_win_rate: Option<f64>,
}

#[derive(Debug, Serialize)]
struct StrategyPerformance {
    #[serde(rename = "strategyId")]
    strategy_id: String,
    #[serde(rename = "historyDays")]
    history_days: i32,
    multiplier: f64,
    #[serde(rename = "bestROI")]
    best_roi: f64,
    #[serde(rename = "bestWinRate")]
    best_win_rate: f64,
    #[serde(rename = "bestPnL")]
    best_pnl: f64,
    #[serde(rename = "avgROI")]
    avg_roi: f64,
    #[serde(rename = "avgWinRate")]
    avg_win_rate: f64,
    #[serde(rename = "tradersAnalyzed")]
    traders_analyzed: usize,
    #[serde(rename = "profitableTraders")]
    profitable_traders: usize,
    #[serde(rename = "filesCount")]
    files_count: usize,
}

#[derive(Debug)]
struct TraderData {
    best_roi: f64,
    best_strategy: String,
    times_found: usize,
}

const DIRS: &[&str] = &[
    "trader_scan_results",
    "trader_analysis_results",
    "top_traders_results",
    "strategy_factory_results",
];

async fn aggregate_results() -> Result<()> {
    println!("\n{}", "╔════════════════════════════════════════════════════════════════╗".cyan());
    println!("{}", "║          📊 АГРЕГАТОР РЕЗУЛЬТАТОВ ВСЕХ СТРАТЕГИЙ              ║".cyan());
    println!("{}\n", "╚════════════════════════════════════════════════════════════════╝".cyan());

    let mut all_strategies: HashMap<String, StrategyPerformance> = HashMap::new();
    let mut all_traders: HashMap<String, TraderData> = HashMap::new();
    let mut total_files = 0;

    // Scan all directories
    for dir in DIRS {
        let dir_path = Path::new(dir);
        if !dir_path.exists() {
            continue;
        }

        let files: Vec<_> = fs::read_dir(dir_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
            .collect();

        println!("{}", format!("📁 Сканирование {}/: найдено {} файлов", dir, files.len()).bright_black());

        for file_entry in files {
            total_files += 1;
            let file_path = file_entry.path();

            if let Ok(content) = fs::read_to_string(&file_path) {
                if let Ok(scan_result) = serde_json::from_str::<ScanResult>(&content) {
                    process_scan_result(&scan_result, &mut all_strategies, &mut all_traders);
                } else if let Ok(analysis_result) = serde_json::from_str::<AnalysisResult>(&content) {
                    process_analysis_result(&analysis_result, &mut all_strategies, &mut all_traders);
                }
            }
        }
    }

    println!("{}", format!("✓ Обработано {} файлов\n", total_files).green());

    // Sort strategies
    let mut strategies: Vec<_> = all_strategies.into_values().collect();
    strategies.sort_by(|a, b| b.best_roi.partial_cmp(&a.best_roi).unwrap_or(std::cmp::Ordering::Equal));

    // Print results
    println!("{}", "═".repeat(100).cyan());
    println!("{}", "  🏆 ТОП СТРАТЕГИЙ ПО ЛУЧШЕМУ ROI".cyan());
    println!("{}\n", "═".repeat(100).cyan());

    println!("{}", "  #  | Strategy      | Best ROI  | Best Win% | Best P&L   | Avg ROI   | Profitable | Files".bold());
    println!("{}", "─".repeat(100).bright_black());

    for (i, s) in strategies.iter().take(15).enumerate() {
        let roi_color = if s.best_roi >= 0.0 { "green" } else { "red" };
        let roi_sign = if s.best_roi >= 0.0 { "+" } else { "" };
        let pnl_sign = if s.best_pnl >= 0.0 { "+" } else { "" };

        println!(
            "  {} | {} | {} | {} | {}${:.0} | {:.1}% | {}/{} | {}",
            format!("{}", i + 1).yellow(),
            format!("{:13}", s.strategy_id).blue(),
            format!("{}{:.1}%", roi_sign, s.best_roi).color(roi_color),
            format!("{:.1}%", s.best_win_rate).yellow(),
            pnl_sign,
            s.best_pnl,
            s.avg_roi,
            s.profitable_traders,
            s.traders_analyzed,
            s.files_count
        );
    }

    println!("\n{}", "═".repeat(100).cyan());
    println!("{}", "  🎯 ТОП ТРЕЙДЕРОВ (найдены в нескольких сканах)".cyan());
    println!("{}\n", "═".repeat(100).cyan());

    let mut top_traders: Vec<_> = all_traders.into_iter().collect();
    top_traders.sort_by(|(_, a), (_, b)| b.best_roi.partial_cmp(&a.best_roi).unwrap_or(std::cmp::Ordering::Equal));

    println!("{}", "  #  | Address                                    | Best ROI  | Best Strategy | Найден раз".bold());
    println!("{}", "─".repeat(100).bright_black());

    for (i, (address, data)) in top_traders.iter().take(10).enumerate() {
        let roi_color = if data.best_roi >= 0.0 { "green" } else { "red" };
        let roi_sign = if data.best_roi >= 0.0 { "+" } else { "" };

        println!(
            "  {} | {} | {} | {} | {}",
            format!("{}", i + 1).yellow(),
            format!("{:42}", address).blue(),
            format!("{}{:.1}%", roi_sign, data.best_roi).color(roi_color),
            format!("{:13}", data.best_strategy).cyan(),
            data.times_found
        );
    }

    // Statistics
    println!("\n{}", "═".repeat(100).cyan());
    println!("{}", "  📈 ОБЩАЯ СТАТИСТИКА".cyan());
    println!("{}\n", "═".repeat(100).cyan());

    let total_traders: usize = strategies.iter().map(|s| s.traders_analyzed).sum();
    let total_profitable: usize = strategies.iter().map(|s| s.profitable_traders).sum();
    let unique_traders = top_traders.len();
    let profitable_rate = if total_traders > 0 {
        (total_profitable as f64 / total_traders as f64) * 100.0
    } else {
        0.0
    };

    println!("  Всего файлов:           {}", total_files.to_string().cyan());
    println!("  Всего стратегий:        {}", strategies.len().to_string().cyan());
    println!("  Всего трейдеров:        {}", total_traders.to_string().cyan());
    println!("  Уникальных трейдеров:   {}", unique_traders.to_string().cyan());
    println!(
        "  Прибыльных трейдеров:   {} ({:.1}%)",
        total_profitable.to_string().green(),
        profitable_rate
    );

    // Best strategy
    if let Some(best) = strategies.first() {
        println!("\n{}", "🌟 ЛУЧШАЯ СТРАТЕГИЯ:".green());
        println!("  ID: {}", best.strategy_id.yellow());
        println!("  ROI: {}", format!("+{:.2}%", best.best_roi).green());
        println!("  Win Rate: {}", format!("{:.1}%", best.best_win_rate).yellow());
        println!("  P&L: {}", format!("+${:.2}", best.best_pnl).green());
    }

    // Save aggregated results
    let output_path = Path::new("strategy_factory_results").join("aggregated_results.json");
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let output = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "summary": {
            "totalFiles": total_files,
            "totalStrategies": strategies.len(),
            "totalTraders": total_traders,
            "uniqueTraders": unique_traders,
            "profitableTraders": total_profitable,
            "profitableRate": profitable_rate
        },
        "strategies": strategies.iter().take(20).collect::<Vec<_>>(),
        "topTraders": top_traders.iter().take(10).map(|(address, data)| serde_json::json!({
            "address": address,
            "bestROI": data.best_roi,
            "bestStrategy": data.best_strategy,
            "timesFound": data.times_found
        })).collect::<Vec<_>>()
    });

    fs::write(&output_path, serde_json::to_string_pretty(&output)?)?;
    println!(
        "\n{}",
        format!("✓ Агрегированные результаты сохранены: {}", output_path.display()).green()
    );
    println!();

    Ok(())
}

fn process_scan_result(
    result: &ScanResult,
    strategies: &mut HashMap<String, StrategyPerformance>,
    traders: &mut HashMap<String, TraderData>,
) {
    let strategy_id = format!(
        "{}d_{}x",
        result.config.history_days,
        result.config.multiplier.unwrap_or(1.0)
    );

    let strategy = strategies.entry(strategy_id.clone()).or_insert_with(|| {
        StrategyPerformance {
            strategy_id: strategy_id.clone(),
            history_days: result.config.history_days,
            multiplier: result.config.multiplier.unwrap_or(1.0),
            best_roi: f64::NEG_INFINITY,
            best_win_rate: 0.0,
            best_pnl: f64::NEG_INFINITY,
            avg_roi: 0.0,
            avg_win_rate: 0.0,
            traders_analyzed: 0,
            profitable_traders: 0,
            files_count: 0,
        }
    });

    strategy.files_count += 1;

    let mut total_roi = 0.0;
    let mut total_win_rate = 0.0;
    let mut traders_count = 0;

    for trader in &result.traders {
        if trader.roi.is_nan() {
            continue;
        }

        traders_count += 1;
        total_roi += trader.roi;
        total_win_rate += trader.win_rate.unwrap_or(0.0);

        if trader.roi > strategy.best_roi {
            strategy.best_roi = trader.roi;
        }
        if trader.win_rate.unwrap_or(0.0) > strategy.best_win_rate {
            strategy.best_win_rate = trader.win_rate.unwrap_or(0.0);
        }
        if trader.total_pnl.unwrap_or(0.0) > strategy.best_pnl {
            strategy.best_pnl = trader.total_pnl.unwrap_or(0.0);
        }
        if trader.roi > 0.0 {
            strategy.profitable_traders += 1;
        }

        if let Some(ref address) = trader.address {
            let trader_data = traders.entry(address.clone()).or_insert_with(|| {
                TraderData {
                    best_roi: trader.roi,
                    best_strategy: strategy_id.clone(),
                    times_found: 1,
                }
            });

            trader_data.times_found += 1;
            if trader.roi > trader_data.best_roi {
                trader_data.best_roi = trader.roi;
                trader_data.best_strategy = strategy_id.clone();
            }
        }
    }

    strategy.traders_analyzed += traders_count;
    if traders_count > 0 {
        strategy.avg_roi = total_roi / traders_count as f64;
        strategy.avg_win_rate = total_win_rate / traders_count as f64;
    }
}

fn process_analysis_result(
    result: &AnalysisResult,
    strategies: &mut HashMap<String, StrategyPerformance>,
    traders: &mut HashMap<String, TraderData>,
) {
    let strategy_id = format!(
        "{}d_{}x",
        result.config.history_days,
        result.config.multiplier.unwrap_or(1.0)
    );

    let strategy = strategies.entry(strategy_id.clone()).or_insert_with(|| {
        StrategyPerformance {
            strategy_id: strategy_id.clone(),
            history_days: result.config.history_days,
            multiplier: result.config.multiplier.unwrap_or(1.0),
            best_roi: f64::NEG_INFINITY,
            best_win_rate: 0.0,
            best_pnl: f64::NEG_INFINITY,
            avg_roi: 0.0,
            avg_win_rate: 0.0,
            traders_analyzed: 0,
            profitable_traders: 0,
            files_count: 0,
        }
    });

    strategy.files_count += 1;

    let mut total_roi = 0.0;
    let mut total_win_rate = 0.0;
    let mut traders_count = 0;

    for trader in &result.results {
        if trader.roi.is_nan() {
            continue;
        }

        traders_count += 1;
        total_roi += trader.roi;
        total_win_rate += trader.win_rate.unwrap_or(0.0);

        if trader.roi > strategy.best_roi {
            strategy.best_roi = trader.roi;
        }
        if trader.win_rate.unwrap_or(0.0) > strategy.best_win_rate {
            strategy.best_win_rate = trader.win_rate.unwrap_or(0.0);
        }
        if trader.total_pnl.unwrap_or(0.0) > strategy.best_pnl {
            strategy.best_pnl = trader.total_pnl.unwrap_or(0.0);
        }
        if trader.roi > 0.0 {
            strategy.profitable_traders += 1;
        }

        if let Some(ref address) = trader.address {
            let trader_data = traders.entry(address.clone()).or_insert_with(|| {
                TraderData {
                    best_roi: trader.roi,
                    best_strategy: strategy_id.clone(),
                    times_found: 1,
                }
            });

            trader_data.times_found += 1;
            if trader.roi > trader_data.best_roi {
                trader_data.best_roi = trader.roi;
                trader_data.best_strategy = strategy_id.clone();
            }
        }
    }

    strategy.traders_analyzed += traders_count;
    if traders_count > 0 {
        strategy.avg_roi = total_roi / traders_count as f64;
        strategy.avg_win_rate = total_win_rate / traders_count as f64;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    aggregate_results().await?;
    Ok(())
}
