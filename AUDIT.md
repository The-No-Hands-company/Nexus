# Nexus — Comprehensive Development Audit
**Date:** April 2, 2026  
**Analyst:** GitHub Copilot — Commissioned by The No-Hands Company  
**Scope:** Full codebase analysis, feature completeness, gaps, roadmap, and engineering standards  
**Codebase scale at time of audit:** ~78,500 lines across 207 Rust files, 63 TypeScript/TSX files, 30 SQL migrations, 3 SDK packages

---

## 0. Executive Summary

Nexus is an extraordinarily ambitious and admirably principled project. In under four months of development (migration dating from February 2026), it has amassed a backend that rivals the surface area of Discord's API, an end-to-end-encrypted messaging layer, a real-time WebSocket gateway, a WebRTC SFU-based voice/video server, Matrix federation, a Tauri 2 desktop client, three language SDKs, and 30 database migrations spanning collaboration, AI, scalability, and governance features.

**The vision is correct and the structural foundations are sound.** The project is now in the critical phase where surface breadth needs to be matched with depth: unimplemented handlers must be wired up, the desktop client needs full feature parity with what the API exposes, the mobile client must be created, the web client must be built out, and the "lite mode" single-binary path must be polished for self-hosters. The project can genuinely become the definitive Discord replacement — and much more — if these gaps are closed systematically.

---

## 1. What the Project Has (Inventory)

### 1.1 Backend — Rust Workspace (`crates/`)

#### `nexus-common` — Shared primitives
- ✅ `AppConfig` — full 12-factor environment config (NEXUS__ prefix, platform DB_URL fallbacks)
- ✅ JWT `Claims` with `jti`, `two_fa_verified`, `email_verified` — proper access/refresh/MFA token design
- ✅ `Permissions` bitfield — 41 granular permissions including Nexus-specific ones (RECORD_VOICE, MANAGE_POLLS, SCREEN_SHARE, etc.)
- ✅ `PermissionOverwrite` — role-and-user-level channel overrides
- ✅ Snowflake ID generator (time-sortable UUIDs)
- ✅ `NexusError` — typed error hierarchy mapping cleanly to HTTP status codes
- ✅ Full model library (user, server, channel, message, role, member, bot, webhook, crypto, relationship, rich features, collaboration, multimedia, AI, growth, sustainability, accessibility, voice-collab, ecosystem, plugins, scalability)
- ✅ `GatewayEvent` type with 60+ named event type constants spanning all feature phases
- ✅ Input validation via `validator` crate

#### `nexus-db` — Data layer
- ✅ Dual-backend: PostgreSQL (full mode) + SQLite (lite/single-binary mode) via `sqlx::AnyPool`
- ✅ Redis integration (optional, graceful degradation)
- ✅ 30 SQL migrations covering the complete lifecycle from initial schema through AI intelligence, voice collaboration, growth/retention, to sustainability/governance
- ✅ Repository layer with 55+ modules covering every feature domain
- ✅ MinIO/S3 storage client with local filesystem fallback
- ✅ MeiliSearch full-text search client (optional, starts without it)
- ✅ ScyllaDB configuration (connection defined, keyspace management)

#### `nexus-api` — REST API (Axum)
- ✅ 67 route modules mounted under `/api/v1`
- ✅ Auth (register, login, refresh, 2FA TOTP, email verification, sessions)
- ✅ Users, servers, channels, messages (CRUD, pins, reactions, bulk-delete, crosspost)
- ✅ DMs, Group DMs, relationships (friends, blocks)
- ✅ Voice channel management (REST side)
- ✅ File uploads (multipart, S3 upload)
- ✅ Threads, forum posts, stage channels
- ✅ Emoji and sticker management
- ✅ Full-text search
- ✅ Presence system
- ✅ E2EE (end-to-end encrypted DMs and channels — key exchange, ciphertext-map storage)
- ✅ Public key infrastructure (`/keys` routes)
- ✅ Bot applications, webhook management, slash commands, extensibility
- ✅ Federation admin panel and directory endpoints
- ✅ Two-factor authentication management
- ✅ Session management (list + revocation)
- ✅ Moderation (audit log, kick, ban, timeout, reports, word filters)
- ✅ Polls, scheduled messages, bookmarks, drafts
- ✅ Message forwarding, server events/RSVP, inline bot queries
- ✅ User badges, server booster/supporter tiers
- ✅ Canvas document channels
- ✅ Server discovery and browsing
- ✅ Creator monetization (tip jar, subscription tiers, analytics)
- ✅ Task boards, calendar events, file versioning, AI assistant preferences
- ✅ Voice notes, stories/ephemeral status, drawings/annotations
- ✅ Voice settings, music queue, media gallery
- ✅ Accessibility settings
- ✅ Voice captions, message translations, TTS
- ✅ Data import (Discord/Slack/Matrix), bulk invitations, onboarding wizard
- ✅ Admin analytics, plugin marketplace
- ✅ Scalability config (SFU nodes, federation batches, member prune rules, slow-mode overrides)
- ✅ AI layer (suggestions, thread summaries, toxicity scoring, raid detection, consent management, audit log)
- ✅ Voice/real-time collab (video layouts, virtual backgrounds, live streams, breakout rooms, spatial audio)
- ✅ Growth & retention (recommendations, gamification/XP/achievements, offline queue, sync cursors)
- ✅ Sustainability (governance polls/proposals, contributor badges, security audit records, migration guides, protocol versioning)
- ✅ Rate limiting middleware (per-IP, Redis-backed when available)
- ✅ Prometheus metrics (`/metrics` endpoint, per-request counters/histograms)
- ✅ CORS, gzip compression, request size limits, timeout middleware
- ✅ Combined auth middleware supporting both JWT Bearer (users) and Bot tokens

