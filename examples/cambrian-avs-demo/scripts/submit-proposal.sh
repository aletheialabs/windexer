#!/bin/bash

# Submit a proposal to the real Windexer-Cambrian AVS

PAYLOAD="basic"
if [ "$1" != "" ]; then
    PAYLOAD=$1
fi

echo "🚀 Submitting proposal with payload: $PAYLOAD"

# Create payloads directory if it doesn't exist
mkdir -p payloads

# Define payload contents based on type
case $PAYLOAD in
    "basic")
        PAYLOAD_CONTENT='{
            "type": "basic",
            "data": {
                "message": "Basic proposal execution"
            }
        }'
        ;;
    "nft")
        PAYLOAD_CONTENT='{
            "type": "nft",
            "data": {
                "action": "create",
                "name": "Demo NFT",
                "symbol": "DEMO",
                "uri": "https://example.com/metadata.json"
            }
        }'
        ;;
    "mpl")
        PAYLOAD_CONTENT='{
            "type": "mpl",
            "data": {
                "action": "create_collection",
                "name": "Demo Collection",
                "symbol": "DCOL",
                "uri": "https://example.com/collection.json"
            }
        }'
        ;;
    "spl")
        PAYLOAD_CONTENT='{
            "type": "spl",
            "data": {
                "action": "transfer",
                "amount": "1000",
                "recipient": "11111111111111111111111111111111"
            }
        }'
        ;;
    "update-nft")
        PAYLOAD_CONTENT='{
            "type": "update-nft",
            "data": {
                "mint": "11111111111111111111111111111111",
                "new_authority": "11111111111111111111111111111111"
            }
        }'
        ;;
    *)
        echo "❌ Unknown payload type: $PAYLOAD"
        echo "Available types: basic, nft, mpl, spl, update-nft"
        exit 1
        ;;
esac

# Save payload to file
echo "$PAYLOAD_CONTENT" > "payloads/${PAYLOAD}.json"

# Submit proposal to AVS
echo "Sending request to AVS..."
curl -s -X POST \
     -H "Content-Type: application/json" \
     -d "$PAYLOAD_CONTENT" \
     http://localhost:3000/api/proposals/submit || echo "Failed to connect to AVS"

echo "✅ Proposal submitted! Check the logs to monitor consensus: docker-compose logs -f" 