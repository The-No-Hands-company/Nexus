#!/usr/bin/env bash
# Nexus — macOS Quick-Start Script
# Tested on: macOS 13 Ventura+, macOS 14 Sonoma, macOS 15 Sequoia
# Requires: Homebrew (will prompt to install if missing)
set -euo pipefail

NEXUS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$NEXUS_DIR/../../.." && pwd)"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; NC='\033[0m'
info()    { echo -e "${BLUE}[nexus]${NC} $*"; }
success() { echo -e "${GREEN}[nexus]${NC} $*"; }
warn()    { echo -e "${YELLOW}[nexus]${NC} $*"; }
error()   { echo -e "${RED}[nexus]${NC} $*" >&2; exit 1; }

# ── Homebrew ──────────────────────────────────────────────────────────────────
if ! command -v brew &>/dev/null; then
    warn "Homebrew not found. Installing …"
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
fi

# ── Dependencies ──────────────────────────────────────────────────────────────
MISSING=()
command -v docker &>/dev/null || MISSING+=(docker)
command -v cargo  &>/dev/null || MISSING+=(rust)

if [[ ${#MISSING[@]} -gt 0 ]]; then
    info "Installing missing dependencies: ${MISSING[*]} …"
    for dep in "${MISSING[@]}"; do
        case "$dep" in
            docker) brew install --cask docker ;;
            rust)   brew install rustup && rustup-init -y && source "$HOME/.cargo/env" ;;
        esac
    done
fi

# ── Start Docker Desktop if not running ───────────────────────────────────────
if ! docker info &>/dev/null 2>&1; then
    info "Starting Docker Desktop …"
    open -a Docker
    echo -n "Waiting for Docker to start"
    for i in $(seq 1 30); do
        docker info &>/dev/null 2>&1 && break
        sleep 2; echo -n "."
    done
    echo ""
    docker info &>/dev/null 2>&1 || error "Docker failed to start. Open Docker Desktop manually and try again."
fi

COMPOSE="docker compose"
info "Using: $COMPOSE"
info "Rust: $(rustc --version)"

# ── Environment setup ─────────────────────────────────────────────────────────
ENV_FILE="$REPO_ROOT/.env"
if [[ ! -f "$ENV_FILE" ]]; then
    info "Creating .env from .env.example …"
    cp "$REPO_ROOT/.env.example" "$ENV_FILE"
    JWT_SECRET=$(openssl rand -hex 64)
    sed -i '' "s|CHANGE_ME_IN_PRODUCTION_use_openssl_rand_hex_64|$JWT_SECRET|" "$ENV_FILE"
    success ".env created with auto-generated JWT secret."
fi

# ── Start infrastructure ──────────────────────────────────────────────────────
info "Starting infrastructure …"
cd "$REPO_ROOT"
$COMPOSE up -d

info "Waiting for services …"
sleep 10
for i in $(seq 1 20); do
    $COMPOSE ps 2>/dev/null | grep -q "healthy" && break
    sleep 3; echo -n "."
done
echo ""

# ── Build & run ───────────────────────────────────────────────────────────────
info "Building Nexus (first build may take 2–5 min) …"
cargo build --release --bin nexus 2>&1 | tail -5

success "Starting Nexus …"
echo ""
echo "  REST API:   http://localhost:8080"
echo "  Gateway:    ws://localhost:8081"
echo "  Voice:      ws://localhost:8082"
echo "  Health:     http://localhost:8080/api/v1/health"
echo ""
exec cargo run --release --bin nexus
