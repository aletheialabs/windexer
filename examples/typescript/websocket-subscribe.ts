import { Connection, PublicKey } from '@solana/web3.js';

async function main() {
  // Connect to local Solana validator with WebSocket endpoint
  const connection = new Connection('http://localhost:8999', {
    wsEndpoint: 'ws://localhost:9000',
    commitment: 'confirmed'
  });

  console.log('=== Subscribing to Solana Events ===');
  console.log('RPC URL: http://localhost:8999');
  console.log('WebSocket URL: ws://localhost:9000');
  
  try {
    // Subscribe to slot changes
    const slotSubscriptionId = connection.onSlotChange(slot => {
      console.log(`Slot update: ${slot.slot}`);
    });
    console.log('✓ Subscribed to slot changes');
    
    // Subscribe to root changes
    const rootSubscriptionId = connection.onRootChange(root => {
      console.log(`Root update: ${root}`);
    });
    console.log('✓ Subscribed to root changes');
    
    // Subscribe to system program account changes
    const systemProgramId = new PublicKey('11111111111111111111111111111111');
    const accountSubscriptionId = connection.onAccountChange(
      systemProgramId,
      (accountInfo, context) => {
        console.log(`Account update at slot: ${context.slot}`);
        console.log(`  Owner: ${accountInfo.owner.toString()}`);
        console.log(`  Lamports: ${accountInfo.lamports}`);
        console.log(`  Executable: ${accountInfo.executable}`);
      }
    );
    console.log('✓ Subscribed to system program account changes');
    
    console.log('\nListening for events... Press Ctrl+C to exit');
    
    // Keep the script running
    await new Promise(() => {});
    
  } catch (err) {
    console.error('Error setting up WebSocket subscriptions:', 
      err instanceof Error ? err.message : String(err));
    console.error('Make sure the validator is running with WebSocket support');
  }
}

main().catch(err => 
  console.error('Unexpected error:', err instanceof Error ? err.message : String(err))); 