//! Server discovery — resolves a bare server name to a reachable HTTPS base URL.
//!
//! Resolution order (mirrors Matrix SRV + well-known spec):
//!
//! 1. **IP literal / explicit port** — `server:8448` → use as-is
//! 2. **`.well-known/nexus/server`** — GET `https://<name>/.well-known/nexus/server`
//!    If found, follow the delegated server name.
//! 3. **Direct HTTPS fallback** — `https://<name>:8448`
//!
//! Results are cached in memory with a 24-hour TTL.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::RwLock;
use tracing::debug;

use crate::{error::FederationError, types::WellKnownServer};

/// Default federation port (analogous to Matrix port 8448).
const DEFAULT_FED_PORT: u16 = 8448;

/// How long to cache a resolved base URL before re-resolving.
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

// ─── Cache ───────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct CacheEntry {
    base_url: String,
    resolved_at: Instant,
}

/// In-memory cache for resolved server base URLs.
///
/// Thread-safe, suitable for sharing across `FederationClient` instances via `Arc`.
#[derive(Debug, Clone)]
pub struct DiscoveryCache {
    inner: Arc<RwLock<HashMap<String, CacheEntry>>>,
    http: reqwest::Client,
}

impl DiscoveryCache {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent(concat!("Nexus-Federation/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build discovery http client");
        Self { inner: Arc::new(RwLock::new(HashMap::new())), http }
    }

    /// Resolve `server_name` to an HTTPS base URL, using cache if valid.
    pub async fn resolve(&self, server_name: &str) -> Result<String, FederationError> {
        // Check cache.
        {
            let cache = self.inner.read().await;
            if let Some(entry) = cache.get(server_name) {
                if entry.resolved_at.elapsed() < CACHE_TTL {
                    debug!("Discovery cache hit: {} → {}", server_name, entry.base_url);
                    return Ok(entry.base_url.clone());
                }
            }
        }

        // Resolve.
        let base_url = self.do_resolve(server_name).await?;

        // Populate cache.
        {
            let mut cache = self.inner.write().await;
            cache.insert(
                server_name.to_owned(),
                CacheEntry { base_url: base_url.clone(), resolved_at: Instant::now() },
            );
        }

        Ok(base_url)
    }

    /// Invalidate cache for a server (e.g. after a connection failure).
    pub async fn invalidate(&self, server_name: &str) {
        self.inner.write().await.remove(server_name);
    }

    // ── Resolution logic ─────────────────────────────────────────────────────

    async fn do_resolve(&self, server_name: &str) -> Result<String, FederationError> {
        // Step 1: explicit port — try HTTPS first, fall back to HTTP.
        // This handles LAN / dev setups (192.168.x.x:8080) that don't have TLS.
        if has_explicit_port(server_name) {
            let https_base = format!("https://{}", server_name);
            if self.probe_base_url(&https_base).await {
                debug!("Discovery (explicit port, HTTPS): {} → {}", server_name, https_base);
                return Ok(https_base);
            }
            let http_base = format!("http://{}", server_name);
            debug!("Discovery (explicit port, HTTP fallback): {} → {}", server_name, http_base);
            return Ok(http_base);
        }

        // Step 2: try .well-known (HTTPS then HTTP).
        if let Some(base) = self.try_well_known(server_name).await {
            debug!("Discovery (well-known): {} → {}", server_name, base);
            return Ok(base);
        }

        // Step 3: localhost / bare IP without port — assume local HTTP on API port.
        if is_local_address(server_name) {
            let base = format!("http://{}:8080", server_name);
            debug!("Discovery (local fallback): {} → {}", server_name, base);
            return Ok(base);
        }

        // Step 4: HTTPS on standard federation port.
        let base = format!("https://{}:{}", server_name, DEFAULT_FED_PORT);
        debug!("Discovery (HTTPS fallback): {} → {}", server_name, base);
        Ok(base)
    }

    /// Quick connectivity probe — returns `true` if the base URL is reachable.
    /// Uses a lightweight GET on `/.well-known/nexus/server`; a 4xx is still
    /// "reachable", only connection errors return `false`.
    async fn probe_base_url(&self, base_url: &str) -> bool {
        let url = format!("{}/.well-known/nexus/server", base_url);
        match self.http.get(&url).send().await {
            Ok(_) => true,
            Err(e) if e.is_connect() || e.is_timeout() => false,
            Err(_) => true, // other errors mean server responded somehow
        }
    }

