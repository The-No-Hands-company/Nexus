//! MeiliSearch integration — full-text search client and indexing helpers.
//!
//! Wraps `meilisearch-sdk` to provide:
//! - Server-scoped message search
//! - Background sync queue processing
//! - Index management (create, configure)

use anyhow::{Context, Result};
use meilisearch_sdk::{client::Client, search::SearchResults};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================
// Search document shapes
// ============================================================

/// Message document indexed in MeiliSearch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDocument {
    /// Primary key (UUID string — MeiliSearch requires a string/int primary key)
    pub id: String,
    pub channel_id: String,
    pub server_id: Option<String>,
    pub author_id: String,
    pub author_username: String,
    pub content: String,
    pub has_attachments: bool,
    pub has_embeds: bool,
    /// Unix timestamp for range-filter support
    pub created_at: i64,
}

/// Server document (for cross-server search in future versions).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerDocument {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub member_count: u32,
}

// ============================================================
// SearchClient
// ============================================================

/// MeiliSearch client wrapper.
#[derive(Clone)]
pub struct SearchClient {
    inner: Client,
}

impl SearchClient {
    /// Construct from URL + master key.
    pub fn new(url: &str, api_key: &str) -> Self {
        Self {
            inner: Client::new(url, Some(api_key)).expect("Failed to create MeiliSearch client"),
        }
    }

    // ------------------------------------------------------------------
    // Index bootstrapping
    // ------------------------------------------------------------------

    /// Create and configure indexes on first run.
    pub async fn bootstrap_indexes(&self) -> Result<()> {
        self.setup_messages_index().await?;
        self.setup_servers_index().await?;
        Ok(())
    }

    async fn setup_messages_index(&self) -> Result<()> {
        // Attempt to create; ignore if already exists
        if let Ok(task) = self
            .inner
            .create_index("messages", Some("id"))
            .await
        {
            // Wait for task to complete (non-critical, best effort)
            let _ = self.inner.wait_for_task(task, None, None).await;
        }

        let index = self.inner.index("messages");

        // Configure searchable attributes
        index
            .set_searchable_attributes(["content", "author_username"])
            .await
            .context("Failed to set searchable attributes for messages index")?;

        // Configure filterable attributes
        index
            .set_filterable_attributes([
                "channel_id",
                "server_id",
                "author_id",
                "has_attachments",
                "has_embeds",
                "created_at",
            ])
            .await
            .context("Failed to set filterable attributes for messages index")?;

        // Configure sortable attributes
        index
            .set_sortable_attributes(["created_at"])
            .await
            .context("Failed to set sortable attributes for messages index")?;

        Ok(())
    }

    async fn setup_servers_index(&self) -> Result<()> {
        if let Ok(task) = self
            .inner
            .create_index("servers", Some("id"))
            .await
        {
            let _ = self.inner.wait_for_task(task, None, None).await;
        }

        let index = self.inner.index("servers");
        index
            .set_searchable_attributes(["name", "description"])
            .await
            .context("Failed to set searchable attributes for servers index")?;

        Ok(())
    }

    // ------------------------------------------------------------------
    // Message indexing
    // ------------------------------------------------------------------

    /// Index (or update) a single message document.
    pub async fn index_message(&self, doc: MessageDocument) -> Result<()> {
        let index = self.inner.index("messages");
        index
            .add_or_update(&[doc], Some("id"))
            .await
            .context("Failed to index message in MeiliSearch")?;
        Ok(())
    }

    /// Index a batch of messages (more efficient for bulk sync).
    pub async fn index_messages_batch(&self, docs: Vec<MessageDocument>) -> Result<()> {
        if docs.is_empty() {
            return Ok(());
        }
        let index = self.inner.index("messages");
        index
            .add_or_update(&docs, Some("id"))
            .await
            .context("Failed to batch-index messages in MeiliSearch")?;
        Ok(())
    }

