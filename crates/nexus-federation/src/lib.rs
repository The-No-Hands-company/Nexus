//! # nexus-federation
//!
//! Matrix-compatible server-to-server (S2S) federation layer for Nexus.
//!
//! ## Architecture
//!
//! Federation enables Nexus servers to communicate with each other and with
//! Matrix homeservers, allowing users on different servers to join shared
//! channels, exchange messages, and resolve identities.
//!
//! ```text
//!  nexus.example.com          nexus.other.tld          matrix.org
//!       │                           │                      │
//!       ├─── PUT /send/{txnId} ──►  │                      │
//!       │                           ├─── m.room.message ─► │ (Matrix AS bridge)
//!       ├◄── GET /event/{id} ──────  │                      │
//!       │                           │                      │
//! ```
//!
//! ## Key concepts
//!
//! - **Server keys** (`keys.rs`): each Nexus server holds an Ed25519 signing key pair.
//!   The public key is advertised via `/_nexus/key/v2/server` so remote servers can
//!   verify request signatures.
//! - **Signed requests** (`signatures.rs`): all S2S HTTP requests are signed with
//!   the originating server's private key using the Nexus Request Authorization scheme
//!   (modelled on Matrix's `X-Matrix` auth).
//! - **Federation client** (`client.rs`): async HTTP client for sending events to
//!   remote servers and resolving remote room state.
//! - **Discovery** (`discovery.rs`): resolves `server.tld` → actual S2S endpoint via
//!   `/.well-known/nexus/server`, SRV DNS, or direct HTTPS fallback.
//! - **Matrix bridge** (`matrix_bridge.rs`): Matrix Application Service (AS) bridge
//!   for relaying messages to/from Matrix homeservers.

pub mod client;
pub mod discovery;
pub mod error;
pub mod key_manager;
pub mod keys;
pub mod matrix_bridge;
pub mod signatures;
pub mod types;

pub use client::FederationClient;
pub use discovery::has_explicit_port as discovery_has_explicit_port;
pub use error::FederationError;
pub use key_manager::KeyManager;
pub use keys::ServerKeyPair;
pub use matrix_bridge::{BridgeConfig, BridgeError, BridgedEvent, MatrixBridge, MatrixTransaction, mxid_to_username};
pub use signatures::sign_event;
pub use types::{
    FederatedFriendRequest, FederatedFriendResponse,
    FederationEvent, FederationStatus, FederationTransaction, PeerHealth, PeerRecord,
    PeerRequest, PeerStatus, RichWellKnownServer, ServerInfo,
};
