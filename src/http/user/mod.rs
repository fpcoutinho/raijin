mod queries;
mod routes;
mod schema;

use axum::Router;
use axum::routing::{get, patch};

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/user/profile",
            get(routes::profile).patch(routes::update_profile),
        )
        .route("/user/password", patch(routes::update_password))
}