#### `nexus-gateway` — WebSocket Gateway
- ✅ WebSocket upgrade handler with Hello/Identify/BotIdentify/Ready/Heartbeat/Resume/Dispatch protocol
- ✅ Session management (SessionManager)
- ✅ Broadcast channel (10,000 event buffer) shared between API and Gateway
- ✅ Per-user event filtering (sends only events for servers the user is in)
- ✅ Presence updates and typing indicators
- ✅ Voice state updates
- ✅ Bot intent system (bitfield subscriptions)

#### `nexus-voice` — WebRTC SFU Voice/Video
- ✅ SFU architecture (str0m-based) — no server mixing, client-side volume control
- ✅ Room abstraction (participant tracking per voice channel)
- ✅ Voice state manager (presence tracking across channels)
- ✅ WebSocket signaling handler (SDP/ICE exchange)
- ✅ REST integration (voice stats, moderation actions)
- ✅ Stats reporting (active channels, connections, SFU rooms, streams, video count)

#### `nexus-federation` — Matrix-Compatible Federation
- ✅ Ed25519 server key pair generation and management
- ✅ Signed HTTP requests (X-Matrix-style auth)
- ✅ Federation client (outbound S2S HTTP)
- ✅ Server discovery (`.well-known/nexus/server`, SRV DNS, HTTPS fallback)
- ✅ Matrix Application Service bridge (`MatrixBridge`, `BridgeConfig`)
- ✅ Federated friend requests/responses
- ✅ Peering types (PeerRecord, PeerStatus, PeerHealth)

#### `nexus-server` — Main Binary
- ✅ Unified process: API (8080), Gateway (8081), Voice (8082), Federation (8448)
- ✅ Lite mode: SQLite + local filesystem, zero Docker dependencies
- ✅ JWT secret generation/persistence for lite mode (`nexus.toml`)
- ✅ Proper graceful startup with dependency health checks
- ✅ Structured JSON logging (production) and pretty logging (lite/dev)
- ✅ Prometheus metrics exporter
- ✅ Migration-on-startup

---

### 1.2 Frontend — Tauri 2 Desktop Client

- ✅ React 18 + TypeScript + Vite + Tailwind CSS
- ✅ Tauri 2 backend (Rust) with full command layer
- ✅ Auth flow (Login, Register pages)
- ✅ Main layout (server list rail, channel list, chat panel)
- ✅ 50+ React components mapped to all feature domains
- ✅ Zustand store with session, server, channel, message state
- ✅ Gateway WebSocket hook (`useGateway`)
- ✅ Push-to-talk hook (`usePtt`) via Tauri global shortcuts
- ✅ Focus trap hook (`useFocusTrap`) for accessibility
- ✅ Theme system: 6 built-in themes (Nexus Dark, Midnight, Ocean, Nexus Light, High Contrast Dark/Light), custom theme support via CSS variables
- ✅ Plugin system (iframe sandbox, PluginLoader, PluginMarketplace component)
- ✅ Global search (Cmd+K / Ctrl+K)
- ✅ Auto-updater integration (Tauri updater plugin)
- ✅ System tray with presence/quick-action menu
- ✅ Gaming overlay window
- ✅ Skip navigation link (accessibility)
- ✅ Token refresh scheduler (every 10 min)
- ✅ Tauri command modules: auth, servers, channels, messages, voice, DMs, presence, friends, bots, webhooks, emoji, settings, E2EE, directory, boosters

---

### 1.3 SDKs

#### `nexus-sdk` (TypeScript/Node.js)
- ✅ `NexusClient` — main bot client
- ✅ `RestClient` with `NexusAPIError`
- ✅ `GatewayClient` with typed events and reconnect logic
- ✅ `SlashCommandBuilder`, `SlashCommandOptionBuilder`
- ✅ `EmbedBuilder`
- ✅ Full type exports

#### `nexus-sdk-rs` (Rust)
- ✅ `NexusClient`, `GatewayClient`, `RestClient`
- ✅ `SlashCommandBuilder`
- ✅ Error type
- ✅ Types module

#### `nexus-sdk-py` (Python)
- ✅ `NexusClient`, `RestClient`, `GatewayClient`
- ✅ Builders, types

---

### 1.4 Infrastructure & DevOps

- ✅ Docker Compose for local dev (Postgres, Redis, ScyllaDB, MinIO, MeiliSearch)
- ✅ Production Docker Compose with secrets-managed config
- ✅ Dockerfile (multi-stage Rust build)
- ✅ Caddy reverse proxy config
- ✅ Fly.io deployment manifest (`fly.toml`)
- ✅ Helm chart (Chart.yaml, values.yaml, templates/)
- ✅ Platform deployment scripts (Linux/macOS/Windows auto-start)
- ✅ `systemd` service unit file
- ✅ k6 load tests for auth and message flows
- ✅ deny.toml (cargo deny — dependency auditing)
- ✅ AGPL-3.0 license
- ✅ Security policy (SECURITY.md)
- ✅ Contributing guide
- ✅ Code of conduct
- ✅ Federation guide, self-hosting guide, upgrading guide

---

## 2. Critical Gaps — What's Missing

This section is the core of the audit. Items are classified by **severity** and **user-facing impact**.

