//! Repository layer — query functions organized by domain.

pub mod attachments;
pub mod audit_log;
pub mod bots;
pub mod channels;
pub mod email_verification;
pub mod password_reset;
pub mod emoji;
pub mod keystore;
pub mod matrix_bridge;
pub mod members;
pub mod messages;
pub mod moderation;
pub mod plugins;
pub mod reactions;
pub mod read_states;
pub mod relationships;
pub mod roles;
pub mod servers;
pub mod sessions;
pub mod slash_commands;
pub mod scylla_outbox;
pub mod threads;
pub mod two_fa;
pub mod users;
pub mod user_experience;
pub mod webhooks;

// v1.5 Collaboration & Productivity
pub mod tasks;
pub mod calendar;
pub mod file_versions;

// v1.6 Multimedia & Expression
pub mod voice_notes;
pub mod stories;
pub mod drawings;
pub mod voice_settings;
pub mod media_gallery;

// v1.7 Accessibility & Inclusivity
pub mod accessibility_settings;
pub mod voice_captions;
pub mod translations;
pub mod tts_requests;

// v1.8 Ecosystem & Onboarding
pub mod analytics_snapshots;
pub mod bulk_invitations;
pub mod import_jobs;
pub mod marketplace;
pub mod onboarding;
pub mod server_templates;

// v1.9 Scalability & Performance Hardening
pub mod scaling_configs;
pub mod sfu_nodes;
pub mod federation_batches;
pub mod voice_quality;
pub mod large_server;
pub mod ops_metrics;

// v2.0 AI & Intelligence Layer
pub mod ai_intelligence;

// v2.1 Voice & Real-Time Collaboration
pub mod voice_collab;

// v2.2 User Growth & Retention
pub mod growth;

// v2.x Sustainability & Extensibility
pub mod sustainability;

/// Push notification subscriptions (Web Push / VAPID)
pub mod push_subscriptions;
