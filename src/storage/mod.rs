//! Storage de objetos — **o único ponto do código específico de provedor**.
//! A porta é `ObjectStorage`; o adaptador concreto (Cloudflare R2 em
//! produção, MinIO em dev) fica em `storage::s3`. Trocar de provedor é trocar
//! `STORAGE_ENDPOINT`/credenciais; trocar de *protocolo* seria escrever outra
//! impl deste trait. `http::` só conhece o trait, nunca `aws_sdk_s3`
//! diretamente.
//!
//! O bucket é **privado**, em dev e em produção. Laudo de instalação elétrica
//! fotografa exatamente onde a edificação está vulnerável (condutor vivo
//! exposto, quadro sem proteção, aterramento improvisado) e o laudo amarra
//! isso a um `location_code` no formato BLOCO-SALA. Bucket público, mesmo com
//! path aleatório, é uma indexação futura ou um vazamento de path de virar
//! mapa de vulnerabilidades física de prédio. Todo acesso — escrita e leitura
//! — passa por URL assinada de vida curta. No R2 isso significa: não habilitar
//! o domínio público `r2.dev` nem custom domain no bucket.

mod s3;

use async_trait::async_trait;

pub use s3::S3CompatibleStorage;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("falha ao assinar URL: {0}")]
    Presign(String),

    #[error("falha ao consultar objeto: {0}")]
    Head(String),

    #[error("falha ao remover objeto: {0}")]
    Delete(String),
}

/// Metadado lido do objeto que **já está** no bucket. É a fonte da verdade na
/// confirmação do upload: o cliente não é consultado sobre o que ele enviou.
#[derive(Debug, Clone)]
pub struct ObjectMetadata {
    pub content_type: Option<String>,
    pub size_bytes: i64,
}

#[async_trait]
pub trait ObjectStorage: Send + Sync {
    /// URL de escrita de vida curta. `content_type` entra na assinatura: o
    /// upload só é aceito se o `Content-Type` do PUT for exatamente esse, o
    /// que impede reaproveitar a URL pra subir outra coisa.
    async fn presigned_put(&self, key: &str, content_type: &str) -> Result<String, StorageError>;

    /// URL de leitura de vida curta — como o bucket é privado, é o único jeito
    /// de o navegador exibir a imagem.
    async fn presigned_get(&self, key: &str) -> Result<String, StorageError>;

    /// `None` quando o objeto não existe (upload nunca aconteceu ou falhou).
    async fn head(&self, key: &str) -> Result<Option<ObjectMetadata>, StorageError>;

    async fn delete(&self, key: &str) -> Result<(), StorageError>;
}
