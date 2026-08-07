use serde::{Deserialize, Serialize};

use crate::domain::User;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct GoogleLoginRequest {
    pub id_token: String,
}

/// O refresh token não aparece aqui — vai só no cookie httpOnly.
#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub access_token: String,
    pub token_type: &'static str,
    /// Segundos até o access token vencer; o `itui` agenda a renovação por
    /// este número em vez de decodificar o JWT no navegador.
    pub expires_in: i64,
    pub user: User,
}
