//! Plugin and theme marketplace repository.

use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use nexus_common::models::plugin::{ClientPlugin, Theme, UserPluginInstall, UserThemeInstall};

fn row_to_plugin(row: &sqlx::postgres::PgRow) -> ClientPlugin {
    ClientPlugin {
        id: row.try_get("id").unwrap(),
        author_id: row.try_get("author_id").unwrap_or(None),
        name: row.try_get("name").unwrap(),
        slug: row.try_get("slug").unwrap(),
        version: row.try_get("version").unwrap_or_default(),
        description: row.try_get("description").unwrap_or(None),
        homepage: row.try_get("homepage").unwrap_or(None),
        repository: row.try_get("repository").unwrap_or(None),
        engine_range: row.try_get("engine_range").unwrap_or_else(|_| "*".to_string()),
        permissions: {
            let v: Option<serde_json::Value> = row.try_get("permissions").unwrap_or(None);
            v.and_then(|j| serde_json::from_value(j).ok()).unwrap_or_default()
        },
        bundle_url: row.try_get("bundle_url").unwrap_or(None),
        bundle_hash: row.try_get("bundle_hash").unwrap_or(None),
        verified: row.try_get("verified").unwrap_or(false),
        active: row.try_get("active").unwrap_or(true),
        install_count: row.try_get("install_count").unwrap_or(0),
        created_at: row.try_get("created_at").unwrap(),
        updated_at: row.try_get("updated_at").unwrap(),
    }
}

fn row_to_theme(row: &sqlx::postgres::PgRow) -> Theme {
    Theme {
        id: row.try_get("id").unwrap(),
        author_id: row.try_get("author_id").unwrap_or(None),
        name: row.try_get("name").unwrap(),
        slug: row.try_get("slug").unwrap(),
        version: row.try_get("version").unwrap_or_default(),
        description: row.try_get("description").unwrap_or(None),
        variables: row.try_get("variables").unwrap_or(serde_json::Value::Object(Default::default())),
        css: row.try_get("css").unwrap_or_default(),
        preview_url: row.try_get("preview_url").unwrap_or(None),
        verified: row.try_get("verified").unwrap_or(false),
        active: row.try_get("active").unwrap_or(true),
        install_count: row.try_get("install_count").unwrap_or(0),
        created_at: row.try_get("created_at").unwrap(),
        updated_at: row.try_get("updated_at").unwrap(),
    }
}

fn row_to_user_plugin(row: &sqlx::postgres::PgRow) -> (UserPluginInstall, Option<Uuid>) {
    let install = UserPluginInstall {
        user_id: row.try_get("user_id").unwrap(),
        plugin_id: row.try_get("plugin_id").unwrap(),
        enabled: row.try_get("enabled").unwrap_or(true),
        settings: row.try_get("settings").unwrap_or(serde_json::Value::Object(Default::default())),
        installed_at: row.try_get("installed_at").unwrap(),
        plugin: None,
    };
    (install, None)
}

fn row_to_user_theme(row: &sqlx::postgres::PgRow) -> UserThemeInstall {
    UserThemeInstall {
        user_id: row.try_get("user_id").unwrap(),
        theme_id: row.try_get("theme_id").unwrap(),
        active: row.try_get("active").unwrap_or(false),
        installed_at: row.try_get("installed_at").unwrap(),
        theme: None,
    }
}

// ============================================================================
// Plugins
// ============================================================================

pub async fn list_plugins(pool: &PgPool, limit: i64, offset: i64) -> Result<Vec<ClientPlugin>> {
    let rows = sqlx::query(
        "SELECT * FROM client_plugins WHERE verified = true ORDER BY install_count DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_plugin).collect())
}

pub async fn get_plugin_by_id(pool: &PgPool, plugin_id: Uuid) -> Result<Option<ClientPlugin>> {
    let row = sqlx::query("SELECT * FROM client_plugins WHERE id = $1")
        .bind(plugin_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_plugin))
}

pub async fn get_plugin_by_slug(pool: &PgPool, slug: &str) -> Result<Option<ClientPlugin>> {
    let row = sqlx::query("SELECT * FROM client_plugins WHERE slug = $1")
        .bind(slug)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_plugin))
}

pub async fn create_plugin(
    pool: &PgPool,
    id: Uuid,
    author_id: Option<Uuid>,
    name: &str,
    slug: &str,
    description: Option<&str>,
    version: &str,
    bundle_url: Option<&str>,
    bundle_hash: Option<&str>,
    permissions: &[String],
) -> Result<ClientPlugin> {
    let perms = serde_json::to_value(permissions)?;
    let row = sqlx::query(
        r#"INSERT INTO client_plugins
               (id, author_id, name, slug, description, version, bundle_url, bundle_hash, permissions)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           RETURNING *"#,
    )
    .bind(id)
    .bind(author_id)
    .bind(name)
    .bind(slug)
    .bind(description)
    .bind(version)
    .bind(bundle_url)
    .bind(bundle_hash)
    .bind(perms)
    .fetch_one(pool)
    .await?;
    Ok(row_to_plugin(&row))
}

// ============================================================================
// User Plugin Installs
// ============================================================================

