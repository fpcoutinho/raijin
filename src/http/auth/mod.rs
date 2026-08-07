mod middleware;
mod queries;
mod routes;
mod schema;

use axum::routing::post;
use axum::Router;

use crate::AppState;

pub use middleware::{require_auth, AuthUser};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(routes::register))
        .route("/login", post(routes::login))
        .route("/google", post(routes::google))
        .route("/refresh", post(routes::refresh))
        .route("/logout", post(routes::logout))
}
