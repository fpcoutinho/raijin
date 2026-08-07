use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub storage: StorageConfig,
}

/// Configuração do storage S3-compatible. Os mesmos campos servem dev e produção,
/// o que muda é só o valor de `endpoint` e das credenciais.
/// Trocar de provedor não passa por aqui nem pelo frontend.
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub endpoint: String,
    pub bucket: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub region: String,
    /// Janela de validade da URL de upload. Curta o bastante pra que uma URL
    /// vazada (log, histórico do navegador) não seja um canal de escrita
    /// duradouro no bucket; longa o bastante pra foto grande em 4G ruim.
    pub upload_url_ttl: Duration,
    /// Validade da URL de leitura. Mais curta ainda: é o que expõe o conteúdo
    /// do laudo, e o frontend sempre pode pedir outra.
    pub download_url_ttl: Duration,
}

#[derive(Debug, thiserror::Error)]
#[error("variável de ambiente ausente ou inválida: {0}")]
pub struct ConfigError(String);

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            database_url: required("DATABASE_URL")?,
            bind_addr: optional("BIND_ADDR", "0.0.0.0:3000"),
            storage: StorageConfig {
                endpoint: required("STORAGE_ENDPOINT")?,
                bucket: required("STORAGE_BUCKET")?,
                access_key_id: required("STORAGE_ACCESS_KEY_ID")?,
                secret_access_key: required("STORAGE_SECRET_ACCESS_KEY")?,
                region: optional("STORAGE_REGION", "auto"),
                upload_url_ttl: Duration::from_secs(15 * 60),
                download_url_ttl: Duration::from_secs(5 * 60),
            },
        })
    }
}

fn required(key: &str) -> Result<String, ConfigError> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ConfigError(key.to_string()))
}

fn optional(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}
