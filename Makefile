# Makefile
PROJECT := windexer
CARGO := cargo
NODES := 3
BASE_PORT := 9000
RPC_PORT_OFFSET := 1000
DATA_DIR := ./data

.PHONY: all build run-node clean demo run-validator-with build-geyser run-indexer-1 run-indexer-2 run-geyser clean-geyser

all: build

build:
	@$(CARGO) build --workspace

run-node-%: build
	@$(CARGO) run --bin node -- \
		--index $* \
		--base-port $(BASE_PORT) \
		--enable-tip-route

run-local-network: build
	@for i in $$(seq 0 $(shell echo $$(($(NODES)-1)))); do \
		$(CARGO) run --bin node -- \
			--index $$i \
			--base-port $(BASE_PORT) \
			--enable-tip-route & \
	done

clean:
	@rm -rf $(DATA_DIR)/node_*
	@pkill -f '$(PROJECT)|solana-test-validator' || true
	@rm -rf ./avs-demo

demo: build
	@./scripts/test-scripts/start-network.sh
	@./scripts/test-scripts/demo-interact.sh

run-validator-with-geyser: build-geyser
	solana-test-validator \
		--geyser-plugin-config config/geyser/windexer-geyser-config.json \
		--reset \
		--faucet-port 9910 \
		--rpc-port 8999 \
		--bind-address 127.0.0.1

build-geyser:
	cargo build --package windexer-geyser

run-indexer-1:
	cargo run --bin indexer -- --index 1 --bootstrap-peers 127.0.0.1:9000

run-indexer-2:
	cargo run --bin indexer -- --index 2 --bootstrap-peers 127.0.0.1:9001

run-geyser:
	@./scripts/setup-windexer-geyser.sh

clean-geyser:
	@rm -rf windexer_geyser_setup

init-avs-devnet:
	@echo "Running camb init with Solana Devnet (recommended):"
	@echo "When prompted, enter the following:"
	@echo "Solana API URL: https://api.devnet.solana.com"
	@echo "Solana WS URL: wss://api.devnet.solana.com"
	@echo "Cambrian Consensus Program name: cambrian_devnet_$(shell date +%s)"
	@echo "The timestamp-based name above should be unique and avoid conflicts"
	@camb init -t avs ./avs-demo

init-avs-local:
	@echo "⚠️ WARNING: Local validator initialization requires Cambrian programs to be deployed first"
	@echo "This will likely fail unless you've deployed the required programs to your local validator"
	@echo "Using Devnet is recommended (run 'make init-avs-devnet' instead)"
	@echo ""
	@echo "If you still want to proceed, when prompted enter:"
	@echo "Solana API URL: http://127.0.0.1:8999"
	@echo "Solana WS URL: ws://127.0.0.1:9000"
	@camb init -t avs ./avs-demo

init-avs: init-avs-devnet

clean-init: clean init-avs-devnet

kill-validator:
	@pkill -f 'solana-test-validator' || true

run-validator-clean: kill-validator run-validator-with-geyser

init-avs-devnet-with-backoff:
	@echo "Running camb init with rate limit handling:"
	@echo "This will wait 30 seconds before trying to initialize to avoid rate limits"
	@echo "When prompted, enter the following:"
	@echo "Solana API URL: https://api.devnet.solana.com"
	@echo "Solana WS URL: wss://api.devnet.solana.com"
	@echo "Cambrian Consensus Program name: cambrian_devnet_$(shell date +%s)"
	@sleep 30
	@camb init -t avs ./avs-demo

init-avs-quicknode:
	@echo "Running camb init with Quicknode (recommended for avoiding rate limits):"
	@echo "You need to create a free Quicknode account at https://quicknode.com"
	@echo "When prompted, enter YOUR Quicknode URLs (examples below):"
	@echo "Solana API URL: https://your-endpoint.solana-devnet.quiknode.pro/your-api-key/"
	@echo "Solana WS URL: wss://your-endpoint.solana-devnet.quiknode.pro/your-api-key/"
	@echo "Cambrian Consensus Program name: cambrian_devnet_$(shell date +%s)"
	@camb init -t avs ./avs-demo

check-solana-devnet:
	@echo "Checking Solana devnet status..."
	@curl -s -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' https://api.devnet.solana.com | jq

init-avs-helius:
	@echo "Running camb init with Helius (free alternative to Quicknode):"
	@echo "You need to create a free Helius account at https://helius.xyz"
	@echo "When prompted, enter the Helius URLs (examples below):"
	@echo "Solana API URL: https://devnet.helius-rpc.com/?api-key=YOUR_API_KEY"
	@echo "Solana WS URL: wss://devnet.helius-rpc.com/?api-key=YOUR_API_KEY"
	@echo "Cambrian Consensus Program name: cambrian_devnet_$(shell date +%s)"
	@camb init -t avs ./avs-demo

