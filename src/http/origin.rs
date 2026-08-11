use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use subtle::ConstantTimeEq;

use crate::AppState;

pub const ORIGIN_HEADER: &str = "x-origin-auth";

/// A Function URL é pública: OAC exigiria o navegador mandar o SHA256 do corpo
/// em todo POST (a Lambda não aceita payload sem assinatura), o que
/// contaminaria cada chamada mutante do frontend. O header injetado pelo
/// CloudFront é o que faz o acesso direto à Function URL não levar a lugar
/// nenhum.
pub async fn require_cloudfront_origin(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let Some(expected) = state.origin_shared_secret.as_deref() else {
        return next.run(request).await;
    };

    let received = request
        .headers()
        .get(ORIGIN_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    if !bool::from(received.as_bytes().ct_eq(expected.as_bytes())) {
        return StatusCode::FORBIDDEN.into_response();
    }

    next.run(request).await
}
