//! Tudo que sabe o que é HTTP — e só o que sabe o que é HTTP.
//! Todas as rotas nascem sob `/api/v1` — versionar desde já.

pub(crate) mod auth;
pub mod error;
mod images;
mod tasks;

use axum::middleware::from_fn_with_state;
use axum::Router;

use crate::AppState;

pub use auth::AuthUser;

// Assinatura recebe &AppState porque o middleware de auth (from_fn_with_state)
// precisa do state na hora de montar a layer.
pub fn router(state: &AppState) -> Router<AppState> {
    // route_layer (não layer): middleware que responde 401 cedo só deve rodar
    // quando a rota existe. Com `layer`, um GET numa rota inexistente sem token
    // viraria 401 em vez de 404 — e isso conta pro cliente que a rota existe.
    //
    // route_layer entra em pânico se não houver rota declarada ainda — daí ser
    // chamado sobre images::router() e não sobre um Router::new() vazio. Ele só
    // cobre rotas adicionadas ANTES da chamada: toda feature futura que precisa
    // de auth entra em `protected` antes do .route_layer(...).
    let protected = images::router().route_layer(from_fn_with_state(state.clone(), auth::require_auth));

    // /tasks fica fora do /api/v1 de aplicação e fora do route_layer de
    // AuthUser: quem chama é o EventBridge Scheduler, não um usuário logado.
    Router::new()
        .nest("/api/v1", Router::new().nest("/auth", auth::router()).merge(protected))
        .nest("/tasks", tasks::router())
}
