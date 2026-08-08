use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::assessment::{ExternalInfluences, InspectionPlanning, QualitativeAssessment, QuantitativeAssessment};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "report_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Draft,
    InReview,
    Approved,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Report {
    pub id: Uuid,
    pub author_id: Uuid,
    pub location_code: String,
    pub inspected_at: DateTime<Utc>,
    pub ambient_temperature_c: Option<i32>,
    pub weather_conditions: Option<String>,
    pub responsible_parties: Vec<String>,
    pub status: ReportStatus,
    // None = seção ainda não preenchida (coluna NULL). Ver 0001_initial.sql.
    pub inspection_planning: Option<sqlx::types::Json<InspectionPlanning>>,
    pub external_influences: Option<sqlx::types::Json<ExternalInfluences>>,
    pub qualitative_assessment: Option<sqlx::types::Json<QualitativeAssessment>>,
    pub quantitative_assessment: Option<sqlx::types::Json<QuantitativeAssessment>>,
    pub document_content: sqlx::types::Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
