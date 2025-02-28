# Makefile
PROJECT := windexer
CARGO := cargo
NODES := 3
BASE_PORT := 9000
RPC_PORT_OFFSET := 1000
DATA_DIR := ./data

.PHONY: all build run-node clean demo

all: build

build:
	@$(CARGO) build --workspace

run-publisher-%: build
	@$(CARGO) run --bin node -- \
		--node-type publisher \
		--index $* \
		--base-port $(BASE_PORT) \
		--enable-tip-route

run-relayer-%: build
	@$(CARGO) run --bin node -- \
		--node-type relayer \
		--index $* \
		--base-port $$(( $(BASE_PORT) + 100 )) \
		--enable-tip-route

run-local-network: build
	@for i in $$(seq 0 $(shell echo $$(($(NODES)-1)))); do \
		$(CARGO) run --bin node -- \
			--node-type publisher \
			--index $$i \
			--base-port $(BASE_PORT) \
			--enable-tip-route & \
	done
	@for i in $$(seq 0 $(shell echo $$(($(NODES)-1)))); do \
		$(CARGO) run --bin node -- \
			--node-type relayer \
			--index $$i \
			--base-port $$(( $(BASE_PORT) + 100 )) \
			--enable-tip-route & \
	done

clean:
	@rm -rf $(DATA_DIR)/node_*
	@pkill -f '$(PROJECT)|solana-test-validator' || true

demo: build
	@./scripts/start-network.sh
	@./scripts/demo-interact.sh
