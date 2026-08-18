use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt};
use serde::Deserialize;
use serde_json::json;

use crate::config::{LlmConfig, ProviderCredentials};

use super::{GenerationError, GenerationEvent, GenerationRequest, TextGenerator, sse_data_lines};

const INTERACTIONS_ENDPOINT: &str = "https://generativelanguage.googleapis.com/v1beta/interactions";

pub struct GeminiGenerator {
    http: reqwest::Client,
    model: String,
    api_key: String,
    temperature: f32,
    max_output_tokens: u32,
    thinking_level: String,
}

impl GeminiGenerator {
    pub fn new(config: &LlmConfig, credentials: &ProviderCredentials) -> Self {
        Self {
            http: reqwest::Client::builder()
                .connect_timeout(config.connect_timeout)
                .timeout(config.read_timeout)
                .build()
                .expect("configuração inválida do client HTTP"),
            model: credentials.model.clone(),
            api_key: credentials.api_key.clone(),
            temperature: config.temperature,
            max_output_tokens: config.max_output_tokens,
            thinking_level: config.thinking_level.clone(),
        }
    }
}

#[derive(Deserialize)]
struct Delta {
    text: Option<String>,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct Step {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    content: Vec<Delta>,
}

#[derive(Deserialize)]
struct Usage {
    total_tokens: i64,
}

#[derive(Deserialize)]
struct Interaction {
    status: Option<String>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct ApiError {
    message: String,
    code: Option<String>,
}

#[derive(Deserialize)]
struct Event {
    event_type: String,
    delta: Option<Delta>,
    step: Option<Step>,
    interaction: Option<Interaction>,
    error: Option<ApiError>,
}

fn text_tokens(parts: impl IntoIterator<Item = Delta>) -> Vec<GenerationEvent> {
    parts
        .into_iter()
        .filter(|part| part.kind == "text")
        .filter_map(|part| part.text.filter(|text| !text.is_empty()))
        .map(|text| GenerationEvent::Token { text })
        .collect()
}

/// Cota estourada e modelo sobrecarregado chegam como evento no meio do stream,
/// sob HTTP 200 — sem isso o `FallbackChain` não teria como saber que vale
/// tentar o próximo elo.
fn stream_error(error: ApiError) -> GenerationError {
    let detail = format!(
        "Gemini: {}",
        error.message.chars().take(500).collect::<String>()
    );
    let retryable = matches!(error.code.as_deref(), Some("api_error"))
        || error.message.contains("quota")
        || error.message.contains("high demand");

    if retryable {
        GenerationError::Unavailable(detail)
    } else {
        GenerationError::Provider(detail)
    }
}

fn parse_event(data: &str) -> Result<Vec<GenerationEvent>, GenerationError> {
    if data == "[DONE]" {
        return Ok(Vec::new());
    }

    let event: Event =
        serde_json::from_str(data).map_err(|error| GenerationError::Parse(error.to_string()))?;

    if let Some(error) = event.error {
        return Err(stream_error(error));
    }

    match event.event_type.as_str() {
        // Raciocínio chega pelos mesmos eventos, como thought_signature ou
        // thought_summary; só o tipo `text` é prosa do laudo.
        "step.delta" => Ok(text_tokens(event.delta)),
        // O primeiro trecho da resposta pode vir junto do step.start em vez de
        // num delta.
        "step.start" => Ok(event
            .step
            .filter(|step| step.kind == "model_output")
            .map(|step| text_tokens(step.content))
            .unwrap_or_default()),
        "interaction.completed" | "interaction.failed" | "interaction.incomplete" => {
            let interaction = event.interaction;

            Ok(vec![GenerationEvent::Done {
                finish_reason: interaction
                    .as_ref()
                    .and_then(|interaction| interaction.status.clone())
                    .unwrap_or_else(|| event.event_type.clone()),
                total_tokens: interaction
                    .and_then(|interaction| interaction.usage)
                    .map(|usage| usage.total_tokens),
            }])
        }
        _ => Ok(Vec::new()),
    }
}

#[async_trait]
impl TextGenerator for GeminiGenerator {
    async fn generate_stream(
        &self,
        request: GenerationRequest,
    ) -> Result<BoxStream<'static, Result<GenerationEvent, GenerationError>>, GenerationError> {
        let response = self
            .http
            .post(INTERACTIONS_ENDPOINT)
            .header("x-goog-api-key", &self.api_key)
            .json(&json!({
                "model": self.model,
                "input": request.user,
                "system_instruction": request.system,
                // Sem isso a Google persiste a interação nos servidores dela — o
                // laudo descreve vulnerabilidade física de uma edificação real.
                "store": false,
                "stream": true,
                "generation_config": {
                    "temperature": self.temperature,
                    "max_output_tokens": self.max_output_tokens,
                    "thinking_level": self.thinking_level,
                    // Explícito porque é padrão do provedor, não garantia de
                    // contrato: resumo de raciocínio não é texto de laudo.
                    "thinking_summaries": "none",
                },
            }))
            .send()
            .await
            .map_err(|error| GenerationError::Provider(error.to_string()))?;