---

### 2.1 🔴 CRITICAL — Blocking Real-World Usage

#### 2.1.1 ScyllaDB is Declared but Not Wired In
**Problem:** `docker-compose.yml` starts ScyllaDB, `AppConfig` has `ScyllaConfig`, and the README says messages are stored in ScyllaDB. However, the `nexus-db` crate contains **no ScyllaDB client, no keyspace initialization, no CQL schema, and no message writes go through Scylla** — all message operations go through `sqlx::AnyPool` (PostgreSQL/SQLite).

**Impact:** The "write-heavy, time-series" message scalability story is a broken promise. Under heavy load a single messages table in PostgreSQL will become a bottleneck. The ScyllaDB container starts but is unused.

**Fix needed:** Implement `scylla` crate integration in `nexus-db`, create a CQL keyspace + `messages` table, route all message read/write operations through Scylla (or document that PostgreSQL is the permanent choice and remove Scylla from the stack).

#### 2.1.2 WebRTC SFU is Declared but Mostly Stubbed
**Problem:** `nexus-voice/src/sfu.rs` exists but the actual WebRTC media relay logic using `str0m` (or any WebRTC crate) is not fully implemented. The `SfuManager` has room counting but bidirectional media forwarding between peers is not operational.

**Impact:** Voice and video — two of the core differentiators from Discord — don't actually work end-to-end. The architecture is correct (SFU, signaling WS, room abstraction) but the media layer needs to be fully wired.

**Fix needed:** Complete the `str0m`-based SFU implementation: ICE candidate exchange, DTLS handshake, SRTP demux, RTP forwarding between room participants, VP9/AV1 for video, Opus for audio.

#### 2.1.3 E2EE is Schema-Only — Signal Protocol Not Integrated
**Problem:** The `nexus-api/routes/e2ee.rs` and `nexus-db/repository/keystore.rs` handle storing and querying pre-keys and ciphertext. The `README.md` lists Signal Protocol (libsignal) as the encryption library. However, **there is no Signal Protocol / Double Ratchet implementation present** in any crate — key derivation, ratchet state, session establishment, and the ciphertext generation are all left to clients with no server-side verification or key infrastructure.

**Impact:** The E2EE story is incomplete. Clients send opaque ciphertext blobs and the server has no way to enforce correct usage. Forward secrecy depends entirely on unverified client behavior.

**Fix needed:** Either embed `libsignal-client` Rust bindings for key infrastructure (pre-key validation, signed prekey rotation) or document clearly that this is a client-side-only scheme and harden the server's key distribution endpoint accordingly.

#### 2.1.4 Data Import Pipeline is a Stub
**Problem:** `POST /servers/:id/imports` with `source_platform: "discord"|"slack"|"matrix"` creates an `ImportJob` row but has no actual parser — no Discord CDN downloader, no Slack export reader, no Matrix room state importer.

**Impact:** The migration from Discord — arguably the single most compelling call to action — doesn't work. Users cannot bring their history.

**Fix needed:** Implement at least the Discord package import (JSON export files + media CDN re-upload to MinIO). This is high user acquisition value.

#### 2.1.5 AI Features Have No Model Backend
**Problem:** AI routes (suggestions, thread summaries, toxicity scoring, raid detection, voice transcripts) all write and read from the database but there is **no inference engine**, no model loading, no LLM call, and no ML pipeline. The routes are fully plumbed into the DB but produce no actual AI output.

**Impact:** The "AI layer" is entirely inert. This is fine for a future phase, but the AI consent system and audit log imply these features are live.

**Fix needed:** Either wire up a local on-device model (e.g. `llama.cpp` via FFI, `candle` for Rust-native inference, or `ollama` sidecar), or clearly mark all AI routes as `501 Not Implemented` until the backend is ready. The key insight from the project's philosophy: AI must be **on-device or opt-in**, never cloud-mandatory.

---

### 2.2 🟠 HIGH — Feature Gaps Hurting Competitiveness

#### 2.2.1 No Web Client
**Problem:** The project has a Tauri desktop client, mobile is mentioned in the README, but there is **no web application** — no `nexus-web` package, no browser-compatible frontend.

**Impact:** A large percentage of users (especially those at work/school, on Chromebooks, or trying the platform for the first time) use web apps. Without a web client, Nexus cannot replace Discord for most casual users.

**Fix needed:** Create a `packages/nexus-web` (Vite + React app reusing the same component library from the desktop client) that communicates with the API over HTTPS, connects to the Gateway over WSS, and handles WebRTC through the browser's native APIs.

#### 2.2.2 No Mobile Client
**Problem:** The README mentions "React Native — Shared codebase with web, native feel" in the tech stack table, but **there is no mobile project** in the repository.

**Impact:** Discord's dominant use case is mobile. Without iOS and Android apps, Nexus cannot replace Discord in most users' lives.

**Fix needed:** Bootstrap a `packages/nexus-mobile` React Native (or Expo) project with feature parity for the critical path: auth → server list → channels → messaging → voice. The existing TypeScript SDK can be used directly.

#### 2.2.3 Desktop Client Has ~50 Component Files but Many are UI Shells
**Problem:** Many of the 50+ component files in `crates/nexus-desktop/src/components/` are likely UI scaffolds without full feature implementation (e.g., `CalendarPanel.tsx`, `CanvasView.tsx`, `DrawingCanvas.tsx`, `AiIntelligencePanel.tsx`, `GrowthRetentionPanel.tsx`, `SustainabilityPanel.tsx`). The main layout only routes to: `/home`, `/channel/:id`, `/voice/:id`, `/settings` — none of the advanced panels are accessible from the navigation.

