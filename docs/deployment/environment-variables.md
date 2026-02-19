# Environment Variable Reference

All configuration is read from environment variables (or `config.toml`). Variables
follow the pattern `SECTION__KEY` (double underscore as separator).

## Server

| Variable | Default | Description |
|---|---|---|
| `SERVER__HOST` | `127.0.0.1` | Bind address for the REST API |
| `SERVER__PORT` | `8080` | REST API port |
| `SERVER__GATEWAY_PORT` | `8081` | WebSocket gateway port |
| `SERVER__VOICE_PORT` | `8082` | Voice signaling port |
| `SERVER__FEDERATION_PORT` | `8448` | Matrix S2S federation port |
| `SERVER__NAME` | `localhost` | Public hostname (used in MXIDs and federation) |

## Authentication

| Variable | Default | Description |
|---|---|---|
| `AUTH__JWT_SECRET` | *(required)* | JWT signing secret — minimum 64 characters |
| `AUTH__JWT_EXPIRY_SECS` | `86400` | Access token lifetime in seconds (default 24 h) |
| `AUTH__REFRESH_EXPIRY_SECS` | `2592000` | Refresh token lifetime (default 30 days) |

## PostgreSQL

| Variable | Default | Description |
|---|---|---|
| `DATABASE__URL` | *(required)* | Full connection string e.g. `postgres://user:pass@host:5432/db` |
| `DATABASE__MAX_CONNECTIONS` | `20` | Connection pool size |

## Redis

| Variable | Default | Description |
|---|---|---|
| `REDIS__URL` | *(required)* | Connection string e.g. `redis://:password@host:6379` |

## ScyllaDB

| Variable | Default | Description |
|---|---|---|
| `SCYLLA__NODES` | `127.0.0.1:9042` | Comma-separated list of seed nodes |
| `SCYLLA__KEYSPACE` | `nexus` | Cassandra keyspace name |
| `SCYLLA__USERNAME` | *(optional)* | ScyllaDB username |
| `SCYLLA__PASSWORD` | *(optional)* | ScyllaDB password |

## Object Storage (MinIO / S3)

| Variable | Default | Description |
|---|---|---|
| `STORAGE__ENDPOINT` | `http://localhost:9000` | S3-compatible endpoint |
| `STORAGE__ACCESS_KEY` | *(required)* | S3 access key ID |
| `STORAGE__SECRET_KEY` | *(required)* | S3 secret access key |
| `STORAGE__BUCKET` | `nexus` | Bucket name for uploads |
| `STORAGE__REGION` | `us-east-1` | S3 region (use any value for MinIO) |
| `STORAGE__PUBLIC_URL` | *(optional)* | CDN/public URL prefix for served files |

## MeiliSearch

| Variable | Default | Description |
|---|---|---|
| `SEARCH__URL` | `http://localhost:7700` | MeiliSearch base URL |
| `SEARCH__KEY` | *(required)* | MeiliSearch master key |

## Federation

| Variable | Default | Description |
|---|---|---|
| `NEXUS_MATRIX_HS_URL` | *(optional)* | Matrix homeserver URL (enables Matrix bridge) |
| `NEXUS_MATRIX_AS_TOKEN` | *(optional)* | Application service token sent *to* the homeserver |
| `NEXUS_MATRIX_HS_TOKEN` | *(optional)* | Token the homeserver sends *to* this AS |
| `NEXUS_MATRIX_BOT_MXID` | *(optional)* | MXID of the bridge bot user |

## Telemetry

| Variable | Default | Description |
|---|---|---|
| `RUST_LOG` | `nexus=info` | Log filter — see [tracing docs](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html) |
| `RUST_BACKTRACE` | `0` | Set to `1` for full backtraces on panic |
