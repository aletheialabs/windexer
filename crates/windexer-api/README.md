# windexer-api

Unified API interface for wIndexer services, including REST endpoints, metrics, health checks, and deployment utilities.

## Features

- **REST API**: Standardized REST endpoints for interacting with wIndexer services
- **Health Checks**: Built-in health check system for monitoring service health
- **Metrics**: Metrics collection and reporting
- **Deployment Utilities**: Utilities for managing wIndexer deployments in various environments
- **Standalone Server**: Can be run as a standalone API server

## Usage

### As a Library

```rust
use windexer_api::{
    ApiConfig, ApiServer, HealthCheck, MetricsCollector, NodeInfo
};
use std::net::SocketAddr;

async fn start_api() -> anyhow::Result<()> {
    // Create API configuration
    let config = ApiConfig {
        bind_addr: "127.0.0.1:3000".parse()?,
        service_name: "my-windexer-service".to_string(),
        version: "1.0.0".to_string(),
        enable_metrics: true,
        node_info: Some(NodeInfo {
            node_id: "node-0".to_string(),
            node_type: "indexer".to_string(),
            listen_addr: "127.0.0.1:9000".to_string(),
            peer_count: 0,
            is_bootstrap: false,
        }),
        path_prefix: Some("/api".to_string()),
    };
    
    // Create and start the API server
    let server = ApiServer::new(config);
    
    // Register health checks
    let health = server.health();
    health.register("my-check", || async {
        // Perform health check
        // ...
    });
    
    // Start the server
    server.start().await?;
    
    Ok(())
}
```

### As a Standalone Server

The API server can be run as a standalone binary:

```bash
# Run with default settings
cargo run --bin windexer-api

# Specify bind address and service name
cargo run --bin windexer-api -- --bind-addr 0.0.0.0:8080 --service-name my-api

# Enable verbose logging
cargo run --bin windexer-api -- --verbose

# Load environment variables from file
cargo run --bin windexer-api -- --env-file .env
```

## API Endpoints

The server exposes the following endpoints by default:

- `/api/health` - Health check endpoint
- `/api/status` - Service status
- `/api/metrics` - Service metrics (if enabled)

Additional endpoints provided for deployment management:

- `/api/deployment` - GET: Get deployment information, POST: Update deployment
- `/api/validator` - Information about the Solana validator

## Docker Deployment

The API server can be included in a Docker container:

```dockerfile
FROM rust:slim-buster as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin windexer-api

FROM debian:buster-slim
WORKDIR /app
COPY --from=builder /app/target/release/windexer-api /app/
EXPOSE 3000
CMD ["./windexer-api", "--bind-addr", "0.0.0.0:3000"]
```

## Environment Variables

The API server supports the following environment variables:

- `RUST_LOG` - Logging level (e.g., `info`, `debug`)
- `BIND_ADDR` - Server bind address (e.g., `0.0.0.0:3000`)
- `SERVICE_NAME` - Service name
- `ENABLE_METRICS` - Enable metrics endpoint (true/false)

## Integration with Kubernetes

The API server provides endpoints compatible with Kubernetes health checks:

- `/api/health` can be used as a readiness/liveness probe
- `/api/metrics` can be scraped by Prometheus

Example Kubernetes deployment:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: windexer-api
spec:
  replicas: 1
  selector:
    matchLabels:
      app: windexer-api
  template:
    metadata:
      labels:
        app: windexer-api
    spec:
      containers:
      - name: windexer-api
        image: windexer-api:latest
        ports:
        - containerPort: 3000
        livenessProbe:
          httpGet:
            path: /api/health
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /api/health
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 5
``` 