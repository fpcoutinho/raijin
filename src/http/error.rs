//! Erro da API e sua serialização HTTP.
//! Detalhe de causa (erro do Postgres, do S3) vai pro log, nunca pro corpo da resposta.
//! Mensagem de erro de storage costuma vazar bucket, endpoint e chave.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    NotFound(String),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::BadRequest(message) => (StatusCode::BAD_REQUEST, message.clone()),
            ApiError::NotFound(message) => (StatusCode::NOT_FOUND, message.clone()),
            ApiError::Database(error) => {
                tracing::error!(%error, "erro de banco");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Erro interno ao acessar os dados.".to_string(),
                )
            }
            ApiError::Storage(error) => {
                tracing::error!(%error, "erro de storage");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Erro interno ao acessar o armazenamento de imagens.".to_string(),
                )
            }
        };

        (status, Json(ErrorBody { error: message })).into_response()
    }
}
