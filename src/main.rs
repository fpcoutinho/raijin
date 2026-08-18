mod auth;
mod config;
mod document;
mod domain;
mod http;
mod llm;
mod storage;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::Request;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderValue, Method};
use axum::middleware::Next;
use axum::response::Response;
use lambda_http::request::RequestContext;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{AllowOrigin, CorsLayer};

use auth::{GoogleIdentityProvider, IdentityProvider, TokenIssuer};
use config::{Config, LlmConfig, LlmProvider, ProviderCredentials};
use llm::{FallbackChain, GeminiGenerator, GroqGenerator, TextGenerator};
use storage::{ObjectStorage, S3CompatibleStorage};

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub storage: Arc<dyn ObjectStorage>,
    pub tokens: Arc<TokenIssuer>,
    pub identity: Arc<dyn IdentityProvider>,
    pub llm: Arc<dyn TextGenerator>,
    /// Janela em que um refresh revogado ainda emite substituto (multi-aba)
    /// em vez de derrubar a cadeia. Copiado de `AuthConfig` pra não passar
    /// `&Config` inteiro pros handlers.
    pub refresh_grace: std::time::Duration,
    /// Autentica `POST /tasks/cleanup-sessions`. Copiado de `AuthConfig` pelo
    /// mesmo motivo que `refresh_grace`.
    pub task_token: String,
    /// Segredo esperado no header de origem do CloudFront. Ver `AuthConfig`.
    pub origin_shared_secret: Option<String>,
    /// Liga a checagem dos códigos NBR nos PATCH de seção. Ver `Config`.
    pub nbr_validation: bool,
}

async fn build_state(config: &Config, in_lambda: bool) -> AppState {
    let db = if in_lambda {
        PgPoolOptions::new()
            // Uma invocação por vez por instância; 2 evita autodeadlock se um
            // handler futuro fizer duas queries concorrentes.
            .max_connections(2)
            // O reaper de min_connections roda em processo — congelado entre
            // invocações, então manter um mínimo aquecido não faz sentido aqui.
            .min_connections(0)
            // Default do sqlx (30s) é maior que o timeout da função — sem
            // isso o erro vira CloudWatch opaco em vez de 503 nosso.
            .acquire_timeout(Duration::from_secs(3))
            // ~ janela de autosuspend do Neon.
            .max_lifetime(Some(Duration::from_secs(10 * 60)))
            // is_beyond_idle_timeout só é checado no loop do reaper, que fica
            // congelado entre invocações — um valor aqui pareceria proteção
            // sem nunca ser avaliado de fato.
            .idle_timeout(None)
            // Mitigação real do freeze/thaw: sem isso, um handler pode herdar
            // uma conexão que o Neon já fechou enquanto o processo congelava.
            .test_before_acquire(true)
            .connect(&config.database_url)
            .await
            .expect("falha ao conectar no Postgres")
    } else {
        PgPoolOptions::new()
            .max_connections(10)
            .connect(&config.database_url)
            .await
            .expect("falha ao conectar no Postgres")
    };

    let storage: Arc<dyn ObjectStorage> = Arc::new(S3CompatibleStorage::new(&config.storage));
    let tokens = Arc::new(TokenIssuer::new(&config.auth));
    let identity: Arc<dyn IdentityProvider> = Arc::new(GoogleIdentityProvider::new(&config.auth));
    let llm = text_generator(&config.llm);

    // Gera o hash de referência no boot, não no primeiro login com e-mail
    // desconhecido — senão essa primeira tentativa paga o custo e fica mais
    // lenta que um login real, invertendo o sinal que ela existe pra esconder.
    let _ = &*auth::DUMMY_PASSWORD_HASH;

    let refresh_grace = config.auth.refresh_grace;
    let task_token = config.auth.task_token.clone();
    let origin_shared_secret = config.auth.origin_shared_secret.clone();
    let nbr_validation = config.nbr_validation;

    if in_lambda && origin_shared_secret.is_none() {
        tracing::warn!(
            "ORIGIN_SHARED_SECRET ausente — a Function URL aceita requisição que não veio do CloudFront"
        );
    }

    if !nbr_validation {
        tracing::warn!("FF_NBR_VALIDATION_ENABLED=off — códigos normativos entram sem checagem");
    }

    AppState {
        db,
        storage,
        tokens,
        identity,
        llm,
        refresh_grace,
        task_token,
        origin_shared_secret,
        nbr_validation,
    }
}

fn text_generator(config: &LlmConfig) -> Arc<dyn TextGenerator> {
    if config.chain.len() == 1 {
        tracing::warn!("cascata de IA com um só elo — limite estourado vira erro 503");
    }

    let links = config
        .chain
        .iter()
        .map(|link| adapter(config, link))
        .collect();

    Arc::new(FallbackChain::new(links))
}

fn adapter(config: &LlmConfig, credentials: &ProviderCredentials) -> Arc<dyn TextGenerator> {
    match credentials.provider {
        LlmProvider::Groq => Arc::new(GroqGenerator::new(config, credentials)),
        LlmProvider::Gemini => Arc::new(GeminiGenerator::new(config, credentials)),
    }
}

