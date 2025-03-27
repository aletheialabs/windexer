FROM rust:1.73-slim-bullseye as builder

WORKDIR /app
COPY . .

# Build the AVS binary
RUN cargo build --package windexer-jito-staking --bin avs --release

# Create a lighter runtime image
FROM debian:bullseye-slim

WORKDIR /app
COPY --from=builder /app/target/release/avs /app/avs

# Install required libraries
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Run the AVS binary
ENTRYPOINT ["/app/avs"] 