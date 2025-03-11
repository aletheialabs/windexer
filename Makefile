# Makefile
PROJECT := windexer
CARGO := cargo
NODES := 3
BASE_PORT := 9000
RPC_PORT_OFFSET := 1000
DATA_DIR := ./data

.PHONY: all build run-node clean demo run-validator-with build-geyser run-indexer-1 run-indexer-2

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

demo: build
	@./scripts/test-scripts/start-network.sh
	@./scripts/test-scripts/demo-interact.sh

run-validator-with-geyser: build-geyser
	solana-test-validator \
		--geyser-plugin-config config/geyser/windexer-geyser-config.json \
		--reset

build-geyser:
	cargo build --package windexer-geyser

run-indexer-1:
	cargo run --bin indexer -- --index 1 --bootstrap-peers 127.0.0.1:9000

run-indexer-2:
	cargo run --bin indexer -- --index 2 --bootstrap-peers 127.0.0.1:9001
