//! Nexus Bot SDK for Rust.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use nexus_sdk::{NexusClient, builders::SlashCommandBuilder};
//!
//! #[tokio::main]
//! async fn main() -> nexus_sdk::Result<()> {
//!     let client = NexusClient::new("Bot mytoken", None, None)?;
//!
//!     client.command(
//!         SlashCommandBuilder::new()
//!             .name("ping")
//!             .description("Replies with Pong!")
//!             .build(),
//!         |_interaction| println!("Pong! 🏓"),
//!     );
//!
//!     // Block until the gateway disconnects.
//!     client.login("your-app-id").await
//! }
//! ```

pub mod builders;
pub mod client;
pub mod error;
pub mod gateway;
pub mod rest;
pub mod types;

pub use client::NexusClient;
pub use error::{NexusError, Result};
pub use gateway::GatewayClient;
pub use rest::RestClient;
pub use types::*;
