# Nexus — Development Roadmap

## Phase 1: Foundation (v0.1)

### 01-01: Project Scaffold & Configuration

- Rust workspace setup (Cargo workspaces)
- Package structure (api, gateway, voice, common, migration)
- Docker Compose for dev dependencies (Postgres, Redis, ScyllaDB, MinIO, MeiliSearch)
- Environment configuration (.env, config.toml)
- CI pipeline (GitHub Actions)

### 01-02: Database Schema & Migrations

- User accounts (email/password, OAuth stubs)
- Servers (guilds), channels, roles, permissions
- Messages table (ScyllaDB schema)
- Session management
- Run migrations via sqlx

### 01-03: Authentication System

- Registration (email + password, argon2 hashing)
- Login (JWT access + refresh tokens)
- Session management (Redis-backed)
- Rate limiting
- Password reset flow
- OAuth2 stubs (GitHub, Google — no mandatory ID)

### 01-04: Core REST API

- User CRUD (profile, settings, avatar)
- Server CRUD (create, update, delete, join, leave)
- Channel CRUD (text, voice, category)
- Role & permission system
- Invite system (codes, links, expiry)

### 01-05: WebSocket Gateway (Basic)

- Connection lifecycle (identify, heartbeat, resume)
- Event dispatch (message_create, presence_update, typing_start)
- Session state management
- Reconnection / resume protocol

## Phase 2: Chat MVP (v0.2)

- Message send/edit/delete with real-time propagation
- DM channels (1:1 and group)
- Message history with pagination
- Typing indicators
- Read state tracking
- Basic embeds (link previews)
- Emoji reactions

## Phase 3: Voice (v0.3)

- WebRTC SFU (Selective Forwarding Unit) architecture
- Voice channel join/leave/move
- Opus codec, noise suppression
- Mute/deafen/server mute
- Voice activity detection
- Screen share (VP9)
- Recording with consent indicators

## Phase 4: Rich Features (v0.4)

- File upload to S3/MinIO (images, video, documents)
- Rich embeds (media, code blocks, previews)
- Threads (proper implementation, not Discord's afterthought)
- Full-text search (MeiliSearch integration)
- Pinned messages
- Reactions with custom emoji
- Server emoji management
- User presence (online, idle, DND, invisible, custom status)

## Phase 5: Encryption (v0.5)

- Signal Protocol for DMs (double ratchet, X3DH key exchange)
- Opt-in E2EE for channels
- Key management UI
- Device verification
- Encrypted file attachments

## Phase 6: Desktop Client (v0.6)

- Tauri 2 application shell
- Full feature parity with web
- System tray, notifications
- Push-to-talk global hotkey
- Auto-update mechanism
- Overlay mode (gaming)

## Phase 7: Extensibility (v0.7)

- Bot API (REST + WebSocket, Discord-compatible shape)
- Bot SDK (TypeScript, Python, Rust)
- Client plugin system (sandboxed)
- Custom themes (CSS + theme API)
- Webhooks
- Slash commands

## Phase 8: Federation (v0.8)

- Matrix-compatible federation protocol
- Server-to-server communication
- Federated identity
- Bridge to Matrix/Discord
- Discovery & directory

## Phase 9: Mobile (v0.9)

- React Native iOS + Android
- Push notifications (FCM/APNs, self-hosted option via UnifiedPush)
- Voice/video on mobile
- Offline message queue

## Phase 10: Launch (v1.0)

- Managed hosting (nexus.chat or similar)
- Self-host documentation & one-click deploy
- Security audit
- Performance benchmarks
- Community governance setup
