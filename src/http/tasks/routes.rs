use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
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

/// Autenticação de máquina, separada da de usuário: EventBridge não tem
/// sessão, e um `AuthUser` aqui obrigaria um usuário-robô no banco.
fn task_authorized(headers: &HeaderMap, expected: &str) -> bool {
    let Some(value) = headers.get("x-task-token").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    // `==` em &str sai cedo no primeiro byte diferente e vaza o prefixo por
    // tempo de resposta — ct_eq compara em tempo constante.
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
