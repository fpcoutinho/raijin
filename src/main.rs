mod auth;
mod config;
mod domain;
mod http;
mod llm;
mod storage;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderValue, Method};
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{AllowOrigin, CorsLayer};

use auth::{GoogleIdentityProvider, IdentityProvider, TokenIssuer};
use config::Config;
use storage::{ObjectStorage, S3CompatibleStorage};

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub storage: Arc<dyn ObjectStorage>,
    pub tokens: Arc<TokenIssuer>,
    pub identity: Arc<dyn IdentityProvider>,
    /// Janela em que um refresh revogado ainda emite substituto (multi-aba)
    /// em vez de derrubar a cadeia. Copiado de `AuthConfig` pra não passar
    /// `&Config` inteiro pros handlers.
    pub refresh_grace: std::time::Duration,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive("raijin=info".parse().unwrap()))
        .init();

    dotenvy::dotenv().ok();
    let config = Config::from_env().expect("configuração inválida — confira as variáveis de ambiente contra .env.example");

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .expect("falha ao conectar no Postgres");

    let storage: Arc<dyn ObjectStorage> = Arc::new(S3CompatibleStorage::new(&config.storage));
    let tokens = Arc::new(TokenIssuer::new(&config.auth));
    let identity: Arc<dyn IdentityProvider> = Arc::new(GoogleIdentityProvider::new(&config.auth));

    // Gera o hash de referência no boot, não no primeiro login com e-mail
    // desconhecido — senão essa primeira tentativa paga o custo e fica mais
    // lenta que um login real, invertendo o sinal que ela existe pra esconder.
    let _ = &*auth::DUMMY_PASSWORD_HASH;

    let allowed_origins: Vec<HeaderValue> = config
        .auth
        .allowed_origins
        .iter()
        .map(|origin| origin.parse().expect("CORS_ALLOWED_ORIGINS com origem inválida"))
        .collect();

    let refresh_grace = config.auth.refresh_grace;
    let state = AppState { db, storage, tokens, identity, refresh_grace };

    let app = http::router(&state)
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(allowed_origins))
                .allow_credentials(true)
                .allow_headers([AUTHORIZATION, CONTENT_TYPE])
                .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE]),
        )
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .expect("falha ao abrir a porta");

    tracing::info!("raijin ouvindo em {}", config.bind_addr);

    // `SmartIpKeyExtractor` (rate limiting de /auth) cai pro IP da conexão
    // TCP quando não há X-Forwarded-For/Forwarded — sem isso a extração falha
    // sempre que não houver proxy na frente (dev local, sem Lambda). Sob
    // Lambda o API Gateway sempre preenche X-Forwarded-For, então esse
    // fallback nem entra em jogo — mas não custa manter.
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .expect("servidor encerrou com erro");
}
