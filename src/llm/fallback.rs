use std::sync::Arc;

use async_trait::async_trait;
use futures::stream::BoxStream;

use super::{GenerationError, GenerationEvent, GenerationRequest, TextGenerator};

/// Tenta os provedores em ordem, caindo para o seguinte enquanto o erro for de
/// capacidade. Só o `Result` de abertura do stream é retentável: depois do
/// primeiro token entregue ao SSE, refazer a chamada duplicaria texto no laudo.
pub struct FallbackChain {
    links: Vec<Arc<dyn TextGenerator>>,
}

impl FallbackChain {
    pub fn new(links: Vec<Arc<dyn TextGenerator>>) -> Self {
        Self { links }
    }
}

#[async_trait]
impl TextGenerator for FallbackChain {
    async fn generate_stream(
        &self,
        request: GenerationRequest,
    ) -> Result<BoxStream<'static, Result<GenerationEvent, GenerationError>>, GenerationError> {
        let mut exhausted = None;

        for (position, link) in self.links.iter().enumerate() {
            match link.generate_stream(request.clone()).await {
                Err(GenerationError::Unavailable(detail)) => {
                    tracing::warn!(position, %detail, "provedor sem capacidade, tentando o próximo");
                    exhausted = Some(detail);
                }
                result => return result,
            }
        }

        Err(GenerationError::Unavailable(
            exhausted.unwrap_or_else(|| "nenhum provedor configurado".to_string()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream::{self, StreamExt};

    struct Fixed {
        error: Option<GenerationError>,
        label: &'static str,
    }

    fn unavailable(label: &'static str) -> Arc<dyn TextGenerator> {
        Arc::new(Fixed {
            error: Some(GenerationError::Unavailable(label.to_string())),
            label,
        })
    }

    fn broken(label: &'static str) -> Arc<dyn TextGenerator> {
        Arc::new(Fixed { error: Some(GenerationError::Provider(label.to_string())), label })
    }

    fn healthy(label: &'static str) -> Arc<dyn TextGenerator> {
        Arc::new(Fixed { error: None, label })
    }

    #[async_trait]
    impl TextGenerator for Fixed {
        async fn generate_stream(
            &self,
            _request: GenerationRequest,
        ) -> Result<BoxStream<'static, Result<GenerationEvent, GenerationError>>, GenerationError>
        {
            match &self.error {
                Some(GenerationError::Unavailable(detail)) => {
                    Err(GenerationError::Unavailable(detail.clone()))
                }
                Some(GenerationError::Provider(detail)) => {
                    Err(GenerationError::Provider(detail.clone()))
                }
                Some(GenerationError::Parse(detail)) => Err(GenerationError::Parse(detail.clone())),
                None => Ok(stream::iter(vec![Ok(GenerationEvent::Token {
                    text: self.label.to_string(),
                })])
                .boxed()),
            }
        }
    }

    fn request() -> GenerationRequest {
        GenerationRequest { system: "s".to_string(), user: "u".to_string() }
    }

    async fn first_token(chain: &FallbackChain) -> String {
        let mut stream = chain.generate_stream(request()).await.expect("stream aberto");
        match stream.next().await {
            Some(Ok(GenerationEvent::Token { text })) => text,
            other => panic!("esperava token, veio {other:?}"),
        }
    }

    #[tokio::test]
    async fn desce_a_cascata_ate_achar_capacidade() {
        let chain = FallbackChain::new(vec![
            unavailable("primeiro"),
            unavailable("segundo"),
            healthy("terceiro"),
        ]);

        assert_eq!(first_token(&chain).await, "terceiro");
    }

    #[tokio::test]
    async fn para_no_primeiro_elo_saudavel() {
        let chain =
            FallbackChain::new(vec![healthy("primeiro"), healthy("segundo")]);

        assert_eq!(first_token(&chain).await, "primeiro");
    }

    #[tokio::test]
    async fn erro_que_nao_e_de_capacidade_interrompe_a_cascata() {
        let chain = FallbackChain::new(vec![broken("401"), healthy("não deve ser alcançado")]);

        let result = chain.generate_stream(request()).await;

        assert!(matches!(result, Err(GenerationError::Provider(_))));
    }

    #[tokio::test]
    async fn cascata_inteira_sem_capacidade_devolve_o_ultimo_erro() {
        let chain = FallbackChain::new(vec![unavailable("primeiro"), unavailable("ultimo")]);

        let result = chain.generate_stream(request()).await;

        assert!(matches!(result, Err(GenerationError::Unavailable(detail)) if detail == "ultimo"));
    }
}
