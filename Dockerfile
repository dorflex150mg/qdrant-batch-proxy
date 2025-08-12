# 1. Build stage
FROM rust:1.82 AS builder

# Create app dir
WORKDIR /app

# Copy Cargo files first for caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main to cache deps
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy real source
COPY src ./src

# Build release binary
RUN cargo build --release

# 2. Runtime stage
FROM debian:bullseye-slim

# Install minimal runtime deps
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/auto_batch_proxy /usr/local/bin/auto_batch_proxy

# Expose port
EXPOSE 3000

# Run proxy
CMD ["auto_batch_proxy"]
