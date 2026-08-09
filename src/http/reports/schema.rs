use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use crate::domain::{
    Circuit, ExternalInfluences, InspectionPlanning, QualitativeAssessment, Report, ReportStatus,
};
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

/// Valor fora da lista de `nbr-5410-choices.json` é conteúdo normativo inválido:
/// passa pelo serde (é `String`), então só esta checagem o barra.
fn check(field: &str, value: &str) -> Result<(), ApiError> {
    match crate::domain::is_allowed(field, value) {
        Some(false) => Err(ApiError::Unprocessable(format!(
            "Valor fora da norma NBR 5410 em {field}: {value}."
        ))),
        _ => Ok(()),
    }
}

fn check_each(field: &str, values: &[String]) -> Result<(), ApiError> {
    values.iter().try_for_each(|value| check(field, value))
}

pub fn validate_inspection_planning(section: &InspectionPlanning) -> Result<(), ApiError> {
    check("professional_qualification", &section.professional_qualification)?;
    check_each("identified_hazards", &section.identified_hazards)?;
    check_each("safety_equipment", &section.safety_equipment)?;
    check_each("signage_used", &section.signage_used)
}

pub fn validate_external_influences(section: &ExternalInfluences) -> Result<(), ApiError> {
    check("ambient_temperature_class", &section.ambient_temperature_class)?;
    check("climatic_conditions_class", &section.climatic_conditions_class)?;
    check("altitude_class", &section.altitude_class)?;
    check("water_presence_class", &section.water_presence_class)?;
    check("solid_bodies_presence_class", &section.solid_bodies_presence_class)?;
    check("corrosive_substances_class", &section.corrosive_substances_class)?;
    check("mechanical_impact_class", &section.mechanical_impact_class)?;
    check("vibration_class", &section.vibration_class)?;
    check("flora_and_mold_class", &section.flora_and_mold_class)?;
    check("fauna_presence_class", &section.fauna_presence_class)?;
    check("electromagnetic_influence_class", &section.electromagnetic_influence_class)?;
    check("solar_radiation_class", &section.solar_radiation_class)?;
    check("lightning_exposure_class", &section.lightning_exposure_class)?;
    check("air_movement_class", &section.air_movement_class)?;
    check("wind_class", &section.wind_class)?;
    check("people_competence_class", &section.people_competence_class)?;
    check("body_electrical_resistance_class", &section.body_electrical_resistance_class)?;
    check("earth_potential_contact_class", &section.earth_potential_contact_class)?;
    check("evacuation_conditions_class", &section.evacuation_conditions_class)?;
    check("processed_materials_class", &section.processed_materials_class)?;
    check("construction_materials_class", &section.construction_materials_class)?;
    check("building_structure_class", &section.building_structure_class)
}

pub fn validate_qualitative_assessment(section: &QualitativeAssessment) -> Result<(), ApiError> {
    check("spare_circuit_capacity", &section.spare_circuit_capacity)?;
    check("earthing_system_type", &section.earthing_system_type)
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
