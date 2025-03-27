#!/bin/bash
set -e

echo "===== Executing Cambrian Proposal with Windexer Integration ====="

if ! docker ps | grep -q "cambrian-avs"; then
  echo "❌ Cambrian AVS demo is not running. Please start it first with scripts/run-cambrian-demo.sh"
  exit 1
fi

PAYLOAD="basic"
if [ "$1" != "" ]; then
    PAYLOAD=$1
fi

echo "📝 Executing proposal with payload: $PAYLOAD"
cd examples/cambrian-avs-demo
./scripts/submit-proposal.sh $PAYLOAD
cd ../..

echo "📊 Showing execution logs (press Ctrl+C to exit):"
cd examples/cambrian-avs-demo
docker-compose logs -f avs-service
cd ../..

echo "✅ Proposal execution command completed"