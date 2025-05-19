#!/bin/bash
# Script to deploy Jito MEV Analyzer to Azure VM

# Set variables
AZURE_VM_IP="172.174.114.6"
AZURE_VM_USER="winuser"
DEPLOY_DIR="/home/winuser/jito-analyzer"

# Print info
echo "Deploying Jito MEV Analyzer to Azure VM"
echo "IP: $AZURE_VM_IP"
echo "User: $AZURE_VM_USER"
echo "Deploy directory: $DEPLOY_DIR"

# Create deployment directory
echo "Creating deployment files..."
mkdir -p ./deploy

# Copy necessary files to deployment directory
cp Dockerfile ./deploy/
cp docker-compose.yml ./deploy/
cp requirements.txt ./deploy/
cp jito_analyzer_high_perf.py ./deploy/
cp jito_web_server.py ./deploy/
cp run_jito_analyzer.sh ./deploy/

# Create a README file
cat > ./deploy/README.md << EOL
# Jito MEV Analyzer

A web UI for analyzing Jito MEV bundles on Solana using wIndexer API.

## Running the service

\`\`\`
# Build and start the service
docker-compose up -d

# View logs
docker-compose logs -f

# Stop the service
docker-compose down
\`\`\`

Access the web UI at http://localhost:8000
EOL

# Make scripts executable
chmod +x ./deploy/run_jito_analyzer.sh

# Package files
echo "Creating deployment package..."
tar -czf jito-analyzer-deploy.tar.gz -C ./deploy .
rm -rf ./deploy

# Option to deploy to Azure VM
read -p "Deploy to Azure VM at $AZURE_VM_IP? (y/n): " DEPLOY_OPTION

if [ "$DEPLOY_OPTION" == "y" ]; then
  echo "Deploying to Azure VM..."
  
  # Copy package to Azure VM
  scp jito-analyzer-deploy.tar.gz $AZURE_VM_USER@$AZURE_VM_IP:/tmp/
  
  # SSH to VM and set up
  ssh $AZURE_VM_USER@$AZURE_VM_IP << EOF
    # Create deployment directory
    mkdir -p $DEPLOY_DIR
    
    # Extract files
    tar -xzf /tmp/jito-analyzer-deploy.tar.gz -C $DEPLOY_DIR
    
    # Navigate to directory
    cd $DEPLOY_DIR
    
    # Check if Docker is installed
    if ! command -v docker &> /dev/null; then
      echo "Docker not found. Please install Docker and Docker Compose first."
      exit 1
    fi
    
    # Build and start services
    docker-compose up -d
    
    # Clean up
    rm /tmp/jito-analyzer-deploy.tar.gz
    
    echo "Deployment complete! The Jito MEV Analyzer is running at http://$AZURE_VM_IP:8000"
EOF
  
  echo "Deployment process completed."
else
  echo "Skipping deployment to Azure VM."
  echo "To deploy manually, copy jito-analyzer-deploy.tar.gz to your VM and extract it."
fi

echo "Done!" 