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
- ✅ Health endpoint (`GET /api/v1/health`) — status + version reported; `uptime_secs` computed from `AppState.started_at` (Instant tracked since process start)

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

- ✅ WebRTC SFU architecture (signalling, room state, peer tracking)
- ✅ RTP packet forwarding — str0m integration wired; `run_sfu_room()` three-arm select loop drives `drain_rtc()` / `forward_media()` / `setup_forwarding_tracks()`
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

## Phase 5: Encryption (v0.5) ✅ Complete

- ✅ E2EE database schema (keys, sessions, devices, prekeys)
- ✅ Key upload / prekey bundle fetch endpoints
- ✅ Opt-in E2EE channel flag
- ✅ Device verification with safety numbers (`verification.rs`)
- ✅ Encrypted file attachment upload/download routes
- ✅ Ratchet step tracking — `upsert_session` / `increment_ratchet_step` wired in E2EE routes
- ✅ `sender_ratchet_step` stored on each message — recipients detect skipped steps by comparing against the last-seen value per sender device
- ✅ `session_exists` guard on type=2 messages — server warns (and skips increment) if a `SignalMessage` arrives without an established session record
- ✅ Type=1 (PreKeySignalMessage) no longer creates empty session rows — client calls `PUT /devices/:id/sessions/:remote` after completing X3DH
- ✅ `LOW_PREKEYS` gateway event — fired to device owner when remaining OTPKs drop below 5 after a key bundle fetch
- ✅ Key management UI — Settings Devices section: lists registered devices, per-device revoke button (`delete_device` Tauri command)

## Phase 6: Desktop Client (v0.6) 🟡 Partially Complete

> **Remaining gaps:** Core chat UX is complete. All management UIs (roles, emoji, webhooks, bots) and the Appearance settings sub-page are now implemented. Keyboard/accessibility complete.

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
- ✅ Messages: emoji reactions rendered in ChatView with EmojiPicker; `reaction_add` / `reaction_remove` gateway events handled
- ✅ Messages: embed/link-preview renderer (`EmbedCard`) in ChatView — title, description, image, colour border
- ✅ Threads: `ThreadPanel` side panel wired in ChatView; reply via `thread_id` on message send
- ✅ Unread indicators: `unreadChannels` map drives dot/badge on ChannelList items
- ✅ OS notifications: `sendOsNotification` wired in `useGateway` for `MESSAGE_CREATE` @mention events
- ✅ Server settings modal: edit name, manage invites (create/copy/revoke), danger zone (delete server)
- ✅ Settings pages: Notifications, Privacy, Devices/Sessions sub-pages implemented; Devices section lists E2EE devices with per-device Revoke button
- ✅ Global search: `SearchModal` with Cmd+K/Ctrl+K shortcut, debounced query, keyboard navigation, channel navigation on select
- ✅ Keyboard navigation and accessibility — `<nav aria-label>` landmarks on ServerList / ChannelList; `role="log" aria-live="polite"` on message list; `role="dialog" aria-modal` + `useFocusTrap` hook on SearchModal and thread overlay; `aria-current` on active server/channel; `aria-pressed` on toggle buttons; `aria-label` on all icon-only buttons; skip-to-content link + `<main id="main-content">` in App

## Phase 7: Extensibility (v0.7) 🟡 Mostly Complete

> **Phase 7 complete.** Bot token scheme, combined auth middleware, and dedicated bot gateway auth (`BotIdentify` opcode) all implemented.

- ✅ Nexus Bot API (REST endpoints)
- ✅ Bot WebSocket gateway events
- ✅ Bot SDK (TypeScript, Python, Rust)
- ✅ Client plugin system (sandboxed)
- ✅ Custom themes (CSS + theme API)
- ✅ Webhooks
- ✅ Slash commands
- ✅ Bot token scheme — `Bot <token>` scheme with SHA-256 hashed tokens stored in DB
- ✅ Bot gateway auth — dedicated `BotIdentify` opcode, separate from user `Identify`

## Phase 8: Federation (v0.8) 🟡 Mostly Complete

> **Phase 8 complete.** Matrix bridge fully implemented with DB persistence.

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

- ✅ Matrix CS-API compatibility layer (`matrix_bridge.rs` — full implementation)
- ✅ Room alias translation (Nexus channel ↔ Matrix room via `matrix_bridge_rooms` DB table)
- ✅ Matrix user puppeting / ghost accounts (`matrix_ghost_users` DB table + `find_or_create_ghost`)
- ✅ Message format translation (Nexus DB messages created from Matrix `m.room.message` events)
- ✅ Outbound relay (Nexus → Matrix via `relay_to_matrix` with stable idempotent txn IDs)
- ✅ `handle_transaction` wired into `PUT /_matrix/app/v1/transactions/{txnId}` handler

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

