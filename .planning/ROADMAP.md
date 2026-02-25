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

## Status Key

- ✅ Complete and production-ready
- 🟡 Partially implemented — backend or scaffold exists, gaps remain
- [ ] Not yet started

---

## Undocumented Features Already Implemented

> These were built as natural extensions during earlier phases and are not explicitly tracked in any phase below.

- ✅ Friends / relationships system — full backend API (`/api/v1/relationships`), DB schema, and desktop UI sidebar
- ✅ Member list sidebar with live presence indicators
- ✅ User profile cards / hover popups in the desktop client
- ✅ User search endpoint (`GET /api/v1/users/search`)
- ✅ SQLite `any_compat.rs` shim for cross-driver query compatibility (lite mode)
- ✅ Health endpoint (`GET /api/v1/health`) — status + version reported; `uptime_secs` currently hardcoded `0` (tracked in Phase 9.6)

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

## Phase 3: Voice (v0.3) 🟡 Mostly Complete

> **Known gap:** `SfuRoom` in `nexus-voice/src/sfu.rs` has `#[allow(dead_code)]` on its entire implementation body. The str0m WebRTC integration and RTP forwarding loop are present but not yet driven. The signalling API (join/leave/mute/screen-share) and gateway events are complete. Tracked for fix in Phase 9.6.

- ✅ WebRTC SFU architecture (signalling, room state, peer tracking)
- 🟡 RTP packet forwarding — str0m integration exists; forwarding loop not wired
- ✅ Voice channel join/leave/move
- ✅ Opus codec, noise suppression
- ✅ Mute/deafen/server mute
- ✅ Voice activity detection
- ✅ Screen share (VP9)
- ✅ Recording with consent indicators

## Phase 4: Rich Features (v0.4) ✅ Complete (backend)

> **Desktop rendering gaps** documented in Phase 6.

- ✅ File upload to S3/MinIO (images, video, documents)
- ✅ Rich embeds (media, code blocks, previews) — backend rendering pipeline complete
- ✅ Threads — full backend API and DB schema
- ✅ Full-text search (MeiliSearch integration + Tantivy fallback)
- ✅ Pinned messages
- ✅ Reactions with custom emoji
- ✅ Server emoji management
- ✅ User presence (online, idle, DND, invisible, custom status)

## Phase 5: Encryption (v0.5) 🟡 Partially Complete

> **Known gap:** Database schema, REST routes, and device verification are all fully implemented. Actual Signal Protocol double-ratchet session state machine (in-memory ratchet state, session resumption, out-of-order message handling) is not wired — the E2EE routes exist but pass data through without true ratchet progression. Tracked for completion in Phase 9.6.

- ✅ E2EE database schema (keys, sessions, devices, prekeys)
- ✅ Key upload / prekey bundle fetch endpoints
- ✅ Opt-in E2EE channel flag
- ✅ Device verification with safety numbers (`verification.rs`)
- ✅ Encrypted file attachment upload/download routes
- 🟡 Signal Protocol session state machine — routes exist; ratchet progression not implemented
- 🟡 Key management UI — desktop screens exist; session resumption flow incomplete

## Phase 6: Desktop Client (v0.6) 🟡 Partially Complete

> **Known gaps:** Declared as "full feature parity with web" but several significant UI areas are missing. Tracked in Phase 9.6.

- ✅ Tauri 2 application shell
- ✅ ChatView, ChannelList, ServerList, MemberList core UI
- ✅ Friends / relationships sidebar
- ✅ User profile cards and hover popups
- ✅ System tray integration
- ✅ Push-to-talk global hotkey
- ✅ Auto-update mechanism (Tauri updater)
- ✅ Overlay mode (gaming)
- ✅ Custom CSS theme system
- ✅ Plugin sandbox
- 🟡 Messages: reactions not rendered in ChatView; no emoji picker for reactions
- 🟡 Messages: embed/link-preview renderer not implemented in client
- 🟡 Threads: thread panel / reply UI not built in desktop
- 🟡 Unread indicators: channel list shows channels but no unread badge or dot
- 🟡 OS / in-app notifications: Tauri notification plugin present but not wired to gateway events
- 🟡 Server settings modal: no UI for editing server name/icon/roles/invites/emoji/webhooks/bots
- 🟡 Settings pages: only server URL + basic profile implemented; Appearance, Notifications, Privacy, Devices sub-pages missing
- 🟡 Global search UI: backend fully functional; no Cmd+K palette or search modal in client
- [ ] Keyboard navigation and accessibility (ARIA labels, focus management)

## Phase 7: Extensibility (v0.7) 🟡 Mostly Complete

> **Known gaps:** Bot token scheme uses a 32-byte random placeholder in `bots.rs`. Bots authenticate through the standard user gateway path rather than a dedicated bot gateway auth flow. Tracked in Phase 9.6.

