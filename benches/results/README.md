# Benchmark Baselines

Baseline measurements captured on: _to be filled after first run_

## Environment

| Field | Value |
|-------|-------|
| CPU | — |
| RAM | — |
| OS | — |
| Rust version | 1.84 |
| nexus version | v0.9.0 |

## Criterion microbenchmarks

Run with `cargo bench -p nexus-api`. Results land in `target/criterion/`.

| Benchmark | Mean (ns) | p95 (ns) | Notes |
|-----------|-----------|----------|-------|
| `message/serialise` | — | — | serde_json to string |
| `message/deserialise` | — | — | serde_json from str |
| `message/size_scaling/64` | — | — | 64-byte content |
| `message/size_scaling/256` | — | — | 256-byte content |
| `message/size_scaling/1024` | — | — | 1 KiB content |
| `message/size_scaling/4096` | — | — | 4 KiB content |
| `id/uuid_v7_generate` | — | — | uuid::Uuid::now_v7() |
| `id/uuid_parse` | — | — | parse from str |
| `auth/argon2_hash` | — | — | hash a password |
| `auth/argon2_verify` | — | — | verify a password |
| `auth/jwt_encode` | — | — | HS256 sign |
| `auth/jwt_decode` | — | — | HS256 verify + parse |

## k6 load tests

### Auth (`tests/load/auth.js`)

Command: `k6 run --vus 50 --duration 60s tests/load/auth.js`

| Metric | Value |
|--------|-------|
| Requests/sec | — |
| p95 response time | — |
| Error rate | — |

### Messages (`tests/load/messages.js`)

Command: `k6 run --vus 20 --duration 60s tests/load/messages.js`

| Metric | Value |
|--------|-------|
| messages/sec | — |
| p95 send latency | — |
| p95 history fetch | — |
| Error rate | — |

## Updating baselines

After each meaningful change to a hot path, run the benchmarks and update the table above via a PR.