        if !response.status().is_success() {
            return Err(super::provider_error("Gemini", response).await);
        }

        let mut buffer = String::new();
        let byte_stream = response.bytes_stream();

        let events = byte_stream.flat_map(move |chunk| {
            let events = match chunk {
                Ok(bytes) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));
                    let mut events = Vec::new();

                    while let Some(separator) = buffer.find("\n\n") {
                        let block = buffer[..separator].to_string();
                        buffer.drain(..separator + 2);

                        for data in sse_data_lines(&block) {
                            match parse_event(data) {
                                Ok(parsed) => events.extend(parsed.into_iter().map(Ok)),
                                Err(error) => events.push(Err(error)),
                            }
                        }
                    }

                    events
                }
                Err(error) => vec![Err(GenerationError::Provider(error.to_string()))],
            };

            stream::iter(events)
        });

        Ok(events.boxed())
    }
}

#[cfg(test)]
mod tests {
    use super::{GenerationError, GenerationEvent, parse_event};

    fn tokens(data: &str) -> Vec<String> {
        parse_event(data)
            .expect("evento válido")
            .into_iter()
            .filter_map(|event| match event {
                GenerationEvent::Token { text } => Some(text),
                GenerationEvent::Done { .. } => None,
            })
            .collect()
    }

    #[test]
    fn emite_o_texto_que_vem_no_step_start() {
        let data = r#"{"index":1,"step":{"content":[{"text":"A instalação","type":"text"}],"type":"model_output"},"event_type":"step.start"}"#;

        assert_eq!(tokens(data), vec!["A instalação"]);
    }

    #[test]
    fn ignora_resumo_e_assinatura_de_raciocinio() {
        let signature = r#"{"index":0,"delta":{"signature":"abc","type":"thought_signature"},"event_type":"step.delta"}"#;
        let summary = r#"{"index":0,"delta":{"text":"analisando os achados","type":"thought_summary"},"event_type":"step.delta"}"#;
        let start = r#"{"index":0,"step":{"summary":[{"text":"analisando","type":"text"}],"type":"thought"},"event_type":"step.start"}"#;

        assert!(tokens(signature).is_empty());
        assert!(tokens(summary).is_empty());
        assert!(tokens(start).is_empty());
    }

    #[test]
    fn erro_de_cota_no_meio_do_stream_vira_unavailable() {
        let data = r#"{"error":{"message":"You exceeded your current quota","code":"invalid_request"},"event_type":"error"}"#;

        assert!(matches!(
            parse_event(data),
            Err(GenerationError::Unavailable(_))
        ));
    }

    #[test]
    fn interaction_completed_reporta_uso() {
        let data = r#"{"interaction":{"status":"completed","usage":{"total_tokens":530}},"event_type":"interaction.completed"}"#;

        let events = parse_event(data).expect("evento válido");

        assert!(matches!(
            events.as_slice(),
            [GenerationEvent::Done { finish_reason, total_tokens: Some(530) }] if finish_reason == "completed"
        ));
    }
}
