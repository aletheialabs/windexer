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

async function main() {
  const connection = new Connection('http://localhost:8899', 'confirmed');
  
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

  let balance = await connection.getBalance(payer.publicKey);
  console.log('Current balance:', balance / LAMPORTS_PER_SOL, 'SOL');
  
  if (balance < LAMPORTS_PER_SOL) {
    console.log('Requesting airdrop of 2 SOL...');
    const signature = await connection.requestAirdrop(
      payer.publicKey,
      2 * LAMPORTS_PER_SOL
    );
    await connection.confirmTransaction(signature);
    
    balance = await connection.getBalance(payer.publicKey);
    console.log('New balance:', balance / LAMPORTS_PER_SOL, 'SOL');
  }
  
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
      const signature = await sendAndConfirmTransaction(
        connection,
        transaction,
        [payer]
      );
      
      console.log(`Transaction ${i+1}: ${signature}`);
      console.log(`  Sent ${amount / LAMPORTS_PER_SOL} SOL to ${recipient.publicKey.toString()}`);
      
      await new Promise(resolve => setTimeout(resolve, 500));
      
    } catch (err) {
      console.error(`Error sending transaction ${i+1}:`, err);
    }
  }
  
  balance = await connection.getBalance(payer.publicKey);
  console.log('\nFinal balance:', balance / LAMPORTS_PER_SOL, 'SOL');
}

main().catch(err => console.error(err)); 