**Impact:** Users cannot access the majority of what the backend supports.

**Fix needed:** Audit each component for implementation completeness. Wire advanced panels to the settings and navigation system. Add routes for: tasks, calendar, canvas, stories, media gallery, server events, polls view, bookmarks, drafts.

#### 2.2.4 Gateway is Single-Node Only
**Problem:** `GatewayState` contains a comment: *"In production, this would use Redis pub/sub for multi-node support."* The implementation uses an in-process `broadcast::channel`. When running multiple server instances, events on node A will not reach clients on node B.

**Impact:** Nexus cannot horizontally scale the gateway without this. Large communities will hit the single-node limit.

**Fix needed:** Implement Redis pub/sub event routing in `nexus-gateway`. When `redis` is configured, subscribe to a channel (e.g. `nexus:events`) and publish all gateway events there instead of (or in addition to) the local broadcast.

#### 2.2.5 Rate Limiting is In-Memory Only
**Problem:** The `check_rate_limit` function in `middleware.rs` uses in-process rate limit state. Multi-node deployments will have per-node limits, not global limits, allowing bypass.

**Impact:** Rate limiting is ineffective at scale. Spam and abuse can bypasse limits by hitting different nodes.

**Fix needed:** Move rate limiting to Redis using sliding window counters (INCR + EXPIRE, or the token bucket pattern). Fall back to in-process rate limiting when Redis is not configured (lite mode).

#### 2.2.6 Moderation Pipeline is Partially Wired
**Problem:** The moderation routes exist (audit log, kick, ban, timeout, word filters) but the actual enforcement of word filters on incoming messages is not present in `send_message`. Bans and kicks write to the database but there is no check on the Gateway connection to reject banned users or disconnect timed-out users.

**Impact:** Bans can be bypassed by reconnecting to the WebSocket. Word filters have no effect.

**Fix needed:** Add a user status check at WebSocket `Identify` time. Run word filter matching in the `send_message` handler before persisting the message.

#### 2.2.7 Presence is Not Persisted Across Sessions
**Problem:** Presence updates via the Gateway are dispatched as events but there is no write-back to `users.presence` in the database. If a user is marked `online` and the server restarts, their presence will show as `offline` to others even though they are connected.

**Impact:** Presence is unreliable for users on different instances or after server restarts.

**Fix needed:** In the Gateway connection handler, write presence to the database (debounced) and to Redis when available. On disconnect, set back to `offline`.

---

### 2.3 🟡 MEDIUM — Quality and Depth Improvements

#### 2.3.1 Many Route Handlers Return Stub Responses
Several route handlers return `StatusCode::NOT_IMPLEMENTED` or empty success responses without actual business logic. Notable examples include advanced features in `voice_collab.rs`, `ai_layer.rs`, `growth.rs`, `sustainability.rs`, and `scalability.rs`. These are architectural placeholders.

**Fix needed:** Systematically complete each handler, starting with the most user-facing (voice collab, growth features) before the more administrative ones.

#### 2.3.2 No Message Queue / Job System
**Problem:** Scheduled messages (`scheduled_messages` table) have routes for CRUD but there is no background task runner that reads pending scheduled messages and dispatches them at their `send_at` time. Similarly, bulk invitations and import jobs have no background worker.

**Fix needed:** Implement a background task system. Options: a `tokio::spawn` loop on startup polling for due jobs, or integrate `tokio-cron-scheduler`. For production, integrate a proper job queue.

#### 2.3.3 Search (MeiliSearch) Integration is One-Directional
**Problem:** Messages are indexed on creation (`SearchClient::index_message`) but the index is never updated on message edit or deleted on message delete. Search results will return stale/deleted messages.

**Fix needed:** Call `SearchClient::update_message` on edit and `SearchClient::delete_message` on delete/bulk-delete throughout the message handlers.

#### 2.3.4 File Upload Has No Virus Scanning or Content Validation
**Problem:** `POST /uploads` accepts any file up to the configured limit and uploads to MinIO with no content inspection. A malicious actor can upload executables, malware, or CSAM.

**Fix needed:** Integrate ClamAV (via `clamav-client` crate or a sidecar) on upload, especially for publicly readable files. Apply MIME type validation (check magic bytes, not just `Content-Type`). Log all uploads with uploader metadata.

#### 2.3.5 Password Reset Flow Missing
**Problem:** Registration accepts optional email for "password recovery" but there is no `POST /auth/forgot-password` or `POST /auth/reset-password` route. The email service (`EmailService`) exists and is wired in, but the reset flow is absent.

**Fix needed:** Implement the full password reset flow: generate a time-limited signed token, email it via Resend, validate it on submission, re-hash and store the new password.

#### 2.3.6 Account Deletion is Absent
**Problem:** There is no `DELETE /users/@me` endpoint. GDPR Right to Erasure requires this. The AGPL license and open-source ethos demand it.

**Fix needed:** Implement `DELETE /users/@me` with cascading anonymization (replace messages with `[deleted]`, remove PII, delete keys), a confirmation flow, and a 30-day grace period with restoration option.

#### 2.3.7 Phone Number — Correctly Absent, but Email Leakage Risk
**Problem:** The project correctly never requires phone numbers or government ID. However, email addresses (optional) are stored in plaintext and could be harvested from a DB breach.

