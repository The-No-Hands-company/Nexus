//! Task & checklist routes — 16-01: Integrated Tasks & Checklists.
//!
//! POST   /servers/:id/tasks                          — Create task
//! GET    /servers/:id/tasks                          — List tasks (Kanban/list)
//! GET    /servers/:id/tasks/:task_id                 — Get single task
//! PATCH  /servers/:id/tasks/:task_id                 — Update task
//! DELETE /servers/:id/tasks/:task_id                 — Delete task
//! GET    /users/@me/tasks                            — My assigned tasks
//!
//! POST   /tasks/:task_id/checklist                   — Add checklist item
//! GET    /tasks/:task_id/checklist                   — List checklist items
//! PATCH  /tasks/:task_id/checklist/:item_id          — Toggle checklist item
//! DELETE /tasks/:task_id/checklist/:item_id          — Delete checklist item
//!
//! POST   /tasks/:task_id/reminders                   — Set reminder
//! GET    /users/@me/reminders                        — List my pending reminders
//! DELETE /reminders/:id                              — Delete reminder

use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::{delete, get, patch, post},
    Json, Router,
};
use nexus_common::error::{NexusError, NexusResult};
use nexus_common::snowflake;
use nexus_db::repository::tasks;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Server-scoped tasks
        .route("/servers/{server_id}/tasks", post(create_task).get(list_tasks))
        .route(
            "/servers/{server_id}/tasks/{task_id}",
            get(get_task).patch(update_task).delete(delete_task),
        )
        // User's assigned tasks across all servers
        .route("/users/@me/tasks", get(my_tasks))
        // Checklist CRUD
        .route(
            "/tasks/{task_id}/checklist",
            post(add_checklist_item).get(list_checklist),
        )
        .route(
            "/tasks/{task_id}/checklist/{item_id}",
            patch(toggle_checklist).delete(delete_checklist_item),
        )
        // Reminders
        .route("/tasks/{task_id}/reminders", post(create_reminder))
        .route("/users/@me/reminders", get(my_reminders))
        .route("/reminders/{reminder_id}", delete(delete_reminder))
        .route_layer(middleware::from_fn(crate::middleware::combined_auth_middleware))
}

