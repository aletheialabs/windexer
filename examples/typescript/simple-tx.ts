import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from '@solana/web3.js';
import fs from 'fs';
import path from 'path';

const __dirname = path.resolve();
const RPC_URL = 'http://localhost:8999';
const NUM_TRANSACTIONS = 5;

async function main() {
  console.log('=== Simple Transaction Generator ===');
  console.log(`Connecting to Solana validator: ${RPC_URL}`);
  
  // Initialize connection
  const connection = new Connection(RPC_URL, 'confirmed');
  
  // Load existing keypair
  let payer: Keypair;
  const keyPath = path.join(__dirname, 'payer-keypair.json');
  
  try {
    if (fs.existsSync(keyPath)) {
      const keypairData = JSON.parse(fs.readFileSync(keyPath, 'utf-8'));
      payer = Keypair.fromSecretKey(new Uint8Array(keypairData));
      console.log('✓ Loaded existing keypair:', payer.publicKey.toString());
    } else {
      payer = Keypair.generate();
      fs.writeFileSync(keyPath, JSON.stringify(Array.from(payer.secretKey)));
      console.log('✓ Created new keypair:', payer.publicKey.toString());
    }
  } catch (err) {
    console.error('Error with keypair:', err instanceof Error ? err.message : String(err));
    return;
  }
  
  // Check balance
  try {
    const balance = await connection.getBalance(payer.publicKey);
    console.log('Current balance:', balance / LAMPORTS_PER_SOL, 'SOL');
    
    if (balance < LAMPORTS_PER_SOL) {
      console.log('⚠️ Insufficient balance. Please airdrop using CLI:');
      console.log(`solana airdrop 2 ${payer.publicKey.toString()} --url ${RPC_URL}`);
      return;
    }
  } catch (err) {
    console.error('Error checking balance:', err instanceof Error ? err.message : String(err));
    return;
  }
  
  // Send test transactions
  console.log(`\nSending ${NUM_TRANSACTIONS} test transactions...`);
  let successCount = 0;
  
  for (let i = 0; i < NUM_TRANSACTIONS; i++) {
    const recipient = Keypair.generate();
    const amount = Math.floor(Math.random() * 0.009 * LAMPORTS_PER_SOL + 0.001 * LAMPORTS_PER_SOL);
    
    try {
      // Create transaction
      const transaction = new Transaction().add(
        SystemProgram.transfer({
          fromPubkey: payer.publicKey,
          toPubkey: recipient.publicKey,
          lamports: amount,
        })
      );
      
      // Set recent blockhash
      transaction.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
      
      // Sign the transaction
      transaction.sign(payer);
      
      // Send transaction without waiting for confirmation
      const signature = await connection.sendRawTransaction(transaction.serialize());
      successCount++;
      
      console.log(`✓ Transaction ${i+1}: ${signature}`);
      console.log(`  Sent ${amount / LAMPORTS_PER_SOL} SOL to ${recipient.publicKey.toString()}`);
      
      // Small delay between transactions
      await new Promise(resolve => setTimeout(resolve, 1000));
    } catch (err) {
      console.error(`❌ Error sending transaction ${i+1}:`, err instanceof Error ? err.message : String(err));
    }
  }
  
  // Check final balance
  try {
    const finalBalance = await connection.getBalance(payer.publicKey);
    console.log('\nFinal balance:', finalBalance / LAMPORTS_PER_SOL, 'SOL');
    console.log(`Successfully sent ${successCount}/${NUM_TRANSACTIONS} transactions`);
  } catch (err) {
    console.error('Error checking final balance:', err instanceof Error ? err.message : String(err));
  }
}

main().catch(err => console.error('Unexpected error:', err instanceof Error ? err.message : String(err))); 