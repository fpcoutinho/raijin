mod queries;
mod routes;
mod schema;

use axum::Router;
use axum::routing::post;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/reports/{report_id}/images",
            post(routes::create_upload).get(routes::list_images),
        )
        .route(
            "/reports/{report_id}/images/{image_id}/confirm",
            post(routes::confirm_upload),
        )
}
