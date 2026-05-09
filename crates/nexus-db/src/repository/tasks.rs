//! Task & checklist repository queries.

use nexus_common::models::collaboration::{ChecklistItem, Task, TaskReminder};
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::TASK_COLS;

// ── Tasks ─────────────────────────────────────────────────────────────────────

pub async fn create_task(
    pool: &AnyPool,
    id: Uuid,
    server_id: Uuid,
    channel_id: Uuid,
    creator_id: Uuid,
    assignee_id: Option<Uuid>,
    title: &str,
    description: Option<&str>,
    priority: &str,
    due_at: Option<&str>,
    position: i32,
) -> Result<Task, sqlx::Error> {
    let q = format!(
        "INSERT INTO tasks (id, server_id, channel_id, creator_id, assignee_id, \
         title, description, priority, due_at, position) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::timestamptz, $10) \
         RETURNING {TASK_COLS}"
    );
    sqlx::query_as::<_, Task>(&q)
        .bind(id.to_string())
        .bind(server_id.to_string())
        .bind(channel_id.to_string())
        .bind(creator_id.to_string())
        .bind(assignee_id.map(|u| u.to_string()))
        .bind(title)
        .bind(description)
        .bind(priority)
        .bind(due_at)
        .bind(position)
        .fetch_one(pool)
        .await
}

pub async fn get_task(pool: &AnyPool, task_id: Uuid) -> Result<Option<Task>, sqlx::Error> {
    let q = format!("SELECT {TASK_COLS} FROM tasks WHERE id = $1");
    sqlx::query_as::<_, Task>(&q)
        .bind(task_id.to_string())
        .fetch_optional(pool)
        .await
}

