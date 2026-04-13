# ============================================================
# VOCAI: Vocab+AI — Dockerfile
# Multi-stage build for Rust + Axum server
# ============================================================

# Build stage
FROM rust:1.94-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy dependency files first for caching
COPY server/Cargo.toml server/Cargo.lock* ./

# Create dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY server/ .

# Build the application
RUN touch src/main.rs && \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates wget && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/vocai-server /usr/local/bin/vocai-server

# Copy static assets
COPY public/ /app/public/

# Set working directory
WORKDIR /app

# Environment variables
ENV RUST_LOG=info
ENV PORT=3010

# Expose port
EXPOSE 3010

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3010/api/health || exit 1

# Run the application
CMD ["vocai-server"]
