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

/// Coluna de ordenação. Enum fechado de propósito: o nome nunca é concatenado
/// no SQL — vira um literal comparado dentro do `ORDER BY` (ver `queries.rs`) —,
/// e valor desconhecido é rejeitado pelo serde antes de chegar ao banco.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportSortField {
    LocationCode,
    InspectedAt,
    Status,
    CreatedAt,
    /// A tela abre pelo trabalho mais recente, não pela data de criação.
    #[default]
    UpdatedAt,
}

impl ReportSortField {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocationCode => "location_code",
            Self::InspectedAt => "inspected_at",
            Self::Status => "status",
            Self::CreatedAt => "created_at",
            Self::UpdatedAt => "updated_at",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Asc,
    #[default]
    Desc,
}

impl SortDirection {
    pub fn is_ascending(self) -> bool {
        self == Self::Asc
    }
}

#[derive(Debug, Deserialize)]
pub struct ListReportsQuery {
    pub status: Option<ReportStatus>,
    /// Prefixo de bloco: "CCHLA" casa CCHLA-102 e CCHLA-205.
    pub location_prefix: Option<String>,
    /// Busca livre, sem diferenciar maiúsculas: trecho em qualquer posição do
    /// `location_code` **ou** de um dos `responsible_parties`. Convive com
    /// `location_prefix` — aquele é âncora de bloco, este é caixa de busca.
    pub search: Option<String>,
    pub sort: Option<ReportSortField>,
    pub order: Option<SortDirection>,
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

    pub fn sort(&self) -> ReportSortField {
        self.sort.unwrap_or_default()
    }

    pub fn order(&self) -> SortDirection {
        self.order.unwrap_or_default()
    }

    /// Busca em branco é o mesmo que busca ausente: `?search=` vem do campo
    /// vazio da UI e não deve virar um `LIKE '%%'` que casa tudo por acidente.
    pub fn search(&self) -> Option<&str> {
        self.search.as_deref().map(str::trim).filter(|term| !term.is_empty())
    }
}

/// Página da listagem. O array cru não dizia quantos laudos existem fora dela —
/// sem isso a UI não sabe desenhar a última página nem o total.
#[derive(Debug, Serialize)]
pub struct ReportPage {
    pub items: Vec<ReportSummary>,
    /// 1-based, derivado de `offset`/`limit`.
    pub page: i64,
    pub page_size: i64,
    pub total_items: i64,
    pub total_pages: i64,
}

impl ReportPage {
    pub fn new(items: Vec<ReportSummary>, total_items: i64, limit: i64, offset: i64) -> Self {
        Self {
            items,
            page: offset / limit + 1,
            page_size: limit,
            total_items,
            // Divisão pra cima. Lista vazia tem zero páginas, não uma em branco.
            total_pages: (total_items + limit - 1) / limit,
        }
    }
}

/// Circuitos vêm embutidos (o wizard carrega a Parte III junto); imagens não —
/// cada uma exigiria assinar uma URL de leitura de vida curta.
#[derive(Debug, Serialize)]
pub struct ReportDetail {
    #[serde(flatten)]
    pub report: Report,
    pub circuits: Vec<Circuit>,
    pub spare_circuits: SpareCircuits,
}

/// Derivado do número real de circuitos a cada leitura, nunca gravado: circuito
/// novo muda a exigência, e valor congelado no JSONB ficaria mentindo.
#[derive(Debug, Serialize)]
pub struct SpareCircuits {
    pub circuit_count: usize,
    /// NBR 5410 6.5.4.7. `None` sem circuito cadastrado.
    pub required: Option<u32>,
}

impl SpareCircuits {
    pub fn of(circuit_count: usize) -> Self {
        Self { circuit_count, required: crate::domain::required_spare_circuits(circuit_count) }
    }
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

/// `image_ids` ausente = todas as imagens confirmadas do laudo — nunca o
/// cliente ditando categoria/descrição diretamente (ver CLAUDE.md, regra de
/// privacidade do prompt).
#[derive(Debug, Deserialize, Default)]
pub struct GenerateRequest {
    #[serde(default)]
    pub image_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize)]
pub struct DraftQuery {
    pub image_ids: Option<String>,
}

impl DraftQuery {
    pub fn image_ids(&self) -> Option<Vec<Uuid>> {
        self.image_ids.as_ref().map(|csv| {
            csv.split(',').filter_map(|id| Uuid::parse_str(id.trim()).ok()).collect()
        })
    }
}

#[derive(Debug, Serialize)]
pub struct DraftResponse {
    pub text: String,
}