- ✅ Nexus Bot API (REST endpoints)
- ✅ Bot WebSocket gateway events
- ✅ Bot SDK (TypeScript, Python, Rust)
- ✅ Client plugin system (sandboxed)
- ✅ Custom themes (CSS + theme API)
- ✅ Webhooks
- ✅ Slash commands
- 🟡 Bot token scheme — currently 32-byte random; no structured `Bot <token>` scheme with scopes
- 🟡 Bot gateway auth — bots use identical auth path as users; no dedicated bot-only identify flow

## Phase 8: Federation (v0.8) 🟡 Mostly Complete

> **Known gap:** `matrix_bridge.rs` is explicitly marked `// Status: stub implementation` in source. The server-to-server Nexus federation protocol is fully implemented. Matrix interoperability is not.

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

### 08-06: Matrix Bridge

- [ ] Matrix CS-API compatibility layer (stub currently exists in `matrix_bridge.rs`)
- [ ] Room alias translation (Nexus channel ↔ Matrix room)
- [ ] Matrix user puppeting / ghost accounts
- [ ] Message format translation (Nexus rich content ↔ Matrix `m.room.message`)

## Phase 9: Launch (v0.9) ✅ Complete

### 09-01: Deployment Infrastructure

- ✅ Multi-stage production Dockerfile (minimal image)
- ✅ `docker-compose.prod.yml` (all services, health checks, named volumes)
- ✅ Kubernetes Helm chart (`nexus-server`, `nexus-gateway`, `nexus-voice`)
- ✅ `fly.toml` for Fly.io deployment
- ✅ Environment variable reference documentation

### 09-02: Self-Host Documentation & One-Click Deploy

- ✅ `docs/` directory structure
- ✅ Self-hosting guide (prerequisites, setup, configuration)
- ✅ `setup.sh` installer (env setup, DB migration, service start)
- ✅ Upgrade / migration guide

### 09-03: Security Hardening

- ✅ `deny.toml` + cargo-deny CI step (audit vulnerabilities & licenses)
- ✅ Security HTTP headers middleware (HSTS, CSP, X-Frame-Options, Referrer-Policy)
- ✅ Auth hardening review (rate limiting, refresh token rotation, token expiry)
- ✅ `SECURITY.md` vulnerability disclosure policy

### 09-04: Performance Benchmarks

- ✅ Criterion microbenchmarks for hot paths (message serialisation, canonical JSON, JWT validation)
- ✅ k6 load test scripts (auth, message send, WebSocket gateway)
- ✅ Baseline benchmark results committed to `benches/results/`

### 09-05: Community Governance

- ✅ `CONTRIBUTING.md`
- ✅ `CODE_OF_CONDUCT.md`
- ✅ GitHub issue templates (bug report, feature request)
- ✅ GitHub PR template
- ✅ `SECURITY.md` (vulnerability disclosure)

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

## Phase 9.6: Polish & Correctness (v0.9.6) 🔲 Planned

> **Goal:** Close the gap between what the roadmap claims is done and what is actually production-ready. Fix every known stub, placeholder, and TODO in the existing implementation before adding new features.

### 09.6-01: Backend Correctness

- [ ] Wire `SfuRoom` RTP forwarding loop in `nexus-voice/src/sfu.rs` (remove `#[allow(dead_code)]`, drive str0m media loop)
- [ ] Enforce channel permissions — replace the 3× `// TODO: proper permission check` in `routes/channels.rs` with real calls to the permission evaluation layer (`repository/roles.rs`)
- [ ] Fix `uptime_secs` in the health endpoint — track process start time and compute actual elapsed seconds
- [ ] Implement structured bot token scheme (`Bot <base64-token>` with scope claims, replace 32-byte random placeholder in `bots.rs`)
- [ ] Implement dedicated bot gateway `IDENTIFY` flow (separate from user auth path)
- [ ] Complete Signal Protocol double-ratchet session state machine (in-memory ratchet progression, out-of-order message handling, session resumption)

### 09.6-02: Desktop Client Completion

- [ ] Render emoji reactions in `ChatView` — show reaction bar below messages, handle `reaction_add` / `reaction_remove` gateway events
- [ ] Emoji picker for adding reactions
- [ ] Embed / link-preview renderer in `ChatView` (consume the embed data already returned by the API)
- [ ] Thread reply panel (open thread in side panel, `POST /channels/{id}/messages` with `thread_id`)
- [ ] Unread indicators — add unread dot / badge to `ChannelList` items, consume `MESSAGE_CREATE` events and mark-read API
- [ ] Wire OS notifications via Tauri notification plugin on `MESSAGE_CREATE` for @mentions
- [ ] In-app notification tray (bell icon, list of recent @mentions, unread count badge)
- [ ] Global search UI — Cmd+K palette that calls the existing `/api/v1/search` endpoint
- [ ] Server settings modal (name, icon, vanity URL, danger zone / delete)
- [ ] Role management UI (create/edit/delete roles, permission matrix, drag-to-reorder)
- [ ] Invite management UI (list active invites, create invite with expiry/max-uses, revoke)
- [ ] Emoji management UI (upload, list, delete server emoji)
- [ ] Webhook management UI (create, list, delete, copy URL)
- [ ] Bot management UI (add bot by client ID, list installed bots, revoke)
- [ ] Settings — Appearance sub-page (theme selector, font size, message density, dark/light/system)
- [ ] Settings — Notifications sub-page (per-server/channel override toggles, @mention sensitivity)
- [ ] Settings — Privacy sub-page (DM permissions, read receipts, presence visibility)
- [ ] Settings — Devices / Sessions sub-page (list active sessions, remote revoke)

