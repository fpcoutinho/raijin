mod middleware;
pub(crate) mod queries;
pub(crate) mod routes;
pub(crate) mod schema;

use std::time::Duration;

use axum::Router;
use axum::routing::post;
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;

use crate::AppState;

pub use middleware::{AuthUser, require_auth};

pub fn router() -> Router<AppState> {
    // Só nas rotas de auth, não global: o perfil de uso de upload de laudo é
    // outro, e um limite pensado pra login atrapalharia ele. Alvo é força
    // bruta de senha + DoS barato (argon2 custa CPU deliberada por tentativa).
    //
    // SmartIpKeyExtractor lê X-Forwarded-For/Forwarded antes do IP da conexão
    // TCP — atrás de proxy, o IP da conexão é sempre o do proxy, e sem isso
    // todo mundo cairia num balde só. Só é confiável se a aplicação não for
    // alcançável direto (só via proxy) e o proxy SOBRESCREVER o header, não
    // concatenar — ver CLAUDE.md/plano para a invariante completa. Sob Lambda,
    // o balde é por instância de execução: não é controle de segurança, é só
    // amortecedor contra cliente mal-comportado numa instância quente.
    let governor_config = GovernorConfigBuilder::default()
        .key_extractor(SmartIpKeyExtractor)
        .period(Duration::from_secs(1))
        .burst_size(10)
        .finish()
        .expect("configuração de rate limiting inválida");

    Router::new()
        .route("/register", post(routes::register))
        .route("/login", post(routes::login))
        .route("/google", post(routes::google))
        .route("/refresh", post(routes::refresh))
        .route("/logout", post(routes::logout))
        .layer(GovernorLayer::new(governor_config))
}
