-- Migration: User relationships (friends, pending requests, blocks)
-- Generated for PostgreSQL 15+

-- ============================================================================
-- Relationship Status Enum
-- ============================================================================

CREATE TYPE relationship_status AS ENUM ('pending', 'accepted', 'blocked');

-- ============================================================================
-- User Relationships
-- ============================================================================
-- A "relationship" row is directional: requester sent the request to addressee.
-- Friendship = two-way logical state tracked as ONE row with status='accepted'.
-- Blocked = the blocker is stored in requester_id.

CREATE TABLE user_relationships (
    id              UUID PRIMARY KEY,
    requester_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    addressee_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status          relationship_status NOT NULL DEFAULT 'pending',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (requester_id, addressee_id),
    CHECK (requester_id != addressee_id)
);

CREATE INDEX idx_relationships_requester ON user_relationships (requester_id);
CREATE INDEX idx_relationships_addressee ON user_relationships (addressee_id);
