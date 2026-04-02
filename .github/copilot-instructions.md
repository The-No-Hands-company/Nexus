# Copilot Instructions for Nexus

You are working in the Nexus monorepo (Rust-first, privacy-first, self-hostable).

## Project Context

Nexus is a self-hostable, federated, privacy-first communication platform (Discord alternative) built in a Rust Cargo workspace monorepo. 

**Core Crates**:
- `nexus-common`: Shared types, error handling, validation, auth primitives
- `nexus-db`: PostgreSQL/SQLite persistence layer with repository abstractions; Redis for caching, presence, and pub/sub
- `nexus-api`: Axum HTTP API, routes, middleware, and request/response handling
- `nexus-gateway`: WebSocket fanout for real-time events (Redis-backed pub/sub)
- `nexus-voice`: WebRTC signaling and voice channel management
- `nexus-federation`: Matrix-compatible federation for cross-instance communication
- `nexus-desktop` (TypeScript/Tauri): Desktop client for Windows, macOS, Linux
- `nexus-sdk` (TypeScript/React), `nexus-sdk-py`, `nexus-sdk-rs`: Official SDKs for client development and integration

**Clients**: Tauri/React desktop, React Native mobile, browser-based. SDKs maintain TypeScript models and Rust types that sync with backend API shapes.

For broader context, refer to [CONTRIBUTING.md](../../CONTRIBUTING.md), crate-level docs, and federation specs in [docs/federation.md](../../docs/federation.md).

## Core Product Principles
- Privacy first: do not introduce centralized collection of legal identity, government ID, or unnecessary personal data.
- Nexus-native direction: avoid framing features as Discord-dependent or requiring external lock-in.
- Security and trust: prefer explicit validation, least privilege, and auditable actions.

## Repository Workflow
- Keep changes scoped and minimal for the requested task.
- Do not revert unrelated user changes.
- If unexpected modifications appear, stop and ask before proceeding.
- Prefer small, clear commits with descriptive messages.

## Rust and API Conventions
- Keep code warning-free (`cargo check` clean for touched crates).
- Use existing repository patterns for errors (`NexusError`) and route responses.
- Reuse repository-layer helpers before adding ad-hoc SQL in routes.
- For performance-sensitive paths, avoid N+1 DB queries and prefer batching.

## Scylla and SQL Hybrid Expectations
- Treat SQL as source of truth unless a path is explicitly Scylla-backed.
- In Scylla canary/prefer reads, preserve API response parity with SQL shape.
- Add telemetry for fallback and hydration behavior when touching Scylla paths.

### Database Nuances

**SQL (PostgreSQL/SQLite)**:
- Messages, users, channels, servers, relationships, and all state live here.
- All mutations flow through SQL first (outbox pattern for Scylla replication).
- Batch repository helpers (e.g., `messages::find_by_ids_map()`, `reactions::get_reaction_counts_for_messages()`) eliminate N+1 queries.

**Scylla (Optional, Read-Optimized)**:
- Replicates message content asynchronously via outbox bridge for scale-out reads.
- Strategies: `off` (SQL-only, default), `canary` (try Scylla, fall back to SQL on error), `prefer` (prefer Scylla, SQL only on error).
- Eventual consistency: metadata (embeds, attachments, thread_id, reaction counts) hydrated from SQL when available for response parity.
- Partitioning: Messages keyed by `(server_id, channel_id, created_at)` for efficient range queries by time.

**Redis**:
- **Caching**: User sessions, presence state, typing indicators, temporary data.
- **Pub/Sub**: Real-time event fanout for WebSocket gateway (server/channel scoped subscriptions).
- **Presence**: User online/offline state and activity status.
- When: Cache expensive queries (e.g., user permissions, channel metadata) with TTL; use pub/sub for real-time updates to clients; avoid storing durable state that isn't recoverable from SQL.

## Safety and Compatibility
- Preserve backward compatibility for public APIs unless explicitly asked to break it.
- Validate authorization and membership boundaries on server/channel scoped endpoints.
- Prefer feature flags for capabilities that depend on incomplete backend components.

## Federation and Clients
- **Federation changes**: Preserve Matrix-compatible protocol invariants and Ed25519 signing patterns. Cross-instance communication must remain cryptographically sound.
- **Client code**: When touching TypeScript models or Rust types used by SDKs, maintain alignment between backend and client definitions. Shared fixtures and type generators help keep them in sync.

## Error Handling and Telemetry
- Prefer structured logging via existing `tracing` patterns over ad-hoc prints.
- Use `NexusError` for domain errors; provide clear context in error responses.
- Add observability (metrics counters, debug logs) for fallback paths, batch lookup outcomes, and non-deterministic behavior.
- Avoid silent failures: if a non-critical operation (e.g., metadata hydration, cache write) fails, log at debug level with context; do not degrade API response without explicit visibility.

## Overarching Rule
**Default to the least invasive change that respects privacy, performance, and existing architecture.**
When in doubt, prefer:
- Smaller scope over broader refactoring.
- Extending existing patterns over creating new ones.
- Metadata hydration or batch ops over N+1 query loops.
- Feature flags over runtime errors for incomplete features.
- Explicit validation over silent degradation.

## Validation Before Finish
- Run targeted checks for changed crates (for example: `cargo check -p nexus-api`).
- If configuration or docs are changed, update relevant deployment docs.
- Summarize what changed, why, and any follow-up risks.

### Testing and Tooling

**Rust**:
- `cargo test --package <changed-crate>` for unit and integration tests.
- `cargo clippy --all-targets -- -D warnings` to catch lints; the repo enforces this in CI.
- `cargo fmt` ensures code style consistency.

**Frontend** (if changes touch client code):
- TypeScript SDK changes: ensure models sync with backend API response shapes (shared fixture in `nexus-sdk/src/`).
- Tauri desktop: run `npm run build` in `nexus-desktop/src-tauri` and test on Windows/macOS/Linux if runtime platform flags are added.
- React Native: validate schema compatibility for shared navigation or auth state.

**Existing CI**:
- The repository enforces `cargo fmt`, `cargo clippy -D warnings`, and `cargo test --all-features` for all Rust code.
- Pull requests will not merge without passing checks.

