# ============================================================================
# Multi-stage Dockerfile for WebRTC Chat Application
# ============================================================================
# Stage 1: Build the Rust backend (server + message crate)
# Stage 2: Build the CSS preprocessor
# Stage 3: Build the frontend (WASM via Trunk)
# Stage 4: Production runtime image
# ============================================================================

# ---------------------------------------------------------------------------
# Base build image with Rust toolchain
# ---------------------------------------------------------------------------
FROM rust:1.88-slim-bookworm AS rust-base

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install trunk for frontend builds
RUN cargo install trunk@0.21.14

# Install WASM target for frontend builds
RUN rustup target add wasm32-unknown-unknown

# Cache dependencies by building a dummy project first
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY message/Cargo.toml message/Cargo.toml
COPY server/Cargo.toml server/Cargo.toml
COPY frontend/Cargo.toml frontend/Cargo.toml
COPY css-processor/Cargo.toml css-processor/Cargo.toml

# Create dummy source files so cargo can resolve the workspace
RUN mkdir -p message/src && echo "pub fn dummy() {}" > message/src/lib.rs \
    && mkdir -p server/src && echo "fn main() {}" > server/src/main.rs \
    && mkdir -p frontend/src && echo "pub fn dummy() {}" > frontend/src/lib.rs \
    && mkdir -p css-processor/src && echo "fn main() {}" > css-processor/src/main.rs

# Build dependencies only (layer caching)
RUN cargo build --release --workspace 2>/dev/null || true

# ---------------------------------------------------------------------------
# Stage 1: Build the server binary
# ---------------------------------------------------------------------------
FROM rust-base AS server-builder

COPY message/ message/
COPY server/ server/

# Touch source files to invalidate the dummy cache
RUN find message/src -type f -exec touch {} + \
    && find server/src -type f -exec touch {} +

RUN cargo build --release -p server

# ---------------------------------------------------------------------------
# Stage 2: Build the CSS preprocessor
# ---------------------------------------------------------------------------
FROM rust-base AS css-builder

COPY css-processor/ css-processor/

RUN find css-processor/src -type f -exec touch {} +

RUN cargo build --release -p css-processor

# ---------------------------------------------------------------------------
# Stage 3: Build the frontend (WASM + Trunk)
# ---------------------------------------------------------------------------
FROM rust-base AS frontend-builder

# Install additional frontend build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    binaryen \
    && rm -rf /var/lib/apt/lists/*

# Copy built CSS preprocessor
COPY --from=css-builder /app/target/release/css-processor /usr/local/bin/css-processor

# Copy message crate (needed by frontend)
COPY message/ message/

# Copy frontend source
COPY frontend/ frontend/

# Copy the build helper scripts
COPY frontend/css-hook.sh frontend/css-hook.sh
COPY frontend/strip-dev-hot-reload.sh frontend/strip-dev-hot-reload.sh

# Run CSS preprocessor to expand composes
RUN css-processor frontend/styles frontend/styles-dist

# Touch frontend source files to invalidate the dummy cache
RUN find frontend/src -type f -exec touch {} + \
    && find message/src -type f -exec touch {} +

# Build frontend with Trunk
WORKDIR /app/frontend
RUN trunk build --release

# ---------------------------------------------------------------------------
# Stage 4: Production runtime
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN groupadd -r webrtc-chat && useradd -r -g webrtc-chat -d /app -s /sbin/nologin webrtc-chat

WORKDIR /app

# Copy server binary
COPY --from=server-builder /app/target/release/server /app/server

# Copy frontend dist
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist

# Create assets directory (sticker assets may be mounted at runtime)
RUN mkdir -p /app/assets

# Set ownership
RUN chown -R webrtc-chat:webrtc-chat /app

# Switch to non-root user
USER webrtc-chat

# Environment defaults (overridable at runtime)
ENV PORT=3000
ENV RUST_LOG=info
ENV RUST_LOG_FORMAT=json
ENV LOG_OUTPUT=stdout
ENV STATIC_DIR=/app/frontend/dist
ENV STICKERS_DIR=/app/assets/stickers

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:${PORT}/api/health || exit 1

EXPOSE ${PORT}

ENTRYPOINT ["/app/server"]
