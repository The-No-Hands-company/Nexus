//! Core domain models shared across all Nexus services.
//!
//! These are the "truth" types — what the database stores and the API serializes.
//! Each model uses Snowflake IDs (like Discord) for globally unique, time-sortable identifiers.

pub mod bot;
pub mod channel;
pub mod crypto;
pub mod member;
pub mod message;
pub mod plugin;
pub mod rich;
pub mod role;
pub mod server;
pub mod slash_command;
pub mod user;
pub mod webhook;

/// Re-export all model types for convenience.
pub use bot::*;
pub use channel::*;
pub use crypto::*;
pub use member::*;
pub use message::*;
pub use plugin::*;
pub use rich::*;
pub use role::*;
pub use server::*;
pub use slash_command::*;
pub use user::*;
pub use webhook::*;
