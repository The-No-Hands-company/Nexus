//! Server-to-server HTTP client.
//!
//! The [`FederationClient`] handles all outbound communication to remote
//! Nexus servers. Every request is signed with this server's key pair before
//! being sent.
//!
//! # Usage
//!
//! ```rust,no_run
//! use nexus_federation::{client::FederationClient, types::FederationTransaction};
//! use nexus_federation::keys::ServerKeyPair;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() {
//!     let kp = Arc::new(ServerKeyPair::generate());
//!     let client = FederationClient::new("nexus.example.com", kp);
//!
//!     let txn = FederationTransaction::new("nexus.example.com", "nexus.other.tld");
//!     client.send_transaction("nexus.other.tld", txn).await.unwrap();
//! }
//! ```

use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tracing::debug;

use crate::{
    discovery::DiscoveryCache,
    error::FederationError,
    keys::ServerKeyPair,
    signatures::sign_request,
    types::{
        DirectoryListingResponse, FederationEvent, FederationTransaction, MakeJoinResponse,
        SendJoinResponse, ServerInfo,
    },
};

// ─── Client ──────────────────────────────────────────────────────────────────

/// Async HTTP client for outbound server-to-server federation requests.
///
/// Internally uses `reqwest` with a connection pool and request signing.
pub struct FederationClient {
    server_name: String,
    key_pair: Arc<ServerKeyPair>,
    http: Client,
    discovery: DiscoveryCache,
}

impl FederationClient {
    /// Create a new federation client for the given `server_name`.
    pub fn new(server_name: impl Into<String>, key_pair: Arc<ServerKeyPair>) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("Nexus-Federation/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build reqwest client");

