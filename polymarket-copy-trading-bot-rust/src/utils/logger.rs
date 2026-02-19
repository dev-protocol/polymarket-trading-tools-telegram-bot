use colored::*;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use chrono::Local;

pub struct Logger;

impl Logger {
    fn get_logs_dir() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("logs")
    }

    fn get_log_file_name() -> PathBuf {
        let date = Local::now().format("%Y-%m-%d").to_string();
        Self::get_logs_dir().join(format!("bot-{}.log", date))
    }

    fn ensure_logs_dir() {
        let logs_dir = Self::get_logs_dir();
        if !logs_dir.exists() {
            let _ = fs::create_dir_all(&logs_dir);
        }
    }

    fn write_to_file(message: &str) {
        if let Err(_) = (|| -> std::io::Result<()> {
            Self::ensure_logs_dir();
            let log_file = Self::get_log_file_name();
            let timestamp = Local::now().to_rfc3339();
            let log_entry = format!("[{}] {}\n", timestamp, message);
            
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_file)?;
            file.write_all(log_entry.as_bytes())?;
            Ok(())
        })() {
            // Silently fail to avoid infinite loops
        }
    }

    #[allow(dead_code)]
    fn strip_ansi(str: &str) -> String {
        // Remove ANSI color codes for file logging
        str.replace("\x1b[", "")
            .replace("\x1b[0m", "")
            .replace("\x1b[33m", "")
            .replace("\x1b[36m", "")
            .replace("\x1b[32m", "")
            .replace("\x1b[31m", "")
            .replace("\x1b[35m", "")
            .replace("\x1b[90m", "")
            .replace("\x1b[1m", "")
    }

    pub fn format_address(address: &str) -> String {
        if address.len() >= 10 {
            format!("{}...{}", &address[..6], &address[address.len() - 4..])
        } else {
            address.to_string()
        }
    }

    fn mask_address(address: &str) -> String {
        if address.len() >= 10 {
            format!("{}****{}", &address[..6], &address[address.len() - 4..])
        } else {
            address.to_string()
        }
    }

    pub fn header(title: &str) {
        println!("\n{}", "━".repeat(70).cyan());
        println!("{}", format!("  {}", title).cyan().bold());
        println!("{}\n", "━".repeat(70).cyan());
        Self::write_to_file(&format!("HEADER: {}", title));
    }

    pub fn info(message: &str) {
        println!("{} {}", "ℹ".blue(), message);
        Self::write_to_file(&format!("INFO: {}", message));
    }

    pub fn success(message: &str) {
        println!("{} {}", "✓".green(), message);
        Self::write_to_file(&format!("SUCCESS: {}", message));
    }

    pub fn warning(message: &str) {
        println!("{} {}", "⚠".yellow(), message);
        Self::write_to_file(&format!("WARNING: {}", message));
    }

    pub fn error(message: &str) {
        println!("{} {}", "✗".red(), message);
        Self::write_to_file(&format!("ERROR: {}", message));
    }

    pub fn trade(trader_address: &str, action: &str, details: &TradeDetails) {
        println!("\n{}", "─".repeat(70).magenta());
        println!("{}", "📊 NEW TRADE DETECTED".magenta().bold());
        println!("{}", format!("Trader: {}", Self::format_address(trader_address)).bright_black());
        println!("{}", format!("Action: {}", action.white().bold()).bright_black());
        if let Some(asset) = &details.asset {
            println!("{}", format!("Asset:  {}", Self::format_address(asset)).bright_black());
        }
        if let Some(side) = &details.side {
            let side_str = if side == "BUY" {
                side.green().bold()
            } else {
                side.red().bold()
            };
            println!("{}", format!("Side:   {}", side_str).bright_black());
        }
        if let Some(amount) = details.amount {
            let amount_str = format!("${:.2}", amount);
            // Amount should be green (matching screenshot)
            println!("{}", format!("Amount: {}", amount_str).green().bright_black());
        }
        if let Some(price) = details.price {
            println!("{}", format!("Price:  {}", price).green().bright_black());
        }
        if let Some(slug) = details.event_slug.as_ref().or(details.slug.as_ref()) {
            let market_url = format!("https://polymarket.com/event/{}", slug);
            println!("{}", format!("Market: {}", market_url).blue().underline().bright_black());
        }
        if let Some(tx_hash) = &details.transaction_hash {
            let tx_url = format!("https://polygonscan.com/tx/{}", tx_hash);
            println!("{}", format!("TX:     {}", tx_url).blue().underline().bright_black());
        }
        println!("{}\n", "─".repeat(70).magenta());

        // Log to file
        let mut trade_log = format!("TRADE: {} - {}", Self::format_address(trader_address), action);
        if let Some(side) = &details.side {
            trade_log.push_str(&format!(" | Side: {}", side));
        }
        if let Some(amount) = details.amount {
            trade_log.push_str(&format!(" | Amount: ${}", amount));
        }
        if let Some(price) = details.price {
            trade_log.push_str(&format!(" | Price: {}", price));
        }
        if let Some(title) = &details.title {
            trade_log.push_str(&format!(" | Market: {}", title));
        }
        if let Some(tx_hash) = &details.transaction_hash {
            trade_log.push_str(&format!(" | TX: {}", tx_hash));
        }
        Self::write_to_file(&trade_log);
    }

    pub fn balance(my_balance: f64, trader_balance: f64, trader_address: &str) {
        println!("{}", "Capital (USDC + Positions):".bright_black());
        println!(
            "{}",
            format!("  Your total capital:   ${:.2}", my_balance)
                .green()
                .bold()
                .bright_black()
        );
        println!(
            "{}",
            format!(
                "  Trader total capital: ${:.2} ({})",
                trader_balance,
                Self::format_address(trader_address)
            )
            .blue()
            .bold()
            .bright_black()
        );
    }

    pub fn order_result(success: bool, message: &str) {
        if success {
            println!("{} {}", "✓".green(), format!("Order executed: {}", message).green().bold());
            Self::write_to_file(&format!("ORDER SUCCESS: {}", message));
        } else {
            println!("{} {}", "✗".red(), format!("Order failed: {}", message).red().bold());
            Self::write_to_file(&format!("ORDER FAILED: {}", message));
        }
    }

    #[allow(dead_code)]
    pub fn monitoring(trader_count: usize) {
        let timestamp = Local::now().format("%H:%M:%S").to_string();
        println!(
            "{} {} {}",
            format!("[{}]", timestamp).bright_black(),
            "👁️  Monitoring".cyan(),
            format!("{} trader(s)", trader_count).yellow()
        );
    }

    pub fn startup(traders: &[String], my_wallet: &str) {
        println!("\n");
        // Cat ASCII art (left side)
        let cat_lines = vec![
            "                               ,----.",
            "                              ( COPY!)                         .-.",
            "                               `----' _                         \\ \\",
            "                                     (_)                         \\ \\",
            "                                         O                       | |",
            "                    |\\ /\\                  o                     | |",
            "    __              |,\\(_\\_                  . /\\---/\\   _,---._ | |",
            "   ( (              |,\\`   `-^.               /^   ^  \\,'       `. ;",
            "    \\ \\             :    `-'   )             ( O   O   )           ;",
            "     \\ \\             \\        ;               `.=o=__,'            \\",
            "      \\ \\             `-.   ,'                  /         _,--.__   \\",
            "       \\ \\ ____________,'  (                   /  _ )   ,'   `-. `-. \\",
            "        ; '                ;                  / ,' /  ,'        \\ \\ \\ \\",
            "        \\                 /___,-.            / /  / ,'          (,_)(,_)",
            "         `,    ,_____|  ;'_____,'           (,;  (,,)      Bob",
            "       ,-\" \\  :      | :",
            "      ( .-\" \\ `.__   | |",
            "       \\__)  `.__,'  |__)  Whale",
        ];
        
        // PolyCopy ASCII art (right side)
        let polycopy_lines = vec![""];
        
        // Print cat and PolyCopy side by side
        let max_lines = cat_lines.len().max(polycopy_lines.len());
        for i in 0..max_lines {
            let cat_part = if i < cat_lines.len() {
                format!("{}", cat_lines[i].yellow())
            } else {
                "".to_string()
            };
            let polycopy_part = if i < polycopy_lines.len() {
                // Add spacing between cat and PolyCopy (cat is ~24 chars wide, add 2 spaces)
                let spacing = if i < cat_lines.len() { "  " } else { "" };
                let colored = if i < 3 {
                    format!("{}{}", spacing, polycopy_lines[i].cyan())
                } else if i == 3 {
                    format!("{}{}", spacing, polycopy_lines[i].cyan().bold())
                } else if i == 4 {
                    format!("{}{}", spacing, polycopy_lines[i].magenta().bold())
                } else {
                    format!("{}{}", spacing, polycopy_lines[i].magenta())
                };
                colored
            } else {
                "".to_string()
            };
            println!("{}{}", cat_part, polycopy_part);
        }
        println!("{}", "               Copy the best, automate success\n".bright_black());

        println!("{}", "━".repeat(70).cyan());
        println!("{}", "📊 Tracking Traders:".cyan());
        for (index, address) in traders.iter().enumerate() {
            println!("{}", format!("   {}. {}", index + 1, address).bright_black());
        }
        println!("{}", "\n💼 Your Wallet:".cyan());
        println!("{}", format!("   {}\n", Self::mask_address(my_wallet)).bright_black());
    }

    pub fn separator() {
        println!("{}", "─".repeat(70).bright_black());
    }

    pub fn waiting(trader_count: usize, extra_info: Option<&str>) {
        let timestamp = Local::now().format("%H:%M:%S").to_string();
        let spinner = "⏳";
        let message = if let Some(info) = extra_info {
            format!("{} Waiting for trades from {} trader(s)... ({})", spinner, trader_count, info)
        } else {
            format!("{} Waiting for trades from {} trader(s)...", spinner, trader_count)
        };
        print!("\r{} {}", format!("[{}]", timestamp).bright_black(), message.cyan());
        let _ = std::io::stdout().flush();
    }

    pub fn clear_line() {
        print!("\r{}\r", " ".repeat(100));
        let _ = std::io::stdout().flush();
    }

    pub fn my_positions(
        wallet: &str,
        count: usize,
        top_positions: &[PositionDisplay],
        overall_pnl: f64,
        total_value: f64,
        initial_value: f64,
        current_balance: f64,
    ) {
        println!("\n{}", "💼 YOUR POSITIONS".magenta().bold());
        println!("{}", format!("   Wallet: {}", Self::format_address(wallet)).bright_black());
        println!("");

        // Show balance and portfolio overview
        let balance_str = format!("${:.2}", current_balance).yellow().bold();
        let total_portfolio = current_balance + total_value;
        let portfolio_str = format!("${:.2}", total_portfolio).cyan().bold();

        println!("{}", format!("   💰 Available Cash:    {}", balance_str).bright_black());
        println!("{}", format!("   📊 Total Portfolio:   {}", portfolio_str).bright_black());

        if count == 0 {
            println!("{}", "\n   No open positions".bright_black());
        } else {
            let count_str = format!("{} position{}", count, if count > 1 { "s" } else { "" }).green();
            let pnl_sign = if overall_pnl >= 0.0 { "+" } else { "" };
            let profit_str = if overall_pnl >= 0.0 {
                format!("{}{:.1}%", pnl_sign, overall_pnl).green().bold()
            } else {
                format!("{}{:.1}%", pnl_sign, overall_pnl).red().bold()
            };
            let value_str = format!("${:.2}", total_value).cyan();
            let initial_str = format!("${:.2}", initial_value).bright_black();

            println!("");
            println!("{}", format!("   📈 Open Positions:    {}", count_str).bright_black());
            println!("{}", format!("      Invested:          {}", initial_str).bright_black());
            println!("{}", format!("      Current Value:     {}", value_str).bright_black());
            println!("{}", format!("      Profit/Loss:       {}", profit_str).bright_black());

            // Show top positions
            if !top_positions.is_empty() {
                println!("{}", "\n   🔝 Top Positions:".bright_black());
                for pos in top_positions {
                    let pnl_sign = if pos.percent_pnl >= 0.0 { "+" } else { "" };
                    let avg_price = pos.avg_price.unwrap_or(0.0);
                    let cur_price = pos.cur_price.unwrap_or(0.0);
                    let pnl_str = format!("{}{:.1}%", pnl_sign, pos.percent_pnl);
                    let pnl_colored = if pos.percent_pnl >= 0.0 {
                        pnl_str.green()
                    } else {
                        pnl_str.red()
                    };
                    
                    let title_display = if pos.title.len() > 45 {
                        format!("{}...", &pos.title[..45])
                    } else {
                        pos.title.clone()
                    };
                    
                    println!(
                        "{}",
                        format!("      • {} - {}", pos.outcome.as_deref().unwrap_or("Unknown"), title_display)
                            .bright_black()
                    );
                    println!(
                        "{}",
                        format!(
                            "        Value: {} | PnL: {}",
                            format!("${:.2}", pos.current_value).cyan(),
                            pnl_colored
                        )
                        .bright_black()
                    );
                    println!(
                        "{}",
                        format!(
                            "        Bought @ {} | Current @ {}",
                            format!("{:.1}¢", avg_price * 100.0).yellow(),
                            format!("{:.1}¢", cur_price * 100.0).yellow()
                        )
                        .bright_black()
                    );
                }
            }
        }
        println!("");
    }

    pub fn traders_positions(
        traders: &[String],
        position_counts: &[usize],
        position_details: Option<&[Vec<PositionDisplay>]>,
        profitabilities: Option<&[f64]>,
    ) {
        println!("\n{}", "📈 TRADERS YOU'RE COPYING".cyan());
        for (index, address) in traders.iter().enumerate() {
            let count = position_counts.get(index).copied().unwrap_or(0);
            let count_str = if count > 0 {
                format!("{} position{}", count, if count > 1 { "s" } else { "" }).green()
            } else {
                "0 positions".bright_black()
            };

            // Add profitability if available
            let mut profit_str = String::new();
            if let Some(profitabilities) = profitabilities {
                if let Some(&pnl) = profitabilities.get(index) {
                    if count > 0 {
                        let pnl_sign = if pnl >= 0.0 { "+" } else { "" };
                        let pnl_formatted = format!("{}{:.1}%", pnl_sign, pnl);
                        let pnl_colored = if pnl >= 0.0 {
                            pnl_formatted.green().bold()
                        } else {
                            pnl_formatted.red().bold()
                        };
                        profit_str = format!(" | {}", pnl_colored);
                    }
                }
            }

            println!("{}", format!("   {}: {}{}", Self::format_address(address), count_str, profit_str).bright_black());

            // Show position details if available
            if let Some(position_details) = position_details {
                if let Some(details) = position_details.get(index) {
                    for pos in details {
                        let pnl_sign = if pos.percent_pnl >= 0.0 { "+" } else { "" };
                        let avg_price = pos.avg_price.unwrap_or(0.0);
                        let cur_price = pos.cur_price.unwrap_or(0.0);
                        let pnl_str = format!("{}{:.1}%", pnl_sign, pos.percent_pnl);
                        let pnl_colored = if pos.percent_pnl >= 0.0 {
                            pnl_str.green()
                        } else {
                            pnl_str.red()
                        };
                        
                        let title_display = if pos.title.len() > 40 {
                            format!("{}...", &pos.title[..40])
                        } else {
                            pos.title.clone()
                        };
                        
                        println!(
                            "{}",
                            format!("      • {} - {}", pos.outcome.as_deref().unwrap_or("Unknown"), title_display)
                                .bright_black()
                        );
                        println!(
                            "{}",
                            format!(
                                "        Value: {} | PnL: {}",
                                format!("${:.2}", pos.current_value).cyan(),
                                pnl_colored
                            )
                            .bright_black()
                        );
                        println!(
                            "{}",
                            format!(
                                "        Bought @ {} | Current @ {}",
                                format!("{:.1}¢", avg_price * 100.0).yellow(),
                                format!("{:.1}¢", cur_price * 100.0).yellow()
                            )
                            .bright_black()
                        );
                    }
                }
            }
        }
        println!("");
    }

    pub fn _db_connection(traders: &[String], counts: &[usize]) {
        println!("\n{}", "📦 Database Status:".cyan());
        for (index, address) in traders.iter().enumerate() {
            let count = counts.get(index).copied().unwrap_or(0);
            let count_str = format!("{} trades", count).yellow();
            println!("{}", format!("   {}: {}", Self::format_address(address), count_str).bright_black());
        }
        println!("");
    }
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct TradeDetails {
    pub asset: Option<String>,
    pub side: Option<String>,
    pub amount: Option<f64>,
    pub price: Option<f64>,
    pub slug: Option<String>,
    pub event_slug: Option<String>,
    pub transaction_hash: Option<String>,
    pub title: Option<String>,
}

/// Position display data for logger functions
#[derive(Debug, Clone)]
pub struct PositionDisplay {
    pub outcome: Option<String>,
    pub title: String,
    pub current_value: f64,
    pub percent_pnl: f64,
    pub avg_price: Option<f64>,
    pub cur_price: Option<f64>,
}

impl PositionDisplay {
    /// Create from UserPosition
    pub fn _from_user_position(pos: &crate::interfaces::UserPosition) -> Self {
        Self {
            outcome: Some(pos.outcome.clone()),
            title: pos.title.clone(),
            current_value: pos.current_value,
            percent_pnl: pos.percent_pnl,
            avg_price: Some(pos.avg_price),
            cur_price: Some(pos.cur_price),
        }
    }

    /// Create from JSON value (API response)
    pub fn from_json_value(value: &serde_json::Value) -> Self {
        Self {
            outcome: value.get("outcome").and_then(|v| v.as_str()).map(|s| s.to_string()),
            title: value
                .get("title")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default(),
            current_value: value
                .get("currentValue")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            percent_pnl: value
                .get("percentPnl")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            avg_price: value.get("avgPrice").and_then(|v| v.as_f64()),
            cur_price: value.get("curPrice").and_then(|v| v.as_f64()),
        }
    }
}

