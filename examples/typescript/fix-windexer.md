# wIndexer API Issues - Diagnosis and Fix

Based on the testing we've done, I've identified the following issues with the wIndexer API:

## Current Status

1. The API health endpoint is responding with "healthy"
2. The transactions endpoint is failing with an error: "Failed to get transactions: error decoding response body: missing field `result` at line 1 column 77"
3. The accounts endpoint is not returning any data
4. The Solana validator is running with the correct geyser plugin configuration
5. The generate-data.sh script successfully sent some transactions to the blockchain
6. Direct Solana RPC queries are not showing transactions for the sender account

## Potential Issues

1. **Geyser Plugin Configuration**: The wIndexer might not be correctly configured to receive transaction data from the Solana validator
2. **Transaction Indexing**: The API might have issues with how it's indexing or storing transactions
3. **Database Connection**: There might be a problem with the database connection or schema
4. **Transaction History Method**: The Solana method being used to get transaction history may not be appropriate for a local test validator

## Recommendations

### 1. Check Geyser Configuration

Make sure the geyser plugin is correctly configured in the validator:

```bash
# Verify geyser plugin config file
cat config/geyser/windexer-geyser-config.json

# Make sure it contains the correct database connection and is properly formatted
```

### 2. Implement Custom API Endpoints

The current API implementation seems to have issues with transaction retrieval. We can implement custom endpoints based on our `api-endpoints-guide.md` file:

```rust
// Add these endpoints to crates/windexer-api/src/endpoints.rs

// GET /api/stats
pub async fn get_stats(
    State(state): State<AppState>
) -> Json<ApiResponse<StatsResponse>> {
    let stats = StatsResponse {
        totalTransactions: 0, // Placeholder, implement with actual DB query
        totalAccounts: 0,     // Placeholder, implement with actual DB query
        avgTransactionsPerBlock: 0.0,
        lastUpdated: chrono::Utc::now().to_rfc3339(),
    };
    
    Json(ApiResponse::success(stats))
}

// Add to create_deployment_router()
.route("/stats", get(get_stats))
```

### 3. Debug Transaction Retrieval

The error appears to be in the transaction retrieval code. We should debug this by:

```bash
# Add more detailed logging to solana_client.rs
# Add this to the get_recent_transactions method:
info!("Response from transaction RPC call: {:?}", tx_response.text().await?);
```

### 4. Fix Direct Solana Transaction Queries

Update the query-all-data.ts script to use a different method for querying transactions:

```typescript
// Update the Solana transaction query section
console.log('\n=== Real Solana Transactions ===');

try {
  // Try multiple methods to get transactions
  let signatures = [];
  
  // Method 1: getSignaturesForAddress
  try {
    signatures = await connection.getSignaturesForAddress(new PublicKey(payerAccount), {
      limit: 5
    });
  } catch (error) {
    console.log(`getSignaturesForAddress failed: ${error.message}`);
  }
  
  // Method 2: If no signatures, try getConfirmedSignaturesForAddress2
  if (signatures.length === 0) {
    try {
      signatures = await connection.getConfirmedSignaturesForAddress2(new PublicKey(payerAccount), {
        limit: 5
      });
    } catch (error) {
      console.log(`getConfirmedSignaturesForAddress2 failed: ${error.message}`);
    }
  }
  
  // Method 3: If still no signatures, query the recipient account
  if (signatures.length === 0) {
    console.log(`No transactions found for ${payerAccount}, checking recipient account...`);
    const recipientAccount = '8u8pNT7SDdh3pot8z2spDLfdL6anbcxrJ8Sap8jxWtua';
    try {
      signatures = await connection.getSignaturesForAddress(new PublicKey(recipientAccount), {
        limit: 5
      });
      console.log(`Found ${signatures.length} transactions for recipient ${recipientAccount}`);
    } catch (error) {
      console.log(`getSignaturesForAddress for recipient failed: ${error.message}`);
    }
  }
  
  console.log(`Found ${signatures.length} recent transactions:`);
  
  // Process signatures as before...
}
```

### 5. Restart Services with Debugging

```bash
# Stop current services
make stop-indexer
make stop-validator-with-geyser

# Start with debug logging
export RUST_LOG=debug
make run-validator-with-geyser
make run-indexer
```

### 6. Check Database Schema

The database schema might not match what the API expects:

```bash
# Check database schema (SQLite)
cd /home/vivek/projects/aletheia/windexer/windexer
sqlite3 data/windexer.db .schema
```

### 7. Use Signature-Based Transaction Lookup

Since we have the transaction signatures from the generate-data.sh script output, we can directly query those transactions:

```typescript
// Add this to query-all-data.ts
console.log('\n=== Known Transactions ===');
const knownSignatures = [
  '3tdZxYH4img9EJVpu3cQANRVYDojABFySorbDViviPgYvduxyvesVrMGp1mHWMvXGUaD9pRF9owCLu1yjPyahAXg',
  'MLqwgPBnZuihPChiQuwvaKH6qSUgMFPAABAFpZXj5iQPFJQRwrFe5pP5a31tFtqz8APKYtQcnbRwSGEyDYAQNRP',
  '4JDkRe5BX21WMzDgV5UY4huoi64RVcPCo7Ae5pg1Rt2M3DDGKELr1o1ZeQ4ih9h5fmZJBSmS2i8mbienFy9tMybY'
  // Add more signatures from your generate-data.sh output
];

for (const signature of knownSignatures) {
  try {
    const tx = await connection.getTransaction(signature);
    if (tx) {
      console.log(`Transaction ${signature}:`);
      console.log(`  Block Time: ${new Date(tx.blockTime * 1000).toISOString()}`);
      console.log(`  Fee: ${tx.meta.fee / LAMPORTS_PER_SOL} SOL`);
      // Print more details as needed
    } else {
      console.log(`Transaction ${signature} not found`);
    }
  } catch (error) {
    console.log(`Error fetching transaction ${signature}: ${error.message}`);
  }
}
```

## Next Steps

1. Implement the fixes suggested above
2. Test with new transactions
3. Verify the data is being correctly indexed
4. If transactions still don't show up, try using the Solana CLI to explore them directly:

```bash
# Check transactions with the CLI
solana --url http://localhost:8899 confirm -v <SIGNATURE>
# Example:
solana --url http://localhost:8899 confirm -v 3tdZxYH4img9EJVpu3cQANRVYDojABFySorbDViviPgYvduxyvesVrMGp1mHWMvXGUaD9pRF9owCLu1yjPyahAXg
```

If the issues persist, we may need to check the source code in more detail, particularly how transactions are parsed and stored in the database. 