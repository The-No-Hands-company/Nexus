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
        // Step 1: if server_name includes a port, use it directly.
        if has_explicit_port(server_name) {
            let base = format!("https://{}", server_name);
            debug!("Discovery (explicit port): {} → {}", server_name, base);
            return Ok(base);
        }

        // Step 2: try .well-known.
        if let Some(base) = self.try_well_known(server_name).await {
            debug!("Discovery (well-known): {} → {}", server_name, base);
            return Ok(base);
        }

        // Step 3: fallback to direct HTTPS on default federation port.
        let base = format!("https://{}:{}", server_name, DEFAULT_FED_PORT);
        debug!("Discovery (fallback): {} → {}", server_name, base);
        Ok(base)
    }

    async fn try_well_known(&self, server_name: &str) -> Option<String> {
        let url = format!("https://{}/.well-known/nexus/server", server_name);
        let resp = self.http.get(&url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let wk: WellKnownServer = resp.json().await.ok()?;
        // Follow the delegated server name.
        if has_explicit_port(&wk.server) {
            Some(format!("https://{}", wk.server))
        } else {
            Some(format!("https://{}:{}", wk.server, DEFAULT_FED_PORT))
        }
    }
}

impl Default for DiscoveryCache {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn has_explicit_port(server_name: &str) -> bool {
    // IPv6 literal with port: [::1]:8448
    if server_name.starts_with('[') {
        return server_name.contains("]:"); // [host]:port
    }
    // hostname:port — but ignore IPv6 with extra colons.
    let colon_count = server_name.chars().filter(|&c| c == ':').count();
    colon_count > 0 && colon_count < 2
}

#[cfg(test)]
mod tests {
    use super::has_explicit_port;

    #[test]
    fn explicit_port_detection() {
        assert!(has_explicit_port("nexus.example.com:8448"));
        assert!(!has_explicit_port("nexus.example.com"));
        assert!(has_explicit_port("[::1]:8448"));
        assert!(!has_explicit_port("::1")); // bare IPv6 — no port
    }
}
