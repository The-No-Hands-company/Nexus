#!/usr/bin/env bash
# dev.sh — Start the full Nexus dev stack in one command.
# Usage: ./dev.sh
set -e

PROJECT_ROOT="$(cd "$(dirname "$0")" && pwd)"

echo "==> Starting Nexus dev stack..."

# 1) Ensure containers are running (PostgreSQL, Redis, Meilisearch, MinIO)
echo "--> Starting containers..."
cd "$PROJECT_ROOT"
podman compose up -d 2>&1 | tail -6

# Wait for PostgreSQL to accept connections
echo "--> Waiting for PostgreSQL..."
for i in $(seq 1 30); do
  if podman exec nexus-postgres pg_isready -U nexus >/dev/null 2>&1; then
    echo "    PostgreSQL ready"
    break
  fi
  sleep 2
  if [ "$i" -eq 30 ]; then
    echo "    WARNING: PostgreSQL didn't respond in 60s, continuing anyway..."
  fi
done

# 2) Start the API server (if not already running)
if ! curl -sf http://localhost:8080/api/v1/health >/dev/null 2>&1; then
  echo "--> Starting Nexus API server..."
  "$PROJECT_ROOT/target/debug/nexus" serve > /tmp/nexus-api.log 2>&1 &
  API_PID=$!
  echo "    API PID: $API_PID"

  # Wait for it to be healthy
  for i in $(seq 1 20); do
    if curl -sf http://localhost:8080/api/v1/health >/dev/null 2>&1; then
      echo "    API ready ✓"
      break
    fi
    sleep 2
    if [ "$i" -eq 20 ]; then
      echo "    ERROR: API server failed to start. Check /tmp/nexus-api.log"
      exit 1
    fi
  done
else
  echo "--> API server already running ✓"
fi

# 3) Launch Tauri dev (Vite + hot-reload desktop) in the foreground so logs are visible
echo "--> Starting nexus-desktop (tauri dev)..."
echo "    Logs: /tmp/nexus-tauri-dev.log"
echo "    Press Ctrl+C to stop everything."
echo ""
cd "$PROJECT_ROOT/crates/nexus-desktop"
exec npm run tauri dev
