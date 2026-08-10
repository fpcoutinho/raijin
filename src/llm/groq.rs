use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt};
use serde::Deserialize;
use serde_json::json;

use crate::config::LlmConfig;

use super::{sse_data_lines, GenerationError, GenerationEvent, GenerationRequest, TextGenerator};

const ENDPOINT: &str = "https://api.groq.com/openai/v1/chat/completions";
const MODEL: &str = "llama-3.3-70b-versatile";

pub struct GroqGenerator {
    http: reqwest::Client,
    api_key: String,
    temperature: f32,
    max_output_tokens: u32,
}

impl GroqGenerator {
    pub fn new(config: &LlmConfig) -> Self {
        Self {
            http: reqwest::Client::builder()
                .connect_timeout(config.connect_timeout)
                .timeout(config.read_timeout)
                .build()
                .expect("configuração inválida do client HTTP"),
            api_key: config.api_key.clone(),
            temperature: config.temperature,
            max_output_tokens: config.max_output_tokens,
        }
    }
}

#[derive(Deserialize)]
struct ChunkChoice {
    delta: Delta,
    finish_reason: Option<String>,
}

#[derive(Deserialize, Default)]
struct Delta {
    content: Option<String>,
}

#[derive(Deserialize)]
struct Usage {
    total_tokens: i64,
}

#[derive(Deserialize)]
struct Chunk {
    choices: Vec<ChunkChoice>,
    #[serde(default)]
    usage: Option<Usage>,
}

fn parse_chunk(data: &str) -> Result<Option<GenerationEvent>, GenerationError> {
    if data == "[DONE]" {
        return Ok(None);
    }

    let chunk: Chunk =
        serde_json::from_str(data).map_err(|error| GenerationError::Parse(error.to_string()))?;

    let Some(choice) = chunk.choices.into_iter().next() else {
        return Ok(None);
    };

    if let Some(finish_reason) = choice.finish_reason {
        return Ok(Some(GenerationEvent::Done {
            finish_reason,
            total_tokens: chunk.usage.map(|usage| usage.total_tokens),
        }));
    }

    match choice.delta.content {
        Some(text) if !text.is_empty() => Ok(Some(GenerationEvent::Token { text })),
        _ => Ok(None),
    }
}

#[async_trait]
impl TextGenerator for GroqGenerator {
    async fn generate_stream(
        &self,
        request: GenerationRequest,
    ) -> Result<BoxStream<'static, Result<GenerationEvent, GenerationError>>, GenerationError> {
        let response = self
            .http
            .post(ENDPOINT)
            .bearer_auth(&self.api_key)
            .json(&json!({
                "model": MODEL,
                "stream": true,
                "stream_options": { "include_usage": true },
                "temperature": self.temperature,
                "max_tokens": self.max_output_tokens,
                "messages": [
                    { "role": "system", "content": request.system },
                    { "role": "user", "content": request.user },
                ],
            }))
            .send()
            .await
            .map_err(|error| GenerationError::Provider(error.to_string()))?;

        if !response.status().is_success() {
            return Err(GenerationError::Provider(format!(
                "Groq respondeu {}",
                response.status()
            )));
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
                                Ok(Some(event)) => events.push(Ok(event)),
                                Ok(None) => {}
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
