# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.9.x   | ✅ Yes     |
| 0.8.x   | ✅ Yes     |
| < 0.8   | ❌ No      |

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

### Option 1 — GitHub Private Vulnerability Reporting (preferred)

Use [GitHub's private vulnerability reporting](https://github.com/The-No-hands-Company/discordkiller/security/advisories/new) to submit a confidential advisory. The maintainers will be notified immediately and can collaborate with you privately.

### Option 2 — Email

Send a detailed report to **security@nexus.chat** (PGP key available on [keys.openpgp.org](https://keys.openpgp.org)).

### What to include

- A clear description of the vulnerability and its potential impact
- Step-by-step reproduction instructions (proof-of-concept code if available)
- The affected component(s) and version(s)
- Your suggested severity (Critical / High / Medium / Low)
- Any proposed mitigations

## Response Timeline

| Stage | Target |
|-------|--------|
| Acknowledgement | 48 hours |
| Triage & severity assessment | 5 business days |
| Fix development | Varies by severity (see below) |
| Coordinated disclosure | 90 days from original report |

### Severity SLAs

| Severity | Fix target |
|----------|-----------|
| Critical (CVSS ≥ 9.0) | 7 days |
| High (CVSS 7.0–8.9) | 30 days |
| Medium (CVSS 4.0–6.9) | 60 days |
| Low (CVSS < 4.0) | 90 days |

## Disclosure Policy

We follow **coordinated disclosure**. We ask that you give us the time frames above to address the issue before any public disclosure. Once a fix is released, we will:

1. Publish a [GitHub Security Advisory](https://github.com/The-No-hands-Company/discordkiller/security/advisories)
2. Release a patched version on the same day
3. Add a changelog entry crediting the reporter (if desired)
4. Request a CVE if warranted

## Scope

### In scope

- Authentication and authorisation bypasses
- Remote code execution (RCE)
- SQL/NoSQL injection
- Cross-site scripting (XSS) in the web client
- Information disclosure of private user data
- Denial of service attacks on the server
- SSRF vulnerabilities in federation / media proxying
- Cryptographic weaknesses in federation key signing

### Out of scope

- Self-hosted deployments running behind misconfigured infrastructure that Nexus does not control
- Social engineering attacks targeting contributors
- Physical attacks
- Issues already known and documented in open GitHub issues

## Automated Dependency Scanning

This project uses:

- **[cargo-audit](https://crates.io/crates/cargo-audit)** — scans for known vulnerabilities in Rust dependencies (runs on every CI push via `cargo audit`)
- **[cargo-deny](https://crates.io/crates/cargo-deny)** — enforces licence and advisory policies on every CI push
- **GitHub Dependabot** — automated pull requests for dependency updates

## Acknowledgements

We sincerely thank all security researchers who responsibly disclose vulnerabilities. Contributors who report valid security issues will be credited in the advisory and in our `CONTRIBUTORS.md` (unless they prefer to remain anonymous).
