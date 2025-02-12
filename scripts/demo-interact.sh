# scripts/demo-interact.sh
#!/bin/bash

# Delegate to node 0 (bootstrap node)
echo "Delegating 50,000 units to bootstrap node..."
curl -X POST http://localhost:9100/delegate -d "amount=50000"

sleep 2

echo "Sending transaction through tip router..."
curl -X POST http://localhost:9100/transfer \
  -H "Content-Type: application/json" \
  -d '{"recipient":"5FZb31...", "amount":100}'

echo -e "\nNetwork Stats:"
echo "Node 0:"
curl -s http://localhost:9100/stats | jq .
echo -e "\nNode 1:"
curl -s http://localhost:9101/stats | jq .
echo -e "\nNode 2:"
curl -s http://localhost:9102/stats | jq .

# Show connected peers
echo -e "\nConnected Peers:"
curl -s http://localhost:9100/metrics | grep connected_peers