### 09.6-03: Infrastructure & Observability

- [ ] Structured JSON logging (replace ad-hoc `println!` / `tracing` calls with consistent field schema)
- [ ] Prometheus metrics endpoint (`/metrics`) — request counts, latency histograms, active WebSocket connections, voice room counts
- [ ] `GET /api/v1/health` extended response — include DB connectivity, Redis connectivity, search backend status
- [ ] Graceful shutdown — drain in-flight WebSocket messages before process exit

## Phase 9.7: Account Security (v0.9.7) 🔲 Planned

> **Goal:** Give users real control over their account security. None of these features exist anywhere in the codebase today.

### 09.7-01: Two-Factor Authentication

- [ ] TOTP (RFC 6238) — generate secret, QR code provisioning, verify code on login
- [ ] Backup codes (8× single-use codes shown at 2FA setup, stored as bcrypt hashes)
- [ ] Enforce 2FA requirement per-server (server setting: members must have 2FA enabled)
- [ ] 2FA state in JWT claims (downstream permission checks can require `2fa_verified: true`)

### 09.7-02: Email Verification

- [ ] Send verification email on registration (token stored in DB with expiry)
- [ ] `GET /api/v1/auth/verify-email?token=…` endpoint
- [ ] Block access to non-auth routes until email is verified (configurable — can disable for self-hosters)
- [ ] Resend verification email endpoint

### 09.7-03: Session Management

- [ ] `GET /api/v1/auth/sessions` — list all active sessions (device name, IP, last seen, created at)
- [ ] `DELETE /api/v1/auth/sessions/{id}` — revoke specific session
- [ ] `DELETE /api/v1/auth/sessions` — revoke all other sessions ("log out everywhere")
- [ ] Surface in desktop Settings → Devices / Sessions page (see Phase 9.6)

### 09.7-04: Account Lifecycle

- [ ] `DELETE /api/v1/users/@me` — account deletion (soft-delete with 30-day grace period before hard purge)
- [ ] `GET /api/v1/users/@me/data-export` — GDPR-compliant data export (JSON archive of messages, servers, files)
- [ ] Account transfer: server ownership transfer before deletion

## Phase 9.8: Moderation & Safety (v0.9.8) 🔲 Planned

> **Goal:** Give server administrators the tools they need to run a community safely. None of these features exist in the current implementation.

### 09.8-01: Server Audit Log

- [ ] `audit_log_entries` DB table (action, target_id, target_type, actor_id, changes JSON, timestamp)
- [ ] Write audit entries for: kick, ban, role change, channel create/delete, invite create/delete, message delete by moderator, webhook create/delete
- [ ] `GET /api/v1/servers/{id}/audit-log` with filter by action type and actor
- [ ] Audit log viewer in server settings UI

### 09.8-02: Timeout & Temp-Ban

- [ ] `user_timeouts` DB table (user_id, server_id, expires_at, moderator_id, reason)
- [ ] Timeout enforcement in message send and voice join routes (check active timeout)
- [ ] `POST /api/v1/servers/{id}/members/{uid}/timeout` — set timeout with duration
- [ ] Temp-ban: `expires_at` column on existing bans table; cron/background task to lift expired bans
- [ ] Gateway event `MEMBER_UPDATE` emitted on timeout apply/lift

### 09.8-03: Message Reporting

- [ ] `message_reports` DB table (message_id, reporter_id, reason, status, resolved_by)
- [ ] `POST /api/v1/messages/{id}/report`
- [ ] `GET /api/v1/servers/{id}/reports` — mod-only report queue with status filter
- [ ] Report resolution actions: dismiss, delete message, timeout user, ban user

### 09.8-04: Content Filters

- [ ] Server-level word filter (blocked words list stored in server settings)
- [ ] Apply filter on message create, edit — reject or auto-delete matching content
- [ ] Configurable filter action: block (return 400), delete-and-log, or delete-and-warn
- [ ] Spam detection: rate-limit duplicate messages per user per channel (configurable threshold)

## Phase 10: Mobile (v1.0) 🔲 Planned

- [ ] React Native iOS + Android
- [ ] Push notifications (FCM/APNs, self-hosted option via UnifiedPush — no Google dependency required)
- [ ] Voice/video on mobile
- [ ] Offline message queue (local store-and-forward, send on reconnect)

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
