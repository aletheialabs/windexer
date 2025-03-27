import axios from 'axios';
import { execSync } from 'child_process';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, '../..');

// Path to payload directory
const payloadDir = path.join(projectRoot, 'examples', 'payloads', 'check-oracle');

async function executeProposal() {
  console.log('Executing wIndexer proposal with Cambrian integration');
  
  try {
    // 1. Get AVS status to ensure it's running
    const avsStatus = await axios.get('http://localhost:8080/api/status');
    console.log('AVS Status:', avsStatus.data);
    
    // 2. Build payload container
    console.log('Building payload container...');
    execSync(`docker build -t payload-check-oracle ${payloadDir}`, { stdio: 'inherit' });
    
    // 3. Execute proposal
    console.log('Executing proposal...');
    const proposalResult = await axios.post('http://localhost:8080/api/payload/run', {
      payloadImage: 'payload-check-oracle'
    });
    
    console.log('Proposal executed successfully!');
    console.log('Signature:', proposalResult.data.signature);
    
  } catch (error) {
    console.error('Error executing proposal:', error);
    if (error.response) {
      console.error('Response:', error.response.data);
    }
  }
}

executeProposal(); 