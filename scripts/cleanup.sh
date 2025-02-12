# scripts/cleanup.sh
#!/bin/bash

pkill -f 'windexer|solana-test-validator'

rm -rf ./data/node_*

rm -f solana-*.log