init-avs-devnet-extreme-backoff:
	@echo "Running camb init with extreme rate limit handling:"
	@echo "This will wait 2 minutes before trying to initialize to avoid rate limits"
	@echo "When prompted, enter the following:"
	@echo "Solana API URL: https://api.devnet.solana.com"
	@echo "Solana WS URL: wss://api.devnet.solana.com"
	@echo "Cambrian Consensus Program name: cambrian_devnet_$(shell date +%s)"
	@echo "Waiting 2 minutes to allow rate limits to reset..."
	@sleep 120
	@camb init -t avs ./avs-demo

init-avs-alternative-rpc:
	@echo "Running camb init with alternative RPC endpoints:"
	@echo "When prompted, enter the following:"
	@echo "Solana API URL: https://floral-still-sun.solana-devnet.quiknode.pro/87336fc9fbaa83cde5a65aee30b5a4c58ba7a88d/"
	@echo "Solana WS URL: wss://floral-still-sun.solana-devnet.quiknode.pro/87336fc9fbaa83cde5a65aee30b5a4c58ba7a88d/"
	@echo "Cambrian Consensus Program name: cambrian_devnet_$(shell date +%s)"
	@camb init -t avs ./avs-demo

init-avs-prefunded:
	@echo "This approach uses a two-stage process to initialize the AVS:"
	@echo "1. First, create a wallet and fund it using the official devnet"
	@echo "2. Then use that funded wallet with Helius for faster processing"
	@echo ""
	@echo "Step 1: Generate a wallet keypair (will be saved to avs-wallet.json)"
	@solana-keygen new -o avs-wallet.json --no-bip39-passphrase
	@echo ""
	@echo "Step 2: Request an airdrop from the official devnet"
	@echo "Getting pubkey from keypair..."
	@PUBKEY=$$(solana-keygen pubkey avs-wallet.json) && \
		echo "Wallet address: $$PUBKEY" && \
		echo "Requesting airdrop of 2 SOL..." && \
		solana airdrop 2 $$PUBKEY --url https://api.devnet.solana.com
	@echo ""
	@echo "Step 3: Now initialize using Helius with your funded wallet"
	@echo "When prompted, enter the following:"
	@echo "AVS IP address: 127.0.0.1"
	@echo "AVS HTTP port: 3000"
	@echo "AVS WS port: 3001"
	@echo "Admin private key: $(shell cat avs-wallet.json)"
	@echo "Solana API URL: https://devnet.helius-rpc.com/?api-key=YOUR_API_KEY"
	@echo "Solana WS URL: wss://devnet.helius-rpc.com/?api-key=YOUR_API_KEY"
	@echo "Cambrian Consensus Program name: cambrian_devnet_$(shell date +%s)"
	@camb init -t avs ./avs-demo

.PHONY: cambrian-demo-setup cambrian-demo-start cambrian-demo-stop cambrian-demo-status cambrian-demo-proposal

cambrian-demo-setup:
	@echo "Setting up Cambrian AVS demo environment..."
	@mkdir -p examples/cambrian-avs-demo/configs examples/cambrian-avs-demo/logs
	@cd examples/cambrian-avs-demo && ./scripts/setup.sh

cambrian-demo-start:
	@./scripts/run-cambrian-demo.sh

cambrian-demo-stop:
	@./scripts/stop-cambrian-demo.sh

cambrian-demo-status:
	@./scripts/cambrian-demo-status.sh

cambrian-demo-proposal:
	@./scripts/execute-cambrian-proposal.sh basic

cambrian-demo-proposal-%:
	@./scripts/execute-cambrian-proposal.sh $*

cambrian-demo-clean:
	@echo "Cleaning Cambrian AVS demo resources..."
	@./scripts/stop-cambrian-demo.sh || true
	@rm -rf examples/cambrian-avs-demo/configs/avs-wallet.json examples/cambrian-avs-demo/logs/*
	@echo "✅ Cambrian AVS demo resources cleaned"

cambrian-demo: cambrian-demo-setup cambrian-demo-start
	@echo "Starting Cambrian demo workflow..."
	@sleep 5
	@./scripts/execute-cambrian-proposal.sh basic
	@echo "Demo is running. Press Ctrl+C to exit."

help::
	@echo ""
	@echo "Cambrian AVS Demo:"
	@echo "  make cambrian-demo-setup      Setup the Cambrian AVS demo environment"
	@echo "  make cambrian-demo-start      Start the Cambrian AVS demo"
	@echo "  make cambrian-demo-stop       Stop the Cambrian AVS demo"
	@echo "  make cambrian-demo-status     Check the status of the Cambrian AVS demo"
	@echo "  make cambrian-demo-proposal   Execute a basic proposal in the Cambrian AVS demo"
	@echo "  make cambrian-demo-proposal-X Execute a specific proposal (X can be: basic, nft, mpl, spl, update-nft)"
	@echo "  make cambrian-demo-clean      Clean Cambrian AVS demo resources"
	@echo "  make cambrian-demo            Run the complete workflow (setup, start, basic proposal)"
