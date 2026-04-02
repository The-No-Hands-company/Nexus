//! Repository for the plugin marketplace.

use nexus_common::models::ecosystem::{MarketplacePlugin, PluginInstall, PluginReview};
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::{MARKETPLACE_PLUGIN_COLS, PLUGIN_INSTALL_COLS, PLUGIN_REVIEW_COLS};

// ── Marketplace Plugins ───────────────────────────────────────────────────────

pub async fn create_plugin(
    pool: &AnyPool,
    id: Uuid,
    name: &str,
    slug: &str,
    description: Option<&str>,
    author_id: Option<Uuid>,
    version: &str,
    manifest_url: &str,
    icon_url: Option<&str>,
    source_url: Option<&str>,
    signature: Option<&str>,
    signing_key_id: Option<&str>,
    category: &str,
    tags: &serde_json::Value,
) -> Result<MarketplacePlugin, sqlx::Error> {
    let q = format!(
        "INSERT INTO marketplace_plugins \
         (id, name, slug, description, author_id, version, manifest_url, icon_url, \
          source_url, signature, signing_key_id, category, tags) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) \
         RETURNING {MARKETPLACE_PLUGIN_COLS}"
    );
    sqlx::query_as::<_, MarketplacePlugin>(&q)
        .bind(id.to_string())
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(author_id.map(|u| u.to_string()))
        .bind(version)
        .bind(manifest_url)
        .bind(icon_url)
        .bind(source_url)
        .bind(signature)
        .bind(signing_key_id)
        .bind(category)
        .bind(serde_json::to_string(tags).unwrap_or_default())
        .fetch_one(pool)
        .await
}

pub async fn get_plugin_by_slug(
    pool: &AnyPool,
    slug: &str,
) -> Result<Option<MarketplacePlugin>, sqlx::Error> {
    let q = format!(
        "SELECT {MARKETPLACE_PLUGIN_COLS} FROM marketplace_plugins WHERE slug = $1 AND is_published = TRUE"
    );
    sqlx::query_as::<_, MarketplacePlugin>(&q)
        .bind(slug)
        .fetch_optional(pool)
        .await
}

