# Upgrading Nexus

This guide covers how to update a running Nexus instance to a newer version.

## Before You Begin

1. **Read the release notes** for all versions between your current and target version.
   Breaking changes and required migration steps are documented there.
2. **Back up your data** — especially PostgreSQL and MinIO — before upgrading.
3. Upgrades should be done to **sequential minor versions** (v0.8 → v0.9, not v0.8 → v1.0 directly)
   unless the release notes explicitly state that skipping is safe.

---

## Docker Compose Upgrade

```bash
# 1. Pull the latest images
docker compose -f deploy/docker-compose.prod.yml pull

# 2. Bring the server down gracefully
docker compose -f deploy/docker-compose.prod.yml stop nexus

# 3. Run any pending database migrations
docker compose -f deploy/docker-compose.prod.yml run --rm nexus /app/nexus migrate

# 4. Start the updated server
docker compose -f deploy/docker-compose.prod.yml up -d nexus

# 5. Verify
curl https://your-domain/api/v1/health
```

---

## Kubernetes (Helm) Upgrade

```bash
helm repo update   # if using the Helm repo

helm upgrade nexus ./deploy/helm \
  --namespace nexus \
  --reuse-values \
  --set image.tag=0.9.0
```

Check rollout status:

```bash
kubectl rollout status deployment/nexus -n nexus
```

Roll back if needed:

```bash
helm rollback nexus -n nexus
```

---

## Fly.io Upgrade

```bash
git pull origin main
fly deploy
```

---

## Version-Specific Notes

### v0.8 → v0.9

- No breaking API changes.
- New environment variables added (all optional — existing deployments are unaffected):
  - `NEXUS_MATRIX_HS_URL` — enables the Matrix AS bridge
  - `NEXUS_MATRIX_AS_TOKEN` / `NEXUS_MATRIX_HS_TOKEN` / `NEXUS_MATRIX_BOT_MXID`
- New DB migration `20260218000006_federation.sql` runs automatically on start.
- Port 8448 is now used for S2S federation — open it in your firewall if you want to federate.

### v0.7 → v0.8

- No breaking API changes.
- The `nexus-federation` crate is now compiled into the main binary — no separate process needed.

---

## Rolling Back

```bash
# Docker Compose — pin to previous image tag
NEXUS_TAG=0.8.0 docker compose -f deploy/docker-compose.prod.yml up -d nexus

# Kubernetes
helm rollback nexus -n nexus
```

> **Note**: Rolling back across migrations that dropped columns is not supported.
> Always have a recent database backup.
