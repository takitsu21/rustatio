# Build stage for the Rust server
FROM rust:1.92-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
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

# Install runtime dependencies (curl for healthcheck)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security (UID/GID 1000 for compatibility with host mounts)
ARG UID=1000
ARG GID=1000
RUN groupadd -g ${GID} rustatio && useradd -u ${UID} -g rustatio rustatio

WORKDIR /app

# Copy the built binary
COPY --from=builder /app/target/release/rustatio-server /app/rustatio-server

# Create data directory and set permissions
RUN mkdir -p /data && chown -R rustatio:rustatio /app /data

# Set environment variables (can be overridden at runtime)
ENV PORT=8080
ENV RUST_LOG=info

# Switch to non-root user
USER rustatio

# Expose default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

# Run the server
CMD ["/app/rustatio-server"]
