#!/bin/bash
set -e

echo "===== Cambrian AVS Demo Status ====="

if docker ps | grep -q "cambrian-avs"; then
  echo "✅ Cambrian AVS demo is running"
  echo "Running containers:"
  docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" | grep cambrian
  
  echo ""
  echo "Available endpoints:"
  echo "- AVS API: http://localhost:3000"
  echo "- Windexer API: http://localhost:9000"
  if docker ps | grep -q "cambrian-demo-ui"; then
    echo "- Demo UI: http://localhost:8080"
  fi
  
  echo ""
  echo "Available actions:"
  echo "- Submit a proposal: ./scripts/execute-cambrian-proposal.sh [payload-type]"
  echo "- Stop the demo: ./scripts/stop-cambrian-demo.sh"
else
  echo "❌ Cambrian AVS demo is not running"
  echo "To start the demo, run: ./scripts/run-cambrian-demo.sh"
fi 