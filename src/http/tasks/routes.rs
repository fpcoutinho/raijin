use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Json;
use serde::Serialize;
use subtle::ConstantTimeEq;

use crate::http::auth::queries::delete_expired_refresh_tokens;
use crate::http::error::ApiError;
use crate::AppState;

#[derive(Serialize)]
pub struct CleanupResponse {
    deleted: u64,
}

fn task_authorized(headers: &HeaderMap, expected: &str) -> bool {
    let Some(value) = headers.get("x-task-token").and_then(|v| v.to_str().ok()) else {
        return false;
    };

    value.as_bytes().ct_eq(expected.as_bytes()).into()
}

pub async fn cleanup_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CleanupResponse>, ApiError> {
    if !task_authorized(&headers, &state.task_token) {
        return Err(ApiError::Unauthorized);
    }

    let deleted = delete_expired_refresh_tokens(&state.db).await?;
    Ok(Json(CleanupResponse { deleted }))
}
