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

# Copy Cargo.toml files to cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY crates/windexer-api/Cargo.toml ./crates/windexer-api/
COPY crates/windexer-common/Cargo.toml ./crates/windexer-common/

# Create dummy source files for dependencies to build
RUN mkdir -p crates/windexer-api/src crates/windexer-common/src && \
    echo "fn main() {}" > crates/windexer-api/src/main.rs && \
    echo "pub fn dummy() {}" > crates/windexer-api/src/lib.rs && \
    echo "pub fn dummy() {}" > crates/windexer-common/src/lib.rs && \
    cargo build --release --bin windexer-api && \
    rm -rf crates/windexer-api/src crates/windexer-common/src

# Copy actual source code
COPY crates/windexer-api/src ./crates/windexer-api/src
COPY crates/windexer-common/src ./crates/windexer-common/src
COPY crates/windexer-common/Cargo.toml ./crates/windexer-common/

# Build the application
ARG CARGO_BUILD_ARGS
RUN cargo build ${CARGO_BUILD_ARGS:-"--release"} --bin windexer-api && \
    mv target/release/windexer-api /usr/local/bin/

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

# Set the entrypoint
ENTRYPOINT ["windexer-api"]

# Health check
HEALTHCHECK --interval=10s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:3000/api/health || exit 1

# Expose the API port
EXPOSE 3000 