// ── Request / Response ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CreateTaskRequest {
    channel_id: Uuid,
    title: String,
    description: Option<String>,
    priority: Option<String>,
    assignee_id: Option<Uuid>,
    due_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateTaskRequest {
    title: Option<String>,
    description: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    assignee_id: Option<Option<Uuid>>,
    due_at: Option<Option<String>>,
    position: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    status: Option<String>,
    channel_id: Option<Uuid>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AddChecklistRequest {
    content: String,
    position: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct ToggleRequest {
    checked: bool,
}

#[derive(Debug, Deserialize)]
struct ReminderRequest {
    remind_at: String,
}

#[derive(Debug, Serialize)]
struct TaskBoard {
    open: Vec<nexus_common::models::collaboration::Task>,
    in_progress: Vec<nexus_common::models::collaboration::Task>,
    done: Vec<nexus_common::models::collaboration::Task>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn create_task(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(server_id): Path<Uuid>,
    Json(body): Json<CreateTaskRequest>,
) -> NexusResult<Json<nexus_common::models::collaboration::Task>> {
    if body.title.is_empty() || body.title.len() > 200 {
        return Err(NexusError::Validation {
            message: "Title must be 1-200 characters".into(),
        });
    }
    let priority = body.priority.as_deref().unwrap_or("medium");
    if !matches!(priority, "low" | "medium" | "high" | "urgent") {
        return Err(NexusError::Validation {
            message: "Priority must be low, medium, high, or urgent".into(),
        });
    }

    let task = tasks::create_task(
        &state.db.pool,
        snowflake::generate_id(),
        server_id,
        body.channel_id,
        ctx.user_id,
        body.assignee_id,
        &body.title,
        body.description.as_deref(),
        priority,
        body.due_at.as_deref(),
        0,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(task))
}

async fn list_tasks(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<Uuid>,
    Query(q): Query<ListQuery>,
) -> NexusResult<Json<Vec<nexus_common::models::collaboration::Task>>> {
    let limit = q.limit.unwrap_or(100).min(200);
    let offset = q.offset.unwrap_or(0);

    let results = if let Some(channel_id) = q.channel_id {
        tasks::list_tasks_by_channel(
            &state.db.pool,
            channel_id,
            q.status.as_deref(),
            limit,
            offset,
        )
        .await
    } else {
        tasks::list_tasks_by_server(
            &state.db.pool,
            server_id,
            q.status.as_deref(),
            limit,
            offset,
        )
        .await
    };

    results.map(Json).map_err(|e| NexusError::Internal(e.into()))
}

async fn get_task(
    State(state): State<Arc<AppState>>,
    Path((_server_id, task_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<nexus_common::models::collaboration::Task>> {
    tasks::get_task(&state.db.pool, task_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?
        .map(Json)
        .ok_or(NexusError::NotFound {
            resource: "Task".into(),
        })
}

async fn update_task(
    State(state): State<Arc<AppState>>,
    Path((_server_id, task_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateTaskRequest>,
) -> NexusResult<Json<nexus_common::models::collaboration::Task>> {
    if let Some(ref s) = body.status {
        if !matches!(s.as_str(), "open" | "in_progress" | "done" | "cancelled") {
            return Err(NexusError::Validation {
                message: "Invalid status".into(),
            });
        }
    }
    let task = tasks::update_task(
        &state.db.pool,
        task_id,
        body.title.as_deref(),
        body.description.as_deref(),
        body.status.as_deref(),
        body.priority.as_deref(),
        body.assignee_id,
        body.due_at
            .as_ref()
            .map(|o| o.as_deref()),
        body.position,
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;

    Ok(Json(task))
}

async fn delete_task(
    State(state): State<Arc<AppState>>,
    Path((_server_id, task_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    let deleted = tasks::delete_task(&state.db.pool, task_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if !deleted {
        return Err(NexusError::NotFound {
            resource: "Task".into(),
        });
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn my_tasks(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> NexusResult<Json<Vec<nexus_common::models::collaboration::Task>>> {
    tasks::list_tasks_assigned_to(&state.db.pool, ctx.user_id, 100)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

// ── Checklist ─────────────────────────────────────────────────────────────────

async fn add_checklist_item(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<Uuid>,
    Json(body): Json<AddChecklistRequest>,
) -> NexusResult<Json<nexus_common::models::collaboration::ChecklistItem>> {
    if body.content.is_empty() || body.content.len() > 500 {
        return Err(NexusError::Validation {
            message: "Content must be 1-500 characters".into(),
        });
    }
    let item = tasks::add_checklist_item(
        &state.db.pool,
        snowflake::generate_id(),
        task_id,
        &body.content,
        body.position.unwrap_or(0),
    )
    .await
    .map_err(|e| NexusError::Internal(e.into()))?;
    Ok(Json(item))
}

async fn list_checklist(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<Uuid>,
) -> NexusResult<Json<Vec<nexus_common::models::collaboration::ChecklistItem>>> {
    tasks::list_checklist_items(&state.db.pool, task_id)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn toggle_checklist(
    State(state): State<Arc<AppState>>,
    Path((_task_id, item_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ToggleRequest>,
) -> NexusResult<Json<nexus_common::models::collaboration::ChecklistItem>> {
    tasks::toggle_checklist_item(&state.db.pool, item_id, body.checked)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn delete_checklist_item(
    State(state): State<Arc<AppState>>,
    Path((_task_id, item_id)): Path<(Uuid, Uuid)>,
) -> NexusResult<Json<serde_json::Value>> {
    let deleted = tasks::delete_checklist_item(&state.db.pool, item_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if !deleted {
        return Err(NexusError::NotFound {
            resource: "ChecklistItem".into(),
        });
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── Reminders ─────────────────────────────────────────────────────────────────

async fn create_reminder(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
    Path(task_id): Path<Uuid>,
    Json(body): Json<ReminderRequest>,
) -> NexusResult<Json<nexus_common::models::collaboration::TaskReminder>> {
    tasks::create_reminder(
        &state.db.pool,
        snowflake::generate_id(),
        task_id,
        ctx.user_id,
        &body.remind_at,
    )
    .await
    .map(Json)
    .map_err(|e| NexusError::Internal(e.into()))
}

async fn my_reminders(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<AuthContext>,
) -> NexusResult<Json<Vec<nexus_common::models::collaboration::TaskReminder>>> {
    tasks::list_pending_reminders(&state.db.pool, ctx.user_id)
        .await
        .map(Json)
        .map_err(|e| NexusError::Internal(e.into()))
}

async fn delete_reminder(
    State(state): State<Arc<AppState>>,
    Path(reminder_id): Path<Uuid>,
) -> NexusResult<Json<serde_json::Value>> {
    let deleted = tasks::delete_reminder(&state.db.pool, reminder_id)
        .await
        .map_err(|e| NexusError::Internal(e.into()))?;
    if !deleted {
        return Err(NexusError::NotFound {
            resource: "Reminder".into(),
        });
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}
