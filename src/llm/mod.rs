//! Proxy de geração de texto — a mesma ideia de `storage::ObjectStorage`,
//! aplicada ao provedor de LLM. `TextGenerator` é a porta; a implementação
//! concreta (Groq hoje, via SSE) é o único ponto do código específico de
//! provedor. `http::` (e futuramente a rota de streaming do parecer técnico)
//! só conhece o trait — trocar Groq → Gemini/outro vira escrever uma nova
//! impl aqui, sem tocar em nada fora deste módulo, e sem o `itui` perceber:
//! ele só consome o SSE já normalizado que sai da rota HTTP.
//!
//! **Ainda não implementado** (Step 3 do roadmap — proxy pra Groq). O
//! contrato nasce antes da implementação de propósito: assim o primeiro
//! protótipo já entra atrás do trait, em vez de grudar `reqwest::Client`
//! direto num handler de `http::` e precisar de refactor depois — o mesmo
//! erro que o storage teria cometido se a URL pré-assinada saísse direto de
//! `aws_sdk_s3` dentro da rota.
//!
//! Decisão de privacidade que qualquer implementação deste trait tem que
//! respeitar: o prompt nunca inclui `location_code` nem qualquer outro
//! identificador do laudo que amarre o achado a um lugar físico real — só a
//! categoria do achado (docs/findings-taxonomy.md) e a descrição textual.
//! Mandar o local pro provedor de LLM contradiz a mesma razão que fez o
//! bucket de imagens ser privado (ver storage::ObjectStorage).

use async_trait::async_trait;
use futures::stream::BoxStream;

#[derive(Debug, thiserror::Error)]
pub enum GenerationError {
    #[error("falha ao chamar o provedor de IA: {0}")]
    Provider(String),

    #[error("resposta do provedor não pôde ser interpretada: {0}")]
    Parse(String),
}

/// Um pedaço de texto gerado, na ordem em que chega — já normalizado. Quem
/// consome o stream (a futura rota SSE em `http::`) não sabe se veio do
/// Groq, do Gemini ou de qualquer outro backend compatível com chat
/// completion; o parsing do formato específico do provedor (frames SSE do
/// Groq, por exemplo) fica inteiramente na implementação.
#[derive(Debug, Clone)]
pub struct TextChunk {
    pub delta: String,
}

#[async_trait]
pub trait TextGenerator: Send + Sync {
    /// Gera a partir de um prompt já montado (few-shot da taxonomia de
    /// achados + contexto do laudo — sem `location_code`, ver o comentário
    /// do módulo) e devolve um stream de pedaços de texto, prontos pra
    /// reemitir como SSE pro `itui`.
    async fn generate_stream(
        &self,
        prompt: &str,
    ) -> Result<BoxStream<'static, Result<TextChunk, GenerationError>>, GenerationError>;
}