pub async fn list_tasks_by_channel(
    pool: &AnyPool,
    channel_id: Uuid,
    status: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Task>, sqlx::Error> {
    let q = if status.is_some() {
        format!(
            "SELECT {TASK_COLS} FROM tasks WHERE channel_id = $1 AND status = $2 \
             ORDER BY position, created_at LIMIT $3 OFFSET $4"
        )
    } else {
        format!(
            "SELECT {TASK_COLS} FROM tasks WHERE channel_id = $1 AND ($2::text IS NULL OR TRUE) \
             ORDER BY position, created_at LIMIT $3 OFFSET $4"
        )
    };
    sqlx::query_as::<_, Task>(&q)
        .bind(channel_id.to_string())
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
}

pub async fn list_tasks_by_server(
    pool: &AnyPool,
    server_id: Uuid,
    status: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Task>, sqlx::Error> {
    let q = if status.is_some() {
        format!(
            "SELECT {TASK_COLS} FROM tasks WHERE server_id = $1 AND status = $2 \
             ORDER BY position, created_at LIMIT $3 OFFSET $4"
        )
    } else {
        format!(
            "SELECT {TASK_COLS} FROM tasks WHERE server_id = $1 AND ($2::text IS NULL OR TRUE) \
             ORDER BY position, created_at LIMIT $3 OFFSET $4"
        )
    };
    sqlx::query_as::<_, Task>(&q)
        .bind(server_id.to_string())
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
}

pub async fn list_tasks_assigned_to(
    pool: &AnyPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<Task>, sqlx::Error> {
    let q = format!(
        "SELECT {TASK_COLS} FROM tasks WHERE assignee_id = $1 AND status != 'done' \
         ORDER BY due_at NULLS LAST, created_at LIMIT $2"
    );
    sqlx::query_as::<_, Task>(&q)
        .bind(user_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await
}

pub async fn update_task(
    pool: &AnyPool,
    task_id: Uuid,
    title: Option<&str>,
    description: Option<&str>,
    status: Option<&str>,
    priority: Option<&str>,
    assignee_id: Option<Option<Uuid>>,
    due_at: Option<Option<&str>>,
    position: Option<i32>,
) -> Result<Task, sqlx::Error> {
    let q = format!(
        "UPDATE tasks SET \
         title = COALESCE($2, title), \
         description = COALESCE($3, description), \
         status = COALESCE($4, status), \
         priority = COALESCE($5, priority), \
         assignee_id = CASE WHEN $6 THEN $7::uuid ELSE assignee_id END, \
         due_at = CASE WHEN $8 THEN $9::timestamptz ELSE due_at END, \
         position = COALESCE($10, position), \
         completed_at = CASE WHEN $4 = 'done' THEN NOW() \
                             WHEN $4 IS NOT NULL AND $4 != 'done' THEN NULL \
                             ELSE completed_at END, \
         updated_at = NOW() \
         WHERE id = $1 \
         RETURNING {TASK_COLS}"
    );
    let has_assignee_change = assignee_id.is_some();
    let assignee_val = assignee_id.flatten().map(|u| u.to_string());
    let has_due_change = due_at.is_some();
    let due_val = due_at.flatten();

    sqlx::query_as::<_, Task>(&q)
        .bind(task_id.to_string())
        .bind(title)
        .bind(description)
        .bind(status)
        .bind(priority)
        .bind(has_assignee_change)
        .bind(assignee_val)
        .bind(has_due_change)
        .bind(due_val)
        .bind(position)
        .fetch_one(pool)
        .await
}

pub async fn delete_task(pool: &AnyPool, task_id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM tasks WHERE id = $1")
        .bind(task_id.to_string())
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// ── Checklist Items ───────────────────────────────────────────────────────────

pub async fn add_checklist_item(
    pool: &AnyPool,
    id: Uuid,
    task_id: Uuid,
    content: &str,
    position: i32,
) -> Result<ChecklistItem, sqlx::Error> {
    sqlx::query_as::<_, ChecklistItem>(
        "INSERT INTO task_checklist_items (id, task_id, content, position) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id::text AS id, task_id::text AS task_id, content, checked, position, \
                   created_at::text AS created_at",
    )
    .bind(id.to_string())
    .bind(task_id.to_string())
    .bind(content)
    .bind(position)
    .fetch_one(pool)
    .await
}

pub async fn list_checklist_items(
    pool: &AnyPool,
    task_id: Uuid,
) -> Result<Vec<ChecklistItem>, sqlx::Error> {
    sqlx::query_as::<_, ChecklistItem>(
        "SELECT id::text AS id, task_id::text AS task_id, content, checked, position, \
                created_at::text AS created_at \
         FROM task_checklist_items WHERE task_id = $1 ORDER BY position",
    )
    .bind(task_id.to_string())
    .fetch_all(pool)
    .await
}

pub async fn toggle_checklist_item(
    pool: &AnyPool,
    item_id: Uuid,
    checked: bool,
) -> Result<ChecklistItem, sqlx::Error> {
    sqlx::query_as::<_, ChecklistItem>(
        "UPDATE task_checklist_items SET checked = $2 WHERE id = $1 \
         RETURNING id::text AS id, task_id::text AS task_id, content, checked, position, \
                   created_at::text AS created_at",
    )
    .bind(item_id.to_string())
    .bind(checked)
    .fetch_one(pool)
    .await
}

pub async fn delete_checklist_item(pool: &AnyPool, item_id: Uuid) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM task_checklist_items WHERE id = $1")
        .bind(item_id.to_string())
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

// ── Task Reminders ────────────────────────────────────────────────────────────

pub async fn create_reminder(
    pool: &AnyPool,
    id: Uuid,
    task_id: Uuid,
    user_id: Uuid,
    remind_at: &str,
) -> Result<TaskReminder, sqlx::Error> {
    sqlx::query_as::<_, TaskReminder>(
        "INSERT INTO task_reminders (id, task_id, user_id, remind_at) \
         VALUES ($1, $2, $3, $4::timestamptz) \
         RETURNING id::text AS id, task_id::text AS task_id, user_id::text AS user_id, \
                   remind_at::text AS remind_at, fired, created_at::text AS created_at",
    )
    .bind(id.to_string())
    .bind(task_id.to_string())
    .bind(user_id.to_string())
    .bind(remind_at)
    .fetch_one(pool)
    .await
}

pub async fn list_pending_reminders(
    pool: &AnyPool,
    user_id: Uuid,
) -> Result<Vec<TaskReminder>, sqlx::Error> {
    sqlx::query_as::<_, TaskReminder>(
        "SELECT id::text AS id, task_id::text AS task_id, user_id::text AS user_id, \
                remind_at::text AS remind_at, fired, created_at::text AS created_at \
         FROM task_reminders WHERE user_id = $1 AND fired = FALSE \
         ORDER BY remind_at LIMIT 100",
    )
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await
}

pub async fn delete_reminder(pool: &AnyPool, reminder_id: Uuid) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM task_reminders WHERE id = $1")
        .bind(reminder_id.to_string())
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}
