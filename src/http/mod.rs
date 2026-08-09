pub(crate) mod auth;
mod circuits;
pub mod error;
mod images;
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

    Router::new()
        .nest("/api/v1", Router::new().nest("/auth", auth::router()).merge(protected))
        .nest("/tasks", tasks::router())
}
