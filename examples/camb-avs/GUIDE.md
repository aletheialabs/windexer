# Cambrian AVS Integration Demo Guide

This guide will help you record a demonstration of the Cambrian AVS integration with the wIndexer Jito staking module.

## Prerequisites

Before starting the demo, ensure you have:

1. Installed the Cambrian CLI
   ```
   # Installation command (replace with actual installation method)
   curl -sSf https://cambrian.one/install.sh | sh
   ```

2. Installed the Solana CLI tools
   ```
   sh -c "$(curl -sSfL https://release.solana.com/v1.16.0/install)"
   ```

3. Have Docker installed and running

## Setup Instructions

1. Open two terminal windows
2. In the first terminal, navigate to your project root
3. Make the demo scripts executable:
   ```bash
   chmod +x scripts/run-cambrian-demo.sh
   chmod +x scripts/execute-cambrian-proposal.sh
   ```

## Demo Script

### Terminal 1: Running the AVS

1. Start the recording
2. Explain: "Today I'll demonstrate the integration of Cambrian's Actively Validated Service framework with wIndexer's Jito staking module."
3. Run the demo script:
   ```bash
   ./scripts/run-cambrian-demo.sh
   ```
4. Narrate what's happening:
   - "The script is setting up our demo environment by creating necessary directories and files."
   - "Now it's initializing our AVS using the Cambrian CLI."
   - "A Solana keypair is being generated for the AVS admin account."
   - "The AVS is starting up and initializing on-chain with the Cambrian CLI."
   - "Note the PoA pubkey shown during initialization - we'll need this for the proposal execution."

### Terminal 2: Executing a Proposal

1. While Terminal 1 is still running the AVS, open Terminal 2
2. Explain: "Now that our AVS is running, let's execute a proposal through it using the Cambrian CLI."
3. Run the proposal execution script:
   ```bash
   POA_PUBKEY=<pubkey from terminal 1> ./scripts/execute-cambrian-proposal.sh
   ```
4. Narrate what's happening:
   - "This script builds a Docker container with our proposal payload."
   - "It then submits the payload to our running AVS using the Cambrian CLI."
   - "The Cambrian CLI executes the payload and returns a proposal file."
   - "Finally, the proposal is submitted to the on-chain PoA program."
   - "This would trigger a consensus process among validators before execution."

### Demo Conclusion

1. Explain the significance:
   - "This demonstrates how wIndexer can directly integrate with Cambrian's AVS framework using the official Cambrian CLI."
   - "Validators can participate in additional network services while staking through Jito."
   - "The Proof-of-Authority model ensures secure consensus for all network actions."
2. Stop the recording

## Troubleshooting

If you encounter any issues during the demo:

- Ensure Cambrian CLI is installed and on your PATH (`cambrian --version`)
- Ensure Solana CLI tools are installed (`solana --version`)
- Ensure Docker is running (`docker --version`)
- Check that ports 8080 and 8081 are not in use by other applications
- If the AVS fails to start, check the logs for specific errors

## Cambrian CLI Commands Reference

Here are the key Cambrian CLI commands used in this demo:

```bash
# Initialize a new AVS
cambrian avs init --keypair <path> --name <name> --url <rpc-url>

# Run an AVS
cambrian avs run --keypair <path> --ip <ip> --http-port <port> --ws-port <port> --url <rpc-url>

# Run a payload
cambrian payload run --keypair <path> --image <image> --poa <pubkey> --url <rpc-url> --output <file>

# Submit a proposal
cambrian proposal submit --keypair <path> --poa <pubkey> --proposal <file> --url <rpc-url>
``` 