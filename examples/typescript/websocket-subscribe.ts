import { Connection, PublicKey } from '@solana/web3.js';

async function main() {
  const connection = new Connection('http://localhost:8899', {
    wsEndpoint: 'ws://localhost:8900',
    commitment: 'confirmed'
  });

  console.log('=== Subscribing to Solana Events ===');
  
  try {
    const slotSubscriptionId = connection.onSlotChange(slot => {
      console.log('Slot update:', slot);
    });
    console.log('Subscribed to slot changes, ID:', slotSubscriptionId);
    
    const rootSubscriptionId = connection.onRootChange(root => {
      console.log('Root update:', root);
    });
    console.log('Subscribed to root changes, ID:', rootSubscriptionId);
    
    const systemProgramId = new PublicKey('11111111111111111111111111111111');
    const accountSubscriptionId = connection.onAccountChange(
      systemProgramId,
      (accountInfo, context) => {
        console.log('Account update:', context.slot);
        console.log('  Owner:', accountInfo.owner.toString());
        console.log('  Lamports:', accountInfo.lamports);
        console.log('  Executable:', accountInfo.executable);
      }
    );
    console.log('Subscribed to system program account changes, ID:', accountSubscriptionId);
    
    console.log('\nListening for events... Press Ctrl+C to exit');
    
    await new Promise(() => {});
    
  } catch (err) {
    console.error('Error setting up WebSocket subscriptions:', err);
  }
}

main(); 