//! # nexus-api
//!
//! REST API layer for Nexus. Provides HTTP endpoints for all CRUD operations,
//! authentication, and client-facing functionality.

pub mod auth;
pub mod middleware;
pub mod routes;

use axum::Router;
use nexus_db::Database;
use std::sync::Arc;

/// Shared application state available to all route handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
}

/// Build the complete API router with all routes and middleware.
pub fn build_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .merge(routes::auth::router())
        .merge(routes::users::router())
        .merge(routes::servers::router())
        .merge(routes::channels::router())
        .merge(routes::health::router());

    Router::new()
        .nest("/api/v1", api_routes)
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(tower_http::compression::CompressionLayer::new())
        .with_state(Arc::new(state))
}
