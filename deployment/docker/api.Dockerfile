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
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/windexer-api ./crates/windexer-api
COPY crates/windexer-common ./crates/windexer-common
COPY crates/windexer-store ./crates/windexer-store

# Build the application with minimal dependencies
RUN cd crates/windexer-api && \
    cargo build --release --bin windexer-api --no-default-features && \
    mv ../../target/release/windexer-api /usr/local/bin/

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
COPY --from=builder /usr/local/bin/windexer-api /usr/local/bin/

# Set the working directory
WORKDIR /app

# Set the entrypoint
ENTRYPOINT ["windexer-api"]

# Health check
HEALTHCHECK --interval=10s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:3000/api/health || exit 1

# Expose the API port
EXPOSE 3000 