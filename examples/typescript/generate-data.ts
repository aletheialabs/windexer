import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction
} from '@solana/web3.js';
import fs from 'fs';
import path from 'path';

const __dirname = path.resolve();
const RPC_URL = 'http://localhost:8999';

async function main() {
  console.log(`Connecting to Solana validator: ${RPC_URL}`);
  const connection = new Connection(RPC_URL, 'confirmed');
  
  // Load or create a keypair for the payer
  let payer: Keypair;
  const keyPath = path.join(__dirname, 'payer-keypair.json');
  
  if (fs.existsSync(keyPath)) {
    const keypairData = JSON.parse(fs.readFileSync(keyPath, 'utf-8'));
    payer = Keypair.fromSecretKey(new Uint8Array(keypairData));
    console.log('Loaded existing payer:', payer.publicKey.toString());
  } else {
    payer = Keypair.generate();
    fs.writeFileSync(keyPath, JSON.stringify(Array.from(payer.secretKey)));
    console.log('Created new payer:', payer.publicKey.toString());
  }

  // Check the payer's balance
  let balance = await connection.getBalance(payer.publicKey);
  console.log('Current balance:', balance / LAMPORTS_PER_SOL, 'SOL');
  
  // Request an airdrop if needed
  if (balance < LAMPORTS_PER_SOL) {
    console.log('Requesting airdrop of 2 SOL...');
    try {
      const signature = await connection.requestAirdrop(
        payer.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      
      // Note: confirmTransaction may fail due to WebSocket issues
      // Consider using the Solana CLI for airdrops if this fails
      try {
        await connection.confirmTransaction(signature);
        balance = await connection.getBalance(payer.publicKey);
        console.log('New balance:', balance / LAMPORTS_PER_SOL, 'SOL');
      } catch (err) {
        console.log('Transaction may have been confirmed, but WebSocket confirmation failed');
        console.log('Check balance manually with: solana balance', payer.publicKey.toString(), `--url ${RPC_URL}`);
        
        // Wait a bit and check balance again
        await new Promise(resolve => setTimeout(resolve, 2000));
        balance = await connection.getBalance(payer.publicKey);
        console.log('Current balance after airdrop attempt:', balance / LAMPORTS_PER_SOL, 'SOL');
      }
    } catch (err) {
      console.error('Airdrop failed:', err instanceof Error ? err.message : String(err));
      console.error('Try using Solana CLI instead:');
      console.error(`solana airdrop 2 ${payer.publicKey.toString()} --url ${RPC_URL}`);
      return;
    }
  }
  
  // Send test transactions
  const NUM_TRANSACTIONS = 10;
  console.log(`\nSending ${NUM_TRANSACTIONS} test transactions...`);
  
  for (let i = 0; i < NUM_TRANSACTIONS; i++) {
    const recipient = Keypair.generate();
    const amount = Math.floor(Math.random() * 0.009 * LAMPORTS_PER_SOL + 0.001 * LAMPORTS_PER_SOL);
    
    const transaction = new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: payer.publicKey,
        toPubkey: recipient.publicKey,
        lamports: amount,
      })
    );
    
    try {
      // This may fail with WebSocket errors
      const signature = await sendAndConfirmTransaction(
        connection,
        transaction,
        [payer]
      );
      
      console.log(`Transaction ${i+1}: ${signature}`);
      console.log(`  Sent ${amount / LAMPORTS_PER_SOL} SOL to ${recipient.publicKey.toString()}`);
      
      await new Promise(resolve => setTimeout(resolve, 500));
      
    } catch (err) {
      console.error(`Error sending transaction ${i+1}:`, err instanceof Error ? err.message : String(err));
      console.log('If you encounter WebSocket errors, try using simple-tx.ts instead:');
      console.log('npm run simple-tx');
      break;
    }
  }
  
  // Check final balance
  try {
    balance = await connection.getBalance(payer.publicKey);
    console.log('\nFinal balance:', balance / LAMPORTS_PER_SOL, 'SOL');
  } catch (err) {
    console.error('Error checking final balance:', err instanceof Error ? err.message : String(err));
  }
}

main().catch(err => console.error('Unexpected error:', err instanceof Error ? err.message : String(err))); 