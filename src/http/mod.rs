pub(crate) mod auth;
mod circuits;
pub mod error;
mod images;
mod origin;
pub(crate) mod reports;
mod tasks;

use axum::middleware::from_fn_with_state;
use axum::Router;

use crate::AppState;

pub use auth::AuthUser;

// Assinatura recebe &AppState porque o middleware de auth (from_fn_with_state)
// precisa do state na hora de montar a layer.
pub fn router(state: &AppState) -> Router<AppState> {
    let protected = images::router()
        .merge(reports::router())
        .merge(circuits::router())
        .route_layer(from_fn_with_state(state.clone(), auth::require_auth));

    // O fallback existe pra que caminho não-casado sob /api/v1 seja atendido
    // aqui dentro e passe pela trava de origem — sem ele o `nest` o entrega ao
    // router de fora, e o 404 responderia a quem chamou a Function URL por
    // fora do CloudFront. `/tasks` fica de fora da trava: o EventBridge invoca
    // a Lambda direto, sem passar pelo CloudFront.
    let public = Router::new()
        .nest("/auth", auth::router())
        .merge(protected)
        .fallback(|| async { axum::http::StatusCode::NOT_FOUND })
        .layer(from_fn_with_state(state.clone(), origin::require_cloudfront_origin));

    Router::new().nest("/api/v1", public).nest("/tasks", tasks::router())
}
