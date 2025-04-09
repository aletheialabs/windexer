# wIndexer API Endpoints Implementation Guide

This guide provides sample implementations for the additional API endpoints used by the `query-all-data.ts` script. If you're seeing "endpoint not available" messages in the script output, you can implement these endpoints in your wIndexer API server.

## Sample Endpoint Implementations

Below are examples of how to implement the API endpoints in a Node.js Express server:

### 1. Health Status Endpoint

```javascript
// GET /api/health
app.get('/api/health', (req, res) => {
  const startTime = process.uptime();
  const currentBlock = db.getLatestProcessedBlock();
  
  res.json({
    status: 'healthy',
    version: process.env.APP_VERSION || '1.0.0',
    uptime: startTime,
    blockHeight: currentBlock.slot,
    syncStatus: currentBlock.timestamp > Date.now() - 60000 ? 'synced' : 'syncing'
  });
});
```

### 2. Statistics Endpoint

```javascript
// GET /api/stats
app.get('/api/stats', async (req, res) => {
  try {
    const totalTransactions = await db.count('transactions');
    const totalAccounts = await db.count('accounts');
    const recentBlocks = await db.getRecentBlocks(100);
    
    // Calculate average transactions per block
    const txsInBlocks = recentBlocks.reduce((sum, block) => sum + block.transactionCount, 0);
    const avgTxPerBlock = recentBlocks.length > 0 ? txsInBlocks / recentBlocks.length : 0;
    
    res.json({
      totalTransactions,
      totalAccounts,
      avgTransactionsPerBlock: avgTxPerBlock.toFixed(2),
      lastUpdated: new Date().toISOString()
    });
  } catch (error) {
    console.error('Error fetching stats:', error);
    res.status(500).json({ error: 'Failed to fetch statistics' });
  }
});
```

### 3. Token Accounts Endpoint

```javascript
// GET /api/tokens/:address
app.get('/api/tokens/:address', async (req, res) => {
  try {
    const { address } = req.params;
    
    // Validate the address is a valid Solana public key
    try {
      new PublicKey(address);
    } catch (error) {
      return res.status(400).json({ error: 'Invalid Solana address' });
    }
    
    // Find token accounts owned by this address
    const tokenAccounts = await db.query(`
      SELECT 
        ta.mint,
        ta.owner,
        ta.amount as balance,
        ti.decimals
      FROM 
        token_accounts ta
      JOIN
        token_info ti ON ta.mint = ti.mint
      WHERE 
        ta.owner = ?
    `, [address]);
    
    res.json(tokenAccounts);
  } catch (error) {
    console.error('Error fetching token accounts:', error);
    res.status(500).json({ error: 'Failed to fetch token accounts' });
  }
});
```

### 4. Transactions by Account Endpoint

```javascript
// GET /api/transactions/byAccount/:address
app.get('/api/transactions/byAccount/:address', async (req, res) => {
  try {
    const { address } = req.params;
    const limit = parseInt(req.query.limit) || 20;
    
    // Validate the address is a valid Solana public key
    try {
      new PublicKey(address);
    } catch (error) {
      return res.status(400).json({ error: 'Invalid Solana address' });
    }
    
    // Find transactions that include this address
    const transactions = await db.query(`
      SELECT 
        tx.signature,
        tx.slot,
        tx.success,
        tx.fee,
        tx.block_time
      FROM 
        transactions tx
      JOIN
        transaction_accounts ta ON tx.signature = ta.transaction_signature
      WHERE 
        ta.account = ?
      ORDER BY
        tx.block_time DESC
      LIMIT ?
    `, [address, limit]);
    
    res.json(transactions);
  } catch (error) {
    console.error('Error fetching transactions by account:', error);
    res.status(500).json({ error: 'Failed to fetch transactions' });
  }
});
```

### 5. Transaction by Signature Endpoint

```javascript
// GET /api/transactions/:signature
app.get('/api/transactions/:signature', async (req, res) => {
  try {
    const { signature } = req.params;
    
    // Find transaction by signature
    const transaction = await db.query(`
      SELECT 
        tx.signature,
        tx.slot,
        tx.success,
        tx.fee,
        tx.block_time,
        (SELECT GROUP_CONCAT(account) FROM transaction_accounts WHERE transaction_signature = tx.signature) as accounts
      FROM 
        transactions tx
      WHERE 
        tx.signature = ?
    `, [signature]);
    
    if (transaction.length === 0) {
      return res.status(404).json({ error: 'Transaction not found' });
    }
    
    // Parse the comma-separated accounts string into an array
    transaction[0].accounts = transaction[0].accounts ? transaction[0].accounts.split(',') : [];
    
    res.json(transaction[0]);
  } catch (error) {
    console.error('Error fetching transaction:', error);
    res.status(500).json({ error: 'Failed to fetch transaction' });
  }
});
```

### 6. Program Accounts Endpoint

```javascript
// GET /api/programs/:programId/accounts
app.get('/api/programs/:programId/accounts', async (req, res) => {
  try {
    const { programId } = req.params;
    const limit = parseInt(req.query.limit) || 100;
    
    // Validate the programId is a valid Solana public key
    try {
      new PublicKey(programId);
    } catch (error) {
      return res.status(400).json({ error: 'Invalid program ID' });
    }
    
    // Find accounts owned by this program
    const accounts = await db.query(`
      SELECT 
        pubkey,
        lamports,
        owner,
        executable
      FROM 
        accounts
      WHERE 
        owner = ?
      LIMIT ?
    `, [programId, limit]);
    
    res.json(accounts);
  } catch (error) {
    console.error('Error fetching program accounts:', error);
    res.status(500).json({ error: 'Failed to fetch program accounts' });
  }
});
```

## Database Schema Considerations

To support these endpoints, you might need to update your database schema to include:

1. A `transactions` table with signature, slot, success, fee, block_time, etc.
2. A `transaction_accounts` join table connecting transactions to accounts
3. A `token_accounts` table for token account data
4. A `token_info` table for token metadata like decimals

## Integration with wIndexer

These endpoints should be integrated with the existing wIndexer API server. The implementation will depend on your database structure and how you're processing Solana data.

## Testing

You can test these endpoints with the `query-all-data.ts` script:

```bash
npm run query-all-data
# or with interactive mode
npm run query-all-data-interactive
```

When properly implemented, you should no longer see "endpoint not available" messages in the output. 