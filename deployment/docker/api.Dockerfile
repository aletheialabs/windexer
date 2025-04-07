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

# First, create a new standalone project for the API
RUN mkdir -p /tmp/standalone && \
    cd /tmp/standalone && \
    cargo init --bin windexer-api && \
    mkdir -p src/health src/api src/metrics src/model

# Copy the API source code from the workspace
COPY crates/windexer-api/src/main.rs /tmp/standalone/src/main.rs
COPY crates/windexer-api/src/health.rs /tmp/standalone/src/health.rs
COPY crates/windexer-api/src/api.rs /tmp/standalone/src/api.rs
COPY crates/windexer-api/src/metrics.rs /tmp/standalone/src/metrics.rs
COPY crates/windexer-api/src/model.rs /tmp/standalone/src/model.rs
COPY crates/windexer-api/src/lib.rs /tmp/standalone/src/lib.rs

# Copy the modified Cargo.toml without workspace dependencies
WORKDIR /tmp/standalone
RUN echo '[package]                                          \n\
name = "windexer-api"                                        \n\
version = "0.1.0"                                            \n\
edition = "2021"                                             \n\
                                                             \n\
[[bin]]                                                      \n\
name = "windexer-api"                                        \n\
path = "src/main.rs"                                         \n\
                                                             \n\
[dependencies]                                               \n\
# API dependencies                                           \n\
axum = { version = "0.7.4", features = ["macros"] }          \n\
tower = "0.4.13"                                             \n\
thiserror = "2.0"                                            \n\
reqwest = { version = "0.11.24", features = ["json"] }       \n\
chrono = "0.4.31"                                            \n\
tokio = { version = "1", features = ["full"] }               \n\
anyhow = "1.0"                                               \n\
serde = { version = "1.0", features = ["derive"] }           \n\
tracing = "0.1"                                              \n\
tracing-subscriber = { version = "0.3", features = ["env-filter"] } \n\
serde_json = "1.0"                                           \n\
' > Cargo.toml

# Modify the main.rs/lib.rs to work without dependencies
RUN sed -i 's/windexer_common//' src/lib.rs || true && \
    sed -i 's/windexer_store//' src/lib.rs || true && \
    sed -i 's/use windexer_common[^;]*;//g' src/*.rs || true && \
    sed -i 's/use windexer_store[^;]*;//g' src/*.rs || true

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