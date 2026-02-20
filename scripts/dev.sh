#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────────────────────
# Nexus — single-command dev launcher
#
# Usage: ./scripts/dev.sh [--build]
#   --build   also rebuilds the Rust server before starting
#
# Starts:  docker infra → nexus-server → Tauri desktop (hot-reload)
# Logs:    /tmp/nexus-server.log
# ──────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_FILE="/tmp/nexus-server.log"
PID_FILE="/tmp/nexus-server.pid"
BUILD=false

for arg in "$@"; do
  [[ "$arg" == "--build" ]] && BUILD=true
done

# ── Colours ───────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; CYAN='\033[0;36m'; RESET='\033[0m'
info()    { echo -e "${CYAN}[nexus]${RESET} $*"; }
success() { echo -e "${GREEN}[nexus]${RESET} $*"; }
die()     { echo -e "${RED}[nexus] ERROR:${RESET} $*" >&2; exit 1; }

# ── 1. Infrastructure (docker / podman compose) ───────────────────────────────
info "Starting infrastructure containers..."
cd "$ROOT_DIR"
docker compose up -d 2>/dev/null || podman-compose up -d

# Wait for postgres to be healthy (up to 30s)
info "Waiting for PostgreSQL..."
for i in $(seq 1 30); do
  if docker compose exec -T postgres pg_isready -U nexus -q 2>/dev/null \
     || podman exec nexus-postgres pg_isready -U nexus -q 2>/dev/null; then
    success "PostgreSQL ready"
    break
  fi
  sleep 1
  [[ $i -eq 30 ]] && die "PostgreSQL did not become ready in 30s"
done

# ── 2. Migrations ─────────────────────────────────────────────────────────────
info "Running migrations..."
DATABASE_URL=postgres://nexus:nexus_dev_password@localhost:5432/nexus \
  sqlx migrate run --source "$ROOT_DIR/crates/nexus-db/migrations" 2>/dev/null \
  && success "Migrations up-to-date" \
  || info "sqlx not found — skipping migration check (run manually if schema changed)"

# ── 3. Build Rust server (optional) ───────────────────────────────────────────
if $BUILD; then
  info "Building nexus-server..."
  cargo build -p nexus-server 2>&1 | tail -5
fi

# ── 4. Nexus API/gateway server ───────────────────────────────────────────────
# Kill any existing instance
if [[ -f "$PID_FILE" ]]; then
  OLD_PID=$(cat "$PID_FILE")
  kill "$OLD_PID" 2>/dev/null && info "Stopped previous nexus-server (pid $OLD_PID)" || true
  rm -f "$PID_FILE"
fi

info "Starting nexus-server → log: $LOG_FILE"
cd "$ROOT_DIR"
nohup cargo run -p nexus-server > "$LOG_FILE" 2>&1 &
echo $! > "$PID_FILE"
NEXUS_PID=$!
success "nexus-server started (pid $NEXUS_PID)"

# Wait for the server to actually bind its port (compilation can take a while)
info "Waiting for nexus-server to be ready on :8080..."
for i in $(seq 1 120); do
  if curl -sf http://localhost:8080/health >/dev/null 2>&1 \
     || curl -sf http://localhost:8080/api/v1/health >/dev/null 2>&1 \
     || nc -z localhost 8080 2>/dev/null; then
    success "nexus-server is up"
    break
  fi
  sleep 1
  [[ $i -eq 120 ]] && { echo ""; die "nexus-server did not bind :8080 within 120s — check: tail -f $LOG_FILE"; }
done

# ── 5. Desktop app ────────────────────────────────────────────────────────────
# Kill any stale nexus-desktop process that might still hold global hotkeys.
pkill -f "nexus-desktop" 2>/dev/null || true
pkill -f "tauri dev"     2>/dev/null || true
sleep 0.5

# Always use `npm run tauri dev` — it starts the Vite frontend server on
# localhost:1420 automatically. The raw debug binary cannot be used directly
# because it points at Vite's dev server and will show "Connection refused"
# if Vite isn't running.
info "Launching desktop app (Vite + hot-reload)..."
cd "$ROOT_DIR/crates/nexus-desktop"
npm run tauri dev &

success "Nexus is running!"
echo ""
echo "  Server log:  tail -f $LOG_FILE"
echo "  Stop all:    $ROOT_DIR/scripts/stop.sh"
echo ""