- ✅ Wire `SfuRoom` RTP forwarding loop in `nexus-voice/src/sfu.rs` — `run_sfu_room()` three-arm select loop fully drives str0m media; `drain_rtc()` / `forward_media()` / `setup_forwarding_tracks()` all implemented
- ✅ Enforce channel permissions — `require_server_permission()` called at all write paths in `routes/channels.rs`; uses `Permissions::from_bits_truncate`, checks server owner and ORs role permissions
- ✅ Fix `uptime_secs` in the health endpoint — `AppState.started_at: std::time::Instant` set on startup; health handler returns `started_at.elapsed().as_secs()`
- ✅ Implement structured bot token scheme — `Bot <base64-token>` scheme with SHA-256 hashed tokens stored in DB (`bots.rs`)
- ✅ Implement dedicated bot gateway `IDENTIFY` flow — `BotIdentify` opcode, separate from user auth path
- ✅ Complete Signal Protocol double-ratchet session state machine — `sender_ratchet_step` on messages; `session_exists` guard for type=2; type=1 no longer creates empty sessions; `LOW_PREKEYS` gateway event at ≤ 5 OTPKs

### 09.6-02: Desktop Client Completion

- ✅ Render emoji reactions in `ChatView` — reaction bar rendered below messages; `reaction_add` / `reaction_remove` gateway events handled
- ✅ Emoji picker for adding reactions — `EmojiPicker` component wired to reaction add flow
- ✅ Embed / link-preview renderer in `ChatView` — `EmbedCard` component consumes embed data from API (title, description, image, colour border)
- ✅ Thread reply panel — `ThreadPanel` side panel wired in `ChatView`; `POST /channels/{id}/messages` with `thread_id`
- ✅ Unread indicators — `unreadChannels` map drives dot/badge on `ChannelList` items
- ✅ Wire OS notifications via Tauri notification plugin on `MESSAGE_CREATE` for @mentions
- ✅ In-app notification tray — bell icon in `ChannelList` header; `NotificationTray` component with dropdown panel showing recent @mentions (per-notification unread indicator, "Clear all", keyboard/focus-trap fully accessible); `addInAppNotification` + `unreadNotificationCount` + `markNotificationsRead` added to Zustand store; `useGateway` feeds tray on every @mention/`@everyone` gateway event (OS notification still fires when window hidden)
- ✅ Global search UI — `SearchModal` Cmd+K/Ctrl+K palette calls `/api/v1/search`, keyboard navigation, jump-to-channel
- ✅ Server settings modal (name edit, invite management with create/copy/revoke, danger zone / delete server)
- ✅ Role management UI — `RoleManagementPanel` with full permission matrix (41 perms, 4 groups), create/edit/delete, colour picker, hoist/mentionable toggles; wired as "Roles" tab in `ServerSettingsModal`; backed by new API routes (`GET/POST/PATCH/DELETE /servers/{id}/roles`) and four Tauri commands (`list_roles`, `create_role`, `update_role`, `delete_role`)
- ✅ Invite management UI — integrated into Server Settings modal (list, create with expiry/max-uses, revoke, copy URL)
- ✅ Emoji management UI — `EmojiManagementPanel` with file upload (PNG/GIF/WebP, FileReader → multipart via `upload_emoji` Tauri command), rename inline, delete with confirmation; animated/managed badges; wired as "Emoji" tab in `ServerSettingsModal`; backed by existing API (`/servers/{id}/emojis` CRUD) and four new Tauri commands (`list_emoji`, `upload_emoji`, `rename_emoji`, `delete_emoji`)
- ✅ Webhook management UI (create, list, delete, copy URL)
- ✅ Bot management UI (add bot by client ID, list installed bots, revoke)
- ✅ Settings — Appearance sub-page (theme selector, font size, message density, dark/light/system)
- ✅ Settings — Notifications sub-page (per-server/channel override toggles, @mention sensitivity — localStorage-backed)
- ✅ Settings — Privacy sub-page (DM permissions, read receipts, presence visibility — localStorage-backed)
- ✅ Settings — Devices / Sessions sub-page (list registered E2EE devices via `list_devices` Tauri command, per-device Revoke via `delete_device`)

### 09.6-03: Infrastructure & Observability

- [x] Structured JSON logging (replace ad-hoc `println!` / `tracing` calls with consistent field schema)
- [x] Prometheus metrics endpoint (`/metrics`) — request counts, latency histograms, active WebSocket connections, voice room counts
- [x] `GET /api/v1/health` extended response — include DB connectivity, Redis connectivity, search backend status
- [x] Graceful shutdown — drain in-flight WebSocket messages before process exit

## Phase 9.7: Account Security (v0.9.7) � In Progress

> **Goal:** Give users real control over their account security. None of these features exist anywhere in the codebase today.

### 09.7-01: Two-Factor Authentication

- ✅ TOTP (RFC 6238) — generate secret, QR code provisioning, verify code on login
- ✅ Backup codes (8× single-use codes shown at 2FA setup, stored as SHA-256 hashes)
- ✅ Enforce 2FA requirement per-server (server setting: members must have 2FA enabled — `require_2fa` column on servers table, enforced in `join_server` and `join_via_invite_route`)
- ✅ 2FA state in JWT claims (downstream permission checks can require `2fa_verified: true`)

