# Workspace Guidance

- Treat Nexus as a production-grade application at all times.
- Prefer real, durable fixes over shortcuts, quick fixes, or temporary workarounds.
- Do not remove dependencies just to make a build pass unless the dependency is genuinely unnecessary and the removal is a deliberate product decision.
- When something breaks, fix the root cause and verify the result.
- Keep implementation quality high: correct behavior, stable builds, maintainable code, and minimal technical debt.
- If a change affects shared infrastructure, verify downstream app behavior after the fix.
- Follow ../docs/ENGINEERING_STANDARDS.md as the ecosystem-wide baseline.
- Rust validation target: cargo fmt --all --check && cargo lint.

---

## Project Overview

**Nexus** — The Discord Killer. Privacy-first, community-owned communication platform.
- Version: v0.14 (Phase 8 Federation UX complete, Phase 10 Mobile is top priority)
- License: AGPL-3.0
- Language: Rust (backend) + TypeScript/React (desktop/mobile)

### Crates (Rust backend)
- `nexus-api` — Axum REST API (port 8080)
- `nexus-gateway` — WebSocket real-time gateway (port 8081)
- `nexus-voice` — WebRTC SFU voice server (port 8082)
- `nexus-server` — Main server binary (combines api + gateway + voice)
- `nexus-db` — Database layer (PostgreSQL + ScyllaDB)
- `nexus-common` — Shared types and utilities
- `nexus-federation` — Federation protocol
- `nexus-desktop` — Tauri 2 + React desktop client (`crates/nexus-desktop/`)
- `packages/nexus-mobile` — React Native / Expo mobile client

### Dev Stack
- Docker/Podman for PostgreSQL, Redis, MeiliSearch, MinIO, ScyllaDB
- Rust 1.84+ with Cargo workspace
- Node 22 / npm / Bun for frontend builds
- `dev.sh` starts full stack (requires `nexus` binary in target/debug/)

### Running the App
```bash
# Full stack (requires Rust binary built)
./dev.sh

# Desktop only (mock mode, no backend)
cd crates/nexus-desktop && npm run dev

# Mobile only
cd packages/nexus-mobile && npm start
```

### Key Files
- `store.ts` (desktop) — 2100+ line Zustand store, full application state
- `useGateway.ts` (desktop) — WebSocket gateway hook with reconnect logic
- `MainLayout.tsx` (desktop) — Main app shell with ServerList + ChannelList + ChatView
- `ChatView.tsx` (desktop) — Message list with virtualized rendering, reactions, embeds
- `lib/api.ts` (mobile) — REST API client
- `lib/store.ts` (mobile) — Zustand-compatible store
- `lib/gateway.ts` (mobile) — WebSocket gateway client

### Current Priorities (Phase 10 — Mobile)
1. **Mobile client completion** — The mobile app has all screens but needs full API wired up
2. **Desktop → main layout bug** — Server list selection/join flow needs testing
3. **Missing `server/[id].tsx` tab** in mobile — Server screen exists but no channel list / server detail tab in bottom nav
4. **Voice/WebRTC** — Both mobile VoiceCallScreen and desktop VoiceChannel need real WebRTC
5. **Push notifications** — Mobile expo-notifications not yet wired to store

### Common Issues
- Desktop `MainLayout` crash on first load: Check `session` + `useGateway` initialization order
- Mobile API calls fail: `DEFAULT_API_BASE` is `http://localhost:8080` — needs config for production
- `gateway.sendTyping` sends wrong opcode — should be `OP_HEARTBEAT` (1) not `OP_DISPATCH` (0)
- Mobile `store.sendMessage` uses `channelId` instead of `activeChannelId` in typing timer ref
