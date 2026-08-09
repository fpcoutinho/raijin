use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use crate::domain::{Circuit, Report, ReportStatus};
use crate::http::error::ApiError;

// As seções (§2–§5) não são redeclaradas aqui: o corpo dos PATCH de seção é a
// própria struct de domain::assessment, desserializada direto pelo handler.
// Redeclarar os ~90 campos criaria duas fontes da verdade.

#[derive(Debug, Deserialize)]
pub struct CreateReportRequest {
    pub location_code: String,
    pub inspected_at: DateTime<Utc>,
    pub ambient_temperature_c: Option<i32>,
    pub weather_conditions: Option<String>,
    pub responsible_parties: Option<Vec<String>>,
}

impl CreateReportRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        if self.location_code.trim().is_empty() {
            return Err(ApiError::Unprocessable("Informe o local da inspeção.".to_string()));
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct CreatedReport {
    #[serde(flatten)]
    pub report: Report,
    /// Avisa a UI que o planejamento veio copiado de outro laudo do mesmo bloco,
    /// pra ela pedir revisão — dado de segurança não se herda em silêncio.
    pub planning_autofilled: bool,
}

/// Listagem não devolve as seções JSONB: são ~90 campos por laudo que a tela de
/// lista não usa. Quem precisa delas busca o laudo individual.
#[derive(Debug, Serialize)]
pub struct ReportSummary {
    pub id: Uuid,
    pub location_code: String,
    pub inspected_at: DateTime<Utc>,
    pub status: ReportStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ListReportsQuery {
    pub status: Option<ReportStatus>,
    /// Prefixo de bloco: "CCHLA" casa CCHLA-102 e CCHLA-205.
    pub location_prefix: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl ListReportsQuery {
    pub const DEFAULT_LIMIT: i64 = 20;
    pub const MAX_LIMIT: i64 = 100;

    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(Self::DEFAULT_LIMIT).clamp(1, Self::MAX_LIMIT)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

/// Circuitos vêm embutidos (o wizard carrega a Parte III junto); imagens não —
/// cada uma exigiria assinar uma URL de leitura de vida curta.
#[derive(Debug, Serialize)]
pub struct ReportDetail {
    #[serde(flatten)]
    pub report: Report,
    pub circuits: Vec<Circuit>,
}

/// Campo ausente fica inalterado. `Option<Option<T>>` nos campos que aceitam
/// null distingue "não mandou" de "mandou null pra limpar".
#[derive(Debug, Deserialize)]
pub struct UpdateReportRequest {
    pub location_code: Option<String>,
    pub inspected_at: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "double_option")]
    pub ambient_temperature_c: Option<Option<i32>>,
    #[serde(default, deserialize_with = "double_option")]
    pub weather_conditions: Option<Option<String>>,
    pub responsible_parties: Option<Vec<String>>,
    pub status: Option<ReportStatus>,
}

impl UpdateReportRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        if self.location_code.as_ref().is_some_and(|code| code.trim().is_empty()) {
            return Err(ApiError::Unprocessable("Informe o local da inspeção.".to_string()));
        }
        Ok(())
    }
}

/// Sem isso o serde colapsaria `null` explícito em `None`, indistinguível de
/// campo ausente — e o PATCH perderia o "limpe este campo".
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::deserialize(deserializer).map(Some)
}
