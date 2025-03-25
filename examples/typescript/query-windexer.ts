import axios from 'axios';

async function main() {
  console.log('=== Querying wIndexer API ===');
  
  try {
    console.log('Attempting to connect to wIndexer service...');
    
    try {
      const statusResponse = await axios.get('http://localhost:10001/api/status', { timeout: 5000 });
      console.log('Indexer Status:', statusResponse.data);
      
      try {
        const txResponse = await axios.get('http://localhost:10001/api/transactions');
        console.log('\nRecent Transactions:');
        if (Array.isArray(txResponse.data) && txResponse.data.length > 0) {
          txResponse.data.slice(0, 5).forEach((tx, i) => {
            console.log(`  ${i+1}. ${tx.signature}`);
          });
        } else {
          console.log('  No transactions available yet');
        }
      } catch (e) {
        console.log('  Transactions endpoint not available or no transactions yet');
      }
      
      try {
        const accountsResponse = await axios.get('http://localhost:10001/api/accounts');
        console.log('\nRecent Accounts:');
        if (Array.isArray(accountsResponse.data) && accountsResponse.data.length > 0) {
          accountsResponse.data.slice(0, 5).forEach((acct, i) => {
            console.log(`  ${i+1}. ${acct.pubkey}`);
          });
        } else {
          console.log('  No account data available yet');
        }
      } catch (e) {
        console.log('  Accounts endpoint not available or no accounts yet');
      }
      
    } catch (error) {
      console.error('\nError connecting to wIndexer service on port 10001.');
      console.error('Did you start the required services?\n');
      console.error('Run these commands in separate terminals:');
      console.error('  1. make run-validator-with-geyser');
      console.error('  2. make run-node-1');
      console.error('  3. make run-indexer-1');
      console.error('\nThen try this script again.\n');
    }
    
  } catch (err) {
    console.error('Unexpected error:', err);
  }
}

main(); 