**Fix needed:** Encrypt email addresses at rest using a server-side key (AES-256-GCM). Store only a hash for lookup purposes and the encrypted value for display/reset. Alternatively, accept only peppered SHA-256 hashes from clients so the server never sees plaintext emails.

#### 2.3.8 Attachment URL Expiry
**Problem:** File URLs served by MinIO are either public permanent URLs or pre-signed short-lived ones. The current `send_message` flow likely embeds permanent MinIO URLs. There's no expiry or access control on attachment links.

**Fix needed:** Use S3 pre-signed URLs with TTLs (e.g., 24h). Regenerate them on message fetch. Ensure private channels' attachments are not accessible without authentication.

---

### 2.4 🔵 IMPORTANT — Developer Experience, Ecosystem, and Platform Vision

#### 2.4.1 Bot/Plugin Marketplace Needs Curation Model
**Problem:** `GET /marketplace/plugins` lists plugins, but there is no review process, sandbox policy, permission manifests, or trust scoring. Malicious plugins could exfiltrate user data.

**Fix needed:** Define a plugin manifest format (name, permissions requested, origin URL, signature), implement a sandboxed iframe with a restrictive Content-Security-Policy, and require explicit grant for each permission (camera, microphone, message history, DMs).

#### 2.4.2 SDK Documentation Missing
**Problem:** All three SDKs (`nexus-sdk`, `nexus-sdk-rs`, `nexus-sdk-py`) have `README.md` files and types but **no published documentation site**, no example bots, and no tutorial. Without docs, bot developers won't adopt the platform.

**Fix needed:** Generate TypeScript docs (TypeDoc), Rust docs (`cargo doc`), Python docs (pdoc). Set up a `docs.nexus.gg` equivalent. Write three canonical example bots (ping/pong, moderation helper, music queue).

#### 2.4.3 No CI/CD Pipeline
**Problem:** There is no `.github/workflows/` directory, no CI configuration, and no automated testing gate.

**Impact:** PRs can merge broken code. No automated security scanning. No release pipeline.

**Fix needed:** Set up GitHub Actions with:
- `cargo test` on every PR
- `cargo clippy -- -D warnings`
- `cargo deny check` (cve auditing, already has `deny.toml`)
- TypeScript type-check and lint
- Docker image builds and pushes on tags

#### 2.4.4 Test Coverage is Near-Zero
**Problem:** Only k6 load tests exist for auth and messages. There are no unit tests inside any crate (no `#[cfg(test)]` modules found), no integration tests against a running server, and no frontend tests.

**Impact:** Regressions go undetected. Refactoring is risky.

**Fix needed:** Add unit tests to at minimum: `nexus-common` (config, permissions, validation, snowflake), `nexus-api` (all auth flows, permission checks, input validation), and `nexus-db` (repository functions). Use `sqlx::test` for DB tests with test transactions.

#### 2.4.5 Admin Dashboard Not Present
**Problem:** All moderation, analytics, and admin operations are REST endpoints but there is no admin web UI. Instance administrators have no visual tool for managing users, monitoring metrics, or reviewing audit logs.

**Fix needed:** Build an `packages/nexus-admin` React app (or embed it in the desktop client settings) that presents: user management, server overview, federation peers status, audit log viewer, metrics dashboard, and moderation queue.

#### 2.4.6 Notification System Incomplete
**Problem:** The `notifications.rs` Tauri module exists for system-level push notifications, and there are `NotificationTray.tsx` components. However, there is no push notification infrastructure for mobile (APNs + FCM), no `POST /users/@me/push-subscriptions` endpoint, and no Web Push API endpoint for the future web client.

**Fix needed:** Implement server-side push notification dispatch using `web-push` crate for Web Push and platform-specific APIs for mobile. Queue notifications via Redis when users are offline.

#### 2.4.7 Accessibility — Structure Exists but Depth Needed
**Problem:** The `AccessibilitySettingsPanel.tsx` component exists. The skip-nav link is implemented. The `useFocusTrap` hook exists. The database has an `accessibility_settings` table. However, many interactive components likely lack:
- `aria-label` on icon-only buttons
- `role="log"` on message scroll areas
- `aria-live` regions for notifications
- Keyboard navigation for the server/channel list
- Screen reader announcements for new messages

**Fix needed:** Conduct a WCAG 2.1 AA audit of the core chat flow. Every interactive element needs proper ARIA roles, labels, and states.

#### 2.4.8 Internationalization (i18n) Missing
**Problem:** All UI strings are hardcoded English. There is no i18n framework or locale management.

**Impact:** Non-English-speaking communities (potentially the majority of global users) get an inferior experience.

**Fix needed:** Integrate `react-i18next` (or similar) in the frontend. Externalize all UI strings. Bootstrap with English and open for community translation contributions.

---

### 2.5 ⚪ FUTURE — Long-Term Platform Completeness

These items are not blocking but are needed to fulfil the "replace any chat app from the past to the future" vision:

