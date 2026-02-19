//! Criterion microbenchmarks for nexus-api hot paths.
//!
//! Run with:
//!   cargo bench -p nexus-api
//!
//! HTML reports are written to `target/criterion/`.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use serde_json::json;

// ── JSON serialisation ────────────────────────────────────────────────────────

/// Benchmark serialising a representative chat-message payload to JSON.
fn bench_message_serialise(c: &mut Criterion) {
    let msg = json!({
        "id": "01929a5e-6e1b-7000-9c4a-dead00000001",
        "channel_id": "01929a5e-6e1b-7000-9c4a-dead00000002",
        "author_id": "01929a5e-6e1b-7000-9c4a-dead00000003",
        "content": "Hello, world! This is a realistic-length chat message that exercises the serialiser.",
        "created_at": "2025-01-01T00:00:00Z",
        "edited_at": null,
        "attachments": [],
        "reactions": [],
        "mentions": []
    });

    c.bench_function("message/serialise", |b| {
        b.iter(|| serde_json::to_string(black_box(&msg)).unwrap())
    });
}

/// Benchmark deserialising the same payload.
fn bench_message_deserialise(c: &mut Criterion) {
    let raw = r#"{
        "id":"01929a5e-6e1b-7000-9c4a-dead00000001",
        "channel_id":"01929a5e-6e1b-7000-9c4a-dead00000002",
        "author_id":"01929a5e-6e1b-7000-9c4a-dead00000003",
        "content":"Hello, world! This is a realistic-length chat message that exercises the serialiser.",
        "created_at":"2025-01-01T00:00:00Z",
        "edited_at":null,
        "attachments":[],
        "reactions":[],
        "mentions":[]
    }"#;

    c.bench_function("message/deserialise", |b| {
        b.iter(|| {
            let _: serde_json::Value = serde_json::from_str(black_box(raw)).unwrap();
        })
    });
}

// ── UUID / ID generation ──────────────────────────────────────────────────────

fn bench_uuid_v7(c: &mut Criterion) {
    c.bench_function("id/uuid_v7_generate", |b| {
        b.iter(|| uuid::Uuid::now_v7())
    });
}

fn bench_uuid_parse(c: &mut Criterion) {
    let s = "01929a5e-6e1b-7000-9c4a-dead00000001";
    c.bench_function("id/uuid_parse", |b| {
        b.iter(|| uuid::Uuid::parse_str(black_box(s)).unwrap())
    });
}

// ── Argon2 password hashing ───────────────────────────────────────────────────

fn bench_argon2_hash(c: &mut Criterion) {
    use argon2::{Argon2, PasswordHasher, password_hash::{SaltString, rand_core::OsRng}};

    // Use a less-intensive config for benchmarking (still realistic for dev)
    c.bench_function("auth/argon2_hash", |b| {
        b.iter(|| {
            let salt = SaltString::generate(&mut OsRng);
            Argon2::default()
                .hash_password(black_box(b"hunter2-password-bench"), &salt)
                .unwrap()
                .to_string()
        })
    });
}

fn bench_argon2_verify(c: &mut Criterion) {
    use argon2::{Argon2, PasswordHasher, PasswordVerifier, password_hash::{SaltString, rand_core::OsRng}};

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(b"hunter2-password-bench", &salt)
        .unwrap()
        .to_string();

    let parsed = argon2::PasswordHash::new(&hash).unwrap();

    c.bench_function("auth/argon2_verify", |b| {
        b.iter(|| {
            Argon2::default()
                .verify_password(black_box(b"hunter2-password-bench"), &parsed)
                .unwrap()
        })
    });
}

// ── JWT encoding ──────────────────────────────────────────────────────────────

fn bench_jwt_encode(c: &mut Criterion) {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
        iat: usize,
    }

    let key = EncodingKey::from_secret(b"bench-secret-key-32-bytes-padded!!");
    let header = Header::new(Algorithm::HS256);
    let claims = Claims {
        sub: "01929a5e-6e1b-7000-9c4a-dead00000001".into(),
        exp: 9_999_999_999,
        iat: 1_700_000_000,
    };

    c.bench_function("auth/jwt_encode", |b| {
        b.iter(|| encode(black_box(&header), black_box(&claims), black_box(&key)).unwrap())
    });
}

fn bench_jwt_decode(c: &mut Criterion) {
    use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
        iat: usize,
    }

    let secret = b"bench-secret-key-32-bytes-padded!!";
    let enc_key = EncodingKey::from_secret(secret);
    let dec_key = DecodingKey::from_secret(secret);
    let header = Header::new(Algorithm::HS256);
    let claims = Claims {
        sub: "01929a5e-6e1b-7000-9c4a-dead00000001".into(),
        exp: 9_999_999_999,
        iat: 1_700_000_000,
    };
    let token = encode(&header, &claims, &enc_key).unwrap();
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = false;

    c.bench_function("auth/jwt_decode", |b| {
        b.iter(|| {
            decode::<Claims>(black_box(&token), black_box(&dec_key), &validation).unwrap()
        })
    });
}

// ── Payload size scaling ──────────────────────────────────────────────────────

/// Benchmark JSON serialisation at different message-content sizes.
fn bench_message_size_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("message/size_scaling");

    for size in [64usize, 256, 1024, 4096] {
        let content = "x".repeat(size);
        let msg = json!({
            "id": "01929a5e-6e1b-7000-9c4a-dead00000001",
            "content": content,
        });

        group.bench_with_input(BenchmarkId::from_parameter(size), &msg, |b, m| {
            b.iter(|| serde_json::to_string(black_box(m)).unwrap())
        });
    }

    group.finish();
}

// ── criterion entrypoints ─────────────────────────────────────────────────────

criterion_group!(
    serialisation,
    bench_message_serialise,
    bench_message_deserialise,
    bench_message_size_scaling,
);

criterion_group!(
    ids,
    bench_uuid_v7,
    bench_uuid_parse,
);

criterion_group!(
    auth,
    bench_argon2_hash,
    bench_argon2_verify,
    bench_jwt_encode,
    bench_jwt_decode,
);

criterion_main!(serialisation, ids, auth);
