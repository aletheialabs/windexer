import axios from 'axios';

const WINDEXER_URL = 'http://localhost:10001';

async function main() {
  console.log('=== Querying wIndexer API ===');
  console.log(`Connecting to: ${WINDEXER_URL}`);
  
  try {
    // Check if wIndexer service is available
    try {
      const statusResponse = await axios.get(`${WINDEXER_URL}/api/status`, { timeout: 5000 });
      console.log('✓ Connected to wIndexer service');
      console.log('Status:', statusResponse.data);
      
      // Get transactions if available
      try {
        const txResponse = await axios.get(`${WINDEXER_URL}/api/transactions`);
        console.log('\nRecent Transactions:');
        if (Array.isArray(txResponse.data) && txResponse.data.length > 0) {
          txResponse.data.slice(0, 5).forEach((tx, i) => {
            console.log(`  ${i+1}. ${tx.signature}`);
          });
        } else {
          console.log('  No transactions available yet');
        }
      } catch (err) {
        console.log('  Transactions endpoint not available or no transactions indexed yet');
      }
      
      // Get accounts if available
      try {
        const accountsResponse = await axios.get(`${WINDEXER_URL}/api/accounts`);
        console.log('\nRecent Accounts:');
        if (Array.isArray(accountsResponse.data) && accountsResponse.data.length > 0) {
          accountsResponse.data.slice(0, 5).forEach((acct, i) => {
            console.log(`  ${i+1}. ${acct.pubkey}`);
          });
        } else {
          console.log('  No account data available yet');
        }
      } catch (err) {
        console.log('  Accounts endpoint not available or no accounts indexed yet');
      }
      
    } catch (error) {
      console.error('\n❌ Error connecting to wIndexer service on port 10001');
      console.error('\nPlease make sure the following services are running:');
      console.error('  1. Solana validator with Geyser plugin (make run-validator-with-geyser)');
      console.error('  2. wIndexer node (make run-node-0)');
      console.error('\nThen try this script again.');
    }
    
  } catch (err) {
    console.error('Unexpected error:', err instanceof Error ? err.message : String(err));
  }
}

main(); 