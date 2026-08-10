use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    Unprocessable(String),

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

    #[error(transparent)]
    Token(#[from] crate::auth::TokenError),

    #[error(transparent)]
    Llm(#[from] crate::llm::GenerationError),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::Unprocessable(message) => (StatusCode::UNPROCESSABLE_ENTITY, message.clone()),
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
            // Só emissão passa por `?` nos handlers; verificação (middleware)
            // trata o erro sem propagar, então aqui é sempre falha nossa.
            ApiError::Token(error) => {
                tracing::error!(%error, "erro ao emitir token");
                (StatusCode::INTERNAL_SERVER_ERROR, "Erro interno ao criar a sessão.".to_string())
            }
            // Indisponibilidade de terceiro, mesmo tratamento de IdentityError::Jwks — a
            // mensagem é a mesma do `event: error` do SSE, pro front tratar um caso só.
            ApiError::Llm(error) => {
                tracing::error!(%error, "erro no provedor de IA");
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Provedor de IA indisponível. Tente novamente.".to_string(),
                )
            }
        };

        (status, Json(ErrorBody { error: message })).into_response()
    }
}
