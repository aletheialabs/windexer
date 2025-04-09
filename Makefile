# Makefile

# --- Configuration ---
PROJECT         := windexer
CARGO           := cargo
NODES           := 3
BASE_PORT       := 10001
RPC_PORT_OFFSET := 1000
DATA_DIR        := ./data
AVS_DEMO_DIR    := ./avs-demo
SCRIPTS_DIR     := ./scripts
GEYSER_CONFIG   := config/geyser/windexer-geyser-config.json
AVS_WALLET_FILE := $(AVS_DEMO_DIR)/configs/avs-wallet.json # Default location for Cambrian demo wallet
SOLANA_RPC      := http://localhost:8899
INDEX_TYPES     := accounts,transactions,blocks
LOG_LEVEL       := info
METRICS_INTERVAL := 30

# Phony targets (targets that don't represent files)
.PHONY: all build run-node-% run-local-network stop-local-network clean \
	demo run-validator-with-geyser build-geyser run-geyser clean-geyser \
	init-avs init-avs-devnet init-avs-local init-avs-devnet-with-backoff \
	init-avs-quicknode init-avs-helius init-avs-devnet-extreme-backoff \
	init-avs-alternative-rpc init-avs-prefunded \
	clean-init kill-validator run-validator-clean check-solana-devnet \
	cambrian-demo-setup cambrian-demo-start cambrian-demo-stop \
	cambrian-demo-status cambrian-demo-proposal cambrian-demo-proposal-% \
	cambrian-demo-clean cambrian-demo help run-ts-examples \
	run-indexer-% run-indexer stop-indexer run-indexer-mainnet \
	run-test-scenario generate-data run-realtime-indexer run-full-scenario

# Default target
all: help

# --- Build ---
build:
	@echo "Building workspace..."
	@$(CARGO) build --workspace

# --- Examples ---
run-ts-examples:
	@echo "Running TypeScript examples..."
	@$(SCRIPTS_DIR)/run-ts-examples.sh

run-test-scenario:
	@echo "Running complete test scenario..."
	@echo "1. Stopping any existing processes..."
	@-$(MAKE) kill-validator
	@-$(MAKE) stop-indexer
	@echo "2. Starting validator with Geyser plugin..."
	@$(MAKE) build-geyser
	@echo "Starting validator with Geyser config: $(GEYSER_CONFIG)"
	@cat $(GEYSER_CONFIG)
	@solana-test-validator \
		--geyser-plugin-config $(GEYSER_CONFIG) \
		--reset \
		--faucet-port 9910 \
		--rpc-port 8899 \
		--bind-address 127.0.0.1 & 
	@echo "Waiting for validator to initialize (5 seconds)..."
	@sleep 5
	@echo "3. Starting indexer with unique port..."
	@TEST_PORT=12001 $(CARGO) run --bin indexer -- \
		--index 1 \
		--base-port 12001 \
		--solana-rpc $(SOLANA_RPC) \
		--data-dir $(DATA_DIR) \
		--index-types $(INDEX_TYPES) \
		--log-level debug \
		--metrics-interval-seconds $(METRICS_INTERVAL) & 
	@echo "Waiting for indexer to initialize (15 seconds)..."
	@sleep 15
	@echo "4. Verifying validator and indexer connection..."
	@echo "Checking validator status..."
	@solana --url $(SOLANA_RPC) cluster-version || (echo "Validator not responding!" && exit 1)
	@echo "Checking indexer status..."
	@for i in {1..5}; do \
		if curl -s http://localhost:12001/api/health > /dev/null; then \
			echo "Indexer is responding!"; \
			break; \
		else \
			echo "Attempt $$i: Indexer not responding, waiting..."; \
			if [ $$i -eq 5 ]; then \
				echo "Indexer failed to start after 5 attempts"; \
				exit 1; \
			fi; \
			sleep 5; \
		fi; \
	done
	@echo "5. Generating test transactions..."
	@$(SCRIPTS_DIR)/test-scripts/generate-data.sh
	@echo "6. Verifying transactions on Solana..."
	@echo "Waiting for transactions to be confirmed (10 seconds)..."
	@sleep 10
	@echo "Checking recent transactions..."
	@echo "Getting payer account from keypair..."
	@PAYER_ACCOUNT=$$(solana-keygen pubkey ../../examples/typescript/payer-keypair.json) && \
		echo "Payer account: $$PAYER_ACCOUNT" && \
		echo "Recent transactions for payer:" && \
		for sig in $$(solana --url $(SOLANA_RPC) transaction-history $$PAYER_ACCOUNT --limit 5 | grep -o 'Signature: .*' | cut -d' ' -f2); do \
			echo "Verifying transaction: $$sig" && \
			solana --url $(SOLANA_RPC) confirm -v $$sig; \
		done
	@echo "7. Waiting for indexer to process transactions (30 seconds)..."
	@sleep 30
	@echo "8. Verifying indexer is still running..."
	@curl -s http://localhost:12001/api/health || (echo "Indexer not responding!" && exit 1)
	@echo "9. Querying data..."
	@cd examples/typescript && WINDEXER_URL=http://localhost:12001 SOLANA_URL=http://localhost:8899 ts-node query-all-data.ts
	@echo "Test scenario complete."
	@echo "NOTE: Validator and indexer are still running. Stop them with:"
	@echo "  make kill-validator"
	@echo "  make stop-indexer"

