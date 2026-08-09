use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ============================================================
// Enums de resposta fechada
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
pub enum TernaryAnswer {
    Yes,
    No,
    Partial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
pub enum BinaryAnswer {
    Yes,
    No,
}

/// Campo "resposta + observação" da avaliação qualitativa (§4 do glossário).
/// No legado era uma string única "Sim: texto" separada por split — aqui é
/// um objeto real.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitativeAnswer {
    pub answer: TernaryAnswer,
    pub notes: String,
}

/// Campo "resposta + observação" dos ensaios da avaliação quantitativa
/// (§5 Parte II do glossário) — binário, não ternário.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAnswer {
    pub answer: BinaryAnswer,
    pub notes: String,
}

// ============================================================
// Seção 2 do glossário — inspection_planning (JSONB)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionPlanning {
    pub professional_qualification: String,
    pub team_fit_for_work: bool,
    pub safety_briefing_held: bool,
    pub has_nr10_training: bool,
    pub service_pre_checked: bool,
    pub identified_hazards: Vec<String>,
    pub safety_equipment: Vec<String>,
    pub requires_shutdown: bool,
    pub signage_used: Vec<String>,
    pub requires_area_delimitation: bool,
    pub requires_utility_assistance: bool,
    pub requires_voltage_check: bool,
    pub requires_temporary_grounding: bool,
    pub work_at_height: bool,
    pub requires_safety_harness: bool,
    pub safety_requirements_met: bool,
    pub requires_reassessment: bool,
}

// ============================================================
// Seção 3 do glossário — external_influences (JSONB)
// ============================================================
// Cada campo guarda o código NBR escolhido (ex.: "AA4"), validado na borda
// da API contra docs/nbr-5410-choices.json.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalInfluences {
    pub ambient_temperature_class: String,
    pub climatic_conditions_class: String,
    pub altitude_class: String,
    pub water_presence_class: String,
    pub solid_bodies_presence_class: String,
    pub corrosive_substances_class: String,
    pub mechanical_impact_class: String,
    pub vibration_class: String,
    pub flora_and_mold_class: String,
    pub fauna_presence_class: String,
    pub electromagnetic_influence_class: String,
    pub solar_radiation_class: String,
    pub lightning_exposure_class: String,
    pub air_movement_class: String,
    pub wind_class: String,
    pub people_competence_class: String,
    pub body_electrical_resistance_class: String,
    pub earth_potential_contact_class: String,
    pub evacuation_conditions_class: String,
    pub processed_materials_class: String,
    pub construction_materials_class: String,
    pub building_structure_class: String,
}

// ============================================================
// Seção 4 do glossário — qualitative_assessment (JSONB)
// ============================================================
// spare_circuit_capacity e earthing_system_type são as duas exceções ao
// padrão resposta+observação (são escolha única, sem campo de observação).

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitativeAssessment {
    pub has_installation_documentation: QualitativeAnswer,
    pub renovation_documentation_updated: QualitativeAnswer,
    pub inspected_before_commissioning: QualitativeAnswer,
    pub wiring_allows_maintenance_access: QualitativeAnswer,
    pub components_selected_for_external_influences: QualitativeAnswer,
    pub wiring_correctly_installed: QualitativeAnswer,
    pub outlets_comply_nbr14136: QualitativeAnswer,
    pub sufficient_outlet_count: QualitativeAnswer,
    pub distribution_board_accessible: QualitativeAnswer,
    /// Faixa escolhida (ex.: "7 a 12"). O espaço-reserva calculado (NBR
    /// 5410 6.5.4.7, ver docs/nbr-5410-tests.md) NÃO fica guardado aqui —
    /// é derivado do número real de circuitos na hora de gerar o laudo,
    /// não congelado neste campo.
    pub spare_circuit_capacity: String,
    pub distribution_board_warning_labels: QualitativeAnswer,
    pub protection_devices_identified: QualitativeAnswer,
    pub protection_matches_conductor_gauge: QualitativeAnswer,
    pub has_neutral_and_earth_busbars: QualitativeAnswer,
    pub terminals_match_conductor_gauge: QualitativeAnswer,
    pub conductors_color_identified: QualitativeAnswer,
    pub has_residual_current_device: QualitativeAnswer,
    pub has_surge_protection_device: QualitativeAnswer,
    pub has_safety_service_equipment: QualitativeAnswer,
    /// Determina o ramo condicional do ensaio 7.3.5 (equipotential_bonding_test)
    /// — ver docs/nbr-5410-tests.md.
    pub earthing_system_type: String,
    pub has_backup_power_source: QualitativeAnswer,
    pub has_safety_power_source: QualitativeAnswer,
    pub has_source_paralleling_prevention: QualitativeAnswer,
}

// ============================================================
// Seção 5 do glossário — quantitative_assessment (JSONB)
// ============================================================
// Partes I e II. circuits (Parte III) é tabela relacional própria, não
// entra aqui — ver circuit.rs.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantitativeAssessment {
    // Parte I — Quadro de distribuição / alimentador principal
    pub busbar_capacity_amps: Decimal,
    pub main_breaker_rating_amps: Decimal,
    pub rcd_rating_amps: Decimal,
    pub spd_rating_amps: Decimal,
    pub voltage_ab_volts: Decimal,
    pub voltage_an_volts: Decimal,
    pub current_phase_a_amps: Decimal,
    pub voltage_bc_volts: Decimal,
    pub voltage_bn_volts: Decimal,
    pub current_phase_b_amps: Decimal,
    pub voltage_ca_volts: Decimal,
    pub voltage_cn_volts: Decimal,
    pub current_phase_c_amps: Decimal,

    // Parte II — Ensaios realizados (procedimento e critério de aceitação
    // de cada um em docs/nbr-5410-tests.md)
    pub continuity_test: TestAnswer,
    pub insulation_resistance_test: TestAnswer,
    pub selv_pelv_separation_test: TestAnswer,
    pub equipotential_bonding_test: TestAnswer,
    pub applied_voltage_test: TestAnswer,
    pub functional_test: TestAnswer,
}
