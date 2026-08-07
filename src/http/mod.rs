//! Tudo que sabe o que é HTTP — e só o que sabe o que é HTTP.
//! Todas as rotas nascem sob `/api/v1` — versionar desde já.

pub mod error;
mod images;

use axum::Router;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new().nest("/api/v1", images::router())
}