        Self {
            server_name: server_name.into(),
            key_pair,
            http,
            discovery: DiscoveryCache::new(),
        }
    }

    // ── Transaction sending ──────────────────────────────────────────────────

    /// Send a federation transaction to a remote server.
    ///
    /// `PUT /_nexus/federation/v1/send/{txnId}`
    pub async fn send_transaction(
        &self,
        destination: &str,
        txn: FederationTransaction,
    ) -> Result<(), FederationError> {
        let txn_id = uuid::Uuid::new_v4().simple().to_string();
        let uri = format!("/_nexus/federation/v1/send/{}", txn_id);
        let body = serde_json::to_value(&txn)?;
        let base_url = self.discovery.resolve(destination).await?;

        self.signed_put::<()>(destination, &base_url, &uri, &body).await?;
        Ok(())
    }

    // ── Event fetching ───────────────────────────────────────────────────────

    /// Fetch a single event by ID from a remote server.
    ///
    /// `GET /_nexus/federation/v1/event/{eventId}`
    pub async fn get_event(
        &self,
        destination: &str,
        event_id: &str,
    ) -> Result<FederationEvent, FederationError> {
        let uri = format!("/_nexus/federation/v1/event/{}", urlencoded(event_id));
        let base_url = self.discovery.resolve(destination).await?;
        self.signed_get(destination, &base_url, &uri).await
    }

    // ── Room state ───────────────────────────────────────────────────────────

    /// Pull the full state of a room at a given event.
    ///
    /// `GET /_nexus/federation/v1/state/{roomId}?at={eventId}`
    pub async fn get_state(
        &self,
        destination: &str,
        room_id: &str,
        at_event_id: Option<&str>,
    ) -> Result<Vec<FederationEvent>, FederationError> {
        let mut uri = format!("/_nexus/federation/v1/state/{}", urlencoded(room_id));
        if let Some(at) = at_event_id {
            uri.push_str(&format!("?at={}", urlencoded(at)));
        }
        let base_url = self.discovery.resolve(destination).await?;
        #[derive(serde::Deserialize)]
        struct StateResponse { pdus: Vec<FederationEvent> }
        let resp: StateResponse = self.signed_get(destination, &base_url, &uri).await?;
        Ok(resp.pdus)
    }

    // ── Join protocol ────────────────────────────────────────────────────────

    /// Request a join event template from a remote server.
    ///
    /// `GET /_nexus/federation/v1/make_join/{roomId}/{userId}`
    pub async fn make_join(
        &self,
        destination: &str,
        room_id: &str,
        user_id: &str,
    ) -> Result<MakeJoinResponse, FederationError> {
        let uri = format!(
            "/_nexus/federation/v1/make_join/{}/{}",
            urlencoded(room_id),
            urlencoded(user_id)
        );
        let base_url = self.discovery.resolve(destination).await?;
        self.signed_get(destination, &base_url, &uri).await
    }

    /// Submit a signed join event to a remote server.
    ///
    /// `PUT /_nexus/federation/v1/send_join/{roomId}/{eventId}`
    pub async fn send_join(
        &self,
        destination: &str,
        room_id: &str,
        event_id: &str,
        join_event: &Value,
    ) -> Result<SendJoinResponse, FederationError> {
        let uri = format!(
            "/_nexus/federation/v1/send_join/{}/{}",
            urlencoded(room_id),
            urlencoded(event_id)
        );
        let base_url = self.discovery.resolve(destination).await?;
        self.signed_put(destination, &base_url, &uri, join_event).await
    }

    // ── Directory ────────────────────────────────────────────────────────────

    /// Query the public room directory on a remote server.
    ///
    /// `GET /_nexus/federation/v1/directory?limit=&since=`
    pub async fn query_directory(
        &self,
        destination: &str,
        limit: Option<u32>,
        since: Option<&str>,
    ) -> Result<DirectoryListingResponse, FederationError> {
        let mut uri = "/_nexus/federation/v1/directory".to_owned();
        let mut params: Vec<String> = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(s) = since {
            params.push(format!("since={}", urlencoded(s)));
        }
        if !params.is_empty() {
            uri.push('?');
            uri.push_str(&params.join("&"));
        }
        let base_url = self.discovery.resolve(destination).await?;
        self.signed_get(destination, &base_url, &uri).await
    }

    // ── Server keys ──────────────────────────────────────────────────────────

    /// Fetch the key document from a remote server.
    ///
    /// `GET /_nexus/key/v2/server`
    pub async fn fetch_server_keys(&self, destination: &str) -> Result<ServerInfo, FederationError> {
        let base_url = self.discovery.resolve(destination).await?;
        // Key fetch is unauthenticated (like Matrix).
        let url = format!("{}{}", base_url, "/_nexus/key/v2/server");
        debug!("Fetching server keys from {}", url);
        let resp = self
            .http
            .get(&url)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| FederationError::RemoteHttp(destination.to_owned(), e.to_string()))?;
        Ok(resp.json().await?)
    }

    // ── Signed request helpers ───────────────────────────────────────────────

    async fn signed_get<T: DeserializeOwned>(
        &self,
        destination: &str,
        base_url: &str,
        uri: &str,
    ) -> Result<T, FederationError> {
        let auth = sign_request(&self.key_pair, &self.server_name, destination, "GET", uri, None);
        let url = format!("{}{}", base_url, uri);
        debug!("Federation GET {}", url);
        let resp = self
            .http
            .get(&url)
            .header("Authorization", auth.to_header())
            .send()
            .await?
            .error_for_status()
            .map_err(|e| FederationError::RemoteHttp(destination.to_owned(), e.to_string()))?;
        Ok(resp.json().await?)
    }

    async fn signed_put<T: DeserializeOwned>(
        &self,
        destination: &str,
        base_url: &str,
        uri: &str,
        body: &Value,
    ) -> Result<T, FederationError> {
        let auth =
            sign_request(&self.key_pair, &self.server_name, destination, "PUT", uri, Some(body));
        let url = format!("{}{}", base_url, uri);
        debug!("Federation PUT {}", url);
        let resp = self
            .http
            .put(&url)
            .header("Authorization", auth.to_header())
            .json(body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| FederationError::RemoteHttp(destination.to_owned(), e.to_string()))?;
        Ok(resp.json().await?)
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn urlencoded(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
