use base64::Engine;
use chrono::{DateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::AuthConfig;

/// Discriminador de tipo do JWT. Sem carga hoje (refresh é opaco, então não há
/// confusão possível), mas futura verificação de e-mail e reset de senha vão assinar
/// com o mesmo `JWT_SECRET` — sem isso, um desses tokens vira Bearer válido.
const ACCESS_TOKEN_TYPE: &str = "access";

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("falha ao emitir o token: {0}")]
    Issue(String),

    #[error("token inválido: {0}")]
    Invalid(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: Uuid,
    pub email: String,
    typ: String,
    pub iat: i64,
    pub exp: i64,
}

pub struct RefreshToken {
    pub plain: String,
    pub hash: Vec<u8>,
    pub expires_at: DateTime<Utc>,
}

pub struct TokenIssuer {
    encoding: EncodingKey,
    decoding: DecodingKey,
    validation: Validation,
    access_ttl: chrono::Duration,
    refresh_ttl: chrono::Duration,
}

impl TokenIssuer {
    pub fn new(config: &AuthConfig) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_aud = false;
        validation.set_required_spec_claims(&["exp", "sub"]);

        Self {
            encoding: EncodingKey::from_secret(config.jwt_secret.as_bytes()),
            decoding: DecodingKey::from_secret(config.jwt_secret.as_bytes()),
            validation,
            access_ttl: chrono::Duration::from_std(config.access_token_ttl)
                .expect("access_token_ttl fora do range representável"),
            refresh_ttl: chrono::Duration::from_std(config.refresh_token_ttl)
                .expect("refresh_token_ttl fora do range representável"),
        }
    }

    /// Retorna o token e os segundos até expirar (pro `expires_in` da resposta).
    pub fn issue_access(&self, user_id: Uuid, email: &str) -> Result<(String, i64), TokenError> {
        let now = Utc::now();
        let claims = AccessClaims {
            sub: user_id,
            email: email.to_string(),
            typ: ACCESS_TOKEN_TYPE.to_string(),
            iat: now.timestamp(),
            exp: (now + self.access_ttl).timestamp(),
        };
        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|error| TokenError::Issue(error.to_string()))?;
        Ok((token, self.access_ttl.num_seconds()))
    }

    pub fn verify_access(&self, token: &str) -> Result<AccessClaims, TokenError> {
        let data = decode::<AccessClaims>(token, &self.decoding, &self.validation)
            .map_err(|error| TokenError::Invalid(error.to_string()))?;
        if data.claims.typ != ACCESS_TOKEN_TYPE {
            return Err(TokenError::Invalid("tipo de token incorreto".to_string()));
        }
        Ok(data.claims)
    }

    /// 32 bytes do CSPRNG do SO, codificados em base64 URL-safe.
    pub fn issue_refresh(&self) -> RefreshToken {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        let plain = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
        let hash = self.refresh_hash(&plain);
        RefreshToken {
            plain,
            hash,
            expires_at: Utc::now() + self.refresh_ttl,
        }
    }

    /// Hasheia a string codificada, não os bytes crus — uma única forma de
    /// entrada, pra rotação e lookup nunca hashearem coisas diferentes.
    pub fn refresh_hash(&self, plain: &str) -> Vec<u8> {
        Sha256::digest(plain.as_bytes()).to_vec()
    }
}
