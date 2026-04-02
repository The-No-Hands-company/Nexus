//! Background import worker.
//!
//! Claims pending import jobs and processes payloads from the `metadata` JSON
//! into channels/messages so import jobs become actionable rather than stubs.

use nexus_common::models::ecosystem::ImportJob;
use nexus_common::snowflake;
use nexus_db::repository::{channels, import_jobs, messages};
use sqlx::AnyPool;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Spawn the import worker loop.
pub fn spawn_import_worker(pool: AnyPool, mut shutdown_rx: broadcast::Receiver<()>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(15));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let pending = match import_jobs::list_pending_import_jobs(&pool, 20).await {
                        Ok(rows) => rows,
                        Err(e) => {
                            tracing::warn!(error = %e, subsystem = "imports", "failed to list pending import jobs");
                            continue;
                        }
                    };

                    for job in pending {
                        let claimed = match import_jobs::try_claim_import_job(&pool, job.id).await {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::warn!(error = %e, job_id = %job.id, subsystem = "imports", "failed to claim import job");
                                continue;
                            }
                        };
                        if !claimed {
                            continue;
                        }

                        match process_import_job(&pool, &job).await {
                            Ok((imported, total)) => {
                                if let Err(e) = import_jobs::mark_import_completed(&pool, job.id, imported, total).await {
                                    tracing::warn!(
                                        error = %e,
                                        job_id = %job.id,
                                        imported,
                                        total,
                                        subsystem = "imports",
                                        "failed to mark import job completed"
                                    );
                                } else {
                                    tracing::info!(
                                        job_id = %job.id,
                                        server_id = %job.server_id,
                                        source = %job.source_platform,
                                        imported,
                                        total,
                                        subsystem = "imports",
                                        "import job completed"
                                    );
                                }
                            }
                            Err((message, imported, total)) => {
                                if let Err(e) = import_jobs::mark_import_failed(&pool, job.id, imported, total, &message).await {
                                    tracing::warn!(
                                        error = %e,
                                        job_id = %job.id,
                                        subsystem = "imports",
                                        "failed to mark import job failed"
                                    );
                                } else {
                                    tracing::warn!(
                                        job_id = %job.id,
                                        server_id = %job.server_id,
                                        source = %job.source_platform,
                                        imported,
                                        total,
                                        error = %message,
                                        subsystem = "imports",
                                        "import job failed"
                                    );
                                }
                            }
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    tracing::debug!(subsystem = "imports", "import worker shutting down");
                    break;
                }
            }
        }
    });
}

/// Process an import job and return `(imported_items, total_items)`.
async fn process_import_job(
    pool: &AnyPool,
    job: &ImportJob,
) -> Result<(i32, i32), (String, i32, i32)> {
    let source = job.source_platform.to_ascii_lowercase();
    if source != "discord" && source != "slack" && source != "matrix" {
        return Err((
            format!("unsupported source platform: {}", job.source_platform),
            0,
            0,
        ));
    }

    let items = extract_messages(&job.metadata);
    let total = items.len() as i32;

    if items.is_empty() {
        return Err((
            "metadata contained no importable messages (expected metadata.messages[] or metadata.channels[].messages[])".to_string(),
            0,
            0,
        ));
    }

    let channel = match resolve_or_create_import_channel(pool, job.server_id, &source).await {
        Ok(id) => id,
        Err(e) => {
            return Err((format!("failed to prepare import channel: {e}"), 0, total));
        }
    };

    let mut imported = 0i32;
    for content in items {
        let content = content.trim();
        if content.is_empty() {
            continue;
        }

        let message_id = snowflake::generate_id();
        match messages::create_message(
            pool,
            message_id,
            channel,
            job.user_id,
            content,
            0,
            None,
            None,
            &[],
            &[],
            false,
        )
        .await
        {
            Ok(_) => imported += 1,
            Err(e) => {
                return Err((
                    format!("failed while inserting imported message: {e}"),
                    imported,
                    total,
                ));
            }
        }
    }

    Ok((imported, total))
}

async fn resolve_or_create_import_channel(
    pool: &AnyPool,
    server_id: Uuid,
    source: &str,
) -> Result<Uuid, sqlx::Error> {
    let target_name = format!("imported-{source}");

    let existing = channels::list_server_channels(pool, server_id)
        .await?
        .into_iter()
        .find(|c| {
            c.name
                .as_deref()
                .map(|n| n.eq_ignore_ascii_case(&target_name))
                .unwrap_or(false)
        });

    if let Some(ch) = existing {
        return Ok(ch.id);
    }

    let created = channels::create_channel(
        pool,
        Uuid::new_v4(),
        Some(server_id),
        None,
        "text",
        Some(&target_name),
        Some("Imported messages"),
        0,
    )
    .await?;

    Ok(created.id)
}

fn extract_messages(metadata: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();

    if let Some(arr) = metadata.get("messages").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(s) = extract_message_text(item) {
                out.push(s);
            }
        }
    }

    if let Some(channels) = metadata.get("channels").and_then(|v| v.as_array()) {
        for channel in channels {
            if let Some(msgs) = channel.get("messages").and_then(|v| v.as_array()) {
                for item in msgs {
                    if let Some(s) = extract_message_text(item) {
                        out.push(s);
                    }
                }
            }
        }
    }

    if out.is_empty() {
        if let Some(text_dump) = metadata.get("text_dump").and_then(|v| v.as_str()) {
            for line in text_dump.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                }
            }
        }
    }

    out
}

fn extract_message_text(value: &serde_json::Value) -> Option<String> {
    if let Some(content) = value.get("content").and_then(|v| v.as_str()) {
        return Some(content.to_string());
    }
    if let Some(text) = value.get("text").and_then(|v| v.as_str()) {
        return Some(text.to_string());
    }
    if let Some(body) = value.get("body").and_then(|v| v.as_str()) {
        return Some(body.to_string());
    }
    if let Some(s) = value.as_str() {
        return Some(s.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_messages_reads_messages_array() {
        let metadata = serde_json::json!({
            "messages": [
                {"content": "hello"},
                {"text": "world"},
                "raw"
            ]
        });

        let out = extract_messages(&metadata);
        assert_eq!(out, vec!["hello", "world", "raw"]);
    }

    #[test]
    fn extract_messages_reads_nested_channels() {
        let metadata = serde_json::json!({
            "channels": [
                {
                    "messages": [
                        {"body": "first"},
                        {"content": "second"}
                    ]
                }
            ]
        });

        let out = extract_messages(&metadata);
        assert_eq!(out, vec!["first", "second"]);
    }

    #[test]
    fn extract_messages_falls_back_to_text_dump() {
        let metadata = serde_json::json!({
            "text_dump": "line one\n\nline two\n"
        });

        let out = extract_messages(&metadata);
        assert_eq!(out, vec!["line one", "line two"]);
    }
}
