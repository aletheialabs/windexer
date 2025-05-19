# Build stage - using the latest Rust stable
FROM rust:latest as builder
WORKDIR /app

# Copy the entire project structure
COPY Cargo.toml Cargo.lock ./

# Copy all crate directories 
COPY crates ./crates

# Set current directory to API crate
WORKDIR /app

# Build the API crate with debug symbols
RUN RUSTFLAGS="-C debuginfo=2" cargo build --release --package windexer-api

# Final image
FROM debian:bookworm-slim

# Install dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl-dev \
    curl \
    netcat-openbsd \
    procps \
    strace \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/windexer-api /usr/local/bin/windexer-api

# Set environment variables
ENV RUST_LOG=debug
ENV API_PORT=3000
ENV BIND_ADDR=0.0.0.0:3000
ENV HELIUS_API_KEY=your-api-key-here
ENV RUST_BACKTRACE=1

# Create startup script
WORKDIR /usr/local/bin
RUN touch start.sh && \
    echo '#!/bin/sh' > start.sh && \
    echo 'echo "Starting API server with environment:"' >> start.sh && \
    echo 'env | grep -E "RUST_|API_|BIND_|HELIUS_|SOLANA_"' >> start.sh && \
    echo '' >> start.sh && \
    echo '# Wait for PostgreSQL to be ready' >> start.sh && \
    echo 'echo "Waiting for PostgreSQL to be ready..."' >> start.sh && \
    echo 'until nc -z windexer-postgres 5432; do' >> start.sh && \
    echo '    echo "PostgreSQL is unavailable - sleeping 2s"' >> start.sh && \
    echo '    sleep 2' >> start.sh && \
    echo 'done' >> start.sh && \
    echo 'echo "PostgreSQL is up - starting API server"' >> start.sh && \
    echo '' >> start.sh && \
    echo '# Run the API server with strace to capture system calls' >> start.sh && \
    echo 'echo "Running with strace: windexer-api"' >> start.sh && \
    echo 'strace -f -o /tmp/api-strace.log windexer-api || {' >> start.sh && \
    echo '    echo "API server crashed with exit code $?"' >> start.sh && \
    echo '    echo "Last 20 lines of strace log:"' >> start.sh && \
    echo '    tail -20 /tmp/api-strace.log' >> start.sh && \
    echo '    exit 1' >> start.sh && \
    echo '}' >> start.sh && \
    chmod +x start.sh

# Expose the API port
EXPOSE ${API_PORT}

# Start the API server using the startup script
CMD ["/usr/local/bin/start.sh"]
