mod auth;
mod config;
mod domain;
mod http;
mod llm;
mod storage;

use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;

use config::Config;
use storage::{ObjectStorage, S3CompatibleStorage};

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub storage: Arc<dyn ObjectStorage>,
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

    let state = AppState { db, storage };

    let app = http::router()
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .expect("falha ao abrir a porta");

    tracing::info!("raijin ouvindo em {}", config.bind_addr);
    axum::serve(listener, app).await.expect("servidor encerrou com erro");
}
