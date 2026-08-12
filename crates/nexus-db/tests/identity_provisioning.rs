//! Provisioning a local user row from an ecosystem identity, against a real
//! database.
//!
//! Postgres, not SQLite: the repository binds ids as `$1::uuid`, which SQLite
//! rejects outright ("unrecognized token"), and Postgres is what production
//! runs. An in-memory SQLite harness would be testing a dialect this code path
//! never meets.
//!
//! Ignored by default because it needs a server. Run it with a scratch
//! database — never the live one, the first thing it does is TRUNCATE:
//!
//! ```text
//! docker exec nexus-systems-postgres-1 psql -U nexus -d postgres \
//!   -c 'CREATE DATABASE nexus_chat_test'
//! # Derived from the deployed config rather than written out here, so no
//! # credential-shaped string lives in the source at all:
//! export NEXUS_TEST_DATABASE_URL="$(
//!   sed -n 's#^NEXUS__DATABASE__URL=##p' deploy/production/nexus-chat.env \
//!     | sed 's#/[^/]*$#/nexus_chat_test#')"
//! cargo test -p nexus-db --test identity_provisioning -- --ignored --test-threads=1
//! ```

use nexus_db::repository::users::{local_id_for_subject, provision_from_identity};
use sqlx::Row;

async fn scratch_pool() -> sqlx::AnyPool {
    let url = std::env::var("NEXUS_TEST_DATABASE_URL")
        .expect("set NEXUS_TEST_DATABASE_URL to a scratch database (see the module docs)");
    assert!(
        url.contains("test"),
        "refusing to run: NEXUS_TEST_DATABASE_URL must name a scratch database, got {url}"
    );

    sqlx::any::install_default_drivers();
    let pool = sqlx::any::AnyPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("scratch database connects");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations apply");

    sqlx::query("TRUNCATE users CASCADE")
        .execute(&pool)
        .await
        .expect("users truncates");

    pool
}

async fn count_users(pool: &sqlx::AnyPool) -> i64 {
    sqlx::query("SELECT COUNT(*) AS n FROM users")
        .fetch_one(pool)
        .await
        .expect("count")
        .get::<i64, _>("n")
}

#[tokio::test]
#[ignore = "needs a scratch Postgres in NEXUS_TEST_DATABASE_URL"]
async fn provisions_on_first_sight_and_is_idempotent() {
    let pool = scratch_pool().await;
    let subject = "usr-msosh4ui-2";

    let first = provision_from_identity(&pool, subject, "founder", Some("founder@tnhc.dev"))
        .await
        .expect("first provision succeeds");
    assert_eq!(count_users(&pool).await, 1, "first sight inserts a row");

    let second = provision_from_identity(&pool, subject, "founder", Some("founder@tnhc.dev"))
        .await
        .expect("second provision succeeds");

    assert_eq!(first, second, "the same account maps to the same local id");
    assert_eq!(
        count_users(&pool).await,
        1,
        "provisioning twice must not create a second account"
    );
}

#[tokio::test]
#[ignore = "needs a scratch Postgres in NEXUS_TEST_DATABASE_URL"]
async fn the_local_id_is_derived_from_the_subject() {
    // Every foreign key in the schema points at users.id, so the derivation
    // being the *only* source of that id is what keeps a returning user
    // attached to their own messages.
    let pool = scratch_pool().await;
    let subject = "usr-derivation-check";

    let id = provision_from_identity(&pool, subject, "deriv", None)
        .await
        .expect("provision");

    assert_eq!(id, local_id_for_subject(subject));

    let row = sqlx::query("SELECT id::text AS id, external_id FROM users WHERE id = $1::uuid")
        .bind(id.to_string())
        .fetch_one(&pool)
        .await
        .expect("row exists");
    assert_eq!(row.get::<String, _>("id"), id.to_string());
    assert_eq!(
        row.get::<String, _>("external_id"),
        subject,
        "the ecosystem account id is recorded so the mapping is legible"
    );
}

#[tokio::test]
#[ignore = "needs a scratch Postgres in NEXUS_TEST_DATABASE_URL"]
async fn two_accounts_get_two_rows() {
    let pool = scratch_pool().await;

    let a = provision_from_identity(&pool, "usr-aaa", "alice", Some("a@tnhc.dev"))
        .await
        .expect("provision a");
    let b = provision_from_identity(&pool, "usr-bbb", "bob", Some("b@tnhc.dev"))
        .await
        .expect("provision b");

    assert_ne!(a, b);
    assert_eq!(count_users(&pool).await, 2);
}

#[tokio::test]
#[ignore = "needs a scratch Postgres in NEXUS_TEST_DATABASE_URL"]
async fn a_taken_username_does_not_block_provisioning() {
    // username is UNIQUE. A collision must not lock the user out of the app —
    // the id is the identity, the username is a label.
    let pool = scratch_pool().await;

    let squatter = provision_from_identity(&pool, "usr-first", "founder", None)
        .await
        .expect("first holder of the name");
    let newcomer = provision_from_identity(&pool, "usr-second", "founder", None)
        .await
        .expect("a taken username must not fail the request");

    assert_ne!(squatter, newcomer);
    assert_eq!(count_users(&pool).await, 2);

    let name = sqlx::query("SELECT username FROM users WHERE id = $1::uuid")
        .bind(newcomer.to_string())
        .fetch_one(&pool)
        .await
        .expect("row exists")
        .get::<String, _>("username");
    assert_ne!(name, "founder", "the second row must be disambiguated");
    assert!(name.starts_with("founder-"), "unexpected fallback: {name}");
}

#[tokio::test]
#[ignore = "needs a scratch Postgres in NEXUS_TEST_DATABASE_URL"]
async fn provisioned_accounts_have_no_usable_password() {
    let pool = scratch_pool().await;

    let id = provision_from_identity(&pool, "usr-nopass", "nopass", None)
        .await
        .expect("provision");

    let hash = sqlx::query("SELECT password_hash FROM users WHERE id = $1::uuid")
        .bind(id.to_string())
        .fetch_one(&pool)
        .await
        .expect("row exists")
        .get::<String, _>("password_hash");

    assert!(
        argon2::password_hash::PasswordHash::new(&hash).is_err(),
        "a provisioned account must have no hash any password could match, got {hash:?}"
    );
}