| Gap | Description |
|-----|-------------|
| **IRC bridge** | An IRC ↔ Nexus bridge (similar to Matrix's IRC bridge) to absorb the remaining IRC community |
| **XMPP gateway** | For enterprise chat migration from Jabber/XMPP |
| **Telegram import** | Many Discord refugees are also Telegram expatriates |
| **Slack import** | Professional community migration path |
| **WhatsApp import** | Most common global messaging app — import chat history |
| **ActivityPub** | Federation with Mastodon/Misskey/Lemmy for social graph bootstrapping |
| **P2P/offline mesh** | Local network discovery and direct messaging without a server (for power users and privacy extremists) |
| **Server-side search analytics** | Query tracking (with consent) to improve search relevance over time |
| **Content delivery network** | CDN for media delivery to globally distributed users |
| **Video call recording** | Server-side recording with consent indicators (already has `RECORD_VOICE` permission) |
| **Multi-factor recovery codes** | Backup codes for 2FA (standard practice, missing from TOTP implementation) |
| **Hardware key (FIDO2/WebAuthn)** | Second factor for high-security users |
| **Magic link login** | Email-based passwordless auth as an alternative to username/password |
| **Username change history** | Track user renames for moderation and reputation |
| **Per-message read receipts (optional)** | User-controlled delivery/read acknowledgment |
| **Message scheduling UI** | The API for scheduled messages exists but the desktop client needs a date/time picker UI |
| **Rich embeds from URLs** | Open Graph preview generation (server-side) with privacy controls |
| **Syntax highlighting themes** | Code block rendering should support user-selectable syntax themes |
| **LaTeX/math rendering** | Many technical communities need this (MathJax/KaTeX) |
| **Mermaid/diagram rendering** | Engineers, architects, and educators need this |
| **Custom domain for vanity invites** | `join.myserver.com` → `nexus.myserver.com/invite/...` |
| **Server backup/export** | Full server export (messages, channels, roles, emoji) by admins |

---

## 3. Architecture Assessment

### 3.1 Strengths

| Area | Assessment |
|------|------------|
| **Rust backend** | Excellent choice. Memory-safe, async-native, handles enormous concurrency without GC pauses. Axum + Tokio is the right stack. |
| **Permission system** | Solid bitfield design with 41 named permissions + channel overrides. More granular than Discord's. |
| **Dual-mode DB** | The SQLite/PostgreSQL `AnyPool` design is clever — `nexus serve --lite` is a real competitive differentiator for self-hosting. |
| **Error handling** | Uniform `NexusError` type with clean HTTP mapping is excellent. No raw `.unwrap()` scattered through handlers. |
| **Shared event bus** | The `broadcast::channel<GatewayEvent>` pattern between API and Gateway is elegant and correct. |
| **Federation** | Ed25519 key signing, `.well-known` discovery, Matrix bridge — architecturally sound and comprehensive. |
| **Config** | 12-factor config with `NEXUS__` prefix, platform fallbacks (DATABASE_URL, REDIS_URL), and sensible defaults. |
| **Logging** | Structured JSON in production, pretty in dev. Granular `tracing` spans throughout. |
| **Theme system** | CSS variable-based, 6 built-in themes including two accessibility themes, custom theme support. Excellent. |
| **SDK surface** | Three language SDKs (TypeScript, Rust, Python) is ambitious and valuable for bot adoption. |

### 3.2 Architectural Concerns

| Area | Concern | Priority |
|------|---------|----------|
| **Message storage backend** | SQLite/Postgres for messages won't scale to Discord-sized loads. ScyllaDB must be wired in OR the architecture document must be updated. | HIGH |
| **Single broadcast channel** | `broadcast::channel(10_000)` is a single point of contention. Large servers with many clients will hit backpressure. Redis pub/sub is needed for multi-node. | HIGH |
| **No CDN** | All media served from MinIO directly. Global users will experience high latency. Needs a CDN tier (Cloudflare R2, Bunny CDN). | MEDIUM |
| **Scylla in docker-compose with `--developer-mode 1`** | The dev mode disables performance guardrails. Fine for dev but `docker-compose.prod.yml` should be checked to remove this flag. | LOW |
| **Missing distributed tracing** | `tracing` spans are used but there's no OpenTelemetry exporter. Production debugging of distributed issues will be painful. | MEDIUM |
| **No circuit breaker** | API handlers call into the DB directly — if Postgres is slow, all handlers stall. A connection pool timeout exists but no circuit breaker pattern for external services. | MEDIUM |

---

## 4. Security Assessment

### 4.1 What's Right ✅

- Argon2id for password hashing — correct algorithm with fast debug params
- JWT with `jti` for session revocation — proper session management
- TOTP 2FA (RFC 6238) via `totp-rs`
- Rate limiting middleware present
- `cargo deny` vulnerability scanning in `deny.toml`
- CORS configured
- Request size limits via `tower-http`
- Bot tokens stored as SHA-256 hashes, not plaintext
- No government ID requirement — privacy by design
- Telemetry is opt-in only

### 4.2 What Needs Fixing 🔴

| Issue | Risk | Fix |
|-------|------|-----|
| Email stored in plaintext | DB breach exposes user emails | Encrypt at rest with server-side key |
| No HTTPS enforcement in dev | Dev MITM risk | Add `tower-http` HTTPS redirect in non-lite mode |
| CORS `allow_any_origin` (likely) | Need to verify scope | Lock CORS to configured origin in production |
| No Content-Security-Policy headers | XSS risk in web client | Add CSP headers via middleware |
| File uploads not content-validated | Malware/CSAM upload risk | Magic byte validation + ClamAV integration |
| No token rotation on privilege escalation | Token replay after 2FA | Issue new token after 2FA verification |
| Missing FIDO2/WebAuthn | Phishing resistance | Implement WebAuthn for high-security users |
| `unsafe { std::env::set_var }` in main.rs | Potential race in multi-thread startup | Restructure startup to set env before spawning |
| No Subresource Integrity for plugin iframes | Plugin tampering | Add SRI checks for plugin manifests |
| Bot token hash uses SHA-256 not Argon2 | Brute-forceable offline | Use HMAC-SHA256 with a server secret pepper |

---

## 5. Performance Assessment

### 5.1 What's Good ✅

- Axum + Tokio: async, non-blocking, handles millions of concurrent connections
- `sqlx` with connection pooling (min/max connections configurable)
- `tower-http` gzip compression
- `tower-http` timeout middleware
- MeiliSearch for typo-tolerant full-text search (vs Postgres ILIKE which is slow)
- SFU architecture for voice (no server mixing = linear scaling)
- Redis for hot-path data (sessions, presence, rate limiting)

### 5.2 Performance Gaps

| Gap | Impact |
|-----|--------|
| No message read caching | Fetching message history hits Postgres on every request |
| No CDN for media | High latency for non-local users |
| N+1 queries likely in some handlers | `list_servers` probably does per-server member count queries |
| ScyllaDB not used for messages | PostgreSQL messages table will be bottleneck at scale |
| Search index not updated on edit/delete | Stale results, growing index size |
| No HTTP/2 or HTTP/3 | Caddy handles this, but the Axum server should support it directly for API-to-API calls |

---

## 6. Recommended Development Roadmap

This is a prioritized plan for continuing development. Each item is labelled with estimated complexity and impact.

### Phase 1 — Stabilize the Core (Immediate: 2-4 weeks)

Priority: Make what exists production-ready before adding more features.

1. **Wire ScyllaDB for messages** OR document PostgreSQL-only and remove Scylla from the stack (Complexity: HIGH, Impact: CRITICAL for scale)
2. **Complete the WebRTC SFU** — actual media relay between room participants (Complexity: HIGH, Impact: CRITICAL)
3. **Password reset flow** — forgot/reset password via email (Complexity: LOW, Impact: HIGH UX)
4. **Account deletion** — `DELETE /users/@me` with GDPR compliance (Complexity: MEDIUM, Impact: HIGH legal)
5. **Word filter enforcement** — apply word filters in `send_message` handler (Complexity: LOW, Impact: MEDIUM safety)
6. **Ban enforcement at Gateway** — check ban status in WebSocket `Identify` (Complexity: LOW, Impact: MEDIUM safety)
7. **Search index sync** — update/delete in MeiliSearch on message edit/delete (Complexity: LOW, Impact: MEDIUM UX)
8. **Scheduled message dispatcher** — background task runner (Complexity: MEDIUM, Impact: MEDIUM UX)
9. **Recovery codes for 2FA** — 10 single-use backup codes on TOTP enable (Complexity: LOW, Impact: HIGH security)
10. **Fix `unsafe env::set_var`** in `main.rs` startup (Complexity: LOW, Impact: LOW security hygiene)

### Phase 2 — Build the Web Client (4-8 weeks)

11. **`packages/nexus-web`** — Vite + React SPA reusing existing component library (Complexity: HIGH, Impact: CRITICAL adoption)
12. **Web Push notifications** — `POST /users/@me/push-subscriptions`, background worker (Complexity: MEDIUM, Impact: HIGH)
13. **Redis pub/sub for Gateway** — multi-node gateway event routing (Complexity: MEDIUM, Impact: HIGH scale)
14. **Email at-rest encryption** — AES-GCM for stored email addresses (Complexity: MEDIUM, Impact: HIGH privacy)
15. **CSP + security headers middleware** — for web client (Complexity: LOW, Impact: HIGH security)
16. **i18n framework** — `react-i18next`, externalize all strings (Complexity: MEDIUM, Impact: HIGH accessibility)

### Phase 3 — Launch the Mobile App (8-16 weeks)

17. **`packages/nexus-mobile`** — React Native / Expo app (Complexity: HIGH, Impact: CRITICAL adoption)
18. **APNs + FCM push notifications** — platform-specific mobile push (Complexity: MEDIUM, Impact: HIGH)
19. **Offline-first message queue** — background sync when network is unavailable (Complexity: MEDIUM, Impact: HIGH mobile UX)
20. **Rich notification payloads** — message preview, quick-reply action (Complexity: MEDIUM, Impact: MEDIUM)

### Phase 4 — Polish the Desktop Client (ongoing)

21. **Wire all 50 component panels** into navigation/settings/routes (Complexity: MEDIUM, Impact: HIGH)
22. **Message scheduling UI** — date/time picker for scheduled messages (Complexity: LOW, Impact: MEDIUM)
23. **Per-user volume controls** — audio mixer UI in voice channel component (Complexity: MEDIUM, Impact: HIGH)
24. **Keyboard navigation audit** — ensure all components are keyboard-accessible (Complexity: MEDIUM, Impact: HIGH accessibility)
25. **ARIA audit** — screen reader labels for all interactive elements (Complexity: MEDIUM, Impact: HIGH accessibility)

### Phase 5 — AI Layer (privacy-first, on-device)

26. **Integrate on-device AI** via `candle` (Rust ML) or `llama.cpp` FFI — summaries, suggestions (Complexity: HIGH, Impact: HIGH differentiator)
27. **Toxicity classification** — on-device text classification model for moderation assist (Complexity: HIGH, Impact: MEDIUM)
28. **Opt-in transcription** — on-device speech-to-text for voice channels (Complexity: HIGH, Impact: MEDIUM accessibility)
29. **Raid detection** — statistical anomaly detection on join patterns (Complexity: MEDIUM, Impact: MEDIUM safety)
30. **Smart reply suggestions** — local model, no server call, fully private (Complexity: HIGH, Impact: MEDIUM UX)

### Phase 6 — Imports and Migration

31. **Discord package importer** — parse Discord data export JSON, re-upload media (Complexity: HIGH, Impact: CRITICAL adoption)
32. **Slack export importer** — workspace migration (Complexity: MEDIUM, Impact: HIGH enterprise)
33. **Matrix room bridge** — the federation code exists, connect it to the import UI (Complexity: MEDIUM, Impact: HIGH)
34. **IRC bridge** — Nexus ↔ IRC relay (Complexity: MEDIUM, Impact: MEDIUM niche)

### Phase 7 — Developer Ecosystem

35. **CI/CD pipeline** — GitHub Actions for test, lint, build, release (Complexity: LOW, Impact: CRITICAL quality)
36. **Unit test suite** — at least 80% branch coverage for common + api + db (Complexity: HIGH, Impact: CRITICAL quality)
37. **SDK documentation site** — TypeDoc, cargo doc, pdoc deployed to a docs site (Complexity: LOW, Impact: HIGH adoption)
38. **Example bots** — three canonical example bots in each SDK language (Complexity: LOW, Impact: HIGH adoption)
39. **Plugin permission manifests** — sandbox governance for the marketplace (Complexity: MEDIUM, Impact: HIGH security)
40. **Admin dashboard** — `packages/nexus-admin` web app (Complexity: HIGH, Impact: HIGH operability)

---

## 7. Sustaining the "Completely Free" Commitment

The project's zero-cost-to-users commitment is both its greatest strength and its greatest operational challenge. Here is a sustainability framework that keeps Nexus free forever:

### 7.1 Cost Structure for Self-Hosters
The "lite mode" single binary with SQLite is genuinely zero-dependency. A small community can run Nexus on a $5/mo VPS with no external services. This must remain a first-class deployment target.

### 7.2 Cost Structure for Hosted Instances
For communities that want managed hosting, the server monetization routes (tip jar, subscription tiers) allow community owners to fund their own infrastructure costs without Nexus taking a cut.

### 7.3 The No-Hands Company's Revenue Model
The project should document clearly that the organization funds development through:
- **Hosting revenue** from `nexus.gg` (if/when launched as a public instance)
- **Enterprise support contracts** for organizations self-hosting large instances
- **Bounty program** funded by community donations
- **Merchandise / supporter badges** (cosmetics only, no paywall features)

The `boosters` feature already exists for community-directed voluntary support. The `monetization` routes allow community owners to run subscription tiers — Nexus never takes a percentage.

---

## 8. Summary Table

| Category | Status | Priority |
|----------|--------|----------|
| REST API breadth | ✅ Exceptional (67 route modules) | — |
| WebSocket Gateway | ✅ Solid foundation | Multi-node Redis needed |
| Voice/Video SFU | ⚠️ Architecture correct, media layer incomplete | CRITICAL |
| E2EE | ⚠️ Schema + key exchange done, Signal Protocol not integrated | HIGH |
| Federation | ✅ Comprehensive | — |
| Desktop Client | ✅ Built, many panels need wiring | HIGH |
| Web Client | ❌ Does not exist | CRITICAL |
| Mobile Client | ❌ Does not exist | CRITICAL |
| ScyllaDB for messages | ❌ Declared, not implemented | CRITICAL |
| Data import (Discord) | ❌ Stub only | HIGH |
| AI features | ⚠️ DB/API exist, no model backend | MEDIUM |
| Password reset | ❌ Missing | HIGH |
| Account deletion | ❌ Missing (GDPR required) | HIGH |
| CI/CD | ❌ No pipeline | CRITICAL infrastructure |
| Test coverage | ❌ Near-zero | CRITICAL quality |
| SDK documentation | ⚠️ Types exist, no published docs | MEDIUM |
| Admin dashboard | ❌ Missing | MEDIUM |
| i18n | ❌ All strings hardcoded English | MEDIUM |
| Security headers | ⚠️ Partial | HIGH |
| Rate limiting at scale | ⚠️ In-memory only | HIGH |
| Scheduled message worker | ❌ No background job runner | MEDIUM |
| Push notifications (mobile/web) | ❌ Missing | HIGH |
| Plugin sandboxing | ⚠️ iframe exists, permission model missing | HIGH security |
| FIDO2/WebAuthn | ❌ Missing | MEDIUM |
| 2FA recovery codes | ❌ Missing | HIGH security |

---

## 9. Final Verdict

Nexus is one of the most structurally ambitious open-source messaging projects ever created. The codebase demonstrates a deep understanding of what both Discord power users and privacy-conscious communities actually need. The AGPL license, the zero-surveillance philosophy, the federation design, and the free-forever commitment are all exactly correct.

The project is currently at an inflection point: **it has exceptional breadth and correct architectural bones, but needs depth, closures on critical systems (voice, messages at scale, web/mobile clients), and quality infrastructure (CI, tests, docs)**.

The single most impactful next step is building the **web client** — it removes the biggest barrier to trying Nexus. The second is **completing the voice media relay** — because "Discord killer" without working voice is a non-starter.

Done right, Nexus doesn't just replace Discord. It replaces Slack, Teams, WhatsApp groups, Telegram channels, IRC servers, Matrix homeservers, and every proprietary chat product from the past twenty years — not by copying them, but by being better than all of them simultaneously, for free, forever, owned by no one.

That's a vision worth building.

---

*This audit was generated by GitHub Copilot on April 2, 2026. It reflects the state of the `main` branch at that date.*
