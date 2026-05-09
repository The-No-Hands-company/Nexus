//! Scylla message sink used by the outbox worker.

use anyhow::{Context, Result};
use nexus_common::config::ScyllaConfig;
use nexus_db::search::MessageDocument;
use scylla::{Session, SessionBuilder};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ScyllaSink {
    session: Arc<Session>,
    keyspace: String,
}

impl ScyllaSink {
    pub async fn connect(cfg: &ScyllaConfig) -> Result<Self> {
        let nodes: Vec<String> = cfg
            .nodes
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        if nodes.is_empty() {
            anyhow::bail!("No Scylla nodes configured");
        }

        let mut builder = SessionBuilder::new();
        for node in &nodes {
            builder = builder.known_node(node);
        }

        let session = builder
            .build()
            .await
            .context("Failed to connect to Scylla")?;

        let keyspace = cfg.keyspace.clone();

        // Bootstrap keyspace + tables used for message dual-write.
        session
            .query_unpaged(
                format!(
                    "CREATE KEYSPACE IF NOT EXISTS {} \
                     WITH REPLICATION = {{ 'class': 'SimpleStrategy', 'replication_factor': 1 }}",
                    keyspace
                ),
                (),
            )
            .await
            .context("Failed to create Scylla keyspace")?;

        session
            .query_unpaged(
                format!(
                    "CREATE TABLE IF NOT EXISTS {}.messages_by_channel (\
                        channel_id text,\
                        created_at_epoch bigint,\
                        message_id text,\
                        server_id text,\
                        author_id text,\
                        author_username text,\
                        content text,\
                        has_attachments boolean,\
                        has_embeds boolean,\
                        PRIMARY KEY ((channel_id), created_at_epoch, message_id)\
                     ) WITH CLUSTERING ORDER BY (created_at_epoch DESC, message_id DESC)",
                    keyspace
                ),
                (),
            )
            .await
            .context("Failed to create Scylla messages_by_channel table")?;

        session
            .query_unpaged(
                format!(
                    "CREATE TABLE IF NOT EXISTS {}.messages_by_id (\
                        message_id text PRIMARY KEY,\
                        channel_id text,\
                        created_at_epoch bigint\
                     )",
                    keyspace
                ),
                (),
            )
            .await
            .context("Failed to create Scylla messages_by_id table")?;

        Ok(Self {
            session: Arc::new(session),
            keyspace,
        })
    }

    pub async fn upsert_message(&self, doc: &MessageDocument) -> Result<()> {
        self.session
            .query_unpaged(
                format!(
                    "INSERT INTO {}.messages_by_channel \
                     (channel_id, created_at_epoch, message_id, server_id, author_id, author_username, content, has_attachments, has_embeds) \
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    self.keyspace
                ),
                (
                    doc.channel_id.clone(),
                    doc.created_at,
                    doc.id.clone(),
                    doc.server_id.clone().unwrap_or_default(),
                    doc.author_id.clone(),
                    doc.author_username.clone(),
                    doc.content.clone(),
                    doc.has_attachments,
                    doc.has_embeds,
                ),
            )
            .await
            .context("Scylla upsert into messages_by_channel failed")?;

        self.session
            .query_unpaged(
                format!(
                    "INSERT INTO {}.messages_by_id (message_id, channel_id, created_at_epoch) VALUES (?, ?, ?)",
                    self.keyspace
                ),
                (doc.id.clone(), doc.channel_id.clone(), doc.created_at),
            )
            .await
            .context("Scylla upsert into messages_by_id failed")?;

        Ok(())
    }

    pub async fn delete_message(
        &self,
        message_id: Uuid,
        payload: Option<&serde_json::Value>,
    ) -> Result<()> {
        let mid = message_id.to_string();

        // Fast path: if payload has channel/time, delete directly.
        if let Some(value) = payload
            && let Ok(doc) = serde_json::from_value::<MessageDocument>(value.clone())
        {
            self.delete_by_doc(&doc).await?;
            return Ok(());
        }

        // Fallback: lookup via messages_by_id then delete from channel table.
        let rows = self
            .session
            .query_unpaged(
                format!(
                    "SELECT channel_id, created_at_epoch FROM {}.messages_by_id WHERE message_id = ?",
                    self.keyspace
                ),
                (mid.clone(),),
            )
            .await
            .context("Scylla select from messages_by_id failed")?
            .into_rows_result()
            .context("Scylla select rows decode failed")?;

        for row in rows
            .rows::<(String, i64)>()
            .context("Scylla typed row decode failed")?
        {
            let (channel_id, created_at_epoch) = row.context("Scylla row read failed")?;
            self.session
                .query_unpaged(
                    format!(
                        "DELETE FROM {}.messages_by_channel \
                         WHERE channel_id = ? AND created_at_epoch = ? AND message_id = ?",
                        self.keyspace
                    ),
                    (channel_id, created_at_epoch, mid.clone()),
                )
                .await
                .context("Scylla delete from messages_by_channel failed")?;
        }

        self.session
            .query_unpaged(
                format!(
                    "DELETE FROM {}.messages_by_id WHERE message_id = ?",
                    self.keyspace
                ),
                (mid,),
            )
            .await
            .context("Scylla delete from messages_by_id failed")?;

        Ok(())
    }

    async fn delete_by_doc(&self, doc: &MessageDocument) -> Result<()> {
        self.session
            .query_unpaged(
                format!(
                    "DELETE FROM {}.messages_by_channel \
                     WHERE channel_id = ? AND created_at_epoch = ? AND message_id = ?",
                    self.keyspace
                ),
                (doc.channel_id.clone(), doc.created_at, doc.id.clone()),
            )
            .await
            .context("Scylla delete-by-doc from messages_by_channel failed")?;

        self.session
            .query_unpaged(
                format!(
                    "DELETE FROM {}.messages_by_id WHERE message_id = ?",
                    self.keyspace
                ),
                (doc.id.clone(),),
            )
            .await
            .context("Scylla delete-by-doc from messages_by_id failed")?;

        Ok(())
    }
}