# --- Local Network ---
# Runs a single node with a specific index
run-node-%: build
	@echo "Running node $*..."
	@$(CARGO) run --bin node -- \
		--index $* \
		--base-port $(BASE_PORT) \
		--enable-tip-route

# Runs multiple nodes locally in the background
run-local-network: build
	@echo "Starting local network with $(NODES) nodes..."
	@for i in $$(seq 0 $(shell echo $$(($(NODES)-1)))); do \
		$(CARGO) run --bin node -- \
			--index $$i \
			--base-port $(BASE_PORT) \
			--enable-tip-route & \
	done
	@echo "Local network started."

# Stops the local network nodes started by run-local-network
stop-local-network:
	@echo "Stopping local network nodes..."
	@pkill -f '$(CARGO) run --bin node' || true
	@echo "Local network nodes stopped."

# --- Cleaning ---
clean: stop-local-network kill-validator clean-geyser cambrian-demo-clean
	@echo "Cleaning project data..."
	@rm -rf $(DATA_DIR)/node_*
	@rm -rf $(AVS_DEMO_DIR)
	@rm -f avs-wallet.json # Clean prefunded wallet if exists
	@echo "Project data cleaned."

# --- Demo (Original Simple Demo) ---
demo: build
	@echo "Running simple demo..."
	@$(SCRIPTS_DIR)/test-scripts/start-network.sh
	@$(SCRIPTS_DIR)/test-scripts/demo-interact.sh

# --- Geyser Plugin ---
build-geyser:
	@echo "Building Geyser plugin..."
	@$(CARGO) build --package windexer-geyser

run-validator-with-geyser: build-geyser
	@echo "Starting Solana test validator with Geyser plugin..."
	solana-test-validator \
		--geyser-plugin-config $(GEYSER_CONFIG) \
		--reset \
		--faucet-port 9910 \
		--rpc-port 8899 \
		--bind-address 127.0.0.1

run-geyser:
	@echo "Setting up Windexer Geyser..."
	@$(SCRIPTS_DIR)/setup-windexer-geyser.sh

clean-geyser:
	@echo "Cleaning Geyser setup..."
	@rm -rf windexer_geyser_setup

kill-validator:
	@echo "Stopping Solana test validator..."
	@-pgrep -f 'solana-test-validator' > /dev/null && pkill -f 'solana-test-validator' && echo "Validator stopped." || echo "No validator running."

run-validator-clean: kill-validator run-validator-with-geyser

# --- AVS Initialization (using camb) ---
# Default init uses Devnet
init-avs: init-avs-devnet

init-avs-devnet:
	@echo "Running camb init with Solana Devnet (recommended):"
	@echo "When prompted, enter the following:"
	@echo "  Solana API URL: https://api.devnet.solana.com"
	@echo "  Solana WS URL: wss://api.devnet.solana.com"
	@echo "  Cambrian Consensus Program name: cambrian_devnet_$(shell date +%s)"
	@echo "The timestamp-based name above should be unique and avoid conflicts"
	@camb init -t avs $(AVS_DEMO_DIR)

init-avs-local:
	@echo "⚠️ WARNING: Local validator initialization requires Cambrian programs to be deployed first"
	@echo "This will likely fail unless you've deployed the required programs to your local validator"
	@echo "Using Devnet is recommended (run 'make init-avs-devnet' instead)"
	@echo ""
	@echo "If you still want to proceed, when prompted enter:"
	@echo "  Solana API URL: http://127.0.0.1:8899"
	@echo "  Solana WS URL: ws://127.0.0.1:9000"
	@camb init -t avs $(AVS_DEMO_DIR)

# Clean existing AVS demo dir and re-initialize with Devnet
clean-init:
	@echo "Cleaning previous AVS setup and re-initializing with Devnet..."
	@rm -rf $(AVS_DEMO_DIR)
	@$(MAKE) init-avs-devnet

