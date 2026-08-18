pub(crate) mod queries;
mod routes;
mod schema;

use axum::Router;
use axum::routing::{patch, post};

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/reports/{report_id}/circuits",
            post(routes::create_circuit).get(routes::list_circuits),
        )
        .route(
            "/reports/{report_id}/circuits/{circuit_id}",
            patch(routes::update_circuit).delete(routes::delete_circuit),
        )
}
