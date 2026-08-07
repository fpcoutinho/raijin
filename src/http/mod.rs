//! Tudo que sabe o que é HTTP — e só o que sabe o que é HTTP.
//! Todas as rotas nascem sob `/api/v1` — versionar desde já.

pub mod error;
mod images;

use axum::Router;

use crate::AppState;

// Assinatura já recebe &AppState porque o middleware de auth (from_fn_with_state)
// precisa do state na hora de montar a layer — chega no passo de auth::router().
pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new().nest("/api/v1", images::router())
}
