//! Core domain models shared across all Nexus services.
//!
//! These are the "truth" types — what the database stores and the API serializes.
//! Each model uses Snowflake IDs (like Discord) for globally unique, time-sortable identifiers.

pub mod channel;
pub mod member;
pub mod message;
pub mod rich;
pub mod role;
pub mod server;
pub mod user;

/// Re-export all model types for convenience.
pub use channel::*;
pub use member::*;
pub use message::*;
pub use rich::*;
pub use role::*;
pub use server::*;
pub use user::*;