pub async fn get_plugin(
    pool: &AnyPool,
    id: Uuid,
) -> Result<Option<MarketplacePlugin>, sqlx::Error> {
    let q = format!(
        "SELECT {MARKETPLACE_PLUGIN_COLS} FROM marketplace_plugins WHERE id = $1"
    );
    sqlx::query_as::<_, MarketplacePlugin>(&q)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

pub async fn search_plugins(
    pool: &AnyPool,
    query: Option<&str>,
    category: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<MarketplacePlugin>, sqlx::Error> {
    // Build dynamic WHERE clauses
    let mut conditions = vec!["is_published = TRUE".to_string()];
    let mut bind_idx = 1;

    if query.is_some() {
        conditions.push(format!("(name ILIKE '%' || ${bind_idx} || '%' OR description ILIKE '%' || ${bind_idx} || '%')"));
        bind_idx += 1;
    }
    if category.is_some() {
        conditions.push(format!("category = ${bind_idx}"));
        bind_idx += 1;
    }

    let where_clause = conditions.join(" AND ");
    let q = format!(
        "SELECT {MARKETPLACE_PLUGIN_COLS} FROM marketplace_plugins \
         WHERE {where_clause} ORDER BY downloads DESC LIMIT ${bind_idx} OFFSET ${next}",
        next = bind_idx + 1
    );

    let mut qb = sqlx::query_as::<_, MarketplacePlugin>(&q);
    if let Some(search) = query {
        qb = qb.bind(search);
    }
    if let Some(cat) = category {
        qb = qb.bind(cat);
    }
    qb = qb.bind(limit).bind(offset);
    qb.fetch_all(pool).await
}

pub async fn increment_downloads(pool: &AnyPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE marketplace_plugins SET downloads = downloads + 1 WHERE id = $1")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_plugin_rating(pool: &AnyPool, plugin_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE marketplace_plugins SET \
         avg_rating = COALESCE((SELECT AVG(rating)::real FROM plugin_reviews WHERE plugin_id = $1), 0), \
         rating_count = (SELECT COUNT(*) FROM plugin_reviews WHERE plugin_id = $1), \
         updated_at = now() \
         WHERE id = $1"
    )
    .bind(plugin_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

// ── Plugin Reviews ────────────────────────────────────────────────────────────

pub async fn create_review(
    pool: &AnyPool,
    id: Uuid,
    plugin_id: Uuid,
    user_id: Uuid,
    rating: i16,
    title: Option<&str>,
    body: Option<&str>,
) -> Result<PluginReview, sqlx::Error> {
    let q = format!(
        "INSERT INTO plugin_reviews (id, plugin_id, user_id, rating, title, body) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (plugin_id, user_id) DO UPDATE SET \
           rating = $4, title = $5, body = $6, updated_at = now() \
         RETURNING {PLUGIN_REVIEW_COLS}"
    );
    sqlx::query_as::<_, PluginReview>(&q)
        .bind(id.to_string())
        .bind(plugin_id.to_string())
        .bind(user_id.to_string())
        .bind(rating)
        .bind(title)
        .bind(body)
        .fetch_one(pool)
        .await
}

pub async fn list_reviews(
    pool: &AnyPool,
    plugin_id: Uuid,
    limit: i64,
) -> Result<Vec<PluginReview>, sqlx::Error> {
    let q = format!(
        "SELECT {PLUGIN_REVIEW_COLS} FROM plugin_reviews \
         WHERE plugin_id = $1 ORDER BY created_at DESC LIMIT $2"
    );
    sqlx::query_as::<_, PluginReview>(&q)
        .bind(plugin_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
}

pub async fn delete_review(
    pool: &AnyPool,
    plugin_id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        "DELETE FROM plugin_reviews WHERE plugin_id = $1 AND user_id = $2"
    )
    .bind(plugin_id.to_string())
    .bind(user_id.to_string())
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

// ── Plugin Installs ───────────────────────────────────────────────────────────

pub async fn install_plugin(
    pool: &AnyPool,
    id: Uuid,
    plugin_id: Uuid,
    server_id: Uuid,
    installed_by: Uuid,
    version: &str,
) -> Result<PluginInstall, sqlx::Error> {
    let q = format!(
        "INSERT INTO plugin_installs (id, plugin_id, server_id, installed_by, version) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (plugin_id, server_id) DO UPDATE SET \
           version = $5, installed_by = $4, is_enabled = TRUE \
         RETURNING {PLUGIN_INSTALL_COLS}"
    );
    sqlx::query_as::<_, PluginInstall>(&q)
        .bind(id.to_string())
        .bind(plugin_id.to_string())
        .bind(server_id.to_string())
        .bind(installed_by.to_string())
        .bind(version)
        .fetch_one(pool)
        .await
}

pub async fn uninstall_plugin(
    pool: &AnyPool,
    plugin_id: Uuid,
    server_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        "DELETE FROM plugin_installs WHERE plugin_id = $1 AND server_id = $2"
    )
    .bind(plugin_id.to_string())
    .bind(server_id.to_string())
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn list_server_installs(
    pool: &AnyPool,
    server_id: Uuid,
) -> Result<Vec<PluginInstall>, sqlx::Error> {
    let q = format!(
        "SELECT {PLUGIN_INSTALL_COLS} FROM plugin_installs \
         WHERE server_id = $1 ORDER BY created_at DESC"
    );
    sqlx::query_as::<_, PluginInstall>(&q)
        .bind(server_id.to_string())
        .fetch_all(pool)
        .await
}

pub async fn toggle_plugin_install(
    pool: &AnyPool,
    plugin_id: Uuid,
    server_id: Uuid,
    enabled: bool,
) -> Result<Option<PluginInstall>, sqlx::Error> {
    let q = format!(
        "UPDATE plugin_installs SET is_enabled = $3 \
         WHERE plugin_id = $1 AND server_id = $2 \
         RETURNING {PLUGIN_INSTALL_COLS}"
    );
    sqlx::query_as::<_, PluginInstall>(&q)
        .bind(plugin_id.to_string())
        .bind(server_id.to_string())
        .bind(enabled)
        .fetch_optional(pool)
        .await
}

// ── Store Governance ──────────────────────────────────────────────────────────

use nexus_common::models::ecosystem::{
    CreatorVetting, MarketplaceMonetization, ReviewStatus, TrustTier,
};

pub async fn submit_for_review(
    pool: &AnyPool,
    plugin_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE marketplace_plugins SET review_status = 'submitted', updated_at = now() \
         WHERE id = $1"
    )
    .bind(plugin_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_review_status(
    pool: &AnyPool,
    plugin_id: Uuid,
    new_status: ReviewStatus,
    reviewer_id: Uuid,
    reason: Option<&str>,
    notes: Option<&str>,
) -> Result<(), sqlx::Error> {
    let status_str = new_status.to_string();
    sqlx::query(
        "UPDATE marketplace_plugins SET \
         review_status = $1, reviewer_id = $2, review_notes = $3, reviewed_at = now(), \
         rejected_reason = CASE WHEN $1 = 'rejected' THEN $4 ELSE rejected_reason END, \
         quarantine_reason = CASE WHEN $1 = 'quarantined' THEN $4 ELSE quarantine_reason END, \
         updated_at = now() \
         WHERE id = $5"
    )
    .bind(&status_str)
    .bind(reviewer_id.to_string())
    .bind(notes)
    .bind(reason)
    .bind(plugin_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_trust_tier(
    pool: &AnyPool,
    plugin_id: Uuid,
    tier: TrustTier,
) -> Result<(), sqlx::Error> {
    let tier_str = tier.to_string();
    sqlx::query(
        "UPDATE marketplace_plugins SET trust_tier = $1, updated_at = now() WHERE id = $2"
    )
    .bind(&tier_str)
    .bind(plugin_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_provenance_verified(
    pool: &AnyPool,
    plugin_id: Uuid,
    verified: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE marketplace_plugins SET provenance_verified = $1, updated_at = now() WHERE id = $2"
    )
    .bind(verified)
    .bind(plugin_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_security_scan(
    pool: &AnyPool,
    plugin_id: Uuid,
    scan_result: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE marketplace_plugins SET \
         security_scan_result = $1, last_scanned_at = now(), updated_at = now() \
         WHERE id = $2"
    )
    .bind(serde_json::to_string(scan_result).unwrap_or_default())
    .bind(plugin_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_review_queue(
    pool: &AnyPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (String, String)>(
        "SELECT id, name FROM marketplace_plugins \
         WHERE review_status IN ('submitted', 'scanning', 'review') \
         ORDER BY created_at DESC LIMIT $1 OFFSET $2"
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn log_review(
    pool: &AnyPool,
    id: Uuid,
    plugin_id: Uuid,
    reviewer_id: Option<Uuid>,
    previous_status: &str,
    new_status: &str,
    action: &str,
    reason: Option<&str>,
    notes: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO marketplace_reviews \
         (id, plugin_id, reviewer_id, previous_status, new_status, action, reason, notes, metadata) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
    )
    .bind(id.to_string())
    .bind(plugin_id.to_string())
    .bind(reviewer_id.map(|u| u.to_string()))
    .bind(previous_status)
    .bind(new_status)
    .bind(action)
    .bind(reason)
    .bind(notes)
    .bind(serde_json::to_string(&serde_json::json!({"timestamp": chrono::Utc::now()})).unwrap_or_default())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn request_takedown(
    pool: &AnyPool,
    takedown_id: Uuid,
    plugin_id: Uuid,
    reported_by: Option<Uuid>,
    reason: &str,
    description: &str,
    evidence_urls: Option<&serde_json::Value>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO marketplace_takedowns \
         (id, plugin_id, reported_by, reason, description, evidence_urls, quarantine_status) \
         VALUES ($1, $2, $3, $4, $5, $6, 'pending')"
    )
    .bind(takedown_id.to_string())
    .bind(plugin_id.to_string())
    .bind(reported_by.map(|u| u.to_string()))
    .bind(reason)
    .bind(description)
    .bind(evidence_urls.map(|v| serde_json::to_string(v).unwrap_or_default()))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn review_takedown(
    pool: &AnyPool,
    takedown_id: Uuid,
    reviewer_id: Uuid,
    status: &str,
    notes: Option<&str>,
) -> Result<(), sqlx::Error> {
    let plugin_result = sqlx::query_scalar::<_, String>(
        "SELECT plugin_id FROM marketplace_takedowns WHERE id = $1"
    )
    .bind(takedown_id.to_string())
    .fetch_optional(pool)
    .await?;

    if let Some(plugin_id_str) = plugin_result {
        sqlx::query(
            "UPDATE marketplace_takedowns SET \
             quarantine_status = $1, reviewer_id = $2, review_notes = $3, reviewed_at = now() \
             WHERE id = $4"
        )
        .bind(status)
        .bind(reviewer_id.to_string())
        .bind(notes)
        .bind(takedown_id.to_string())
        .execute(pool)
        .await?;

        // If permanent_takedown, update plugin status
        if status == "permanent_takedown" {
            sqlx::query(
                "UPDATE marketplace_plugins SET \
                 review_status = 'takedown', is_published = FALSE, \
                 takedown_reason = $1, takedown_requested_by = $2, takedown_requested_at = now() \
                 WHERE id = $3"
            )
            .bind("Takedown reviewed and approved")
            .bind(reviewer_id.to_string())
            .bind(&plugin_id_str)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

pub async fn reinstate_plugin(
    pool: &AnyPool,
    takedown_id: Uuid,
) -> Result<(), sqlx::Error> {
    let plugin_result = sqlx::query_scalar::<_, String>(
        "SELECT plugin_id FROM marketplace_takedowns WHERE id = $1"
    )
    .bind(takedown_id.to_string())
    .fetch_optional(pool)
    .await?;

    if let Some(plugin_id_str) = plugin_result {
        sqlx::query(
            "UPDATE marketplace_takedowns SET \
             quarantine_status = 'reinstated', reinstated_at = now() \
             WHERE id = $1"
        )
        .bind(takedown_id.to_string())
        .execute(pool)
        .await?;

        sqlx::query(
            "UPDATE marketplace_plugins SET \
             review_status = 'approved', is_published = TRUE, \
             takedown_reason = NULL, takedown_requested_by = NULL, takedown_requested_at = NULL, \
             updated_at = now() \
             WHERE id = $1"
        )
        .bind(&plugin_id_str)
        .execute(pool)
        .await?;
    }
    Ok(())
}

// ── Creator Vetting ───────────────────────────────────────────────────────────

pub async fn get_creator_vetting(
    pool: &AnyPool,
    user_id: Uuid,
) -> Result<Option<CreatorVetting>, sqlx::Error> {
    sqlx::query_as::<_, CreatorVetting>(
        "SELECT id, user_id, identity_level, domain, domain_verified, \
         signing_key_fingerprint, signing_key_type, rights_attestation, \
         two_factor_enabled, ip_whitelist, \
         status, approved_by, approved_at, rejection_reason, created_at, updated_at \
         FROM creator_vetting WHERE user_id = $1"
    )
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await
}

pub async fn create_creator_vetting(
    pool: &AnyPool,
    id: Uuid,
    user_id: Uuid,
) -> Result<CreatorVetting, sqlx::Error> {
    sqlx::query_as::<_, CreatorVetting>(
        "INSERT INTO creator_vetting (id, user_id, identity_level, status) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id, user_id, identity_level, domain, domain_verified, \
         signing_key_fingerprint, signing_key_type, rights_attestation, \
         two_factor_enabled, ip_whitelist, \
         status, approved_by, approved_at, rejection_reason, created_at, updated_at"
    )
    .bind(id.to_string())
    .bind(user_id.to_string())
    .bind("unverified")
    .bind("pending")
    .fetch_one(pool)
    .await
}

pub async fn update_creator_identity_level(
    pool: &AnyPool,
    user_id: Uuid,
    level: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE creator_vetting SET identity_level = $1, updated_at = now() WHERE user_id = $2"
    )
    .bind(level)
    .bind(user_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn upsert_creator_attestation(
    pool: &AnyPool,
    user_id: Uuid,
    rights_attestation: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE creator_vetting SET \
         rights_attestation = $1, status = 'pending', rejection_reason = NULL, updated_at = now() \
         WHERE user_id = $2"
    )
    .bind(rights_attestation)
    .bind(user_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn approve_creator_vetting(
    pool: &AnyPool,
    user_id: Uuid,
    reviewer_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE creator_vetting SET \
         status = 'approved', approved_by = $1, approved_at = now(), updated_at = now() \
         WHERE user_id = $2"
    )
    .bind(reviewer_id.to_string())
    .bind(user_id.to_string())
    .execute(pool)
    .await?;
    
    // Grant MARKETPLACE_CREATOR flag to user
    sqlx::query(
        "UPDATE users SET flags = flags | $1 WHERE id = $2"
    )
    .bind(nexus_common::models::user_flags::MARKETPLACE_CREATOR)
    .bind(user_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn reject_creator_vetting(
    pool: &AnyPool,
    user_id: Uuid,
    reason: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE creator_vetting SET \
         status = 'rejected', rejection_reason = $1, updated_at = now() \
         WHERE user_id = $2"
    )
    .bind(reason)
    .bind(user_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_creator_vetting_queue(
    pool: &AnyPool,
    status: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<(String, String, String)>, sqlx::Error> {
    // Returns (id, user_id, identity_level)
    sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, user_id, identity_level FROM creator_vetting \
         WHERE status = $1 ORDER BY created_at ASC LIMIT $2 OFFSET $3"
    )
    .bind(status)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

// ── Monetization ──────────────────────────────────────────────────────────────

pub async fn create_monetization_record(
    pool: &AnyPool,
    id: Uuid,
    plugin_id: Uuid,
    creator_id: Uuid,
) -> Result<MarketplaceMonetization, sqlx::Error> {
    use nexus_common::models::ecosystem::MarketplaceMonetization;
    
    sqlx::query_as::<_, MarketplaceMonetization>(
        "INSERT INTO marketplace_monetization \
         (id, plugin_id, creator_id, is_monetized, currency) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING id, plugin_id, creator_id, is_monetized, price_cents, currency, \
         payment_link, purchase_count, created_at, updated_at"
    )
    .bind(id.to_string())
    .bind(plugin_id.to_string())
    .bind(creator_id.to_string())
    .bind(false)
    .bind("USD")
    .fetch_one(pool)
    .await
}

pub async fn get_monetization_by_plugin(
    pool: &AnyPool,
    plugin_id: Uuid,
) -> Result<Option<MarketplaceMonetization>, sqlx::Error> {
    sqlx::query_as::<_, MarketplaceMonetization>(
        "SELECT id, plugin_id, creator_id, is_monetized, price_cents, currency, \
         payment_link, purchase_count, created_at, updated_at \
         FROM marketplace_monetization WHERE plugin_id = $1"
    )
    .bind(plugin_id.to_string())
    .fetch_optional(pool)
    .await
}

pub async fn update_monetization_price(
    pool: &AnyPool,
    plugin_id: Uuid,
    price_cents: Option<i32>,
    payment_link: Option<String>,
) -> Result<(), sqlx::Error> {
    let is_monetized = price_cents.map(|p| p > 0).unwrap_or(false);
    sqlx::query(
        "UPDATE marketplace_monetization SET \
         price_cents = $1, is_monetized = $2, payment_link = $3, updated_at = now() \
         WHERE plugin_id = $4"
    )
    .bind(price_cents)
    .bind(is_monetized)
    .bind(payment_link)
    .bind(plugin_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

/// Increment the install/purchase count for a paid plugin.
/// TNHC does not process or record financial transactions.
pub async fn increment_purchase_count(
    pool: &AnyPool,
    plugin_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE marketplace_monetization SET \
         purchase_count = purchase_count + 1, updated_at = now() \
         WHERE plugin_id = $1"
    )
    .bind(plugin_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}
