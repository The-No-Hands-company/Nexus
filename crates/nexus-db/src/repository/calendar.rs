//! Calendar event repository queries.

use nexus_common::models::collaboration::{CalendarEvent, CalendarRsvp};
use sqlx::AnyPool;
use uuid::Uuid;

use crate::select_cols::CALENDAR_EVENT_COLS;

pub async fn create_event(
    pool: &AnyPool,
    id: Uuid,
    server_id: Uuid,
    channel_id: Option<Uuid>,
    creator_id: Uuid,
    title: &str,
    description: Option<&str>,
    location: Option<&str>,
    starts_at: &str,
    ends_at: &str,
    all_day: bool,
    rrule: Option<&str>,
    color: Option<&str>,
) -> Result<CalendarEvent, sqlx::Error> {
    let q = format!(
        "INSERT INTO calendar_events \
         (id, server_id, channel_id, creator_id, title, description, location, \
          starts_at, ends_at, all_day, rrule, color) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8::timestamptz, $9::timestamptz, $10, $11, $12) \
         RETURNING {CALENDAR_EVENT_COLS}"
    );
    sqlx::query_as::<_, CalendarEvent>(&q)
        .bind(id.to_string())
        .bind(server_id.to_string())
        .bind(channel_id.map(|u| u.to_string()))
        .bind(creator_id.to_string())
        .bind(title)
        .bind(description)
        .bind(location)
        .bind(starts_at)
        .bind(ends_at)
        .bind(all_day)
        .bind(rrule)
        .bind(color)
        .fetch_one(pool)
        .await
}

pub async fn get_event(
    pool: &AnyPool,
    event_id: Uuid,
) -> Result<Option<CalendarEvent>, sqlx::Error> {
    let q = format!("SELECT {CALENDAR_EVENT_COLS} FROM calendar_events WHERE id = $1");
    sqlx::query_as::<_, CalendarEvent>(&q)
        .bind(event_id.to_string())
        .fetch_optional(pool)
        .await
}

pub async fn list_events_by_server(
    pool: &AnyPool,
    server_id: Uuid,
    after: &str,
    before: &str,
    limit: i64,
) -> Result<Vec<CalendarEvent>, sqlx::Error> {
    let q = format!(
        "SELECT {CALENDAR_EVENT_COLS} FROM calendar_events \
         WHERE server_id = $1 AND starts_at >= $2::timestamptz AND starts_at <= $3::timestamptz \
         ORDER BY starts_at LIMIT $4"
    );
    sqlx::query_as::<_, CalendarEvent>(&q)
        .bind(server_id.to_string())
        .bind(after)
        .bind(before)
        .bind(limit)
        .fetch_all(pool)
        .await
}

pub async fn update_event(
    pool: &AnyPool,
    event_id: Uuid,
    title: Option<&str>,
    description: Option<&str>,
    location: Option<&str>,
    starts_at: Option<&str>,
    ends_at: Option<&str>,
    all_day: Option<bool>,
    rrule: Option<Option<&str>>,
    color: Option<&str>,
) -> Result<CalendarEvent, sqlx::Error> {
    let has_rrule_change = rrule.is_some();
    let rrule_val = rrule.flatten();
    let q = format!(
        "UPDATE calendar_events SET \
         title = COALESCE($2, title), \
         description = COALESCE($3, description), \
         location = COALESCE($4, location), \
         starts_at = COALESCE($5::timestamptz, starts_at), \
         ends_at = COALESCE($6::timestamptz, ends_at), \
         all_day = COALESCE($7, all_day), \
         rrule = CASE WHEN $8 THEN $9 ELSE rrule END, \
         color = COALESCE($10, color), \
         updated_at = NOW() \
         WHERE id = $1 \
         RETURNING {CALENDAR_EVENT_COLS}"
    );
    sqlx::query_as::<_, CalendarEvent>(&q)
        .bind(event_id.to_string())
        .bind(title)
        .bind(description)
        .bind(location)
        .bind(starts_at)
        .bind(ends_at)
        .bind(all_day)
        .bind(has_rrule_change)
        .bind(rrule_val)
        .bind(color)
        .fetch_one(pool)
        .await
}

pub async fn delete_event(pool: &AnyPool, event_id: Uuid) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM calendar_events WHERE id = $1")
        .bind(event_id.to_string())
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

// ── RSVP ──────────────────────────────────────────────────────────────────────

pub async fn upsert_rsvp(
    pool: &AnyPool,
    event_id: Uuid,
    user_id: Uuid,
    status: &str,
) -> Result<CalendarRsvp, sqlx::Error> {
    sqlx::query_as::<_, CalendarRsvp>(
        "INSERT INTO calendar_rsvps (event_id, user_id, status) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (event_id, user_id) DO UPDATE SET status = EXCLUDED.status \
         RETURNING event_id::text AS event_id, user_id::text AS user_id, status, \
                   created_at::text AS created_at",
    )
    .bind(event_id.to_string())
    .bind(user_id.to_string())
    .bind(status)
    .fetch_one(pool)
    .await
}

pub async fn list_rsvps(pool: &AnyPool, event_id: Uuid) -> Result<Vec<CalendarRsvp>, sqlx::Error> {
    sqlx::query_as::<_, CalendarRsvp>(
        "SELECT event_id::text AS event_id, user_id::text AS user_id, status, \
                created_at::text AS created_at \
         FROM calendar_rsvps WHERE event_id = $1 ORDER BY created_at",
    )
    .bind(event_id.to_string())
    .fetch_all(pool)
    .await
}

pub async fn delete_rsvp(
    pool: &AnyPool,
    event_id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM calendar_rsvps WHERE event_id = $1 AND user_id = $2")
        .bind(event_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

// ── ICS Export ────────────────────────────────────────────────────────────────

/// Generate an ICS (iCalendar) string for events in a server within a date range.
pub async fn export_ics(
    pool: &AnyPool,
    server_id: Uuid,
    after: &str,
    before: &str,
) -> Result<String, sqlx::Error> {
    let events = list_events_by_server(pool, server_id, after, before, 500).await?;

    let mut ics =
        String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Nexus//Calendar//EN\r\n");
    for ev in &events {
        ics.push_str("BEGIN:VEVENT\r\n");
        ics.push_str(&format!("UID:{}\r\n", ev.id));
        ics.push_str(&format!("SUMMARY:{}\r\n", ics_escape(&ev.title)));
        if let Some(desc) = &ev.description {
            ics.push_str(&format!("DESCRIPTION:{}\r\n", ics_escape(desc)));
        }
        if let Some(loc) = &ev.location {
            ics.push_str(&format!("LOCATION:{}\r\n", ics_escape(loc)));
        }
        ics.push_str(&format!("DTSTART:{}\r\n", to_ics_dt(&ev.starts_at)));
        ics.push_str(&format!("DTEND:{}\r\n", to_ics_dt(&ev.ends_at)));
        if let Some(rrule) = &ev.rrule {
            ics.push_str(&format!("RRULE:{}\r\n", rrule));
        }
        ics.push_str("END:VEVENT\r\n");
    }
    ics.push_str("END:VCALENDAR\r\n");
    Ok(ics)
}

fn ics_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

fn to_ics_dt(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y%m%dT%H%M%SZ").to_string()
}
