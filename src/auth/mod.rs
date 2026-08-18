mod google;
mod password;
mod token;

use async_trait::async_trait;

pub use google::GoogleIdentityProvider;
pub use password::{DUMMY_PASSWORD_HASH, PasswordError, hash_password, verify_password};
pub use token::{TokenError, TokenIssuer};

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("falha ao obter as chaves públicas do provedor de identidade: {0}")]
    Jwks(String),

    #[error("falha ao validar o token de identidade: {0}")]
    Verification(String),
}

/// Identidade já verificada por um provedor externo — quem consome não sabe
/// de qual provedor veio, só que a assinatura confere e o e-mail é real.
#[derive(Debug, Clone)]
pub struct VerifiedIdentity {
    pub subject: String,
    pub email: String,
    pub avatar_url: Option<String>,
}

#[async_trait]
pub trait IdentityProvider: Send + Sync {
    /// `email_verified == false` é rejeitado aqui dentro — senão um e-mail
    /// não verificado no provedor tomaria a conta de qualquer usuário de senha.
    async fn verify_id_token(&self, id_token: &str) -> Result<VerifiedIdentity, IdentityError>;
}
