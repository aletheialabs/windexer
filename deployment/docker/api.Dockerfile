# deployment/docker/api.Dockerfile

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

# Create a minimal API implementation
WORKDIR /tmp/standalone

# Create source files for a simple API
COPY <<'EOF' src/main.rs
use axum::{routing::get, Router};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use serde_json::json;

async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(json!({
        "status": "ok",
        "uptime": 0,
        "version": "0.1.0"
    }))
}

async fn status() -> axum::Json<serde_json::Value> {
    axum::Json(json!({
        "status": "operational",
        "version": "0.1.0",
        "environment": "development"
    }))
}

async fn metrics() -> axum::Json<serde_json::Value> {
    axum::Json(json!({
        "nodes": 3,
        "indexers": 2,
        "metrics": {
            "requests": 0,
            "errors": 0
        }
    }))
}

async fn deployment() -> axum::Json<serde_json::Value> {
    axum::Json(json!({
        "deployment": {
            "id": "default",
            "type": "docker",
            "nodes": 3,
            "indexers": 2
        }
    }))
}

async fn validator() -> axum::Json<serde_json::Value> {
    axum::Json(json!({
        "validator": {
            "status": "running",
            "rpc_port": 8999,
            "ws_port": 8900
        }
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    // Get port from env or use default 3000
    let port = std::env::var("API_PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port).parse::<SocketAddr>()?;
    
    // Create our application with routes
    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/status", get(status))
        .route("/api/metrics", get(metrics))
        .route("/api/deployment", get(deployment))
        .route("/api/validator", get(validator));
    
    // Start the server
    println!("Starting API server on {}", addr);
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
EOF

# Create a minimal Cargo.toml
COPY <<'EOF' Cargo.toml
[package]
name = "windexer-api"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "windexer-api"
path = "src/main.rs"

[dependencies]
axum = { version = "0.7.4", features = ["macros", "json"] }
tokio = { version = "1", features = ["full", "macros", "rt-multi-thread"] }
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing-subscriber = "0.3"
EOF

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