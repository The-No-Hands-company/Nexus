#!/usr/bin/env bash
# Nexus — stop everything started by dev.sh
set -euo pipefail

PID_FILE="/tmp/nexus-server.pid"
CYAN='\033[0;36m'; GREEN='\033[0;32m'; RESET='\033[0m'
info()    { echo -e "${CYAN}[nexus]${RESET} $*"; }
success() { echo -e "${GREEN}[nexus]${RESET} $*"; }

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Stop nexus-server
if [[ -f "$PID_FILE" ]]; then
  PID=$(cat "$PID_FILE")
  if kill "$PID" 2>/dev/null; then
    info "Stopped nexus-server (pid $PID)"
  fi
  rm -f "$PID_FILE"
else
  pkill -f "target/debug/nexus" 2>/dev/null || true
  pkill -f "target/release/nexus-desktop" 2>/dev/null || true
fi

# Stop desktop app
pkill -f "nexus-desktop" 2>/dev/null || true

# Stop containers
cd "$ROOT_DIR"
docker compose stop 2>/dev/null || podman-compose stop 2>/dev/null || true

success "All Nexus processes stopped"
