# Self-Hosting Nexus

This guide walks through deploying a production Nexus instance on your own server.
For a fully automated setup run [`setup.sh`](../setup.sh).

## Prerequisites

| Requirement | Minimum | Notes |
|---|---|---|
| OS | Ubuntu 22.04+ / Debian 12+ | Other Linux distros work; macOS for dev only |
| RAM | 2 GB | 4 GB+ recommended for production |
| CPU | 2 cores | |
| Disk | 20 GB | Depends on media storage volume |
| Docker | 24+ | |
| Docker Compose | v2.20+ | Built in to Docker Desktop |
| Domain | Required | For TLS and federation |
| Open ports | 80, 443, 8448 | 8448 required for Matrix/Nexus federation |

---

## Quick Start (Docker Compose)

```bash
# 1. Clone the repository
git clone https://github.com/The-No-hands-Company/Nexus.git
cd Nexus

# 2. Run the interactive setup script
chmod +x setup.sh && ./setup.sh

# 3. Start the stack
docker compose -f deploy/docker-compose.prod.yml up -d

# 4. Verify it's running
curl https://your-domain.com/api/v1/health
```

---

## Quick Start (Lite / Single Binary)

For home labs, small teams, or local testing you can run the entire backend as a
single binary with an embedded SQLite database and embedded full-text search —
no Docker, Postgres, Redis, MinIO, or MeiliSearch required.

### Option A — one-line install (pre-built binary)

```bash
curl -fsSL https://get.nexus.chat | sh
nexus serve --lite
```

That's it. Your server starts at **http://localhost:8080**. SQLite, file storage,
and full-text search (Tantivy) are all created automatically on first run.

> Pre-built binaries are published to [GitHub Releases](https://github.com/The-No-hands-Company/Nexus/releases)
> for Linux x86\_64, Linux aarch64, macOS x86\_64, macOS Apple Silicon, and Windows.

### Option B — build from source

```bash
# Build the release binary (first time ~2–3 min)
cargo build -p nexus-server --release

# Start in lite mode — SQLite is created automatically at ./nexus.db
./target/release/nexus serve --lite
```

> **What `--lite` bundles**
> - SQLite via `sqlx::AnyPool` (no external database needed)
> - In-process auth (JWT, secrets auto-generated to `nexus.toml`)
> - Local filesystem storage for uploads (no MinIO / S3)
> - Tantivy embedded full-text search (no MeiliSearch)
>
> **What you lose vs. the full stack**
> - Horizontal scaling (single process only)
> - Voice/video calls (SFU disabled)
> - ActivityPub / Matrix federation

---

## Manual Setup

### 1. Create environment file

```bash
cp .env.example .env.prod
$EDITOR .env.prod
```

Required values:

```env
NEXUS_DOMAIN=nexus.example.com
NEXUS_JWT_SECRET=<at least 64 random chars — use: openssl rand -hex 64>
NEXUS_POSTGRES_PASSWORD=<strong password>
NEXUS_REDIS_PASSWORD=<strong password>
NEXUS_MINIO_PASSWORD=<strong password>
NEXUS_MEILI_KEY=<strong key>
```

### 2. Configure Caddy (reverse proxy + TLS)

Create `deploy/Caddyfile`:

```caddyfile
nexus.example.com {
    # REST API + WebSocket Gateway
    reverse_proxy /api/* nexus:8080
    reverse_proxy /ws    nexus:8081
    
    # Federation endpoints
    handle /_nexus/* {
        reverse_proxy nexus:8448
    }
    handle /.well-known/nexus/* {
        reverse_proxy nexus:8448
    }
    handle /_matrix/* {
        reverse_proxy nexus:8448
    }
    
    # Static files / client app (if using the web client)
    root * /srv/nexus-web
    file_server
}
```

### 3. Run database migrations

```bash
docker compose -f deploy/docker-compose.prod.yml run --rm nexus /app/nexus migrate
```

### 4. Start all services

```bash
docker compose -f deploy/docker-compose.prod.yml up -d
```

### 5. Create the first admin account

```bash
curl -X POST https://nexus.example.com/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","email":"admin@example.com","password":"<strong>"}'
```

---

## Kubernetes (Helm)

```bash
# Add the Nexus Helm repo (or use the local chart)
helm install nexus ./deploy/helm \
  --namespace nexus --create-namespace \
  --set config.serverName=nexus.example.com \
  --set secrets.secretValues.jwtSecret=$(openssl rand -hex 64) \
  --set secrets.secretValues.postgresPassword=$(openssl rand -hex 32) \
  --set secrets.secretValues.redisPassword=$(openssl rand -hex 32) \
  --set secrets.secretValues.minioPassword=$(openssl rand -hex 32) \
  --set secrets.secretValues.meiliKey=$(openssl rand -hex 32)
```

---

## Fly.io

```bash
# Install Fly CLI: https://fly.io/docs/hands-on/install-flyctl/
fly launch --copy-config --no-deploy

# Set secrets
fly secrets set \
  AUTH__JWT_SECRET=$(openssl rand -hex 64) \
  DATABASE__URL="<your postgres URL>" \
  REDIS__URL="<your redis URL>" \
  SERVER__NAME=nexus.example.com

fly deploy
```

---

## Backups

### PostgreSQL (daily automatic)

```bash
# Manual backup
docker exec nexus-postgres pg_dump -U nexus nexus | gzip > backup-$(date +%F).sql.gz

# Restore
gunzip < backup-2026-02-19.sql.gz | docker exec -i nexus-postgres psql -U nexus nexus
```

### MinIO data

Use `mc mirror` or any S3-compatible backup tool to replicate the bucket to
another location.

---

## Updating

See [upgrading.md](upgrading.md).

---

## Troubleshooting

| Problem | Solution |
|---|---|
| Health check fails | Check logs: `docker compose -f deploy/docker-compose.prod.yml logs nexus` |
| DB migration fails | Ensure PostgreSQL is healthy before the server starts |
| Federation unreachable | Verify port 8448 is open and `SERVER__NAME` matches your domain |
| TLS errors | Caddy handles certificates automatically — check caddy logs |
| Can't connect to ScyllaDB | ScyllaDB takes ~60s to initialise on first run — wait and retry |
