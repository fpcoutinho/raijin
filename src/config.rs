use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub auth: AuthConfig,
    pub storage: StorageConfig,
}

/// Configuração de autenticação.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub google_client_id: String,
    pub allowed_origins: Vec<String>,
    pub access_token_ttl: Duration,
    pub refresh_token_ttl: Duration,
    /// Janela em que um refresh token recém-revogado ainda emite um substituto
    /// em vez de derrubar a sessão — cobre duas abas renovando ao mesmo tempo.
    pub refresh_grace: Duration,
    /// Validade do cache do JWKS da Google quando a resposta não trouxer
    /// Cache-Control. As chaves rodam de poucos em poucos dias; buscar a cada
    /// login seria lento e rate-limited.
    pub jwks_fallback_ttl: Duration,
    /// Autentica o EventBridge Scheduler em `POST /tasks/cleanup-sessions` —
    /// máquina, não usuário, então não passa pelo `AuthUser` de sessão.
    pub task_token: String,
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
            auth: AuthConfig {
                jwt_secret: jwt_secret()?,
                google_client_id: required("GOOGLE_CLIENT_ID")?,
                allowed_origins: allowed_origins()?,
                access_token_ttl: Duration::from_secs(15 * 60),
                refresh_token_ttl: Duration::from_secs(30 * 24 * 60 * 60),
                refresh_grace: Duration::from_secs(10),
                jwks_fallback_ttl: Duration::from_secs(60 * 60),
                task_token: task_token()?,
            },
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

/// HS256 com chave menor que o bloco do SHA-256 (32 bytes) é fraqueza real,
/// não pedantismo — e `required` só rejeita vazio.
fn jwt_secret() -> Result<String, ConfigError> {
    let secret = required("JWT_SECRET")?;
    if secret.len() < 32 {
        return Err(ConfigError("JWT_SECRET".to_string()));
    }
    Ok(secret)
}

/// Mesma regra de tamanho mínimo do `JWT_SECRET` — token curto é força bruta
/// barata pro endpoint de máquina.
fn task_token() -> Result<String, ConfigError> {
    let token = required("TASK_TOKEN")?;
    if token.len() < 32 {
        return Err(ConfigError("TASK_TOKEN".to_string()));
    }
    Ok(token)
}

/// Lista separada por vírgula. Vazio é aceito (nenhuma origem permitida) em
/// vez de tratado como "faltando" — dev isolado sem frontend rodando é caso
/// válido, diferente de um segredo ausente.
fn allowed_origins() -> Result<Vec<String>, ConfigError> {
    Ok(std::env::var("CORS_ALLOWED_ORIGINS")
        .ok()
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(str::to_string)
        .collect())
}