# --- AVS Initialization (Rate Limit Handling & Alternatives) ---
init-avs-devnet-with-backoff:
	@echo "Running camb init with rate limit handling (30s delay):"
	@echo "When prompted, enter the following:"
	@echo "  Solana API URL: https://api.devnet.solana.com"
	@echo "  Solana WS URL: wss://api.devnet.solana.com"
	@echo "  Cambrian Consensus Program name: cambrian_devnet_$(shell date +%s)"
	@echo "Waiting 30 seconds..."
	@sleep 30
	@camb init -t avs $(AVS_DEMO_DIR)

init-avs-quicknode:
	@echo "Running camb init with Quicknode (recommended for avoiding rate limits):"
	@echo "You need to create a free Quicknode account at https://quicknode.com"
	@echo "When prompted, enter YOUR Quicknode URLs (examples below):"
	@echo "  Solana API URL: https://your-endpoint.solana-devnet.quiknode.pro/your-api-key/"
	@echo "  Solana WS URL: wss://your-endpoint.solana-devnet.quiknode.pro/your-api-key/"
	@echo "  Cambrian Consensus Program name: cambrian_devnet_$(shell date +%s)"
	@camb init -t avs $(AVS_DEMO_DIR)

init-avs-helius:
	@echo "Running camb init with Helius (free alternative to Quicknode):"
	@echo "You need to create a free Helius account at https://helius.xyz"
	@echo "When prompted, enter the Helius URLs (examples below):"
	@echo "  Solana API URL: https://devnet.helius-rpc.com/?api-key=YOUR_API_KEY"
	@echo "  Solana WS URL: wss://devnet.helius-rpc.com/?api-key=YOUR_API_KEY"
	@echo "  Cambrian Consensus Program name: cambrian_devnet_$(shell date +%s)"
	@camb init -t avs $(AVS_DEMO_DIR)

init-avs-devnet-extreme-backoff:
	@echo "Running camb init with extreme rate limit handling (2 min delay):"
	@echo "When prompted, enter the following:"
	@echo "  Solana API URL: https://api.devnet.solana.com"
	@echo "  Solana WS URL: wss://api.devnet.solana.com"
	@echo "  Cambrian Consensus Program name: cambrian_devnet_$(shell date +%s)"
	@echo "Waiting 2 minutes to allow rate limits to reset..."
	@sleep 120
	@camb init -t avs $(AVS_DEMO_DIR)

init-avs-alternative-rpc:
	@echo "Running camb init with alternative public RPC endpoints:"
	@echo "When prompted, enter the following:"
	@echo "  Solana API URL: https://floral-still-sun.solana-devnet.quiknode.pro/87336fc9fbaa83cde5a65aee30b5a4c58ba7a88d/"
	@echo "  Solana WS URL: wss://floral-still-sun.solana-devnet.quiknode.pro/87336fc9fbaa83cde5a65aee30b5a4c58ba7a88d/"
	@echo "  Cambrian Consensus Program name: cambrian_devnet_$(shell date +%s)"
	@camb init -t avs $(AVS_DEMO_DIR)

init-avs-prefunded:
	@echo "Initializing AVS using a pre-funded wallet (two-stage process):"
	@echo "1. Create a wallet (avs-wallet.json) and fund it using the official devnet."
	@echo "2. Initialize camb using that funded wallet with Helius (or another RPC)."
	@echo ""
	@echo "Step 1: Generating wallet keypair..."
	@solana-keygen new -o avs-wallet.json --no-bip39-passphrase
	@echo ""
	@echo "Step 2: Requesting airdrop from official devnet..."
	@PUBKEY=$$(solana-keygen pubkey avs-wallet.json) && \
		echo "Wallet address: $$PUBKEY" && \
		echo "Requesting airdrop of 2 SOL (this may take a moment)..." && \
		solana airdrop 2 $$PUBKEY --url https://api.devnet.solana.com && \
		echo "Airdrop successful." || echo "Airdrop failed. Please check devnet status or try again."
	@echo ""
	@echo "Step 3: Initialize using Helius (or your preferred RPC) with the funded wallet."
	@echo "When prompted, enter the following:"
	@echo "  AVS IP address: 127.0.0.1"
	@echo "  AVS HTTP port: 3000"
	@echo "  AVS WS port: 3001"
	@echo "  Admin private key: $(shell cat avs-wallet.json)"
	@echo "  Solana API URL: https://devnet.helius-rpc.com/?api-key=YOUR_API_KEY  <-- REPLACE WITH YOUR KEY"
	@echo "  Solana WS URL: wss://devnet.helius-rpc.com/?api-key=YOUR_API_KEY   <-- REPLACE WITH YOUR KEY"
	@echo "  Cambrian Consensus Program name: cambrian_devnet_$(shell date +%s)"
	@camb init -t avs $(AVS_DEMO_DIR)

