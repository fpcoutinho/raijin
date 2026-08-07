//! Tudo que sabe o que é HTTP — e só o que sabe o que é HTTP.
//! Todas as rotas nascem sob `/api/v1` — versionar desde já.

mod auth;
pub mod error;
mod images;

use axum::Router;

use crate::AppState;

pub use auth::AuthUser;

// Assinatura já recebe &AppState porque o middleware de auth (from_fn_with_state)
// precisa do state na hora de montar a layer — chega no passo de auth::router().
// Nest de /auth e route_layer no grupo protegido ainda faltam (próximo passo).
pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new().nest("/api/v1", images::router())
}
