use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};

use crate::AppState;
use crate::auth::{self, DUMMY_PASSWORD_HASH};
use crate::domain::User;
use crate::http::error::ApiError;

use super::queries;
use super::queries::RotationOutcome;
use super::schema::{GoogleLoginRequest, LoginRequest, RegisterRequest, SessionResponse};

const REFRESH_COOKIE: &str = "refresh_token";

/// `users.email` é UNIQUE case-sensitive no Postgres; sem normalizar nos três
/// pontos de entrada, o vínculo de conta por e-mail nunca dispara.
fn normalize_email(raw: &str) -> String {
    raw.trim().to_lowercase()
}

pub async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, CookieJar, Json<SessionResponse>), ApiError> {
    let email = normalize_email(&body.email);

    if let Some(existing) = queries::find_user_by_email(&state.db, &email).await? {
        let message = if existing.google_id.is_some() {
            "Esta conta usa login pelo Google. Entre com o Google."
        } else {
            "E-mail já cadastrado."
        };
        return Err(ApiError::Conflict(message.to_string()));
    }

    let password_hash = auth::hash_password(body.password).await?;
    let full_name = body
        .full_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty());
    let user =
        queries::insert_user_with_password(&state.db, &email, &password_hash, full_name).await?;

    let (jar, session) = issue_session(&state, jar, user).await?;
    Ok((StatusCode::CREATED, jar, Json(session)))
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> Result<(CookieJar, Json<SessionResponse>), ApiError> {
    let email = normalize_email(&body.email);
    let found = queries::find_user_by_email(&state.db, &email).await?;

    // Verifica um hash sempre, mesmo sem usuário ou sem senha — senão o
    // tempo de resposta vira oráculo de "esse e-mail tem conta".
    let phc = found
        .as_ref()
        .and_then(|user| user.password_hash.clone())
        .unwrap_or_else(|| DUMMY_PASSWORD_HASH.clone());
    let matches = auth::verify_password(body.password, phc).await?;

    let user = found
        .filter(|user| user.password_hash.is_some())
        .filter(|_| matches)
        .ok_or(ApiError::InvalidCredentials)?;

    let (jar, session) = issue_session(&state, jar, user).await?;
    Ok((jar, Json(session)))
}

pub async fn google(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<GoogleLoginRequest>,
) -> Result<(CookieJar, Json<SessionResponse>), ApiError> {
    let identity = state.identity.verify_id_token(&body.id_token).await?;
    let email = normalize_email(&identity.email);

    let already_linked = queries::find_user_by_google_id(&state.db, &identity.subject)
        .await?
        .is_some();
    let (user, had_password) = if already_linked {
        let user = queries::update_google_user(
            &state.db,
            &identity.subject,
            &email,
            identity.avatar_url.as_deref(),
        )
        .await?;
        (user, false)
    } else {
        queries::link_google_to_existing(
            &state.db,
            &email,
            &identity.subject,
            identity.avatar_url.as_deref(),
        )
        .await?
    };

    // Takeover: a conta tinha só senha, o Google (verificado) assume. O
    // invasor que pré-registrou não fica com sessão viva.
    if had_password {
        queries::revoke_all_refresh_tokens(&state.db, user.id).await?;
        tracing::warn!(user_id = %user.id, "conta com senha convertida pra Google — refresh tokens revogados");
    }

    let (jar, session) = issue_session(&state, jar, user).await?;
    Ok((jar, Json(session)))
}

pub async fn refresh(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<SessionResponse>), ApiError> {
    let token = jar
        .get(REFRESH_COOKIE)
        .map(|cookie| cookie.value().to_string())
        .ok_or(ApiError::Unauthorized)?;
    let old_hash = state.tokens.refresh_hash(&token);
    let new_refresh = state.tokens.issue_refresh();

    let outcome = queries::rotate_refresh_token(
        &state.db,
        &old_hash,
        &new_refresh.hash,
        new_refresh.expires_at,
        state.refresh_grace,
    )
    .await?;

    let user_id = match outcome {
        RotationOutcome::Rotated { user_id } | RotationOutcome::GraceReplay { user_id } => user_id,
        RotationOutcome::Reused { user_id } => {
            queries::revoke_all_refresh_tokens(&state.db, user_id).await?;
            tracing::warn!(user_id = %user_id, "refresh token reutilizado — cadeia revogada");
            return Err(ApiError::Unauthorized);
        }
        RotationOutcome::Invalid => return Err(ApiError::Unauthorized),
    };

    let user = queries::find_user_by_id(&state.db, user_id)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    let jar = jar.add(refresh_cookie(new_refresh.plain, new_refresh.expires_at));
    let (access_token, expires_in) = state.tokens.issue_access(user.id, &user.email)?;

    Ok((
        jar,
        Json(SessionResponse {
            access_token,
            token_type: "Bearer",
            expires_in,
            user,
        }),
    ))
}

pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, StatusCode), ApiError> {
    if let Some(cookie) = jar.get(REFRESH_COOKIE) {
        let hash = state.tokens.refresh_hash(cookie.value());
        // Idempotente por desenho: 204 tenha ou não afetado linha, senão
        // logout vira oráculo de "esse token existe".
        queries::revoke_refresh_token_by_hash(&state.db, &hash).await?;
    }

    // `.path(...)` explícito: sem ele, a remoção funciona hoje só por
    // coincidência (o path default do RFC 6265 pra um POST em
    // /api/v1/auth/logout calha de bater com /api/v1/auth).
    let jar = jar.remove(Cookie::build(REFRESH_COOKIE).path("/api/v1/auth").build());
    Ok((jar, StatusCode::NO_CONTENT))
}

/// Emite access+refresh, persiste o hash do refresh e monta o cookie. Todo
/// caminho que cria sessão passa por aqui.
pub(crate) async fn issue_session(
    state: &AppState,
    jar: CookieJar,
    user: User,
) -> Result<(CookieJar, SessionResponse), ApiError> {
    let refresh = state.tokens.issue_refresh();
    queries::insert_refresh_token(&state.db, user.id, &refresh.hash, refresh.expires_at).await?;

    let jar = jar.add(refresh_cookie(refresh.plain, refresh.expires_at));
    let (access_token, expires_in) = state.tokens.issue_access(user.id, &user.email)?;

    Ok((
        jar,
        SessionResponse {
            access_token,
            token_type: "Bearer",
            expires_in,
            user,
        },
    ))
}

fn refresh_cookie(value: String, expires_at: chrono::DateTime<chrono::Utc>) -> Cookie<'static> {
    let max_age = time::Duration::seconds((expires_at - chrono::Utc::now()).num_seconds().max(0));
    Cookie::build((REFRESH_COOKIE, value))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::None)
        .path("/api/v1/auth")
        .max_age(max_age)
        .build()
}
