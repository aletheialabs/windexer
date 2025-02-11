.PHONY: run-local clean

data:
	mkdir -p data

run-local: data
	cargo run --bin local-network

clean:
	rm -rf ./data/node_*