### 09.7-02: Email Verification

- ✅ Send verification email on registration (token stored in DB with expiry)
- ✅ `GET /api/v1/auth/verify-email?token=…` endpoint
- ✅ Block access to non-auth routes until email is verified (configurable — `NEXUS__FEATURES__REQUIRE_EMAIL_VERIFICATION=false` to disable for self-hosters; `email_verified` embedded in JWT, checked in both auth middlewares)
- ✅ Resend verification email endpoint

### 09.7-03: Session Management

- ✅ `GET /api/v1/auth/sessions` — list all active sessions (device name, IP, last seen, created at)
- ✅ `DELETE /api/v1/auth/sessions/{id}` — revoke specific session
- ✅ `DELETE /api/v1/auth/sessions` — revoke all other sessions ("log out everywhere")
- ✅ Surface in desktop Settings → Devices / Sessions page — Active Sessions section with per-session revoke and "revoke all other sessions" button

### 09.7-04: Account Lifecycle

- ✅ `DELETE /api/v1/users/@me` — account deletion (soft-delete with 30-day grace period before hard purge)
- ✅ `GET /api/v1/users/@me/data-export` — GDPR-compliant data export (JSON archive of messages, servers, files)
- ✅ Account transfer: server ownership transfer before deletion — `POST /api/v1/servers/{id}/transfer-ownership`, UI in Server Settings Danger Zone

## Phase 9.8: Moderation & Safety (v0.9.8) ✅ Complete

> **Goal:** Give server administrators the tools they need to run a community safely.

### 09.8-01: Server Audit Log

- ✅ `audit_log` DB table (action, target_id, target_type, actor_id, changes JSON, timestamp) — fixed pre-existing NOT NULL bug via migration 00010
- ✅ Write audit entries for: kick, ban, unban, timeout, message delete by moderator (word-filter-triggered deletes write to log)
- ✅ `GET /api/v1/servers/{id}/audit-log` with filter by action type and actor (`?action=&actor_id=&limit=`)
- ✅ Audit writes for role create/update/delete (servers.rs)
- ✅ Audit writes for channel create/update/delete (channels.rs)
- ✅ Audit writes for webhook create/update/delete (webhooks.rs)
- ✅ Audit write for invite create (servers.rs)
- ✅ Audit log viewer in server settings UI (`AuditLogPanel.tsx` — filter, colour-coded badges, collapsible details, load-more pagination)

### 09.8-02: Timeout & Temp-Ban

- ✅ `user_timeouts` DB table (user_id, server_id, expires_at, moderator_id, reason) — migration 00010
- ✅ Timeout enforcement in message send route (returns 400 if `communication_disabled_until > now`)
- ✅ `PUT /api/v1/servers/{id}/members/{uid}/timeout` — set timeout with `duration_secs` + optional reason
- ✅ `DELETE /api/v1/servers/{id}/members/{uid}/timeout` — lift timeout early
- ✅ Temp-ban: `expires_at` column added to `bans` table; `purge_expired_bans` + `purge_expired_timeouts` DB helpers
- ✅ `SERVER_MEMBER_UPDATE` gateway event emitted when timeout is applied or lifted
- ✅ Background sweep task in `nexus-server` — runs every 5 minutes, purges expired bans and timeouts, shuts down cleanly with the server
- ✅ Timeout enforcement in voice join route — `voice_join_preflight` checks `communication_disabled_until > Utc::now()` before allowing entry

### 09.8-03: Message Reporting

- ✅ `message_reports` DB table (message_id, channel_id, server_id, reporter_id, reason, status, resolved_by, resolution_action) — migration 00010
- ✅ Unique index prevents duplicate pending reports per user per message
- ✅ `POST /api/v1/channels/{cid}/messages/{mid}/report` — any member
- ✅ `GET /api/v1/servers/{id}/reports` — mod-only report queue with `?status=pending|resolved|dismissed`
- ✅ `POST /api/v1/servers/{id}/reports/{rid}/resolve` — record resolution action string
- ✅ `POST /api/v1/servers/{id}/reports/{rid}/dismiss`

### 09.8-04: Content Filters

- ✅ `server_word_filters` DB table (server_id, pattern, action: block|delete|warn) — migration 00010
- ✅ `GET /api/v1/servers/{id}/word-filters` — list server filters (MANAGE_SERVER)
- ✅ `POST /api/v1/servers/{id}/word-filters` — add filter pattern with configurable action
- ✅ `DELETE /api/v1/servers/{id}/word-filters/{fid}` — remove filter
- ✅ Apply filter on message create — `block`/`delete` returns 400; `warn` logs and allows
- ✅ Spam detection: Redis-backed rate-limit on duplicate messages per user per channel (30 s window, 3 msg threshold)
- ✅ Apply filter on message edit — same block/warn logic before persisting the edit
- ✅ Configurable spam threshold via server settings — `spam_window_secs` (1–300 s) and `spam_max_messages` (1–20) per-server columns; editable in Server Settings Moderation panel

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
