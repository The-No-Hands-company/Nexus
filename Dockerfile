# Nexus Server Dockerfile
# Multi-stage build for minimal production image

# === Build Stage ===
FROM rust:bookworm AS builder

WORKDIR /build

# Install build dependencies (needed by aws-lc-sys and other C-based crates)
RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake clang libclang-dev golang-go perl pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Limit parallel compilation to reduce peak memory usage on shared builders
ENV CARGO_BUILD_JOBS=2

# Cache dependencies by building them first
COPY Cargo.toml Cargo.lock ./
COPY crates/nexus-common/Cargo.toml crates/nexus-common/Cargo.toml
COPY crates/nexus-db/Cargo.toml crates/nexus-db/Cargo.toml
COPY crates/nexus-api/Cargo.toml crates/nexus-api/Cargo.toml
COPY crates/nexus-gateway/Cargo.toml crates/nexus-gateway/Cargo.toml
COPY crates/nexus-voice/Cargo.toml crates/nexus-voice/Cargo.toml
COPY crates/nexus-federation/Cargo.toml crates/nexus-federation/Cargo.toml
COPY crates/nexus-server/Cargo.toml crates/nexus-server/Cargo.toml
COPY crates/nexus-desktop/src-tauri/Cargo.toml crates/nexus-desktop/src-tauri/Cargo.toml

# Create dummy source files for dependency caching
RUN mkdir -p crates/nexus-common/src && echo "pub fn dummy() {}" > crates/nexus-common/src/lib.rs && \
    mkdir -p crates/nexus-db/src && echo "pub fn dummy() {}" > crates/nexus-db/src/lib.rs && \
    mkdir -p crates/nexus-api/src && echo "pub fn dummy() {}" > crates/nexus-api/src/lib.rs && \
    mkdir -p crates/nexus-gateway/src && echo "pub fn dummy() {}" > crates/nexus-gateway/src/lib.rs && \
    mkdir -p crates/nexus-voice/src && echo "pub fn dummy() {}" > crates/nexus-voice/src/lib.rs && \
    mkdir -p crates/nexus-federation/src && echo "pub fn dummy() {}" > crates/nexus-federation/src/lib.rs && \
    mkdir -p crates/nexus-server/src && echo "fn main() {}" > crates/nexus-server/src/main.rs && \
    mkdir -p crates/nexus-desktop/src-tauri/src && echo "fn main() {}" > crates/nexus-desktop/src-tauri/src/main.rs && \
    echo "fn main() {}" > crates/nexus-desktop/src-tauri/src/lib.rs

# Build dependencies only for the server binary (cached layer)
# Target --bin nexus to skip Tauri/desktop deps and improve build speed
RUN cargo build --release --bin nexus 2>/dev/null || true

# Copy actual source code
COPY crates/ crates/

# Touch all source files to invalidate the dummy builds
RUN find crates -name "*.rs" -exec touch {} +

# Build the actual application
RUN cargo build --release --bin nexus

# === Runtime Stage ===
FROM debian:bookworm-slim AS runtime

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl3 curl && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --shell /bin/bash nexus

WORKDIR /app

# Copy binary from build stage
COPY --from=builder /build/target/release/nexus /app/nexus

# Copy migrations
COPY crates/nexus-db/migrations/ /app/migrations/

RUN chown -R nexus:nexus /app

USER nexus

EXPOSE 8080 8081 8082

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s \
    CMD curl -f http://localhost:8080/api/v1/health || exit 1

ENTRYPOINT ["/app/nexus", "serve"]