# --- Network Utilities ---
check-solana-devnet:
	@echo "Checking Solana devnet status..."
	@curl -s -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' https://api.devnet.solana.com | jq

# --- Cambrian AVS Demo ---
cambrian-demo-setup:
	@echo "Setting up Cambrian AVS demo environment in $(AVS_DEMO_DIR)..."
	@mkdir -p $(AVS_DEMO_DIR)/configs $(AVS_DEMO_DIR)/logs
	@cd $(AVS_DEMO_DIR) && ../$(SCRIPTS_DIR)/cambrian-avs-demo/setup.sh

cambrian-demo-start:
	@echo "Starting Cambrian AVS demo..."
	@$(SCRIPTS_DIR)/cambrian-avs-demo/run-cambrian-demo.sh

cambrian-demo-stop:
	@echo "Stopping Cambrian AVS demo..."
	@$(SCRIPTS_DIR)/cambrian-avs-demo/stop-cambrian-demo.sh

cambrian-demo-status:
	@echo "Checking Cambrian AVS demo status..."
	@$(SCRIPTS_DIR)/cambrian-avs-demo/cambrian-demo-status.sh

cambrian-demo-proposal: cambrian-demo-proposal-basic # Default proposal

cambrian-demo-proposal-%:
	@echo "Executing Cambrian proposal: $*..."
	@$(SCRIPTS_DIR)/cambrian-avs-demo/execute-cambrian-proposal.sh $*

