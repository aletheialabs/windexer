#!/bin/bash

# Submit a proposal to the Cambrian AVS

PAYLOAD="basic"
if [ "$1" != "" ]; then
    PAYLOAD=$1
fi

case $PAYLOAD in
    "basic")
        PAYLOAD_FILE="payload-demo.ts"
        ;;
    "nft")
        PAYLOAD_FILE="payload-nft-demo.ts"
        ;;
    "mpl")
        PAYLOAD_FILE="payload-mpl-demo.ts"
        ;;
    "spl")
        PAYLOAD_FILE="payload-spl-demo.ts"
        ;;
    "update-nft")
        PAYLOAD_FILE="payload-update-mint-nft-auth.ts"
        ;;
    *)
        echo "❌ Unknown payload: $PAYLOAD"
        echo "Available payloads: basic, nft, mpl, spl, update-nft"
        exit 1
        ;;
esac

echo "🚀 Submitting proposal with payload: $PAYLOAD_FILE"

# Use the appropriate command to submit the proposal
docker-compose exec avs-service node ./src/submit-proposal.js --payload ./payloads/$PAYLOAD_FILE

echo "✅ Proposal submitted! Check the logs to monitor consensus: docker-compose logs -f" 