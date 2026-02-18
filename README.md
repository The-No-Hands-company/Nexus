<div align="center">

# 🌀 Nexus

### The Discord Killer — Privacy-First, Community-Owned Communication

**No ID Required. No Surveillance. No Enshittification. Ever.**

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.84+-orange.svg)](https://www.rust-lang.org)
[![Status](https://img.shields.io/badge/Status-v0.8%20Federation-purple.svg)]()
[![v0.7](https://img.shields.io/badge/v0.7%20Extensibility-Complete-brightgreen.svg)]()

*Built in response to Discord's mandatory age verification, data breaches, and pre-IPO enshittification.*

</div>

---

## Why Nexus Exists

In March 2026, Discord began requiring government ID uploads, facial age estimation, or AI inference to unlock full platform features. This came after their 2025 data breach that exposed ~70,000 users' IDs and selfies. Combined with aggressive monetization, ads, and pre-IPO pressure (Goldman Sachs / JP Morgan), the platform that was once a chill pseudonymous hangout has become a surveillance tool.

**Nexus is what Discord should have been:**

- All of Discord's strengths (servers, channels, voice, bots, rich UX)
- Zero of its weaknesses (no ID, no surveillance, no data harvesting, no paywalled features)
- Plus everything users have been asking for but never got

## What Makes Nexus Different

| Feature | Discord (2026) | Nexus |
|---------|---------------|-------|
| Account creation | Email + phone + age verification | Username + password. That's it. |
| Government ID | Required for full features | **Never. Not now, not ever.** |
| E2E Encryption | None | DMs encrypted by default, opt-in for channels |
| Screen share quality | 720p (free) / 1080p60 (Nitro $10/mo) | **1080p60 for everyone** |
| File upload limit | 25MB (free) / 500MB (Nitro) | **Configurable by server admin** (default 100MB) |
| Self-hosting | Not possible | **First-class citizen** |
| Federation | Not possible | Matrix-compatible protocol |
| Custom themes | Against ToS | **Built-in theme engine** |
| Data export | Limited GDPR dump | **Full export, anytime, your data** |
| Source code | Proprietary | **Open Source (AGPL-3.0)** |
| Telemetry | Extensive, opt-out buried | **None by default, explicit opt-in** |
| Bot API | Proprietary, rate-limited | **Open, Discord-compatible shape for easy migration** |
| Algorithms | "Suggested servers", dark patterns | **Zero. No recommendations. No manipulation.** |

## Features Users Wanted But Never Got

These are pulled from years of Discord feedback forums, Reddit threads, and X posts:

- **True E2E encryption** for DMs and optional per-channel
- **Proper thread management** (not Discord's bolted-on afterthought)
- **Per-channel notification granularity** (not just mute/unmute)
- **Built-in polls, scheduling, and event planning**
- **Native markdown with live preview**
- **Code collaboration** — syntax-highlighted snippets, shared code blocks
- **Better search** — full-text with filters, saved searches, typo tolerance
- **Voice recording** with visible consent indicator
- **Noise suppression and spatial audio** built in
- **Offline message queue** — messages send when you reconnect
- **Client-side plugins/extensions** — not just bots, actual customization
- **Custom profiles and themes** without paying
- **No arbitrary limits** behind a paywall

## Tech Stack

Built for performance, privacy, and developer happiness:

| Layer | Technology | Why |
|-------|-----------|-----|
| **Backend** | Rust (Axum + Tokio) | Memory-safe, zero-cost abstractions, handles millions of connections |
| **Gateway** | WebSocket (Tokio + Tungstenite) | Real-time events, typing indicators, presence |
| **Voice** | WebRTC SFU | Low-latency voice/video, screen share, no GC pauses |
| **Database** | PostgreSQL | Users, servers, channels, roles — battle-tested relational data |
| **Messages** | ScyllaDB | Write-heavy, time-series, partitioned by channel |
| **Cache** | Redis | Sessions, presence, rate limiting, pub/sub |
| **Search** | MeiliSearch | Typo-tolerant full-text search, self-hostable |
| **Storage** | S3/MinIO | Avatars, attachments, any S3-compatible backend |
| **Desktop** | Tauri 2 + React + TypeScript | Native performance, tiny binary, cross-platform |
| **Mobile** | React Native | Shared codebase with web, native feel |
| **Encryption** | Signal Protocol (libsignal) | Gold standard E2E, double ratchet, forward secrecy |

## Architecture

```
                    ┌─────────────────────────┐
                    │        CLIENTS          │
                    │  Desktop · Web · Mobile │
                    │        · Bots           │
                    └───────────┬─────────────┘
                                │
              ┌─────────────────┼─────────────────┐
              │                 │                  │
        ┌─────▼─────┐   ┌──────▼──────┐   ┌─────▼─────┐
        │  REST API  │   │  WebSocket  │   │   Voice   │
        │   :8080    │   │  Gateway    │   │   Server  │
        │   (Axum)   │   │   :8081     │   │   :8082   │
        └─────┬──────┘   └──────┬──────┘   └─────┬─────┘
              │                 │                  │
        ┌─────▼─────────────────▼──────────────────▼──┐
        │              SERVICE LAYER                   │
        │  Auth · Users · Servers · Channels · Msgs   │
        │  Roles · Members · Search · Presence · E2EE │
        └─────┬──────────┬──────────┬─────────┬───────┘
              │          │          │         │
        ┌─────▼───┐ ┌───▼────┐ ┌──▼───┐ ┌──▼──────┐
        │Postgres │ │ScyllaDB│ │Redis │ │  MinIO  │
        │(relat.) │ │(msgs)  │ │(cache)│ │(files)  │
        └─────────┘ └────────┘ └──────┘ └─────────┘
```

## Quick Start

### Prerequisites

- [Rust 1.84+](https://rustup.rs/)
- [Docker Compose](https://docs.docker.com/compose/install/)

### Development Setup

```bash
# Clone the repository
git clone https://github.com/The-No-hands-Company/nexus.git
cd nexus

# Start dependencies (Postgres, Redis, ScyllaDB, MinIO, MeiliSearch)
docker compose up -d

# Copy environment config
cp .env.example .env

# Run database migrations
cargo run --bin nexus -- migrate  # (or migrations run on startup)

# Start the server
cargo run --bin nexus

# Server is now running:
#   REST API:  http://localhost:8080
#   Gateway:   ws://localhost:8081
#   Voice:     ws://localhost:8082
```

### Docker Deployment

```bash
# Build
docker build -t nexus .

# Run (connect to your own Postgres/Redis/etc.)
docker run -d \
  --name nexus \
  -p 8080:8080 -p 8081:8081 -p 8082:8082 \
  -e NEXUS__DATABASE__URL=postgres://... \
  -e NEXUS__REDIS__URL=redis://... \
  -e NEXUS__AUTH__JWT_SECRET=$(openssl rand -hex 64) \
  nexus
```

## API Overview

### Authentication (No ID Required!)

```bash
# Register — just username + password. Email optional.
curl -X POST http://localhost:8080/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username": "alice", "password": "my_secure_password"}'

# Login
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "alice", "password": "my_secure_password"}'

# Response includes JWT tokens:
# { "user": {...}, "access_token": "...", "refresh_token": "...", "expires_in": 900 }
```

### Servers

```bash
# Create a server
curl -X POST http://localhost:8080/api/v1/servers \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"name": "My Gaming Community", "is_public": true}'

# Join a server
curl -X POST http://localhost:8080/api/v1/servers/<id>/join \
  -H "Authorization: Bearer <token>"
```

### WebSocket Gateway

```javascript
// Connect to real-time gateway
const ws = new WebSocket('ws://localhost:8081/gateway');

// Authenticate
ws.send(JSON.stringify({ op: 'Identify', d: { token: 'your_jwt' } }));

// Receive real-time events
ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  // { op: 'Dispatch', d: { event: 'MESSAGE_CREATE', data: {...} } }
};
```

## Project Structure

```
nexus/
├── Cargo.toml                    # Workspace root
├── docker-compose.yml            # Development dependencies
├── Dockerfile                    # Production container
├── .env.example                  # Configuration template
├── .github/workflows/ci.yml     # CI pipeline
│
├── crates/
│   ├── nexus-common/             # Shared types, config, errors, models
│   │   └── src/
│   │       ├── config.rs         # App configuration
│   │       ├── error.rs          # Error types (HTTP-friendly)
│   │       ├── models/           # Domain models (User, Server, Channel, etc.)
│   │       ├── permissions.rs    # Bitfield permission system
│   │       ├── snowflake.rs      # UUID v7 ID generation
│   │       └── validation.rs     # Input validation
│   │
│   ├── nexus-db/                 # Database layer
│   │   ├── migrations/           # SQL migrations
│   │   └── src/
│   │       ├── repository/       # Query functions (users, servers, channels, etc.)
│   │       ├── postgres.rs       # PostgreSQL helpers
│   │       └── redis_pool.rs     # Redis helpers
│   │
│   ├── nexus-api/                # REST API
│   │   └── src/
│   │       ├── auth.rs           # JWT + Argon2 auth
│   │       ├── middleware.rs     # Auth extraction, rate limiting
│   │       └── routes/           # HTTP endpoints
│   │
│   ├── nexus-gateway/            # WebSocket real-time gateway
│   │   └── src/
│   │       ├── events.rs         # Event types
│   │       └── session.rs        # Session management
│   │
│   ├── nexus-voice/              # Voice/Video WebRTC server
│   │   └── src/
│   │       ├── room.rs           # Voice room management
│   │       └── signaling.rs      # WebRTC signaling
│   │
│   ├── nexus-federation/         # v0.8 Federation (Matrix-compatible S2S)
│   │   └── src/
│   │       ├── types.rs          # Federated event shapes
│   │       ├── keys.rs           # Ed25519 signing keys
│   │       ├── signatures.rs     # Request signing & verification
│   │       ├── client.rs         # S2S HTTP client
│   │       ├── discovery.rs      # .well-known resolver
│   │       └── matrix_bridge.rs  # Matrix AS bridge protocol
│   │
│   ├── nexus-desktop/            # v0.6 Desktop client (Tauri 2 + React)
│   │   ├── src/                  # React/TypeScript frontend
│   │   │   ├── themes/           # Built-in theme engine (4 themes)
│   │   │   ├── plugins/          # Client plugin system (sandboxed iframes)
│   │   │   ├── pages/            # App pages (channels, settings, etc.)
│   │   │   └── components/       # Reusable UI components
│   │   └── src-tauri/            # Rust Tauri backend
│   │
│   └── nexus-server/             # Main binary (orchestrates everything)
│       └── src/main.rs
│
├── packages/
│   ├── nexus-sdk/                # v0.7 TypeScript Bot SDK (@nexus/sdk)
│   ├── nexus-sdk-py/             # v0.7 Python Bot SDK (nexus-sdk)
│   └── nexus-sdk-rs/             # v0.7 Rust Bot SDK (nexus-sdk)
│
└── .planning/                    # Development planning docs
    ├── BRIEF.md                  # Project vision & architecture
    └── ROADMAP.md                # Development phases
```

## Roadmap

| Version | Status | Focus |
|---------|--------|-------|
| **v0.1** | ✅ Complete | Foundation — scaffold, DB, auth, basic API & gateway |
| **v0.2** | ✅ Complete | Chat MVP — messages, DMs, real-time, typing, reactions |
| **v0.3** | ✅ Complete | Voice — WebRTC SFU, mute/deafen, screen share |
| **v0.4** | ✅ Complete | Rich Features — files, embeds, threads, search, emoji |
| **v0.5** | ✅ Complete | E2E Encryption — Signal protocol for DMs + opt-in channels |
| **v0.6** | ✅ Complete | Desktop Client — Tauri 2 app with full feature parity |
| **v0.7** | ✅ Complete | Extensibility — Bot API, TypeScript/Python/Rust SDKs, plugin system, custom themes |
| **v0.8** | 🟡 In Progress | Federation — Matrix-compatible server-to-server protocol |
| **v0.9** | ⚪ Planned | Mobile — React Native iOS + Android |
| **v1.0** | ⚪ Planned | Public Launch — managed hosting + self-host docs |

## Contributing

Nexus is open source under AGPL-3.0. We welcome contributions!

```bash
# Fork, clone, and create a branch
git checkout -b feature/my-feature

# Make your changes, then:
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all

# Submit a PR
```

### Development Philosophy

- **Privacy is non-negotiable** — Never add tracking, telemetry, or ID requirements
- **Performance matters** — Rust isn't just for fun; every millisecond counts in voice/real-time
- **User respect** — No dark patterns, no manipulation, no algorithms
- **Sustainability over growth** — We'd rather have 10K happy users than 10M surveilled ones

## License

**AGPL-3.0-or-later** — This means:

- You can use, modify, and distribute Nexus freely
- If you run a modified version as a service, you must share your source code
- This prevents corporate capture and enshittification

Why AGPL? Because we watched what happened to every other chat platform that went proprietary. Not this time.

---

<div align="center">

**Built with 🦀 Rust and righteous anger at what Discord became.**

*The No-Hands Company · 2026*

</div>
