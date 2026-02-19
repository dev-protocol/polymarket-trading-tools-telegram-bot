# Makefile for Polymarket Copy Trading Bot
# Provides convenient commands for building and running binaries

.PHONY: help build release check test clean run setup health check-allowance verify-allowance \
	check-stats check-positions check-activity check-proxy check-pnl find-eoa check-both \
	manual-sell sell-large close-resolved close-stale trigger-claim redeem-positions \
	transfer-usdc swap-usdc find-best find-low-risk scan-best scan-markets fetch-historical \
	simulate simulate-old run-sims aggregate compare audit audit-fixed claim-test \
	find-proxy find-gnosis compute-gnosis list-binaries

# Default target
.DEFAULT_GOAL := help

# Binary directory
BIN_DIR := target/release
PROJECT_ROOT := $(shell pwd)

# Helper function to check if binary exists and run it, or show helpful error
define check_and_run
	@if [ ! -f "$(BIN_DIR)/$(1)" ]; then \
		echo "$(YELLOW)⚠️  Binary not found: $(BIN_DIR)/$(1)$(RESET)"; \
		echo ""; \
		echo "$(CYAN)💡 Solution:$(RESET)"; \
		echo "  Run $(YELLOW)make release$(RESET) or $(YELLOW)cargo build --release$(RESET) first to build all binaries."; \
		echo ""; \
		echo "$(CYAN)Example:$(RESET)"; \
		echo "  $(YELLOW)cargo build --release$(RESET)"; \
		echo "  $(YELLOW)make $(2)$(RESET)"; \
		echo ""; \
		exit 1; \
	fi; \
	$(BIN_DIR)/$(1)
endef

