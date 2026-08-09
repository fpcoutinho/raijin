pub(crate) mod queries;
mod routes;
mod schema;

use axum::Router;
use axum::routing::{get, patch, post};

use crate::AppState;

pub(crate) use routes::require_ownership;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/reports", post(routes::create_report).get(routes::list_reports))
        .route(
            "/reports/{report_id}",
            get(routes::get_report)
                .patch(routes::update_report)
                .delete(routes::delete_report),
        )
        .route(
            "/reports/{report_id}/inspection-planning",
            patch(routes::update_inspection_planning),
        )
        .route(
            "/reports/{report_id}/external-influences",
            patch(routes::update_external_influences),
        )
        .route(
            "/reports/{report_id}/qualitative-assessment",
            patch(routes::update_qualitative_assessment),
        )
        .route(
            "/reports/{report_id}/quantitative-assessment",
            patch(routes::update_quantitative_assessment),
        )
        .route(
            "/reports/{report_id}/document-content",
            patch(routes::update_document_content),
        )
}
