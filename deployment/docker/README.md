## Environment Variables

The following environment variables are supported:

- `NODES`: Number of nodes to start (default: 3)
- `INDEXERS`: Number of indexers to start (default: 2)
- `BASE_PORT`: Base port for node services (default: 9000)
- `RPC_PORT`: Solana validator RPC port (default: 8899)
- `WS_PORT`: Solana validator WebSocket port (default: 8900)
- `FAUCET_PORT`: Solana faucet port (default: 9900)
- `API_PORT`: API service port (default: 3000)
- `LOG_LEVEL`: Logging level (default: info)

## Services

The following services are included in the wIndexer Docker environment:

- **solana-validator**: Runs a Solana test validator with the Geyser plugin
- **geyser-plugin**: Builds and provides the Geyser plugin to the validator
- **postgres**: PostgreSQL database for storing indexed data
- **node-0, node-1, node-2, etc.**: wIndexer nodes that form the P2P network
- **indexer-1, indexer-2, etc.**: wIndexer indexers that process and store blockchain data
- **data-generator**: Generates sample transactions for testing
- **api-server**: Unified API server for monitoring and managing the deployment

## API Service

The API service provides a unified interface for monitoring and managing the wIndexer deployment. It exposes the following endpoints:

- `/api/health`: Health check endpoint for all services
- `/api/status`: Service status and configuration
- `/api/metrics`: Service metrics (if enabled)
- `/api/deployment`: Deployment information and management
- `/api/validator`: Validator information

### Using the API

Once the environment is running, you can access the API at `http://localhost:3000`. Examples:

```bash
# Check system health
curl http://localhost:3000/api/health

# Get system status
curl http://localhost:3000/api/status

# Get deployment information
curl http://localhost:3000/api/deployment
```

The API is particularly useful for:
- Integrating with monitoring systems like Prometheus
- Setting up health checks for orchestration systems
- Automating deployment management
- Building management dashboards 