pub async fn get_user_plugins(pool: &PgPool, user_id: Uuid) -> Result<Vec<UserPluginInstall>> {
    let rows = sqlx::query(
        "SELECT * FROM user_plugin_installs WHERE user_id = $1 ORDER BY installed_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(|r| row_to_user_plugin(r).0).collect())
}

pub async fn install_plugin(
    pool: &PgPool,
    user_id: Uuid,
    plugin_id: Uuid,
) -> Result<UserPluginInstall> {
    sqlx::query("UPDATE client_plugins SET install_count = install_count + 1 WHERE id = $1")
        .bind(plugin_id)
        .execute(pool)
        .await?;
    let row = sqlx::query(
        r#"INSERT INTO user_plugin_installs (user_id, plugin_id)
           VALUES ($1, $2)
           ON CONFLICT (user_id, plugin_id) DO UPDATE SET enabled = user_plugin_installs.enabled
           RETURNING *"#,
    )
    .bind(user_id)
    .bind(plugin_id)
    .fetch_one(pool)
    .await?;
    Ok(row_to_user_plugin(&row).0)
}

pub async fn update_plugin_install(
    pool: &PgPool,
    user_id: Uuid,
    plugin_id: Uuid,
    enabled: Option<bool>,
    settings: Option<serde_json::Value>,
) -> Result<Option<UserPluginInstall>> {
    let row = sqlx::query(
        r#"UPDATE user_plugin_installs SET
               enabled  = COALESCE($3, enabled),
               settings = COALESCE($4, settings)
           WHERE user_id = $1 AND plugin_id = $2
           RETURNING *"#,
    )
    .bind(user_id)
    .bind(plugin_id)
    .bind(enabled)
    .bind(settings)
    .fetch_optional(pool)
    .await?;
    Ok(row.as_ref().map(|r| row_to_user_plugin(r).0))
}

pub async fn uninstall_plugin(pool: &PgPool, user_id: Uuid, plugin_id: Uuid) -> Result<bool> {
    let result = sqlx::query(
        "DELETE FROM user_plugin_installs WHERE user_id = $1 AND plugin_id = $2",
    )
    .bind(user_id)
    .bind(plugin_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

// ============================================================================
// Themes
// ============================================================================

pub async fn list_themes(pool: &PgPool, limit: i64, offset: i64) -> Result<Vec<Theme>> {
    let rows = sqlx::query(
        "SELECT * FROM themes WHERE verified = true ORDER BY install_count DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_theme).collect())
}

pub async fn get_theme_by_id(pool: &PgPool, theme_id: Uuid) -> Result<Option<Theme>> {
    let row = sqlx::query("SELECT * FROM themes WHERE id = $1")
        .bind(theme_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_theme))
}

pub async fn get_theme_by_slug(pool: &PgPool, slug: &str) -> Result<Option<Theme>> {
    let row = sqlx::query("SELECT * FROM themes WHERE slug = $1")
        .bind(slug)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(row_to_theme))
}

pub async fn create_theme(
    pool: &PgPool,
    id: Uuid,
    author_id: Option<Uuid>,
    name: &str,
    slug: &str,
    description: Option<&str>,
    version: &str,
    preview_url: Option<&str>,
    css: &str,
    variables: &serde_json::Value,
) -> Result<Theme> {
    let row = sqlx::query(
        r#"INSERT INTO themes
               (id, author_id, name, slug, description, version, preview_url, css, variables)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           RETURNING *"#,
    )
    .bind(id)
    .bind(author_id)
    .bind(name)
    .bind(slug)
    .bind(description)
    .bind(version)
    .bind(preview_url)
    .bind(css)
    .bind(variables)
    .fetch_one(pool)
    .await?;
    Ok(row_to_theme(&row))
}

// ============================================================================
// User Theme Installs
// ============================================================================

pub async fn get_user_themes(pool: &PgPool, user_id: Uuid) -> Result<Vec<UserThemeInstall>> {
    let rows = sqlx::query(
        "SELECT * FROM user_theme_installs WHERE user_id = $1 ORDER BY installed_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_user_theme).collect())
}

pub async fn install_theme(
    pool: &PgPool,
    user_id: Uuid,
    theme_id: Uuid,
) -> Result<UserThemeInstall> {
    sqlx::query("UPDATE themes SET install_count = install_count + 1 WHERE id = $1")
        .bind(theme_id)
        .execute(pool)
        .await?;
    let row = sqlx::query(
        r#"INSERT INTO user_theme_installs (user_id, theme_id)
           VALUES ($1, $2)
           ON CONFLICT (user_id, theme_id) DO UPDATE SET active = user_theme_installs.active
           RETURNING *"#,
    )
    .bind(user_id)
    .bind(theme_id)
    .fetch_one(pool)
    .await?;
    Ok(row_to_user_theme(&row))
}

pub async fn activate_theme(pool: &PgPool, user_id: Uuid, theme_id: Uuid) -> Result<bool> {
    sqlx::query(
        "UPDATE user_theme_installs SET active = false WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    let result = sqlx::query(
        "UPDATE user_theme_installs SET active = true WHERE user_id = $1 AND theme_id = $2",
    )
    .bind(user_id)
    .bind(theme_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn uninstall_theme(pool: &PgPool, user_id: Uuid, theme_id: Uuid) -> Result<bool> {
    let result =
        sqlx::query("DELETE FROM user_theme_installs WHERE user_id = $1 AND theme_id = $2")
            .bind(user_id)
            .bind(theme_id)
            .execute(pool)
            .await?;
    Ok(result.rows_affected() > 0)
}
