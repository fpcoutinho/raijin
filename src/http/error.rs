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

    #[error("{0}")]
    Conflict(String),

    #[error("não autenticado")]
    Unauthorized,

    #[error("credenciais inválidas")]
    InvalidCredentials,

    #[error(transparent)]
    Identity(#[from] crate::auth::IdentityError),

    #[error(transparent)]
    Password(#[from] crate::auth::PasswordError),
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
            ApiError::Conflict(message) => (StatusCode::CONFLICT, message.clone()),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Autenticação necessária.".to_string()),
            ApiError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "E-mail ou senha inválidos.".to_string()),
            // As duas variantes de IdentityError mapeiam pra status diferentes:
            // token ruim é problema do cliente, JWKS fora do ar é nosso/da Google.
            ApiError::Identity(crate::auth::IdentityError::Verification(error)) => {
                tracing::debug!(%error, "ID token da Google rejeitado");
                (StatusCode::UNAUTHORIZED, "Não foi possível validar o login pelo Google.".to_string())
            }
            ApiError::Identity(crate::auth::IdentityError::Jwks(error)) => {
                tracing::error!(%error, "erro ao obter o JWKS");
                (StatusCode::SERVICE_UNAVAILABLE, "Login pelo Google indisponível no momento. Tente novamente.".to_string())
            }
            ApiError::Password(error) => {
                tracing::error!(%error, "erro no subsistema de senha");
                (StatusCode::INTERNAL_SERVER_ERROR, "Erro interno ao processar as credenciais.".to_string())
            }
        };

        (status, Json(ErrorBody { error: message })).into_response()
    }
}
