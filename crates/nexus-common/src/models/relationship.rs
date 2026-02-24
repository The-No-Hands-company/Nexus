//! User relationship model — friends, pending requests, blocks.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of a relationship between two users.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RelationshipStatus {
    /// Request sent, waiting for the other party to accept.
    Pending,
    /// Both parties have accepted — they are friends.
    Accepted,
    /// The requester has blocked the addressee.
    Blocked,
}

/// A directional relationship entry.
///
/// One row represents: `requester_id` sent / manages the relationship toward
/// `addressee_id`.  Friendship is a single accepted row (not two rows).
/// A block is stored with the blocker as `requester_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: Uuid,
    pub requester_id: Uuid,
    pub addressee_id: Uuid,
    pub status: RelationshipStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
