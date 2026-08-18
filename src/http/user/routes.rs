use axum::extract::State;
use axum::response::Json;
use axum_extra::extract::cookie::CookieJar;

use crate::AppState;
use crate::auth;
use crate::domain::User;
use crate::http::AuthUser;
use crate::http::auth::queries as auth_queries;
use crate::http::auth::routes::issue_session;
use crate::http::auth::schema::SessionResponse;
use crate::http::error::ApiError;

use super::queries;
use super::schema::{UpdatePasswordRequest, UpdateProfileRequest};

pub async fn profile(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<User>, ApiError> {
    let profile = auth_queries::find_user_by_id(&state.db, user.id)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Json(profile))
}

pub async fn update_profile(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<UpdateProfileRequest>,
) -> Result<Json<User>, ApiError> {
    body.validate()?;

    let updated = queries::update_profile(&state.db, user.id, &body)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Json(updated))
}

/// Devolve sessão nova porque o passo de revogação derruba **todos** os refresh
/// tokens do usuário, inclusive o de quem acabou de trocar a senha.
pub async fn update_password(
    State(state): State<AppState>,
    jar: CookieJar,
    user: AuthUser,
    Json(body): Json<UpdatePasswordRequest>,
) -> Result<(CookieJar, Json<SessionResponse>), ApiError> {
    let current = auth_queries::find_user_by_id(&state.db, user.id)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    let phc = current.password_hash.clone().ok_or_else(|| {
        ApiError::Conflict("Esta conta usa login pelo Google. Entre com o Google.".to_string())
    })?;

    if !auth::verify_password(body.current_password.clone(), phc).await? {
        return Err(ApiError::Unprocessable(
            "Senha atual incorreta.".to_string(),
        ));
    }

    body.validate()?;

    let new_hash = auth::hash_password(body.new_password).await?;
    queries::update_password_hash(&state.db, current.id, &new_hash).await?;
    auth_queries::revoke_all_refresh_tokens(&state.db, current.id).await?;

    let (jar, session) = issue_session(&state, jar, current).await?;
    Ok((jar, Json(session)))
}
