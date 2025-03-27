import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, '../..');

console.log('Setting up wIndexer AVS with Cambrian integration');

// Ensure keypair directory exists
const keypairDir = path.join(projectRoot, 'data', 'cambrian');
if (!fs.existsSync(keypairDir)) {
  fs.mkdirSync(keypairDir, { recursive: true });
}

// Generate keypair if it doesn't exist
const keypairPath = path.join(keypairDir, 'admin-keypair.json');
if (!fs.existsSync(keypairPath)) {
  console.log('Generating admin keypair...');
  execSync(`solana-keygen new --no-passphrase -o ${keypairPath}`);
  console.log(`Admin keypair saved to ${keypairPath}`);
}

// Start the AVS
console.log('Starting wIndexer AVS...');
try {
  execSync(
    `cargo run --bin windexer-avs -- --ip 127.0.0.1 --http-port 8080 --ws-port 8081 --admin-keypair ${keypairPath} --initialize`,
    { stdio: 'inherit' }
  );
} catch (error) {
  console.error('Error running AVS:', error);
  process.exit(1);
} 