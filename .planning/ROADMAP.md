# Nexus — Development Roadmap

## What Nexus Is

Nexus is not a Discord clone. It is the platform that comes **after** Discord — built for the moment people are finally ready to leave.

The UX should feel immediately familiar. Servers, channels, voice, bots, rich embeds. But Nexus is architecturally and philosophically a completely different thing:

- **Privacy is a guarantee, not a setting.** No ID, no phone, no face scan. Ever.
- **Your data stays yours.** Self-hostable. Exportable. No surveillance.
- **Its own everything.** Nexus has its own protocol, its own bot API, its own federation model. The concepts — servers, channels, DMs, bots, presence — feel familiar because they are good ideas, not because we copied anyone. We reinvent them properly.
- **No bridges, no adapters.** Nexus does not try to stay compatible with Discord, IRC, or Matrix. Users feel at home because the design is good, not because we kept a compatibility shim alive.
- **Phantom as the long-term privacy backbone.** The [Phantom](https://github.com/The-No-Hands-company/Phantom) anonymous networking protocol will eventually be integrated to make privacy guarantees mathematical, not just policy.

---

## Phase 1: Foundation (v0.1) ✅ Complete

### 01-01: Project Scaffold & Configuration

- ✅ Rust workspace setup (Cargo workspaces)
- ✅ Package structure (api, gateway, voice, common, migration)
- ✅ Docker Compose for dev dependencies (Postgres, Redis, ScyllaDB, MinIO, MeiliSearch)
- ✅ Environment configuration (.env, config.toml)
- ✅ CI pipeline (GitHub Actions)

### 01-02: Database Schema & Migrations

- ✅ User accounts (email/password, OAuth stubs)
- ✅ Servers (guilds), channels, roles, permissions
- ✅ Messages table (ScyllaDB schema)
- ✅ Session management
- ✅ Run migrations via sqlx

### 01-03: Authentication System

- ✅ Registration (email + password, argon2 hashing)
- ✅ Login (JWT access + refresh tokens)
- ✅ Session management (Redis-backed)
- ✅ Rate limiting
- ✅ Password reset flow
- ✅ OAuth2 stubs (GitHub, Google — no mandatory ID)

### 01-04: Core REST API

- ✅ User CRUD (profile, settings, avatar)
- ✅ Server CRUD (create, update, delete, join, leave)
- ✅ Channel CRUD (text, voice, category)
- ✅ Role & permission system
- ✅ Invite system (codes, links, expiry)

### 01-05: WebSocket Gateway (Basic)

- ✅ Connection lifecycle (identify, heartbeat, resume)
- ✅ Event dispatch (message_create, presence_update, typing_start)
- ✅ Session state management
- ✅ Reconnection / resume protocol

## Phase 2: Chat MVP (v0.2) ✅ Complete

- ✅ Message send/edit/delete with real-time propagation
- ✅ DM channels (1:1 and group)
- ✅ Message history with pagination
- ✅ Typing indicators
- ✅ Read state tracking
- ✅ Basic embeds (link previews)
- ✅ Emoji reactions

## Phase 3: Voice (v0.3) ✅ Complete

- ✅ WebRTC SFU (Selective Forwarding Unit) architecture
- ✅ Voice channel join/leave/move
- ✅ Opus codec, noise suppression
- ✅ Mute/deafen/server mute
- ✅ Voice activity detection
- ✅ Screen share (VP9)
- ✅ Recording with consent indicators

## Phase 4: Rich Features (v0.4) ✅ Complete

- ✅ File upload to S3/MinIO (images, video, documents)
- ✅ Rich embeds (media, code blocks, previews)
- ✅ Threads (proper implementation, not Discord's afterthought)
- ✅ Full-text search (MeiliSearch integration)
- ✅ Pinned messages
- ✅ Reactions with custom emoji
- ✅ Server emoji management
- ✅ User presence (online, idle, DND, invisible, custom status)

## Phase 5: Encryption (v0.5) ✅ Complete

- ✅ Signal Protocol for DMs (double ratchet, X3DH key exchange)
- ✅ Opt-in E2EE for channels
- ✅ Key management UI
- ✅ Device verification
- ✅ Encrypted file attachments

## Phase 6: Desktop Client (v0.6) ✅ Complete

- ✅ Tauri 2 application shell
- ✅ Full feature parity with web
- ✅ System tray, notifications
- ✅ Push-to-talk global hotkey
- ✅ Auto-update mechanism
- ✅ Overlay mode (gaming)

## Phase 7: Extensibility (v0.7) ✅ Complete

- ✅ Nexus Bot API (REST + WebSocket — native Nexus protocol)
- ✅ Bot SDK (TypeScript, Python, Rust)
- ✅ Client plugin system (sandboxed)
- ✅ Custom themes (CSS + theme API)
- ✅ Webhooks
- ✅ Slash commands

## Phase 8: Federation (v0.8) ✅ Complete

### 08-01: Core Infrastructure

- ✅ nexus-federation crate (key management, signing, event types)
- ✅ Ed25519 server signing keys (generate, persist, rotate)
- ✅ Server discovery via `.well-known/nexus/server`
- ✅ Signed federation requests (HMAC + Ed25519 authorization headers)

### 08-02: Server-to-Server Protocol

- ✅ `PUT /_nexus/federation/v1/send/{txnId}` — receive events from remote servers
- ✅ `GET /_nexus/federation/v1/event/{eventId}` — serve individual events
- ✅ `GET /_nexus/federation/v1/state/{roomId}` — channel state exchange
- ✅ `GET/_PUT /_nexus/federation/v1/make_join/{roomId}/{userId}` — join protocol
- ✅ Federation backfill (`/backfill`, `/get_missing_events`)

### 08-03: Federated Identity

- ✅ federated_servers table + server trust list
- ✅ federated_users table (remote user profiles)
- ✅ `@user:server.tld` address format for cross-server mentions
- ✅ Remote user avatar/display-name resolution

### 08-04: Discovery & Directory

- ✅ Public server directory API (`/api/v1/directory`)
- ✅ Cross-server join flow via directory
- ✅ Server search by name/topic

### 08-05: Federation Tooling

- ✅ Server-to-server rate limiting and trust scoring
- ✅ Admin federation management UI (trust, block, inspect remote servers)
- ✅ Federation event audit log

## Phase 9: Launch (v0.9) ✅ Complete

### 09-01: Deployment Infrastructure

- [x] Multi-stage production Dockerfile (minimal image)
- [x] `docker-compose.prod.yml` (all services, health checks, named volumes)
- [x] Kubernetes Helm chart (`nexus-server`, `nexus-gateway`, `nexus-voice`)
- [x] `fly.toml` for Fly.io deployment
- [x] Environment variable reference documentation

### 09-02: Self-Host Documentation & One-Click Deploy

- [x] `docs/` directory structure
- [x] Self-hosting guide (prerequisites, setup, configuration)
- [x] `setup.sh` installer (env setup, DB migration, service start)
- [x] Upgrade / migration guide

### 09-03: Security Hardening

- [x] `deny.toml` + cargo-deny CI step (audit vulnerabilities & licenses)
- [x] Security HTTP headers middleware (HSTS, CSP, X-Frame-Options, Referrer-Policy)
- [x] Auth hardening review (rate limiting, refresh token rotation, token expiry)
- [x] `SECURITY.md` vulnerability disclosure policy

### 09-04: Performance Benchmarks

- [x] Criterion microbenchmarks for hot paths (message serialisation, canonical JSON, JWT validation)
- [x] k6 load test scripts (auth, message send, WebSocket gateway)
- [x] Baseline benchmark results committed to `benches/results/`

### 09-05: Community Governance

- [x] `CONTRIBUTING.md`
- [x] `CODE_OF_CONDUCT.md`
- [x] GitHub issue templates (bug report, feature request)
- [x] GitHub PR template
- [x] `SECURITY.md` (vulnerability disclosure)

## Phase 9.5: Lite / Zero-Infra Mode (v0.9.5) ✅ Complete

> **Goal:** A single `nexus` binary you can download and run with zero external dependencies — no Postgres, no Redis, no Docker required. Install it, run it, invite friends to your server. The simplest possible path from download to running community.

### 09.5-01: Embedded Storage Backend

- ✅ Add `storage-lite` feature flag to `nexus-db`
- ✅ Swap Postgres for **SQLite** (`sqlx` SQLite driver, same migration files)
- ✅ Swap ScyllaDB for SQLite append-only messages table (partitioned by channel)
- ✅ Swap MinIO for local filesystem storage (`tokio::fs`, configurable path)
- ✅ Replace Redis pub/sub with in-process `tokio::sync::broadcast` channels
- ✅ Feature-gate the heavy backend crates behind `storage-full` (default for prod builds)

### 09.5-02: Embedded Search

- ✅ Replace Meilisearch with `tantivy` (embedded Rust full-text search engine)
- ✅ Index guilds, channels, users, messages in a local `tantivy` directory
- ✅ Keep Meilisearch path active when `NEXUS_SEARCH_URL` env var is set

### 09.5-03: Single-Binary Server Mode

- ✅ `nexus serve --lite` flag that activates embedded backends automatically
- ✅ Auto-create SQLite DB + data directories on first run
- ✅ Auto-generate secrets and write a `nexus.toml` config on first run
- ✅ Print a "Your server is running at http://localhost:8080" startup message

### 09.5-04: Lite Distribution

- ✅ GitHub Releases: attach pre-built `nexus-linux-x86_64`, `nexus-linux-aarch64`, `nexus-macos`, `nexus-windows.exe` binaries (via CI)
- ✅ Single-line install script: `curl -fsSL https://get.nexus.chat | sh`
- ✅ Update `docs/self-hosting.md` with a "Quick — no Docker" section at the top

## Phase 10: Mobile (v1.0)

- React Native iOS + Android
- Push notifications (FCM/APNs, self-hosted option via UnifiedPush — no Google dependency required)
- Voice/video on mobile
- Offline message queue (local store-and-forward, send on reconnect)

## Phase 11: Phantom Privacy Layer (v1.x — depends on Phantom maturity)

> **Depends on:** [Phantom](https://github.com/The-No-Hands-company/Phantom) reaching production readiness
> — a post-quantum anonymous networking protocol being developed in parallel.

This phase integrates Phantom into Nexus to make privacy guarantees **mathematical rather than policy-based**. Nodes routing traffic between Nexus servers will be unable to determine who is talking to whom, where messages originate, or which users are online — by cryptographic construction, not by trust in the operator.

Phantom is an infant today. This phase will happen when it is ready, not before.

### 11-01: Transport Integration

- [ ] Phantom as an optional transport layer for server-to-server federation traffic
- [ ] FHE-based oblivious routing so relay nodes learn nothing about the path
- [ ] Post-quantum key exchange on all Nexus connections (Kyber-1024 / X25519 hybrid)
- [ ] Phantom node embedded in the `nexus` binary (opt-in, can run as relay)

### 11-02: Anonymous Identity Layer

- [ ] Proof-of-personhood Sybil resistance (no government ID — cryptographic uniqueness)
- [ ] Anonymous account creation: accounts that are mathematically unlinkable to IP addresses
- [ ] Zero-knowledge login: authenticate without revealing which account you have

### 11-03: User-Facing Privacy Guarantees

- [ ] "Phantom mode" toggle per-server (routes traffic through the anonymous network)
- [ ] Verifiable privacy: users can independently verify that traffic is being handled correctly
- [ ] Threat model documentation that users can actually read and understand