fn build_router(config: &Config, state: AppState) -> Router {
    let allowed_origins: Vec<HeaderValue> = config
        .auth
        .allowed_origins
        .iter()
        .map(|origin| {
            origin
                .parse()
                .expect("CORS_ALLOWED_ORIGINS com origem inválida")
        })
        .collect();

    http::router(&state)
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(allowed_origins))
                .allow_credentials(true)
                .allow_headers([AUTHORIZATION, CONTENT_TYPE])
                .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
                // Teto do Chrome pro preflight — sem isso, toda chamada
                // POST/PATCH/DELETE cross-origin do itui dispara OPTIONS de
                // novo a cada request.
                .max_age(Duration::from_secs(7200)),
        )
        // Níveis explícitos: o default do TraceLayer é DEBUG, que o filtro
        // `raijin=info` esconderia.
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
                .make_span_with(
                    tower_http::trace::DefaultMakeSpan::new().level(tracing::Level::INFO),
                )
                .on_response(
                    tower_http::trace::DefaultOnResponse::new().level(tracing::Level::INFO),
                ),
        )
        .with_state(state)
}

/// CloudWatch já carimba hora em cada linha — ANSI e timestamp do próprio
/// tracing só duplicam ruído no log de produção.
fn init_tracing(in_lambda: bool) {
    let filter = || {
        tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("raijin=info".parse().unwrap())
            .add_directive("tower_http=info".parse().unwrap())
    };
    if in_lambda {
        tracing_subscriber::fmt()
            .with_env_filter(filter())
            .with_ansi(false)
            .without_time()
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter()).init();
    }
}

fn viewer_address(req: &Request) -> Option<std::net::IpAddr> {
    let value = req
        .headers()
        .get("cloudfront-viewer-address")?
        .to_str()
        .ok()?;

    parse_viewer_address(value)
}

/// `CloudFront-Viewer-Address` vem como `IP:porta` — e em IPv6 o endereço
/// também tem `:`, então a porta é o que vem depois do último.
fn parse_viewer_address(value: &str) -> Option<std::net::IpAddr> {
    let (address, _port) = value.rsplit_once(':')?;

    address
        .trim_start_matches('[')
        .trim_end_matches(']')
        .parse()
        .ok()
}

/// Atrás do CloudFront isto é o IP do edge, não o do usuário — só serve como
/// fallback pra invocação direta (EventBridge, `cargo lambda invoke`).
fn request_context_source_ip(req: &Request) -> Option<std::net::IpAddr> {
    match req.extensions().get::<RequestContext>() {
        Some(RequestContext::ApiGatewayV2(cx)) => cx.http.source_ip.as_deref(),
        _ => None,
    }
    .and_then(|source_ip| source_ip.parse().ok())
}

/// Sob Lambda não existe conexão TCP: sintetiza `ConnectInfo` a partir do IP
/// que a API Gateway/Function URL já observou, pro `SmartIpKeyExtractor`
/// (rate limiting de /auth) não cair sempre em `UnableToExtractKey`.
async fn lambda_source_ip(mut req: Request, next: Next) -> Response {
    let ip = viewer_address(&req).or_else(|| request_context_source_ip(&req));

    if let Some(ip) = ip {
        // SmartIpKeyExtractor lê X-Forwarded-For primeiro, e o valor mais à
        // esquerda é controlado pelo cliente — sobrescrever com o IP que o
        // CloudFront (ou a Lambda) de fato observou, não confiar no recebido.
        if let Ok(value) = ip.to_string().parse() {
            req.headers_mut().insert("x-forwarded-for", value);
        }
        req.extensions_mut()
            .insert(axum::extract::ConnectInfo(SocketAddr::new(ip, 0)));
    }
    next.run(req).await
}

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    let in_lambda = std::env::var_os("AWS_LAMBDA_RUNTIME_API").is_some();
    init_tracing(in_lambda);

    dotenvy::dotenv().ok();
    let config = Config::from_env()
        .expect("configuração inválida — confira as variáveis de ambiente contra .env.example");

    let state = build_state(&config, in_lambda).await;
    let app = build_router(&config, state);

    if in_lambda {
        // Entrega incremental do SSE de /generate só existe atrás de uma
        // Function URL em InvokeMode = RESPONSE_STREAM — API Gateway HTTP API
        // não suporta. Sob qualquer outro invoke mode a resposta chega
        // inteira de uma vez, sem quebrar a rota, só sem streaming de verdade.
        lambda_http::run_with_streaming_response(
            app.layer(axum::middleware::from_fn(lambda_source_ip)),
        )
        .await
    } else {
        let listener = tokio::net::TcpListener::bind(&config.bind_addr)
            .await
            .expect("falha ao abrir a porta");

        tracing::info!("raijin ouvindo em {}", config.bind_addr);

        // `SmartIpKeyExtractor` (rate limiting de /auth) cai pro IP da conexão
        // TCP quando não há X-Forwarded-For/Forwarded — sem isso a extração falha
        // sempre que não houver proxy na frente (dev local, sem Lambda). Sob
        // Lambda o middleware `lambda_source_ip` cobre esse caso.
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .expect("servidor encerrou com erro");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::parse_viewer_address;

    #[test]
    fn extrai_ipv4_descartando_a_porta() {
        let ip = parse_viewer_address("203.0.113.42:54321").unwrap();

        assert_eq!(ip.to_string(), "203.0.113.42");
    }

    #[test]
    fn extrai_ipv6_entre_colchetes() {
        let ip = parse_viewer_address("[2001:db8::1]:54321").unwrap();

        assert_eq!(ip.to_string(), "2001:db8::1");
    }

    #[test]
    fn recusa_valor_sem_porta() {
        assert!(parse_viewer_address("203.0.113.42").is_none());
    }
}
