mod fallback;
mod gemini;
mod groq;
pub mod prompt;

pub use fallback::FallbackChain;
pub use gemini::GeminiGenerator;
pub use groq::GroqGenerator;

use async_trait::async_trait;
use futures::stream::BoxStream;

#[derive(Debug, thiserror::Error)]
pub enum GenerationError {
    #[error("falha ao chamar o provedor de IA: {0}")]
    Provider(String),

    /// Separado de `Provider` porque é o único erro que vale tentar em outro
    /// provedor: limite de token por minuto estourado (413/429) ou provedor
    /// fora do ar (5xx) — ver llm::FallbackGenerator.
    #[error("provedor de IA sem capacidade no momento: {0}")]
    Unavailable(String),

    #[error("resposta do provedor não pôde ser interpretada: {0}")]
    Parse(String),
}

/// System + user já montados (ver llm::prompt::build_request) — a porta não
/// sabe como o prompt foi composto, só entrega as duas partes no formato que
/// todo provedor de chat completion espera.
#[derive(Clone)]
pub struct GenerationRequest {
    pub system: String,
    pub user: String,
}

/// Evento normalizado do stream — mapeia 1:1 nos três tipos de evento SSE do
/// contrato (docs/api-contract.md, `POST .../generate`). `total_tokens` é
/// `Option` porque nem todo provedor reporta uso.
#[derive(Debug, Clone)]
pub enum GenerationEvent {
    Token {
        text: String,
    },
    Done {
        finish_reason: String,
        total_tokens: Option<i64>,
    },
}

#[async_trait]
pub trait TextGenerator: Send + Sync {
    async fn generate_stream(
        &self,
        request: GenerationRequest,
    ) -> Result<BoxStream<'static, Result<GenerationEvent, GenerationError>>, GenerationError>;
}

/// Classificação de resposta malsucedida, compartilhada pelos adaptadores. O
/// corpo entra na mensagem porque é onde os dois provedores dizem qual limite
/// foi estourado — só o status não distingue prompt grande de cota esgotada.
pub(crate) async fn provider_error(name: &str, response: reqwest::Response) -> GenerationError {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let detail = format!(
        "{name} respondeu {status}: {}",
        body.chars().take(500).collect::<String>()
    );

    if status.is_server_error()
        || status == reqwest::StatusCode::PAYLOAD_TOO_LARGE
        || status == reqwest::StatusCode::TOO_MANY_REQUESTS
    {
        GenerationError::Unavailable(detail)
    } else {
        GenerationError::Provider(detail)
    }
}

/// Enquadramento SSE compartilhado pelos adaptadores: acumula bytes num
/// buffer e corta em blocos separados por linha em branco, devolvendo só as
/// linhas `data:` de cada bloco. O que muda entre Groq e Gemini é o JSON
/// dentro de cada `data:`, não o enquadramento do protocolo.
pub(crate) fn sse_data_lines(block: &str) -> Vec<&str> {
    block
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim)
        .filter(|data| !data.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::sse_data_lines;

    #[test]
    fn extrai_linhas_data_de_um_bloco_sse() {
        let block = "event: message\ndata: {\"a\":1}\n";
        assert_eq!(sse_data_lines(block), vec!["{\"a\":1}"]);
    }

    #[test]
    fn ignora_linhas_sem_data() {
        let block = "event: ping\n\n";
        assert!(sse_data_lines(block).is_empty());
    }
}
