use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Circuit {
    pub id: Uuid,
    pub report_id: Uuid,
    pub circuit_model: Option<String>,
    pub phase: Option<String>,
    pub breaker: Option<String>,
    pub description: Option<String>,
    pub conductor: Option<String>,
    pub current: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
