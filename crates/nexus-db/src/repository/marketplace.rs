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
