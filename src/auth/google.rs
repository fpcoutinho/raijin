use std::time::{Duration, Instant};

use async_trait::async_trait;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::config::AuthConfig;

use super::{IdentityError, IdentityProvider, VerifiedIdentity};

const GOOGLE_JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";
const GOOGLE_ISSUERS: [&str; 2] = ["accounts.google.com", "https://accounts.google.com"];

#[derive(Debug, Deserialize)]
struct GoogleIdTokenClaims {
    sub: String,
    email: String,
    #[serde(default)]
    email_verified: bool,
    picture: Option<String>,
}

struct CachedJwks {
    keys: JwkSet,
    expires_at: Instant,
}

pub struct GoogleIdentityProvider {
    http: reqwest::Client,
    client_id: String,
    fallback_ttl: Duration,
    cache: RwLock<Option<CachedJwks>>,
}

impl GoogleIdentityProvider {
    pub fn new(config: &AuthConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            client_id: config.google_client_id.clone(),
            fallback_ttl: config.jwks_fallback_ttl,
            cache: RwLock::new(None),
        }
    }

    /// Double-checked locking: evita que uma rajada de logins simultâneos
    /// dispare N requests à Google quando o cache expira.
    async fn jwks(&self, force_refresh: bool) -> Result<JwkSet, IdentityError> {
        if !force_refresh
            && let Some(cached) = self.cache.read().await.as_ref()
            && cached.expires_at > Instant::now()
        {
            return Ok(cached.keys.clone());
        }

        let mut guard = self.cache.write().await;
        if !force_refresh
            && let Some(cached) = guard.as_ref()
            && cached.expires_at > Instant::now()
        {
            return Ok(cached.keys.clone());
        }

        match self.fetch_jwks().await {
            Ok(fresh) => {
                let keys = fresh.keys.clone();
                *guard = Some(fresh);
                Ok(keys)
            }
            // Google fora do ar: serve chave velha em vez de derrubar todo login.
            Err(error) => match guard.as_ref() {
                Some(stale) => {
                    tracing::warn!(%error, "JWKS da Google indisponível, usando cache expirado");
                    Ok(stale.keys.clone())
                }
                None => Err(error),
            },
        }
    }

    async fn fetch_jwks(&self) -> Result<CachedJwks, IdentityError> {
        let response = self
            .http
            .get(GOOGLE_JWKS_URL)
            .send()
            .await
            .map_err(|error| IdentityError::Jwks(error.to_string()))?;

        let ttl = response
            .headers()
            .get(reqwest::header::CACHE_CONTROL)
            .and_then(|value| value.to_str().ok())
            .and_then(parse_max_age)
            .map(|secs| Duration::from_secs(secs).clamp(Duration::from_secs(5 * 60), Duration::from_secs(24 * 60 * 60)))
            .unwrap_or(self.fallback_ttl);

        let keys = response
            .json::<JwkSet>()
            .await
            .map_err(|error| IdentityError::Jwks(error.to_string()))?;

        Ok(CachedJwks { keys, expires_at: Instant::now() + ttl })
    }
}

fn parse_max_age(cache_control: &str) -> Option<u64> {
    cache_control
        .split(',')
        .find_map(|directive| directive.trim().strip_prefix("max-age="))
        .and_then(|value| value.parse().ok())
}

#[async_trait]
impl IdentityProvider for GoogleIdentityProvider {
    async fn verify_id_token(&self, id_token: &str) -> Result<VerifiedIdentity, IdentityError> {
        let header = decode_header(id_token).map_err(|error| IdentityError::Verification(error.to_string()))?;
        if header.alg != Algorithm::RS256 {
            return Err(IdentityError::Verification("algoritmo inesperado".to_string()));
        }
        let kid = header.kid.ok_or_else(|| IdentityError::Verification("token sem kid".to_string()))?;

        let keys = self.jwks(false).await?;
        let jwk = match keys.find(&kid) {
            Some(jwk) => jwk,
            // `kid` desconhecido força refresh no máximo uma vez — em loop,
            // um kid forjado vira amplificador de requests à Google.
            None => {
                let refreshed = self.jwks(true).await?;
                return self.decode_with(id_token, refreshed.find(&kid));
            }
        };
        self.decode_with(id_token, Some(jwk))
    }
}

impl GoogleIdentityProvider {
    fn decode_with(&self, id_token: &str, jwk: Option<&jsonwebtoken::jwk::Jwk>) -> Result<VerifiedIdentity, IdentityError> {
        let jwk = jwk.ok_or_else(|| IdentityError::Verification("chave desconhecida".to_string()))?;
        let key = DecodingKey::from_jwk(jwk).map_err(|error| IdentityError::Verification(error.to_string()))?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[self.client_id.as_str()]);
        validation.set_issuer(&GOOGLE_ISSUERS);
        validation.set_required_spec_claims(&["exp", "aud", "iss", "sub"]);

        let data = decode::<GoogleIdTokenClaims>(id_token, &key, &validation)
            .map_err(|error| IdentityError::Verification(error.to_string()))?;

        if !data.claims.email_verified {
            return Err(IdentityError::Verification("e-mail não verificado".to_string()));
        }

        Ok(VerifiedIdentity {
            subject: data.claims.sub,
            email: data.claims.email,
            avatar_url: data.claims.picture,
        })
    }
}
