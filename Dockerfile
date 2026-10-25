# Build stage for the Rust server
FROM rust:1.99.1-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    --no-install-recommends \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files (use Docker-specific Cargo.toml with minimal workspace)
COPY Cargo.docker.toml ./Cargo.toml
COPY Cargo.lock ./
COPY rustatio-core ./rustatio-core
COPY rustatio-server ./rustatio-server
COPY rustatio-watch ./rustatio-watch

# Copy the pre-built UI
COPY ui/dist ./ui/dist

# Build the server with embedded UI
RUN cargo build --release -p rustatio-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
# (tini = init/reaper de zombies, python3-* = dépendances de rustatio_daemon.py)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    gosu \
    tini \
    tzdata \
    python3 \
    python3-requests \
    python3-flask \
    --no-install-recommends \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user (will be modified at runtime via PUID/PGID)
RUN groupadd -g 1000 rustatio && useradd -u 1000 -g rustatio rustatio

WORKDIR /app

# Copy the built binary, the daemon, defaults and the entrypoint
COPY --from=builder /app/target/release/rustatio-server /app/rustatio-server
COPY scripts/entrypoint.sh /app/entrypoint.sh
COPY scripts/rustatio_daemon.py /app/rustatio_daemon.py
COPY scripts/rules.txt /app/rules.default.txt
COPY scripts/state.json /app/state.default.json
RUN chmod +x /app/entrypoint.sh

# Commande `rustatio_daemon` utilisable via `docker exec` (suit DAEMON_SCRIPT)
RUN printf "#!/bin/sh\nexec python3 \"\$DAEMON_SCRIPT\" \"\$@\"\n" > /usr/local/bin/rustatio_daemon \
    && chmod +x /usr/local/bin/rustatio_daemon

# Create data directory and set permissions
RUN mkdir -p /data && chown -R rustatio:rustatio /app /data

# Set environment variables (can be overridden at runtime)
ENV PORT=8080
ENV RUST_LOG=info
ENV DATA_DIR=/data
ENV TZ=Europe/Paris
# Watch folder is auto-detected: enabled only if WATCH_DIR exists (mounted volume)
ENV WATCH_DIR=/torrents
# WATCH_ENABLED is intentionally unset to enable auto-detection
ENV WATCH_AUTO_START=false
# PUID/PGID for permission handling (LinuxServer.io style)
# Override these to match your host user's UID/GID for mounted volumes
ENV PUID=1000
ENV PGID=1000
# Daemon Python : DAEMON_ENABLED=false pour le désactiver,
# DAEMON_SCRIPT pour utiliser une autre version (ex. volume monté hors de /app)
ENV DAEMON_ENABLED=true
ENV DAEMON_SCRIPT=/app/rustatio_daemon.py

# Expose default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["sh", "-c", "curl -f \"http://localhost:${PORT}/health\" || exit 1"]

ENTRYPOINT ["/usr/bin/tini", "-g", "--", "/app/entrypoint.sh"]
CMD ["/app/rustatio-server"]
