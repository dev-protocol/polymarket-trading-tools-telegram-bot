# Polymarket Copy Trading Bot (Rust)

A high-performance Rust-based automated trading bot that copies trades from successful Polymarket traders in real-time. Built for maximum performance, ultra-low latency, and enterprise-grade reliability.

## 🚀 Features

- **Real-Time Copy Trading** - WebSocket-based RTDS monitoring with sub-second trade detection
- **Multiple Copy Strategies** - Percentage, Fixed, or Adaptive position sizing
- **Take Profit & Stop Loss** - Automated profit locking and loss limiting
- **Trade Aggregation** - Combines small trades into larger orders for better execution
- **Tiered Multipliers** - Apply different multipliers based on trade size categories
- **Auto-Claim System** - Automatic detection and claiming of resolved positions
- **Multi-Trader Support** - Track and copy from multiple traders simultaneously
- **Position Tracking** - MongoDB-based persistent storage of all trades and positions
- **Risk Management** - Maximum order size, position size, and daily volume limits
- **Preview Mode** - Test strategies without executing actual trades (free version)

## 📋 Prerequisites

- **Rust** (1.70+): Install from [rustup.rs](https://rustup.rs/)
- **MongoDB** (optional, defaults to localhost)
- **Polygon RPC URL** - Get one from [Alchemy](https://www.alchemy.com/) or [Infura](https://www.infura.io/)
- **Polymarket Account** - For accessing the CLOB API

## ⚡ Quick Start

### 1. Clone & Build

```bash
git clone <repo-url>
cd polymarket-copy-trading-bot-rust
cargo build --release
```

### 2. Setup Configuration

Run the interactive setup wizard:

```bash
cargo run --bin setup
```

Or manually create a `.env` file with the following variables:

```bash
# Required Configuration
USER_ADDRESSES=0xABC...,0xDEF...  # Comma-separated trader addresses
PROXY_WALLET=0x123...              # Your Polygon wallet address
PRIVATE_KEY=abc123...              # Your wallet private key (64 hex chars, no 0x)
CLOB_HTTP_URL=https://clob.polymarket.com
CLOB_WS_URL=wss://ws-subscriptions-clob.polymarket.com/ws
MONGO_URI=mongodb://localhost:27017/polymarket_bot
RPC_URL=https://polygon-rpc.com
USDC_CONTRACT_ADDRESS=0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174

# Trading Strategy (choose one)
COPY_STRATEGY=PERCENTAGE           # PERCENTAGE, FIXED, or ADAPTIVE
COPY_SIZE=10.0                      # Percentage or fixed amount
MAX_ORDER_SIZE_USD=100.0
MIN_ORDER_SIZE_USD=0.01

# Optional: Take Profit / Stop Loss
TAKE_PROFIT_PERCENT=10.0
STOP_LOSS_PERCENT=10.0
TP_SL_CHECK_INTERVAL_MS=1000

# Optional: Trade Aggregation
TRADE_AGGREGATION_ENABLED=true
TRADE_AGGREGATION_WINDOW_SECONDS=300

# Optional: Auto-Claim
AUTO_CLAIM_ENABLED=false
AUTO_CLAIM_INTERVAL_MS=3600000

# Preview Mode (Free version)
PREVIEW_MODE=true                   # Set to false for live trading (premium only)
```

### 3. Setup Token Allowance

Before trading, approve USDC spending:

```bash
cargo run --bin set_token_allowance
```

### 4. Run Health Check

Verify your configuration:

```bash
cargo run --bin health_check
```

### 5. Start the Bot

```bash
cargo run --release
```

## 📖 Configuration Guide

### Required Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `USER_ADDRESSES` | Traders to copy (comma-separated or JSON array) | `0xABC...,0xDEF...` |
| `PROXY_WALLET` | Your Polygon wallet address | `0x123...` |
| `PRIVATE_KEY` | Wallet private key (64 hex chars, no 0x) | `abc123...` |
| `CLOB_HTTP_URL` | Polymarket CLOB HTTP endpoint | `https://clob.polymarket.com` |
| `CLOB_WS_URL` | Polymarket WebSocket endpoint | `wss://ws-subscriptions-clob.polymarket.com/ws` |
| `MONGO_URI` | MongoDB connection string | `mongodb://localhost:27017/polymarket_bot` |
| `RPC_URL` | Polygon RPC endpoint | `https://polygon-rpc.com` |
| `USDC_CONTRACT_ADDRESS` | USDC token contract on Polygon | `0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174` |

### Copy Strategy Options

**PERCENTAGE** (Default):
- Copies a fixed percentage of each trader's order
- Example: `COPY_SIZE=10.0` copies 10% of each trade

**FIXED**:
- Uses a fixed dollar amount for each trade
- Example: `COPY_SIZE=50.0` uses $50 per trade

**ADAPTIVE**:
- Dynamically adjusts based on trade magnitude
- Requires: `ADAPTIVE_MIN_PERCENT`, `ADAPTIVE_MAX_PERCENT`, `ADAPTIVE_THRESHOLD_USD`

### Advanced Options

- `TRADE_MULTIPLIER` - Apply a multiplier to all trades (e.g., `1.5` for 1.5x)
- `TIERED_MULTIPLIERS` - Different multipliers per trade size (JSON format)
- `MAX_POSITION_SIZE_USD` - Maximum position size limit
- `MAX_DAILY_VOLUME_USD` - Daily trading volume limit
- `FETCH_INTERVAL` - Monitoring interval in seconds (default: 1)
- `RETRY_LIMIT` - Order retry attempts (default: 3)
- `DB_CLEANUP_ENABLED` - Clean old database entries on startup (default: true)

## 🛠️ Available Commands

### Main Bot
- `cargo run --release` - Start the main trading bot

### Setup & Configuration
- `cargo run --bin setup` - Interactive setup wizard
- `cargo run --bin health_check` - Check system status and configuration
- `cargo run --bin set_token_allowance` - Approve USDC spending
- `cargo run --bin verify_allowance` - Check current USDC allowance

### Position Management
- `cargo run --bin check_my_stats` - View trading statistics
- `cargo run --bin check_positions_detailed` - Detailed position information
- `cargo run --bin check_recent_activity` - View recent trading activity
- `cargo run --bin manual_sell` - Manually sell positions
- `cargo run --bin sell_large_positions` - Sell positions above threshold
- `cargo run --bin close_resolved_positions` - Close resolved market positions
- `cargo run --bin close_stale_positions` - Close old/stale positions

### Wallet Management
- `cargo run --bin check_proxy_wallet` - Check proxy wallet status
- `cargo run --bin check_both_wallets` - Check both EOA and proxy wallets
- `cargo run --bin find_my_eoa` - Find your EOA wallet address
- `cargo run --bin find_real_proxy_wallet` - Find actual proxy wallet
- `cargo run --bin find_gnosis_safe_proxy` - Find Gnosis Safe proxy
- `cargo run --bin compute_gnosis_safe_address` - Compute Gnosis Safe address
- `cargo run --bin transfer_usdc_from_proxy` - Transfer USDC from proxy wallet
- `cargo run --bin swap_native_to_bridged_usdc` - Swap native to bridged USDC

### Auto-Claim
- `cargo run --bin trigger_auto_claim` - Manually trigger auto-claim
- `cargo run --bin auto_claim_test` - Test auto-claim functionality
- `cargo run --bin redeem_resolved_positions` - Redeem resolved positions

### Analytics & Simulation
- `cargo run --bin find_best_traders` - Find top-performing traders
- `cargo run --bin find_low_risk_traders` - Find low-risk traders
- `cargo run --bin scan_best_traders` - Scan for best traders
- `cargo run --bin scan_traders_from_markets` - Scan traders from markets
- `cargo run --bin simulate_profitability` - Simulate trading profitability
- `cargo run --bin run_simulations` - Run trading simulations
- `cargo run --bin fetch_historical_trades` - Fetch historical trade data
- `cargo run --bin check_pnl_discrepancy` - Check PnL discrepancies

### Utilities
- `cargo run --bin help` - Show help information
- `cargo run --bin audit_copy_trading_algorithm` - Audit copy trading algorithm
- `cargo run --bin aggregate_results` - Aggregate simulation results
- `cargo run --bin compare_results` - Compare different results

## 💎 Version Information

### Free Version (Preview Mode)

The free version runs in **Preview Mode** by default. In this mode:
- ✅ All features are available for testing
- ✅ Trade simulation and logging
- ✅ Position tracking and monitoring
- ✅ Strategy testing and backtesting
- ❌ **No actual trades are executed**

To run in preview mode, set in your `.env`:
```bash
PREVIEW_MODE=true
```

### Premium Version (Live Trading)

**Live trading is available in premium version only.**

To enable live trading and execute actual trades:
- Set `PREVIEW_MODE=false` in your `.env` file
- **Note**: The bot will exit if `PREVIEW_MODE=false` is set without a premium license

**To contact the developer for premium version access, please contact:**
- Telegram: [https://t.me/soulcrancerdev](https://t.me/soulcrancerdev)

## 🔒 Security Best Practices

1. **Never commit your `.env` file** - It contains sensitive private keys
2. **Use environment variables** - Consider using secure secret management
3. **Test in preview mode first** - Always test strategies before live trading
4. **Start with small amounts** - Begin with minimal capital to test
5. **Monitor regularly** - Check bot status and positions frequently
6. **Keep private keys secure** - Store private keys in secure, encrypted storage

## 📊 How It Works

1. **Monitoring**: The bot connects to Polymarket's RTDS (Real-Time Data Stream) WebSocket to monitor trader activity
2. **Detection**: When a tracked trader opens a position, the bot detects it within milliseconds
3. **Calculation**: The bot calculates your position size based on your selected strategy and available balance
4. **Execution**: Orders are placed on Polymarket's CLOB (Central Limit Order Book) API
5. **Tracking**: All positions are tracked in MongoDB for monitoring and analysis
6. **Management**: Take Profit/Stop Loss monitors positions and automatically closes them at target thresholds

## 🎯 Copy Strategies Explained

### Percentage Strategy
Copies a fixed percentage of each trader's order size. Best for consistent proportional exposure.

**Example**: If trader buys $1000 and `COPY_SIZE=10.0`, you buy $100.

### Fixed Strategy
Uses a fixed dollar amount for every trade. Best for consistent position sizing regardless of trader's order size.

**Example**: With `COPY_SIZE=50.0`, every trade uses exactly $50.

### Adaptive Strategy
Dynamically adjusts position size based on trade magnitude. Applies smaller percentages to large trades and larger percentages to small trades.

**Example**: 
- Small trade ($100): Uses `ADAPTIVE_MAX_PERCENT` (e.g., 20%)
- Large trade ($10,000): Uses `ADAPTIVE_MIN_PERCENT` (e.g., 5%)

## 🐛 Troubleshooting

### Bot won't start
- Check all required environment variables are set
- Run `cargo run --bin health_check` to diagnose issues
- Verify MongoDB is running (if using local instance)

### No trades being copied
- Verify `USER_ADDRESSES` contains valid trader addresses
- Check trader has recent activity
- Ensure `PREVIEW_MODE=true` for testing (won't execute in preview)
- Check balance and USDC allowance

### Connection errors
- Verify `RPC_URL` is correct and accessible
- Check `CLOB_HTTP_URL` and `CLOB_WS_URL` are correct
- Ensure network connectivity

### Allowance issues
- Run `cargo run --bin set_token_allowance` to set allowance
- Verify allowance with `cargo run --bin verify_allowance`
- Ensure sufficient USDC balance

## 📝 Notes

- Works on **Polygon network** only
- Supports **EOA** (Externally Owned Accounts) and **Gnosis Safe** wallets
- Trades execute via **Polymarket CLOB API**
- All positions tracked in **MongoDB**
- Built with **Rust** for maximum performance and reliability

## 📞 Support & Contact

- **Telegram**: [https://t.me/soulcrancerdev](https://t.me/soulcrancerdev)
- **Premium Version**: Contact for live trading access

## 📄 License

See LICENSE file for details.

---

**⚠️ Disclaimer**: Trading involves risk. Always test in preview mode first and start with small amounts. The developers are not responsible for any financial losses.

