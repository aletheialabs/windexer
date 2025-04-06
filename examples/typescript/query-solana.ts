import { Connection, PublicKey, LAMPORTS_PER_SOL } from '@solana/web3.js';

async function main() {
  // Connect to local Solana validator
  const connection = new Connection('http://localhost:8999', 'confirmed');
  console.log('=== Solana Validator Info ===');
  
  try {
    // Get basic validator info
    const version = await connection.getVersion();
    console.log('Solana Version:', version);
    
    const clusterNodes = await connection.getClusterNodes();
    const validatorIdentity = clusterNodes.length > 0 ? clusterNodes[0].pubkey : 'Unknown';
    console.log('Validator Identity:', validatorIdentity);
    
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
    console.log('Recent Blockhash:', blockhash);
    console.log('Block Height:', lastValidBlockHeight);
    
    const slot = await connection.getSlot();
    console.log('Current Slot:', slot);
    
    const txCount = await connection.getTransactionCount();
    console.log('Transaction Count:', txCount);
    
    // Get largest accounts
    try {
      const largestAccounts = await connection.getLargestAccounts();
      console.log('\nLargest Accounts:');
      largestAccounts.value.slice(0, 5).forEach((account, i) => {
        console.log(`  ${i+1}. ${account.address.toString()}: ${account.lamports / LAMPORTS_PER_SOL} SOL`);
      });
    } catch (err) {
      console.log('\nCould not fetch largest accounts:', err instanceof Error ? err.message : String(err));
    }
    
  } catch (err) {
    console.error('Error connecting to Solana validator:', err instanceof Error ? err.message : String(err));
    console.error('Make sure the validator is running on port 8999');
  }
}

main().catch(err => console.error('Unexpected error:', err.message)); 