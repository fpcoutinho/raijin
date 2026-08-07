mod routes;

use axum::routing::post;
use axum::Router;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/cleanup-sessions", post(routes::cleanup_sessions))
}
