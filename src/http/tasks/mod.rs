mod routes;

use axum::Router;
use axum::routing::post;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/cleanup-sessions", post(routes::cleanup_sessions))
}
