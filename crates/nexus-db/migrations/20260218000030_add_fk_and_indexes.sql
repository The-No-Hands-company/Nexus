-- ============================================================================
-- Phase 20-24: Add missing foreign key constraints and indexes
-- ============================================================================

-- ── Migration 25: Scalability Hardening ────────────────────────────────────

ALTER TABLE voice_quality_logs
    ADD CONSTRAINT fk_vql_channel FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE,
    ADD CONSTRAINT fk_vql_user    FOREIGN KEY (user_id)    REFERENCES users(id)    ON DELETE CASCADE;

ALTER TABLE member_prune_rules
    ADD CONSTRAINT fk_mpr_server FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE;

ALTER TABLE slow_mode_overrides
    ADD CONSTRAINT fk_smo_channel FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_vql_channel   ON voice_quality_logs(channel_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_vql_user      ON voice_quality_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_mpr_server    ON member_prune_rules(server_id);
CREATE INDEX IF NOT EXISTS idx_smo_channel   ON slow_mode_overrides(channel_id);

-- ── Migration 26: AI Intelligence ─────────────────────────────────────────

ALTER TABLE search_embeddings
    ADD CONSTRAINT fk_se_message FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE,
    ADD CONSTRAINT fk_se_channel FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE;

ALTER TABLE search_queries
    ADD CONSTRAINT fk_sq_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ai_suggestions
    ADD CONSTRAINT fk_as_user    FOREIGN KEY (user_id)    REFERENCES users(id)    ON DELETE CASCADE,
    ADD CONSTRAINT fk_as_channel FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE;

ALTER TABLE thread_summaries
    ADD CONSTRAINT fk_ts_channel FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE;

ALTER TABLE toxicity_scores
    ADD CONSTRAINT fk_tox_message FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE,
    ADD CONSTRAINT fk_tox_server  FOREIGN KEY (server_id)  REFERENCES servers(id)  ON DELETE CASCADE;

ALTER TABLE raid_detections
    ADD CONSTRAINT fk_rd_server FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE;

ALTER TABLE voice_transcripts
    ADD CONSTRAINT fk_vt_channel FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE;

ALTER TABLE voice_commands
    ADD CONSTRAINT fk_vc_user    FOREIGN KEY (user_id)    REFERENCES users(id)    ON DELETE CASCADE,
    ADD CONSTRAINT fk_vc_channel FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE;

ALTER TABLE ai_consent
    ADD CONSTRAINT fk_ac_user   FOREIGN KEY (user_id)   REFERENCES users(id)   ON DELETE CASCADE,
    ADD CONSTRAINT fk_ac_server FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE;

ALTER TABLE ai_audit_log
    ADD CONSTRAINT fk_aal_server FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_se_message    ON search_embeddings(message_id);
CREATE INDEX IF NOT EXISTS idx_se_channel    ON search_embeddings(channel_id);
CREATE INDEX IF NOT EXISTS idx_sq_user       ON search_queries(user_id);
CREATE INDEX IF NOT EXISTS idx_as_user       ON ai_suggestions(user_id, channel_id);
CREATE INDEX IF NOT EXISTS idx_ts_channel    ON thread_summaries(channel_id);
CREATE INDEX IF NOT EXISTS idx_tox_message   ON toxicity_scores(message_id);
CREATE INDEX IF NOT EXISTS idx_tox_server    ON toxicity_scores(server_id);
CREATE INDEX IF NOT EXISTS idx_rd_server     ON raid_detections(server_id, detected_at DESC);
CREATE INDEX IF NOT EXISTS idx_vt_channel    ON voice_transcripts(channel_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_vc_user       ON voice_commands(user_id, channel_id);
CREATE INDEX IF NOT EXISTS idx_ac_user       ON ai_consent(user_id, server_id);
CREATE INDEX IF NOT EXISTS idx_aal_server    ON ai_audit_log(server_id, created_at DESC);

-- ── Migration 27: Voice & Collaboration ───────────────────────────────────

ALTER TABLE video_layouts
    ADD CONSTRAINT fk_vl_channel FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE,
    ADD CONSTRAINT fk_vl_user    FOREIGN KEY (user_id)    REFERENCES users(id)    ON DELETE CASCADE;

ALTER TABLE virtual_backgrounds
    ADD CONSTRAINT fk_vb_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE live_streams
    ADD CONSTRAINT fk_ls_channel  FOREIGN KEY (channel_id)  REFERENCES channels(id) ON DELETE CASCADE,
    ADD CONSTRAINT fk_ls_streamer FOREIGN KEY (streamer_id) REFERENCES users(id)    ON DELETE CASCADE;

ALTER TABLE stream_viewers
    ADD CONSTRAINT fk_sv_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE breakout_rooms
    ADD CONSTRAINT fk_br_parent  FOREIGN KEY (parent_channel) REFERENCES channels(id) ON DELETE CASCADE,
    ADD CONSTRAINT fk_br_creator FOREIGN KEY (created_by)     REFERENCES users(id)    ON DELETE SET NULL;

ALTER TABLE collab_sessions
    ADD CONSTRAINT fk_cs_channel FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE;

ALTER TABLE spatial_audio_configs
    ADD CONSTRAINT fk_sac_channel FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_vl_channel    ON video_layouts(channel_id);
CREATE INDEX IF NOT EXISTS idx_vl_user       ON video_layouts(user_id);
CREATE INDEX IF NOT EXISTS idx_vb_user       ON virtual_backgrounds(user_id);
CREATE INDEX IF NOT EXISTS idx_ls_channel    ON live_streams(channel_id);
CREATE INDEX IF NOT EXISTS idx_ls_streamer   ON live_streams(streamer_id);
CREATE INDEX IF NOT EXISTS idx_sv_user       ON stream_viewers(user_id);
CREATE INDEX IF NOT EXISTS idx_br_parent     ON breakout_rooms(parent_channel);
CREATE INDEX IF NOT EXISTS idx_cs_channel    ON collab_sessions(channel_id);

-- ── Migration 28: Growth & Retention ──────────────────────────────────────

ALTER TABLE server_recommendations
    ADD CONSTRAINT fk_sr_user   FOREIGN KEY (user_id)   REFERENCES users(id)   ON DELETE CASCADE,
    ADD CONSTRAINT fk_sr_server FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE;

ALTER TABLE onboarding_flows
    ADD CONSTRAINT fk_of_server FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE;

ALTER TABLE device_sessions
    ADD CONSTRAINT fk_ds_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE clipboard_sync
    ADD CONSTRAINT fk_cbs_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE user_xp
    ADD CONSTRAINT fk_ux_user   FOREIGN KEY (user_id)   REFERENCES users(id)   ON DELETE CASCADE,
    ADD CONSTRAINT fk_ux_server FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE;

ALTER TABLE gamification_configs
    ADD CONSTRAINT fk_gc_server FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE;

ALTER TABLE achievements
    ADD CONSTRAINT fk_ach_server FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE;

ALTER TABLE user_achievements
    ADD CONSTRAINT fk_ua_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE activity_streaks
    ADD CONSTRAINT fk_ast_user   FOREIGN KEY (user_id)   REFERENCES users(id)   ON DELETE CASCADE,
    ADD CONSTRAINT fk_ast_server FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE;

ALTER TABLE sync_cursors
    ADD CONSTRAINT fk_sc_user    FOREIGN KEY (user_id)    REFERENCES users(id)    ON DELETE CASCADE,
    ADD CONSTRAINT fk_sc_channel FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE;

ALTER TABLE offline_queue
    ADD CONSTRAINT fk_oq_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_sr_user       ON server_recommendations(user_id);
CREATE INDEX IF NOT EXISTS idx_sr_server     ON server_recommendations(server_id);
CREATE INDEX IF NOT EXISTS idx_of_server     ON onboarding_flows(server_id);
CREATE INDEX IF NOT EXISTS idx_ds_user       ON device_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_ux_user       ON user_xp(user_id, server_id);
CREATE INDEX IF NOT EXISTS idx_ach_server    ON achievements(server_id);
CREATE INDEX IF NOT EXISTS idx_ast_user      ON activity_streaks(user_id, server_id);
CREATE INDEX IF NOT EXISTS idx_sc_user       ON sync_cursors(user_id, channel_id);

-- ── Migration 29: Sustainability ──────────────────────────────────────────

ALTER TABLE governance_polls
    ADD CONSTRAINT fk_gp_server  FOREIGN KEY (server_id)  REFERENCES servers(id) ON DELETE CASCADE,
    ADD CONSTRAINT fk_gp_creator FOREIGN KEY (created_by) REFERENCES users(id)   ON DELETE SET NULL;

ALTER TABLE poll_votes
    ADD CONSTRAINT fk_pv_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE governance_proposals
    ADD CONSTRAINT fk_gprop_server FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE,
    ADD CONSTRAINT fk_gprop_author FOREIGN KEY (author_id) REFERENCES users(id)   ON DELETE SET NULL;

ALTER TABLE contributor_badges
    ADD CONSTRAINT fk_cb_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE vulnerability_records
    DROP CONSTRAINT IF EXISTS vulnerability_records_audit_id_fkey,
    ADD CONSTRAINT fk_vr_audit FOREIGN KEY (audit_id) REFERENCES security_audits(id) ON DELETE CASCADE;

ALTER TABLE tutorial_progress
    ADD CONSTRAINT fk_tp_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_gp_server     ON governance_polls(server_id);
CREATE INDEX IF NOT EXISTS idx_gp_creator    ON governance_polls(created_by);
CREATE INDEX IF NOT EXISTS idx_pv_user       ON poll_votes(user_id);
CREATE INDEX IF NOT EXISTS idx_gprop_server  ON governance_proposals(server_id);
CREATE INDEX IF NOT EXISTS idx_gprop_author  ON governance_proposals(author_id);
CREATE INDEX IF NOT EXISTS idx_cb_user       ON contributor_badges(user_id);
CREATE INDEX IF NOT EXISTS idx_tp_user       ON tutorial_progress(user_id);
