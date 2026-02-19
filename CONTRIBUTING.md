# Contributing to Nexus

Thank you for your interest in contributing! This document outlines the process for reporting issues, proposing features, and submitting pull requests.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How to Contribute](#how-to-contribute)
  - [Reporting Bugs](#reporting-bugs)
  - [Suggesting Features](#suggesting-features)
  - [Submitting Pull Requests](#submitting-pull-requests)
- [Development Setup](#development-setup)
- [Coding Standards](#coding-standards)
- [Commit Messages](#commit-messages)
- [Security Vulnerabilities](#security-vulnerabilities)

---

## Code of Conduct

This project adheres to our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you agree to abide by it.

---

## Getting Started

1. **Fork** the repository and clone your fork:
   ```bash
   git clone https://github.com/<your-username>/discordkiller.git
   cd discordkiller
   ```
2. Add the upstream remote:
   ```bash
   git remote add upstream https://github.com/The-No-hands-Company/discordkiller.git
   ```
3. Follow the [Self-Hosting Guide](docs/self-hosting.md) to get a local development environment running.
4. Run the test suite to make sure everything passes before you make changes:
   ```bash
   cargo test --workspace
   ```

---

## How to Contribute

### Reporting Bugs

- Search [existing issues](https://github.com/The-No-hands-Company/discordkiller/issues) first to avoid duplicates.
- Open a [Bug Report](https://github.com/The-No-hands-Company/discordkiller/issues/new?template=bug_report.yml) using the provided template.
- Include the Nexus version, OS, steps to reproduce, expected vs actual behaviour, and relevant logs.

### Suggesting Features

- Check the [roadmap](ROADMAP.md) to see if the feature is already planned.
- Open a [Feature Request](https://github.com/The-No-hands-Company/discordkiller/issues/new?template=feature_request.yml) and describe the problem it solves.
- Discussion is welcome before investing significant effort.

### Submitting Pull Requests

1. Create a feature branch from `main`:
   ```bash
   git checkout -b feat/my-feature
   ```
2. Make your changes with small, focused commits (see [Commit Messages](#commit-messages)).
3. Add or update tests where appropriate.
4. Run the full test suite:
   ```bash
   cargo test --workspace
   cargo clippy --workspace -- -D warnings
   cargo fmt --check
   cargo deny check
   ```
5. Push your branch and open a PR using the [PR template](.github/PULL_REQUEST_TEMPLATE.md).
6. A maintainer will review your PR. Please respond to feedback promptly.

**Branch naming conventions:**

| Type | Pattern | Example |
|------|---------|---------|
| Feature | `feat/<slug>` | `feat/voice-noise-cancellation` |
| Bug fix | `fix/<slug>` | `fix/typo-in-auth-error` |
| Documentation | `docs/<slug>` | `docs/k8s-helm-guide` |
| Refactor | `refactor/<slug>` | `refactor/extract-jwt-helper` |
| Chore | `chore/<slug>` | `chore/bump-sqlx` |

---

## Development Setup

### Prerequisites

| Tool | Minimum version |
|------|----------------|
| Rust (stable) | 1.80 |
| Docker + Docker Compose | 24 |
| PostgreSQL | 16 (via Docker) |
| Redis | 7 (via Docker) |

### Quick start

```bash
# Start infrastructure
docker compose up -d postgres redis

# Copy and edit env vars
cp .env.template .env

# Run migrations
cargo run -p nexus-db --example migrate

# Start the server
cargo run -p nexus-server
```

### Running tests

```bash
# Unit + integration
cargo test --workspace

# Single crate
cargo test -p nexus-api

# With output
cargo test --workspace -- --nocapture
```

---

## Coding Standards

- **Formatting**: `cargo fmt` (enforced in CI).
- **Linting**: `cargo clippy -- -D warnings` (enforced in CI).
- **Error handling**: use `NexusError` from `nexus-common`; avoid `unwrap()` in library code.
- **Async**: prefer `tokio`; keep futures `Send + 'static` compatible.
- **Public API**: document every `pub` item with a doc comment.
- **Database**: migrations go in `crates/nexus-db/migrations/`; use `sqlx::query_as!` macros.

---

## Commit Messages

This project follows [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short summary>

[optional body]

[optional footer(s)]
```

**Types**: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`, `ci`

**Examples:**

```
feat(api): add rate-limiting middleware
fix(gateway): handle malformed WebSocket frames gracefully
docs(self-hosting): document Fly.io secrets injection
```

---

## Security Vulnerabilities

Please **do not** open public issues for security vulnerabilities. See [SECURITY.md](SECURITY.md) for responsible-disclosure instructions.
