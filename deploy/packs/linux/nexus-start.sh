#!/usr/bin/env bash
# Nexus — Linux Quick-Start Script
# Tested on: Ubuntu 22.04+, Fedora 40+, Arch, Debian 12+
# Requires: docker (or podman) + docker compose (or podman-compose)
set -euo pipefail

NEXUS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$NEXUS_DIR/../../.." && pwd)"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; NC='\033[0m'
info()    { echo -e "${BLUE}[nexus]${NC} $*"; }
success() { echo -e "${GREEN}[nexus]${NC} $*"; }
warn()    { echo -e "${YELLOW}[nexus]${NC} $*"; }
error()   { echo -e "${RED}[nexus]${NC} $*" >&2; exit 1; }

# ── Detect container runtime ──────────────────────────────────────────────────
detect_compose() {
    if command -v docker &>/dev/null && docker compose version &>/dev/null 2>&1; then
        echo "docker compose"
    elif command -v podman-compose &>/dev/null; then
        echo "podman-compose"
    elif command -v docker-compose &>/dev/null; then
        echo "docker-compose"
    else
        error "No container runtime found. Install Docker or Podman first.\n  Ubuntu/Debian: sudo apt install docker.io docker-compose-v2\n  Fedora:        sudo dnf install podman podman-compose\n  Arch:          sudo pacman -S docker docker-compose"
    fi
}

COMPOSE="$(detect_compose)"
info "Using compose: $COMPOSE"

# ── Check Rust ────────────────────────────────────────────────────────────────
if ! command -v cargo &>/dev/null; then
    warn "Rust not found. Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    warn "Then re-run this script."
    error "Rust is required to build Nexus."
fi
RUST_VERSION=$(rustc --version | awk '{print $2}')
info "Rust: $RUST_VERSION"

# ── Environment setup ─────────────────────────────────────────────────────────
ENV_FILE="$REPO_ROOT/.env"
if [[ ! -f "$ENV_FILE" ]]; then
    info "Creating .env from .env.example …"
    cp "$REPO_ROOT/.env.example" "$ENV_FILE"
    # Auto-generate JWT secret
    JWT_SECRET=$(openssl rand -hex 64 2>/dev/null || head -c 64 /dev/urandom | base64 | tr -dc 'a-zA-Z0-9' | head -c 64)
    sed -i "s|CHANGE_ME_IN_PRODUCTION_use_openssl_rand_hex_64|$JWT_SECRET|" "$ENV_FILE"
    success ".env created with generated JWT secret."
    warn "Review $ENV_FILE before production use."
fi

# ── Start infrastructure ──────────────────────────────────────────────────────
info "Starting infrastructure (Postgres, Redis, ScyllaDB, MinIO, MeiliSearch) …"
cd "$REPO_ROOT"
$COMPOSE up -d

info "Waiting for services to become healthy …"
for i in $(seq 1 30); do
    if $COMPOSE ps --format json 2>/dev/null | grep -q '"Health":"healthy"' || \
       $COMPOSE ps 2>/dev/null | grep -v "starting" | grep -q "healthy"; then
        break
    fi
    sleep 2
    echo -n "."
done
echo ""

# ── Build Nexus ───────────────────────────────────────────────────────────────
info "Building Nexus server (first build may take 2–5 min) …"
cargo build --release --bin nexus 2>&1 | tail -5

# ── Run ───────────────────────────────────────────────────────────────────────
success "Infrastructure ready. Starting Nexus …"
echo ""
echo "  REST API:    http://localhost:8080"
echo "  Gateway WS:  ws://localhost:8081"
echo "  Voice:       ws://localhost:8082"
echo "  Federation:  http://localhost:8448"
echo "  Health:      http://localhost:8080/api/v1/health"
echo ""
exec cargo run --release --bin nexus