cambrian-demo-clean:
	@echo "Cleaning Cambrian AVS demo resources..."
	@$(SCRIPTS_DIR)/cambrian-avs-demo/stop-cambrian-demo.sh || true
	@rm -rf $(AVS_DEMO_DIR)/configs/avs-wallet.json $(AVS_DEMO_DIR)/logs/*
	@echo "✅ Cambrian AVS demo resources cleaned."

# Runs the full Cambrian demo workflow
cambrian-demo: cambrian-demo-setup cambrian-demo-start
	@echo "Starting Cambrian demo workflow..."
	@echo "Waiting 5 seconds for services to stabilize..."
	@sleep 5
	@$(MAKE) cambrian-demo-proposal-basic
	@echo "Demo is running. Press Ctrl+C to stop the services (or run 'make cambrian-demo-stop')."
	@# Keep running until Ctrl+C - might need a better way if services run detached
	@tail -f /dev/null

# --- Indexer Commands ---
run-indexer: build
	@echo "Running indexer with index 1..."
	@$(CARGO) run --bin indexer -- \
		--index 1 \
		--base-port $(BASE_PORT) \
		--solana-rpc $(SOLANA_RPC) \
		--data-dir $(DATA_DIR) \
		--index-types $(INDEX_TYPES) \
		--log-level $(LOG_LEVEL) \
		--metrics-interval-seconds $(METRICS_INTERVAL)

run-indexer-%: build
	@echo "Running indexer $*..."
	@$(CARGO) run --bin indexer -- \
		--index $* \
		--base-port $(BASE_PORT) \
		--solana-rpc $(SOLANA_RPC) \
		--data-dir $(DATA_DIR) \
		--index-types $(INDEX_TYPES) \
		--log-level $(LOG_LEVEL) \
		--metrics-interval-seconds $(METRICS_INTERVAL)

run-indexer-mainnet: build
	@echo "Running indexer on Solana mainnet..."
	@$(CARGO) run --bin indexer -- \
		--index 1 \
		--base-port 9000 \
		--bootstrap-peers 127.0.0.1:9000,127.0.0.1:9100 \
		--solana-rpc https://api.mainnet-beta.solana.com \
		--data-dir ./my-data \
		--index-types accounts,transactions \
		--log-level debug \
		--metrics-interval-seconds 60

stop-indexer:
	@echo "Stopping all indexer processes..."
	@pkill -f '$(CARGO) run --bin indexer' || echo "No indexer processes found."
	@echo "Indexer processes stopped."

# --- Help ---
help:
	@echo "Usage: make [target]"
	@echo ""
	@echo "Core Targets:"
	@echo "  help                          Show this help message"
	@echo "  build                         Build the Rust workspace"
	@echo "  clean                         Clean build artifacts, data, logs, and stop services"
	@echo ""
	@echo "Local Network:"
	@echo "  run-local-network             Start $(NODES) nodes locally in the background"
	@echo "  stop-local-network            Stop the local network nodes"
	@echo "  run-node-<index>              Run a single node (e.g., make run-node-0)"
	@echo ""
	@echo "Indexer:"
	@echo "  run-indexer                   Run indexer with index 1 and default settings"
	@echo "  run-indexer-<index>           Run indexer with specified index (e.g., make run-indexer-2)"
	@echo "  run-indexer-mainnet           Run indexer on Solana mainnet with preset configuration"
	@echo "  stop-indexer                  Stop all running indexer processes"
	@echo "  SOLANA_RPC=<url>              Override Solana RPC URL (default: $(SOLANA_RPC))"
	@echo "  DATA_DIR=<path>               Override data directory (default: $(DATA_DIR))"
	@echo "  INDEX_TYPES=<types>           Override index types (default: $(INDEX_TYPES))"
	@echo "  LOG_LEVEL=<level>             Override log level (default: $(LOG_LEVEL))"
	@echo "  METRICS_INTERVAL=<seconds>    Override metrics interval (default: $(METRICS_INTERVAL))"
	@echo ""
	@echo "Geyser & Validator:"
	@echo "  build-geyser                  Build the Geyser plugin"
	@echo "  run-validator-with-geyser     Start solana-test-validator with the Geyser plugin"
	@echo "  run-validator-clean           Kill existing validator and restart with Geyser"
	@echo "  kill-validator                Stop the solana-test-validator process"
	@echo "  run-geyser                    Run the Geyser setup script"
	@echo "  clean-geyser                  Clean Geyser setup files"
	@echo ""
	@echo "Examples:"
	@echo "  run-ts-examples               Run TypeScript examples"
	@echo "  run-test-scenario             Run a complete test scenario"
	@echo ""
	@echo "AVS Initialization (using camb):"
	@echo "  init-avs                      Initialize AVS using Solana Devnet (default)"
	@echo "  init-avs-devnet               Initialize AVS using Solana Devnet"
	@echo "  init-avs-local                Initialize AVS using local validator (requires setup)"
	@echo "  init-avs-quicknode            Initialize AVS using Quicknode RPC (requires account)"
	@echo "  init-avs-helius               Initialize AVS using Helius RPC (requires account)"
	@echo "  init-avs-prefunded            Initialize AVS using a pre-funded wallet (Helius recommended)"
	@echo "  init-avs-devnet-with-backoff  Initialize AVS using Devnet with 30s delay"
	@echo "  init-avs-devnet-extreme-backoff Initialize AVS using Devnet with 2m delay"
	@echo "  init-avs-alternative-rpc      Initialize AVS using alternative public RPC"
	@echo "  clean-init                    Clean previous AVS setup and re-initialize with Devnet"
	@echo ""
	@echo "Network Utilities:"
	@echo "  check-solana-devnet           Check the health status of Solana Devnet"
	@echo ""
	@echo "Cambrian AVS Demo:"
	@echo "  cambrian-demo-setup           Setup the Cambrian AVS demo environment"
	@echo "  cambrian-demo-start           Start the Cambrian AVS demo services"
	@echo "  cambrian-demo-stop            Stop the Cambrian AVS demo services"
	@echo "  cambrian-demo-status          Check the status of the Cambrian AVS demo services"
	@echo "  cambrian-demo-proposal        Execute a basic proposal in the demo"
	@echo "  cambrian-demo-proposal-<type> Execute a specific proposal (e.g., basic, nft, mpl, spl, update-nft)"
	@echo "  cambrian-demo-clean           Clean Cambrian AVS demo resources (stops services first)"
	@echo "  cambrian-demo                 Run the complete demo workflow (setup, start, basic proposal)"
	@echo ""
	@echo "Simple Demo (Original):"
	@echo "  demo                          Run the original simple start-network/demo-interact scripts"

# Data generation and indexing targets
generate-data:
	@echo "=== Generating test data ==="
	@chmod +x scripts/test-scripts/generate-data.sh
	@./scripts/test-scripts/generate-data.sh

run-realtime-indexer:
	@echo "=== Starting real-time indexer ==="
	@bun run examples/typescript/index-realtime-data.ts

run-full-scenario: stop-all run-validator-with-geyser
	@echo "=== Running full test scenario ==="
	@sleep 5
	@echo "Starting indexer..."
	@make run-indexer
	@sleep 15
	@echo "Generating test data..."
	@make generate-data
	@echo "Starting real-time indexer..."
	@make run-realtime-indexer
