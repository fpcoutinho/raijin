use axum::extract::{FromRequestParts, Request, State};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

use crate::http::error::ApiError;
use crate::AppState;

/// Identidade autenticada, injetada nas extensions por `require_auth` e lida
/// pelos handlers via extractor.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
}

pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // O 401 nunca diz qual etapa falhou (header ausente, esquema errado, JWT
    // malformado, assinatura ruim, expirado, typ errado) — todas colapsam
    // aqui. Detalhe vai só pro log dentro de `verify_access`, não pra resposta.
    let claims = bearer_token(&request)
        .and_then(|token| state.tokens.verify_access(token).ok())
        .ok_or(ApiError::Unauthorized)?;

    request.extensions_mut().insert(AuthUser { id: claims.sub, email: claims.email });
    Ok(next.run(request).await)
}

/// RFC 7235: o esquema do Authorization é case-insensitive.
fn bearer_token(request: &Request) -> Option<&str> {
    let value = request.headers().get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    scheme.eq_ignore_ascii_case("Bearer").then(|| token.trim())
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<AuthUser>().cloned().ok_or(ApiError::Unauthorized)
    }
}
