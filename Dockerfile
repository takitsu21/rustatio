# Build stage for the Rust server
FROM rust:1.93-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files (use Docker-specific Cargo.toml with minimal workspace)
COPY Cargo.docker.toml ./Cargo.toml
COPY Cargo.lock ./
COPY rustatio-core ./rustatio-core
COPY rustatio-server ./rustatio-server

# Copy the pre-built UI
COPY ui/dist ./ui/dist

# Build the server with embedded UI
RUN cargo build --release -p rustatio-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    gosu \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user (will be modified at runtime via PUID/PGID)
RUN groupadd -g 1000 rustatio && useradd -u 1000 -g rustatio rustatio

WORKDIR /app

# Copy the built binary and entrypoint
COPY --from=builder /app/target/release/rustatio-server /app/rustatio-server
COPY scripts/entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

# Create data directory and set permissions
RUN mkdir -p /data && chown -R rustatio:rustatio /app /data

# Set environment variables (can be overridden at runtime)
ENV PORT=8080
ENV RUST_LOG=info
ENV DATA_DIR=/data
# Watch folder is auto-detected: enabled only if WATCH_DIR exists (mounted volume)
ENV WATCH_DIR=/torrents
# WATCH_ENABLED is intentionally unset to enable auto-detection
ENV WATCH_AUTO_START=false
# PUID/PGID for permission handling (LinuxServer.io style)
# Override these to match your host user's UID/GID for mounted volumes
ENV PUID=1000
ENV PGID=1000

# Expose default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

# Use entrypoint for PUID/PGID handling
ENTRYPOINT ["/app/entrypoint.sh"]
CMD ["/app/rustatio-server"]
