# Nexus — The Discord Killer

## Codename: `discordkiller` → Product Name: **Nexus**

## Vision

A privacy-first, community-owned communication platform that carries all of Discord's strengths (servers, channels, voice/video, bots, rich UX) and none of its weaknesses (surveillance, mandatory ID, data harvesting, enshittification). Built for the 2026 exodus.

## Why Now

- Discord's March 2026 "teen-by-default" age assurance rollout requiring government ID / facial estimation
- 2025 data breach exposed ~70K users' IDs and selfies — trust is destroyed
- Pre-IPO monetization pressure (Goldman Sachs / JP Morgan filing)
- Real user exodus — alternatives like Stoat overwhelmed with signups
- No single alternative has "won" — the window is wide open

## Core Principles

1. **Privacy by Default** — Zero ID requirements, E2EE for DMs/voice, minimal data collection, no telemetry without explicit opt-in
2. **User Sovereignty** — Self-hostable, federated, your data is YOUR data, export everything anytime
3. **Discord-Grade UX** — Not "almost as good" — actually good. Servers, channels, roles, bots, rich embeds, screen share, low-latency voice
4. **Community-Driven** — Open source (AGPL-3.0), community governance, transparent roadmap
5. **Migration-First** — Import Discord history/channels/roles, Matrix bridges, gradual transition path
6. **Sustainable Monetization** — Optional cosmetics, hosting services, no ads, no data selling, ever

## What Users Want (That Discord Never Delivered)

- True E2E encryption for DMs and optional for channels
- Self-hosting without enterprise pricing
- Offline message queue (send when reconnected)
- Per-channel notification granularity (not just mute/unmute)
- Built-in polls, scheduling, event planning
- Native markdown with live preview
- Code collaboration (shared snippets with syntax highlighting, live code blocks)
- Proper thread management (not Discord's half-baked threads)
- User-controlled algorithms (no "suggested servers," no dark patterns)
- Plugin/extension system (not just bots — client-side customization)
- Voice channel features: noise suppression, spatial audio, recording with consent
- Screen share at 1080p60 without Nitro
- File sharing without arbitrary limits (configurable by server admin)
- Better search (full-text, filters, saved searches)
- Vanity without payment (custom profiles, themes)

## Tech Stack Decision

| Layer | Technology | Why |
|-------|-----------|-----|
| Backend API | **Rust (Axum)** | Memory-safe, blazing fast, zero-cost abstractions, perfect for real-time |
| Real-time Gateway | **Rust (Tokio + Tungstenite)** | Native async, handles millions of concurrent WebSocket connections |
| Voice/Media | **Rust (WebRTC via str0m/webrtc-rs)** | Low-latency, SFU architecture, no GC pauses |
| Database | **PostgreSQL + ScyllaDB** | Postgres for relational (users, servers), ScyllaDB for messages (write-heavy, time-series) |
| Cache | **Redis (DragonflyDB compatible)** | Presence, sessions, rate limiting, pub/sub |
| Search | **MeiliSearch** | Typo-tolerant full-text search, fast, self-hostable |
| File Storage | **S3-compatible (MinIO)** | Self-hostable object storage for attachments |
| Frontend Desktop | **Tauri 2 + React + TypeScript** | Native performance, small binary, cross-platform |
| Frontend Web | **React + TypeScript (Vite)** | Shared codebase with desktop |
| Frontend Mobile | **React Native** | Code sharing with web, native feel |
| E2E Encryption | **Signal Protocol (libsignal)** | Gold standard, double ratchet, forward secrecy |
| Federation | **Matrix-compatible protocol** | Interop with existing Matrix ecosystem |
| Bot API | **REST + WebSocket + SDK (TS/Python/Rust)** | Discord-compatible API shape for easy bot migration |

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        CLIENTS                                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
│  │  Desktop  │  │   Web    │  │  Mobile  │  │   Bot    │       │
│  │  (Tauri)  │  │ (React)  │  │  (RN)    │  │  (SDK)   │       │
│  └─────┬─────┘  └─────┬────┘  └─────┬────┘  └─────┬────┘       │
│        │              │              │              │             │
│        └──────────────┴──────────────┴──────────────┘             │
│                          │                                        │
├──────────────────────────┼────────────────────────────────────────┤
│                   API GATEWAY (Rust)                              │
│         ┌────────────────┼────────────────┐                      │
│         │                │                │                      │
│   ┌─────▼─────┐  ┌──────▼──────┐  ┌─────▼─────┐               │
│   │  REST API  │  │  WebSocket   │  │  Voice    │               │
│   │   (Axum)   │  │  Gateway     │  │  Server   │               │
│   │            │  │  (Tokio)     │  │  (WebRTC) │               │
│   └─────┬─────┘  └──────┬──────┘  └─────┬─────┘               │
│         │                │                │                      │
├─────────┼────────────────┼────────────────┼──────────────────────┤
│         │           SERVICES              │                      │
│   ┌─────▼──────────────────────────────────▼─────┐              │
│   │  Auth │ Users │ Servers │ Channels │ Messages │              │
│   │  Voice│ Media │ Search  │ Presence │ Federation│             │
│   └─────┬──────────────────────────────────┬─────┘              │
│         │                                  │                      │
├─────────┼──────────────────────────────────┼──────────────────────┤
│         │            DATA LAYER            │                      │
│   ┌─────▼─────┐  ┌────────┐  ┌───────┐  ┌▼────────┐           │
│   │ PostgreSQL │  │ScyllaDB│  │ Redis  │  │ MinIO   │           │
│   │ (metadata) │  │ (msgs) │  │(cache) │  │ (files) │           │
│   └───────────┘  └────────┘  └───────┘  └─────────┘           │
└─────────────────────────────────────────────────────────────────┘
```

## Milestones

- **v0.1 — Foundation** ✅: Project scaffold, DB schema, core API, auth, basic WebSocket
- **v0.2 — Chat MVP** ✅: Text channels, DMs, message CRUD, real-time delivery
- **v0.3 — Voice** ✅: WebRTC voice channels, basic mixing, mute/deafen
- **v0.4 — Rich Features** ✅: File upload, embeds, reactions, threads, search
- **v0.5 — E2EE** ✅: Signal protocol integration for DMs, opt-in for channels
- **v0.6 — Desktop Client** ✅: Tauri 2 app with full feature parity
- **v0.7 — Extensibility** ✅: Bot API, TypeScript/Python/Rust SDKs, sandboxed plugin system, custom theme engine
- **v0.8 — Federation** 🟡 (Current): Matrix-compatible federation protocol, S2S communication, federated identity, Matrix/Discord bridge
- **v0.9 — Mobile**: React Native client
- **v1.0 — Public Launch**: Self-hosted + managed hosting options
