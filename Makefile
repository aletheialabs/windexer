# Makefile
PROJECT := windexer
CARGO := cargo
NODES := 3
BASE_PORT := 9000
RPC_PORT_OFFSET := 1000
DATA_DIR := ./data

.PHONY: all build run-local-network run-node clean demo

all: build

build:
	@$(CARGO) build --workspace

run-node-%:
	@$(CARGO) run --example node -- \
		--index $* \
		--base-port $(BASE_PORT) \
		--enable-tip-route

run-local-network: build
	@for i in $$(seq 0 $(shell echo $$(($(NODES)-1)))); do \
		$(CARGO) run --example node -- \
			--index $$i \
			--base-port $(BASE_PORT) \
			--enable-tip-route & \
	done

clean:
	@rm -rf $(DATA_DIR)/node_*
	@pkill -f '$(PROJECT)|solana-test-validator' || true

demo: build
	@./scripts/start-network.sh
	@./scripts/demo-interact.sh
