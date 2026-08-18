use std::sync::LazyLock;

use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};

#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("falha ao gerar o hash da senha: {0}")]
    Hash(String),

    #[error("hash de senha armazenado é inválido: {0}")]
    Parse(String),

    #[error("falha ao executar o hash em thread dedicada: {0}")]
    Join(String),
}

/// argon2id padrão do crate (OWASP: m=19 MiB, t=2, p=1) — ~19 MiB e dezenas de
/// ms de CPU, por isso roda em `spawn_blocking` e não numa worker do tokio.
pub async fn hash_password(plain: String) -> Result<String, PasswordError> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(plain.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|error| PasswordError::Hash(error.to_string()))
    })
    .await
    .map_err(|error| PasswordError::Join(error.to_string()))?
}

/// Senha errada é `Ok(false)`, não `Err` — não é falha do hashing.
pub async fn verify_password(plain: String, phc: String) -> Result<bool, PasswordError> {
    tokio::task::spawn_blocking(move || {
        let parsed =
            PasswordHash::new(&phc).map_err(|error| PasswordError::Parse(error.to_string()))?;
        Ok(Argon2::default()
            .verify_password(plain.as_bytes(), &parsed)
            .is_ok())
    })
    .await
    .map_err(|error| PasswordError::Join(error.to_string()))?
}

/// Hash descartável, verificado quando o e-mail não existe ou é só-Google —
/// sem isso o tempo de resposta vira oráculo de "esse e-mail tem conta".
/// Tocar no boot, não só no primeiro login, pra não pagar a geração ali.
pub static DUMMY_PASSWORD_HASH: LazyLock<String> = LazyLock::new(|| {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(b"raijin-dummy-password", &salt)
        .expect("hash de referência não pôde ser gerado")
        .to_string()
});