    async fn try_well_known(&self, server_name: &str) -> Option<String> {
        // Try HTTPS first, then HTTP (for servers behind a proxy or with TLS).
        for scheme in &["https", "http"] {
            let url = format!("{}://{}/.well-known/nexus/server", scheme, server_name);
            if let Ok(resp) = self.http.get(&url).send().await {
                if resp.status().is_success() {
                    if let Ok(wk) = resp.json::<WellKnownServer>().await {
                        // ── SSRF guard ────────────────────────────────────────
                        // The delegated server in the well-known response MUST NOT
                        // point at a private/loopback address. An attacker on a
                        // public server could otherwise redirect federation requests
                        // to localhost or internal services.
                        let delegated_host = wk.server.split(':').next().unwrap_or(&wk.server);
                        if is_private_or_loopback(delegated_host) {
                            tracing::warn!(
                                server = server_name,
                                delegated = %wk.server,
                                "SSRF guard: rejected well-known delegation to private address"
                            );
                            return None;
                        }
                        // Follow the delegated server name.
                        let resolved = if has_explicit_port(&wk.server) {
                            // Preserve whatever scheme was reachable.
                            format!("{}://{}", scheme, wk.server)
                        } else {
                            format!("https://{}:{}", wk.server, DEFAULT_FED_PORT)
                        };
                        return Some(resolved);
                    }
                }
            }
        }
        None
    }
}

impl Default for DiscoveryCache {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

pub fn has_explicit_port(server_name: &str) -> bool {
    // IPv6 literal with port: [::1]:8448
    if server_name.starts_with('[') {
        return server_name.contains("]:"); // [host]:port
    }
    // hostname:port — but ignore IPv6 with extra colons.
    let colon_count = server_name.chars().filter(|&c| c == ':').count();
    colon_count > 0 && colon_count < 2
}

/// Returns `true` for localhost, loopback, RFC-1918, link-local, and
/// other non-routable addresses. Used as an SSRF guard on well-known delegation
/// responses and for local-dev HTTP scheme selection.
///
/// Covers all ranges that should never be reachable via public federation:
/// - Loopback: 127.0.0.0/8, ::1
/// - RFC-1918: 10/8, 172.16/12, 192.168/16
/// - Link-local: 169.254/16, fe80::/10
/// - ULA IPv6: fc00::/7
/// - Metadata: 169.254.169.254 (AWS/GCP/Azure IMDS)
fn is_local_address(server_name: &str) -> bool {
    is_private_or_loopback(server_name.split(':').next().unwrap_or(server_name))
}

fn is_private_or_loopback(host: &str) -> bool {
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if matches!(host, "localhost" | "ip6-localhost" | "ip6-loopback") {
        return true;
    }
    // IPv6 loopback
    if host == "::1" || host == "0:0:0:0:0:0:0:1" {
        return true;
    }
    // IPv4 loopback 127.0.0.0/8
    if host.starts_with("127.") {
        return true;
    }
    // Unspecified / broadcast
    if host == "0.0.0.0" || host == "255.255.255.255" {
        return true;
    }
    // RFC-1918
    if host.starts_with("10.") || host.starts_with("192.168.") {
        return true;
    }
    if let Some(rest) = host.strip_prefix("172.") {
        if let Some(octet) = rest.split('.').next() {
            if let Ok(n) = octet.parse::<u8>() {
                if (16..=31).contains(&n) {
                    return true;
                }
            }
        }
    }
    // Link-local IPv4 (includes cloud metadata 169.254.169.254)
    if host.starts_with("169.254.") {
        return true;
    }
    // Link-local IPv6 fe80::/10 and ULA fc00::/7
    let lower = host.to_lowercase();
    if lower.starts_with("fe80:")
        || lower.starts_with("fc")
        || lower.starts_with("fd")
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{has_explicit_port, is_local_address};

    #[test]
    fn explicit_port_detection() {
        assert!(has_explicit_port("nexus.example.com:8448"));
        assert!(!has_explicit_port("nexus.example.com"));
        assert!(has_explicit_port("[::1]:8448"));
        assert!(!has_explicit_port("::1")); // bare IPv6 — no port
    }

    #[test]
    fn local_address_detection() {
        assert!(is_local_address("localhost"));
        assert!(is_local_address("127.0.0.1"));
        assert!(is_local_address("192.168.0.179"));
        assert!(is_local_address("10.0.0.1"));
        assert!(is_local_address("172.16.0.1"));
        assert!(!is_local_address("nexus.example.com"));
        assert!(!is_local_address("8.8.8.8"));
    }
}
