use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Slugs válidos de `finding_category` — espelha a tabela "Identificadores
/// canônicos" em docs/findings-taxonomy.md. Lista pequena e estável (5
/// categorias fechadas por um levantamento de domínio, não pela norma), por
/// isso validada aqui em vez de round-trip a um arquivo JSON como as opções
/// da NBR 5410; ainda assim não é enum de banco, pra poder crescer sem
/// migration — só atualizando esta constante e o doc.
pub const FINDING_CATEGORIES: &[&str] = &[
    "exposed_live_conductors",
    "improvised_earthing",
    "splice_conditions",
    "poorly_installed_wiring",
    "short_circuit_or_hotspot_signs",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "image_upload_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ImageUploadStatus {
    Pending,
    Uploaded,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReportImage {
    pub id: Uuid,
    pub report_id: Uuid,
    pub storage_path: String,
    pub finding_category: Option<String>,
    pub upload_status: ImageUploadStatus,
    pub content_type: Option<String>,
    pub size_bytes: Option<i64>,
    pub uploaded_at: Option<DateTime<Utc>>,
    pub caption: Option<String>,
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
