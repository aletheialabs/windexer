# Build stage
FROM rust:1.75-slim-bookworm as builder
WORKDIR /app

# Install dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    build-essential \
    libssl-dev \
    pkg-config \
    git \
    && rm -rf /var/lib/apt/lists/*

# Create a standalone project for the API
RUN mkdir -p /tmp/standalone && \
    cd /tmp/standalone && \
    cargo init --bin windexer-api && \
    mkdir -p src

# Move to the project directory
WORKDIR /tmp/standalone

# Write main.rs file using separate statements to avoid any potential parsing issues
RUN echo 'use axum::{routing::get, Router};' > src/main.rs
RUN echo 'use std::net::SocketAddr;' >> src/main.rs
RUN echo 'use tokio::net::TcpListener;' >> src/main.rs
RUN echo 'use serde_json::json;' >> src/main.rs
RUN echo '' >> src/main.rs
RUN echo 'async fn health() -> axum::Json<serde_json::Value> {' >> src/main.rs
RUN echo '    axum::Json(json!({' >> src/main.rs
RUN echo '        "status": "ok",' >> src/main.rs
RUN echo '        "uptime": 0,' >> src/main.rs
RUN echo '        "version": "0.1.0"' >> src/main.rs
RUN echo '    }))' >> src/main.rs
RUN echo '}' >> src/main.rs
RUN echo '' >> src/main.rs
RUN echo 'async fn status() -> axum::Json<serde_json::Value> {' >> src/main.rs
RUN echo '    axum::Json(json!({' >> src/main.rs
RUN echo '        "status": "operational",' >> src/main.rs
RUN echo '        "version": "0.1.0",' >> src/main.rs
RUN echo '        "environment": "development"' >> src/main.rs
RUN echo '    }))' >> src/main.rs
RUN echo '}' >> src/main.rs
RUN echo '' >> src/main.rs
RUN echo 'async fn metrics() -> axum::Json<serde_json::Value> {' >> src/main.rs
RUN echo '    axum::Json(json!({' >> src/main.rs
RUN echo '        "nodes": 3,' >> src/main.rs
RUN echo '        "indexers": 2,' >> src/main.rs
RUN echo '        "metrics": {' >> src/main.rs
RUN echo '            "requests": 0,' >> src/main.rs
RUN echo '            "errors": 0' >> src/main.rs
RUN echo '        }' >> src/main.rs
RUN echo '    }))' >> src/main.rs
RUN echo '}' >> src/main.rs
RUN echo '' >> src/main.rs
RUN echo 'async fn deployment() -> axum::Json<serde_json::Value> {' >> src/main.rs
RUN echo '    axum::Json(json!({' >> src/main.rs
RUN echo '        "deployment": {' >> src/main.rs
RUN echo '            "id": "default",' >> src/main.rs
RUN echo '            "type": "docker",' >> src/main.rs
RUN echo '            "nodes": 3,' >> src/main.rs
RUN echo '            "indexers": 2' >> src/main.rs
RUN echo '        }' >> src/main.rs
RUN echo '    }))' >> src/main.rs
RUN echo '}' >> src/main.rs
RUN echo '' >> src/main.rs
RUN echo 'async fn validator() -> axum::Json<serde_json::Value> {' >> src/main.rs
RUN echo '    axum::Json(json!({' >> src/main.rs
RUN echo '        "validator": {' >> src/main.rs
RUN echo '            "status": "running",' >> src/main.rs
RUN echo '            "rpc_port": 8999,' >> src/main.rs
RUN echo '            "ws_port": 8900' >> src/main.rs
RUN echo '        }' >> src/main.rs
RUN echo '    }))' >> src/main.rs
RUN echo '}' >> src/main.rs
RUN echo '' >> src/main.rs
RUN echo '#[tokio::main]' >> src/main.rs
RUN echo 'async fn main() -> anyhow::Result<()> {' >> src/main.rs
RUN echo '    // Initialize tracing' >> src/main.rs
RUN echo '    tracing_subscriber::fmt::init();' >> src/main.rs
RUN echo '' >> src/main.rs
RUN echo '    // Get port from env or use default 3000' >> src/main.rs
RUN echo '    let port = std::env::var("API_PORT").unwrap_or_else(|_| "3000".to_string());' >> src/main.rs
RUN echo '    let addr = format!("0.0.0.0:{}", port).parse::<SocketAddr>()?;' >> src/main.rs
RUN echo '' >> src/main.rs
RUN echo '    // Create our application with routes' >> src/main.rs
RUN echo '    let app = Router::new()' >> src/main.rs
RUN echo '        .route("/api/health", get(health))' >> src/main.rs
RUN echo '        .route("/api/status", get(status))' >> src/main.rs
RUN echo '        .route("/api/metrics", get(metrics))' >> src/main.rs
RUN echo '        .route("/api/deployment", get(deployment))' >> src/main.rs
RUN echo '        .route("/api/validator", get(validator));' >> src/main.rs
RUN echo '' >> src/main.rs
RUN echo '    // Start the server' >> src/main.rs
RUN echo '    println!("Starting API server on {}", addr);' >> src/main.rs
RUN echo '    let listener = TcpListener::bind(addr).await?;' >> src/main.rs
RUN echo '    axum::serve(listener, app).await?;' >> src/main.rs
RUN echo '' >> src/main.rs
RUN echo '    Ok(())' >> src/main.rs
RUN echo '}' >> src/main.rs

# Create Cargo.toml file line by line
RUN echo '[package]' > Cargo.toml
RUN echo 'name = "windexer-api"' >> Cargo.toml
RUN echo 'version = "0.1.0"' >> Cargo.toml
RUN echo 'edition = "2021"' >> Cargo.toml
RUN echo '' >> Cargo.toml
RUN echo '[[bin]]' >> Cargo.toml
RUN echo 'name = "windexer-api"' >> Cargo.toml
RUN echo 'path = "src/main.rs"' >> Cargo.toml
RUN echo '' >> Cargo.toml
RUN echo '[dependencies]' >> Cargo.toml
RUN echo 'axum = { version = "0.7.4", features = ["macros", "json"] }' >> Cargo.toml
RUN echo 'tokio = { version = "1", features = ["full", "macros", "rt-multi-thread"] }' >> Cargo.toml
RUN echo 'anyhow = "1.0"' >> Cargo.toml
RUN echo 'serde = { version = "1.0", features = ["derive"] }' >> Cargo.toml
RUN echo 'serde_json = "1.0"' >> Cargo.toml
RUN echo 'tracing-subscriber = "0.3"' >> Cargo.toml

# Build the application
RUN cargo build --release

# Production stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    netcat-openbsd \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /tmp/standalone/target/release/windexer-api /usr/local/bin/

# Set the working directory
WORKDIR /app

# Set the entrypoint
ENTRYPOINT ["windexer-api"]

# Health check
HEALTHCHECK --interval=10s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:3000/api/health || exit 1

# Expose the API port
EXPOSE 3000