# Colors for output
CYAN := \033[0;36m
GREEN := \033[0;32m
YELLOW := \033[0;33m
RESET := \033[0m

#-------------------------------------------------------------------------------
# Help & Information
#-------------------------------------------------------------------------------

help: ## Show this help message
	@echo "$(CYAN)━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$(RESET)"
	@echo "$(CYAN)     🤖 POLYMARKET COPY TRADING BOT - MAKEFILE COMMANDS$(RESET)"
	@echo "$(CYAN)━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$(RESET)"
	@echo ""
	@echo "$(GREEN)📦 BUILD COMMANDS$(RESET)"
	@echo "  $(YELLOW)make build$(RESET)              Build in debug mode"
	@echo "  $(YELLOW)make release$(RESET)            Build in release mode (optimized)"
	@echo "  $(YELLOW)make check$(RESET)              Check code without building"
	@echo "  $(YELLOW)make test$(RESET)                Run tests"
	@echo "  $(YELLOW)make clean$(RESET)              Clean build artifacts"
	@echo ""
	@echo "$(GREEN)🚀 MAIN BOT$(RESET)"
	@echo "  $(YELLOW)make run$(RESET)                Run the main trading bot"
	@echo ""
	@echo "$(GREEN)⚙️  SETUP & CONFIGURATION$(RESET)"
	@echo "  $(YELLOW)make setup$(RESET)              Interactive configuration wizard"
	@echo "  $(YELLOW)make health$(RESET)             Comprehensive health check"
	@echo ""
	@echo "$(GREEN)💰 WALLET & BALANCE$(RESET)"
	@echo "  $(YELLOW)make check-allowance$(RESET)    Check USDC balance and allowance"
	@echo "  $(YELLOW)make verify-allowance$(RESET)    Quick allowance verification"
	@echo "  $(YELLOW)make set-allowance$(RESET)       Set ERC1155 token allowance"
	@echo "  $(YELLOW)make check-proxy$(RESET)         Verify proxy wallet setup"
	@echo "  $(YELLOW)make find-eoa$(RESET)           Find EOA address from private key"
	@echo "  $(YELLOW)make check-both$(RESET)         Compare two wallet addresses"
	@echo "  $(YELLOW)make find-proxy$(RESET)         Find real proxy wallet"
	@echo "  $(YELLOW)make find-gnosis$(RESET)        Find Gnosis Safe proxy"
	@echo "  $(YELLOW)make compute-gnosis$(RESET)     Compute Gnosis Safe address"
	@echo "  $(YELLOW)make transfer-usdc$(RESET)       Transfer USDC from proxy"
	@echo "  $(YELLOW)make swap-usdc$(RESET)          Swap native to bridged USDC"
	@echo ""
	@echo "$(GREEN)📊 POSITIONS & STATS$(RESET)"
	@echo "  $(YELLOW)make check-stats$(RESET)        Display wallet statistics"
	@echo "  $(YELLOW)make check-positions$(RESET)    Detailed position information"
	@echo "  $(YELLOW)make check-activity$(RESET)   Recent trade activity"
	@echo "  $(YELLOW)make check-pnl$(RESET)         Analyze P&L discrepancies"
	@echo ""
	@echo "$(GREEN)🛒 TRADING OPERATIONS$(RESET)"
	@echo "  $(YELLOW)make manual-sell$(RESET)        Manually sell a position"
	@echo "  $(YELLOW)make sell-large$(RESET)         Auto-sell large positions"
	@echo "  $(YELLOW)make close-resolved$(RESET)     Close resolved positions"
	@echo "  $(YELLOW)make close-stale$(RESET)        Close stale positions"
	@echo "  $(YELLOW)make redeem-positions$(RESET)  Redeem resolved positions"
	@echo ""
	@echo "$(GREEN)🔄 AUTO-CLAIM$(RESET)"
	@echo "  $(YELLOW)make trigger-claim$(RESET)      Manually trigger auto-claim"
	@echo "  $(YELLOW)make claim-test$(RESET)         Test auto-claim configuration"
	@echo ""
	@echo "$(GREEN)🔍 TRADER RESEARCH$(RESET)"
	@echo "  $(YELLOW)make find-best$(RESET)         Find best performing traders"
	@echo "  $(YELLOW)make find-low-risk$(RESET)      Find low-risk traders"
	@echo "  $(YELLOW)make scan-best$(RESET)          Scan and analyze top traders"
	@echo "  $(YELLOW)make scan-markets$(RESET)       Scan traders from markets"
	@echo "  $(YELLOW)make fetch-historical$(RESET)  Fetch historical trade data"
	@echo ""
	@echo "$(GREEN)🧪 SIMULATION & TESTING$(RESET)"
	@echo "  $(YELLOW)make simulate$(RESET)           Simulate profitability"
	@echo "  $(YELLOW)make simulate-old$(RESET)       Simulate with old logic"
	@echo "  $(YELLOW)make run-sims$(RESET)          Run comprehensive simulations"
	@echo "  $(YELLOW)make aggregate$(RESET)         Aggregate trading results"
	@echo "  $(YELLOW)make compare$(RESET)            Compare simulation results"
	@echo "  $(YELLOW)make audit$(RESET)              Audit copy trading algorithm"
	@echo "  $(YELLOW)make audit-fixed$(RESET)        Audit algorithm (fixed version)"
	@echo ""
	@echo "$(GREEN)📋 UTILITIES$(RESET)"
	@echo "  $(YELLOW)make list-binaries$(RESET)      List all available binaries"
	@echo "  $(YELLOW)make help$(RESET)               Show this help message"
	@echo ""
	@echo "$(CYAN)━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$(RESET)"

#-------------------------------------------------------------------------------
# Build Commands
#-------------------------------------------------------------------------------

build: ## Build in debug mode
	@echo "$(CYAN)Building in debug mode...$(RESET)"
	cargo build

release: ## Build in release mode (optimized)
	@echo "$(CYAN)Building in release mode...$(RESET)"
	cargo build --release
	@echo "$(GREEN)✓ Build complete! Binaries are in $(BIN_DIR)/$(RESET)"

check: ## Check code without building
	@echo "$(CYAN)Checking code...$(RESET)"
	cargo check

test: ## Run tests
	@echo "$(CYAN)Running tests...$(RESET)"
	cargo test

clean: ## Clean build artifacts
	@echo "$(CYAN)Cleaning build artifacts...$(RESET)"
	cargo clean
	@echo "$(GREEN)✓ Clean complete!$(RESET)"

#-------------------------------------------------------------------------------
# Main Bot
#-------------------------------------------------------------------------------

run: ## Run the main trading bot
	@echo "$(CYAN)Starting trading bot...$(RESET)"
	@$(call check_and_run,polymarket-copy-trading-bot-rust,run)

#-------------------------------------------------------------------------------
# Setup & Configuration
#-------------------------------------------------------------------------------

setup: ## Interactive configuration wizard
	@$(call check_and_run,setup,setup)

health: ## Comprehensive health check
	@$(call check_and_run,health_check,health)

#-------------------------------------------------------------------------------
# Wallet & Balance
#-------------------------------------------------------------------------------

check-allowance: ## Check USDC balance and allowance
	@$(call check_and_run,check_allowance,check-allowance)

verify-allowance: ## Quick allowance verification
	@$(call check_and_run,verify_allowance,verify-allowance)

set-allowance: ## Set ERC1155 token allowance
	@$(call check_and_run,set_token_allowance,set-allowance)

check-proxy: ## Verify proxy wallet setup
	@$(call check_and_run,check_proxy_wallet,check-proxy)

find-eoa: ## Find EOA address from private key
	@$(call check_and_run,find_my_eoa,find-eoa)

check-both: ## Compare two wallet addresses
	@$(call check_and_run,check_both_wallets,check-both)

find-proxy: ## Find real proxy wallet
	@$(call check_and_run,find_real_proxy_wallet,find-proxy)

find-gnosis: ## Find Gnosis Safe proxy
	@$(call check_and_run,find_gnosis_safe_proxy,find-gnosis)

compute-gnosis: ## Compute Gnosis Safe address
	@$(call check_and_run,compute_gnosis_safe_address,compute-gnosis)

transfer-usdc: ## Transfer USDC from proxy
	@$(call check_and_run,transfer_usdc_from_proxy,transfer-usdc)

swap-usdc: ## Swap native to bridged USDC
	@$(call check_and_run,swap_native_to_bridged_usdc,swap-usdc)

#-------------------------------------------------------------------------------
# Positions & Stats
#-------------------------------------------------------------------------------

check-stats: ## Display wallet statistics
	@$(call check_and_run,check_my_stats,check-stats)

check-positions: ## Detailed position information
	@$(call check_and_run,check_positions_detailed,check-positions)

check-activity: ## Recent trade activity
	@$(call check_and_run,check_recent_activity,check-activity)

check-pnl: ## Analyze P&L discrepancies
	@$(call check_and_run,check_pnl_discrepancy,check-pnl)

#-------------------------------------------------------------------------------
# Trading Operations
#-------------------------------------------------------------------------------

manual-sell: ## Manually sell a position
	@$(call check_and_run,manual_sell,manual-sell)

sell-large: ## Auto-sell large positions
	@$(call check_and_run,sell_large_positions,sell-large)

close-resolved: ## Close resolved positions
	@$(call check_and_run,close_resolved_positions,close-resolved)

close-stale: ## Close stale positions
	@$(call check_and_run,close_stale_positions,close-stale)

redeem-positions: ## Redeem resolved positions
	@$(call check_and_run,redeem_resolved_positions,redeem-positions)

#-------------------------------------------------------------------------------
# Auto-Claim
#-------------------------------------------------------------------------------

trigger-claim: ## Manually trigger auto-claim
	@$(call check_and_run,trigger_auto_claim,trigger-claim)

claim-test: ## Test auto-claim configuration
	@$(call check_and_run,auto_claim_test,claim-test)

#-------------------------------------------------------------------------------
# Trader Research
#-------------------------------------------------------------------------------

find-best: ## Find best performing traders
	@$(call check_and_run,find_best_traders,find-best)

find-low-risk: ## Find low-risk traders
	@$(call check_and_run,find_low_risk_traders,find-low-risk)

scan-best: ## Scan and analyze top traders
	@$(call check_and_run,scan_best_traders,scan-best)

scan-markets: ## Scan traders from markets
	@$(call check_and_run,scan_traders_from_markets,scan-markets)

fetch-historical: ## Fetch historical trade data
	@$(call check_and_run,fetch_historical_trades,fetch-historical)

#-------------------------------------------------------------------------------
# Simulation & Testing
#-------------------------------------------------------------------------------

simulate: ## Simulate profitability
	@$(call check_and_run,simulate_profitability,simulate)

simulate-old: ## Simulate with old logic
	@$(call check_and_run,simulate_profitability_old_logic,simulate-old)

run-sims: ## Run comprehensive simulations
	@$(call check_and_run,run_simulations,run-sims)

aggregate: ## Aggregate trading results
	@$(call check_and_run,aggregate_results,aggregate)

compare: ## Compare simulation results
	@$(call check_and_run,compare_results,compare)

audit: ## Audit copy trading algorithm
	@$(call check_and_run,audit_copy_trading_algorithm,audit)

audit-fixed: ## Audit algorithm (fixed version)
	@$(call check_and_run,audit_copy_trading_algorithm_fixed,audit-fixed)

#-------------------------------------------------------------------------------
# Utilities
#-------------------------------------------------------------------------------

list-binaries: ## List all available binaries
	@echo "$(CYAN)Available binaries:$(RESET)"
	@for bin in $$(ls -1 $(BIN_DIR)/ 2>/dev/null | grep -v "\.d$$" | grep -v "^build$$" | grep -v "^deps$$" | grep -v "^examples$$" | grep -v "^incremental$$" | sort); do \
		if [ -x "$(BIN_DIR)/$$bin" ] && [ -f "$(BIN_DIR)/$$bin" ]; then \
			echo "  $$bin"; \
		fi; \
	done
	@echo ""
	@echo "$(CYAN)Main bot:$(RESET)"
	@echo "  $(BIN_DIR)/polymarket-copy-trading-bot-rust"