    /// Delete a message from the index.
    pub async fn delete_message(&self, message_id: Uuid) -> Result<()> {
        let index = self.inner.index("messages");
        index
            .delete_document(message_id.to_string())
            .await
            .context("Failed to delete message from MeiliSearch")?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Search
    // ------------------------------------------------------------------

    /// Full-text search messages within a server.
    pub async fn search_messages(
        &self,
        query: &str,
        server_id: Option<Uuid>,
        channel_id: Option<Uuid>,
        author_id: Option<Uuid>,
        limit: usize,
        offset: usize,
    ) -> Result<SearchResults<MessageDocument>> {
        let index = self.inner.index("messages");

        // Build filter string
        let mut filters: Vec<String> = Vec::new();
        if let Some(sid) = server_id {
            filters.push(format!("server_id = \"{}\"", sid));
        }
        if let Some(cid) = channel_id {
            filters.push(format!("channel_id = \"{}\"", cid));
        }
        if let Some(aid) = author_id {
            filters.push(format!("author_id = \"{}\"", aid));
        }

        let filter_str = filters.join(" AND ");

        let mut search = index.search();
        search.with_query(query)
            .with_limit(limit)
            .with_offset(offset)
            .with_sort(&["created_at:desc"]);

        if !filter_str.is_empty() {
            search.with_filter(&filter_str);
        }

        search
            .execute::<MessageDocument>()
            .await
            .context("MeiliSearch query failed")
    }

    // ------------------------------------------------------------------
    // Sync queue processing
    // ------------------------------------------------------------------

    /// Process pending sync queue entries from the database.
    /// Call this from a background task every few seconds.
    pub async fn process_sync_queue(&self, pool: &sqlx::PgPool) -> Result<()> {
        #[derive(sqlx::FromRow)]
        struct QueueRow {
            id: i64,
            operation: String,
            index_name: String,
            document_id: String,
            payload: Option<serde_json::Value>,
        }

        let rows = sqlx::query_as::<_, QueueRow>(
            r#"
            SELECT id, operation, index_name, document_id, payload
            FROM search_sync_queue
            WHERE processed = false
            ORDER BY created_at
            LIMIT 100
            "#,
        )
        .fetch_all(pool)
        .await
        .context("Failed to fetch search sync queue")?;

        for row in rows {
            let result: Result<()> = async {
                match row.operation.as_str() {
                    "index" | "update" => {
                        if row.index_name == "messages" {
                            if let Some(payload) = row.payload {
                                let doc: MessageDocument =
                                    serde_json::from_value(payload)
                                        .context("Failed to deserialise MessageDocument")?;
                                self.index_message(doc).await?;
                            }
                        }
                    }
                    "delete" => {
                        if row.index_name == "messages" {
                            let id = Uuid::parse_str(&row.document_id)
                                .context("Invalid UUID in sync queue")?;
                            self.delete_message(id).await?;
                        }
                    }
                    other => {
                        tracing::warn!("Unknown search sync operation: {}", other);
                    }
                }
                Ok(())
            }
            .await;

            if let Err(e) = result {
                tracing::error!(
                    queue_id = row.id,
                    error = %e,
                    "Failed to process search sync queue entry"
                );
            } else {
                sqlx::query(
                    "UPDATE search_sync_queue SET processed = true WHERE id = $1",
                )
                .bind(row.id)
                .execute(pool)
                .await
                .context("Failed to mark sync queue item processed")?;
            }
        }

        Ok(())
    }

    /// Enqueue a message to be indexed (called after message creation/edit).
    pub async fn enqueue_message_index(
        pool: &sqlx::PgPool,
        message_id: Uuid,
        doc: &MessageDocument,
    ) -> Result<()> {
        let payload = serde_json::to_value(doc).context("Failed to serialise MessageDocument")?;
        sqlx::query(
            r#"
            INSERT INTO search_sync_queue (operation, index_name, document_id, payload)
            VALUES ('index', 'messages', $1, $2)
            "#,
        )
        .bind(message_id.to_string())
        .bind(payload)
        .execute(pool)
        .await
        .context("Failed to enqueue message for search indexing")?;
        Ok(())
    }

    /// Enqueue a message deletion from the index.
    pub async fn enqueue_message_delete(pool: &sqlx::PgPool, message_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO search_sync_queue (operation, index_name, document_id)
            VALUES ('delete', 'messages', $1)
            "#,
        )
        .bind(message_id.to_string())
        .execute(pool)
        .await
        .context("Failed to enqueue message delete for search")?;
        Ok(())
    }
}
