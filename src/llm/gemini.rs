use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt};
use serde::Deserialize;
use serde_json::json;

use crate::config::{LlmConfig, ProviderCredentials};

use super::{sse_data_lines, GenerationError, GenerationEvent, GenerationRequest, TextGenerator};

pub struct GeminiGenerator {
    http: reqwest::Client,
    endpoint: String,
    api_key: String,
    temperature: f32,
    max_output_tokens: u32,
}

impl GeminiGenerator {
    pub fn new(config: &LlmConfig, credentials: &ProviderCredentials) -> Self {
        Self {
            http: reqwest::Client::builder()
                .connect_timeout(config.connect_timeout)
                .timeout(config.read_timeout)
                .build()
                .expect("configuração inválida do client HTTP"),
            endpoint: format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse",
                credentials.model
            ),
            api_key: credentials.api_key.clone(),
            temperature: config.temperature,
            max_output_tokens: config.max_output_tokens,
        }
    }
}

#[derive(Deserialize, Default)]
struct Part {
    text: Option<String>,
}

#[derive(Deserialize, Default)]
struct Content {
    #[serde(default)]
    parts: Vec<Part>,
}

#[derive(Deserialize)]
struct Candidate {
    #[serde(default)]
    content: Content,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct UsageMetadata {
    #[serde(rename = "totalTokenCount")]
    total_token_count: i64,
}

#[derive(Deserialize)]
struct Chunk {
    #[serde(default)]
    candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
}

fn parse_chunk(data: &str) -> Result<Vec<GenerationEvent>, GenerationError> {
    let chunk: Chunk =
        serde_json::from_str(data).map_err(|error| GenerationError::Parse(error.to_string()))?;

    let Some(candidate) = chunk.candidates.into_iter().next() else {
        return Ok(Vec::new());
    };

    let mut events = Vec::new();

    for part in candidate.content.parts {
        if let Some(text) = part.text.filter(|text| !text.is_empty()) {
            events.push(GenerationEvent::Token { text });
        }
    }

    if let Some(finish_reason) = candidate.finish_reason {
        events.push(GenerationEvent::Done {
            finish_reason,
            total_tokens: chunk.usage_metadata.map(|usage| usage.total_token_count),
        });
    }

    Ok(events)
}

#[async_trait]
impl TextGenerator for GeminiGenerator {
    async fn generate_stream(
        &self,
        request: GenerationRequest,
    ) -> Result<BoxStream<'static, Result<GenerationEvent, GenerationError>>, GenerationError> {
        let response = self
            .http
            .post(&self.endpoint)
            .header("x-goog-api-key", &self.api_key)
            .json(&json!({
                "systemInstruction": { "parts": [{ "text": request.system }] },
                "contents": [{ "role": "user", "parts": [{ "text": request.user }] }],
                "generationConfig": {
                    "temperature": self.temperature,
                    "maxOutputTokens": self.max_output_tokens,
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
                            match parse_chunk(data) {
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
