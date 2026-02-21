# Stage 1: Build the Rust application
FROM rust:1.75-bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy the entire workspace (needed for workspace dependencies)
COPY Cargo.toml Cargo.lock ./
COPY rs/ ./rs/

# Build the application in release mode
RUN cargo build --release -p cloud-engine-controller-backend

# Stage 2: Create minimal runtime image
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --system --create-home --uid 1001 appuser

# Copy the binary from builder
COPY --from=builder /build/target/release/cloud-engine-controller-backend /app/

# Create directories for data
RUN mkdir -p /app/data /app/registry && chown -R appuser:appuser /app

USER appuser

# Expose the port
EXPOSE 8000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

# Default command
CMD ["/app/cloud-engine-controller-backend", "--targets-dir", "/app/registry", "--port", "8000"]
