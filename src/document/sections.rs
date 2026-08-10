use rust_decimal::Decimal;

use crate::domain::{ExternalInfluences, InspectionPlanning, QualitativeAssessment, QuantitativeAssessment};

use super::labels::{
    self, EXTERNAL_INFLUENCES, INSPECTION_PLANNING, QUALITATIVE_ASSESSMENT,
    QUANTITATIVE_MEASUREMENTS, QUANTITATIVE_TESTS,
};
use super::{Finding, ReportInput, Section, SectionState, Table};

/// Ordem canônica do laudo — títulos verbatim de docs/report-template.md
/// §"Ordem e títulos das seções". `circuits` não tem título ali (Parte III
/// não tinha seção própria no template legado, era sub-tabela da Parte I) —
/// nome curto compatível com o resto.
const TITLES: &[(&str, &str)] = &[
    ("inspection_planning", "Avaliação e planejamento da execução"),
    ("external_influences", "Avaliação das influências externas da instalação elétrica"),
    ("qualitative_assessment", "Avaliação qualitativa da instalação elétrica"),
    ("quantitative_assessment", "Avaliação quantitativa da instalação"),
    ("circuits", "Circuitos"),
];

fn title_of(key: &'static str) -> &'static str {
    TITLES.iter().find(|(k, _)| *k == key).map(|(_, t)| *t).unwrap_or(key)
}

fn findings_for(findings: &[Finding], key: &str) -> Vec<Finding> {
    findings
        .iter()
        .filter(|f| f.report_section.as_deref() == Some(key))
        .map(|f| Finding {
            category: f.category.clone(),
            description: f.description.clone(),
            report_section: f.report_section.clone(),
        })
        .collect()
}

impl Section {
    fn new(key: &'static str, tables: Vec<Table>, state: SectionState) -> Self {
        Section { key, title: title_of(key), tables, state, findings: Vec::new() }
    }

    fn with_findings(mut self, all: &[Finding]) -> Self {
        self.findings = findings_for(all, self.key);
        self
    }
}

/// Numeração `Item` das tabelas do modelo. Linha derivada (não é item do
/// formulário original) entra sem número.
const DERIVED: &str = "—";

fn numbered(rows: Vec<Vec<String>>) -> Vec<Vec<String>> {
    rows.into_iter()
        .enumerate()
        .map(|(index, mut row)| {
            row.insert(0, (index + 1).to_string());
            row
        })
        .collect()
}

fn inspection_planning_table(section: &InspectionPlanning) -> Table {
    let label = |field| labels::field_label(INSPECTION_PLANNING, field).to_string();
    // A coluna "Observação" do modelo existe pra anotação à mão em campo:
    // InspectionPlanning não tem campo de observação por item, então ela sai
    // vazia — mantida pela fidelidade à grade, não por ter conteúdo.
    let row = |field, detail: String| vec![label(field), detail, DERIVED.to_string()];
    let yes_no = |field, value: bool| row(field, labels::bool_label(value).to_string());

    Table {
        caption: None,
        headers: vec!["Item", "Descrição", "Detalhamento", "Observação"],
        rows: numbered(vec![
            row("professional_qualification", section.professional_qualification.clone()),
            yes_no("team_fit_for_work", section.team_fit_for_work),
            yes_no("safety_briefing_held", section.safety_briefing_held),
            yes_no("has_nr10_training", section.has_nr10_training),
            yes_no("service_pre_checked", section.service_pre_checked),
            row("identified_hazards", section.identified_hazards.join(", ")),
            row("safety_equipment", section.safety_equipment.join(", ")),
            yes_no("requires_shutdown", section.requires_shutdown),
            row("signage_used", section.signage_used.join(", ")),
            yes_no("requires_area_delimitation", section.requires_area_delimitation),
            yes_no("requires_utility_assistance", section.requires_utility_assistance),
            yes_no("requires_voltage_check", section.requires_voltage_check),
            yes_no("requires_temporary_grounding", section.requires_temporary_grounding),
            yes_no("work_at_height", section.work_at_height),
            yes_no("requires_safety_harness", section.requires_safety_harness),
            yes_no("safety_requirements_met", section.safety_requirements_met),
            yes_no("requires_reassessment", section.requires_reassessment),
        ]),
    }
}

fn external_influences_table(section: &ExternalInfluences) -> Table {
    let row = |field: &str, code: &str| {
        vec![
            labels::field_label(EXTERNAL_INFLUENCES, field).to_string(),
            code.to_string(),
            labels::nbr_class_type(field, code),
            crate::domain::clause_of(field).unwrap_or(DERIVED).to_string(),
        ]
    };

    Table {
        caption: None,
        headers: vec!["Item", "Descrição", "Classificação", "Tipo", "Item da norma NBR 5410"],
        rows: numbered(vec![
            row("ambient_temperature_class", &section.ambient_temperature_class),
            row("climatic_conditions_class", &section.climatic_conditions_class),
            row("altitude_class", &section.altitude_class),
            row("water_presence_class", &section.water_presence_class),
            row("solid_bodies_presence_class", &section.solid_bodies_presence_class),
            row("corrosive_substances_class", &section.corrosive_substances_class),
            row("mechanical_impact_class", &section.mechanical_impact_class),
            row("vibration_class", &section.vibration_class),
            row("flora_and_mold_class", &section.flora_and_mold_class),
            row("fauna_presence_class", &section.fauna_presence_class),
            row("electromagnetic_influence_class", &section.electromagnetic_influence_class),
            row("solar_radiation_class", &section.solar_radiation_class),
            row("lightning_exposure_class", &section.lightning_exposure_class),
            row("air_movement_class", &section.air_movement_class),
            row("wind_class", &section.wind_class),
            row("people_competence_class", &section.people_competence_class),
            row("body_electrical_resistance_class", &section.body_electrical_resistance_class),
            row("earth_potential_contact_class", &section.earth_potential_contact_class),
            row("evacuation_conditions_class", &section.evacuation_conditions_class),
            row("processed_materials_class", &section.processed_materials_class),
            row("construction_materials_class", &section.construction_materials_class),
            row("building_structure_class", &section.building_structure_class),
        ]),
    }
}

fn qualitative_assessment_table(
    section: &QualitativeAssessment,
    required_spare_circuits: Option<u32>,
    circuit_count: usize,
) -> Table {
    let clause = |field: &str| crate::domain::clause_of(field).unwrap_or(DERIVED).to_string();
    let notes_cell = |notes: &str| {
        if notes.trim().is_empty() { DERIVED.to_string() } else { notes.to_string() }
    };
    let item = |field: &'static str, answer, notes: &str| {
        vec![
            labels::field_label(QUALITATIVE_ASSESSMENT, field).to_string(),
            labels::ternary_label(answer).to_string(),
            notes_cell(notes),
            clause(field),
        ]
    };
    let choice = |field: &'static str, value: &str| {
        vec![
            labels::field_label(QUALITATIVE_ASSESSMENT, field).to_string(),
            value.to_string(),
            DERIVED.to_string(),
            clause(field),
        ]
    };

    // spare_circuit_capacity é a única conta real do domínio (NBR 5410
    // 6.5.4.7): a faixa que o engenheiro escolheu entra como resposta, e o
    // espaço-reserva exigido de verdade (derivado do número real de
    // circuitos, não congelado) entra como fato separado — nunca é a IA
    // quem calcula isso.
    let spare_circuit_required = match required_spare_circuits {
        Some(required) => format!("{required} circuito(s)"),
        None => "não calculado".to_string(),
    };

    // Confronto pronto, com veredito explícito: comparar "7 a 12" com um
    // número de circuitos é juízo que nem a IA nem o leitor deveriam ter de
    // fazer no meio do texto.
    let (declared_bracket_answer, declared_bracket_notes) =
        match crate::domain::spare_circuit_bracket(circuit_count) {
            None => (
                "Não verificável".to_string(),
                "Nenhum circuito cadastrado no laudo.".to_string(),
            ),
            Some(actual) if actual == section.spare_circuit_capacity => (
                "Sim".to_string(),
                format!("{circuit_count} circuito(s) cadastrado(s), faixa \"{actual}\"."),
            ),
            Some(actual) => (
                "Não".to_string(),
                format!(
                    "Faixa declarada \"{}\"; os {circuit_count} circuito(s) cadastrado(s) caem em \"{actual}\". Inconsistência de preenchimento, não da instalação.",
                    section.spare_circuit_capacity
                ),
            ),
        };

    let mut rows = numbered(vec![
        item("has_installation_documentation", section.has_installation_documentation.answer, &section.has_installation_documentation.notes),
        item("renovation_documentation_updated", section.renovation_documentation_updated.answer, &section.renovation_documentation_updated.notes),
        item("inspected_before_commissioning", section.inspected_before_commissioning.answer, &section.inspected_before_commissioning.notes),
        item("wiring_allows_maintenance_access", section.wiring_allows_maintenance_access.answer, &section.wiring_allows_maintenance_access.notes),
        item("components_selected_for_external_influences", section.components_selected_for_external_influences.answer, &section.components_selected_for_external_influences.notes),
        item("wiring_correctly_installed", section.wiring_correctly_installed.answer, &section.wiring_correctly_installed.notes),
        item("outlets_comply_nbr14136", section.outlets_comply_nbr14136.answer, &section.outlets_comply_nbr14136.notes),
        item("sufficient_outlet_count", section.sufficient_outlet_count.answer, &section.sufficient_outlet_count.notes),
        item("distribution_board_accessible", section.distribution_board_accessible.answer, &section.distribution_board_accessible.notes),
        choice("spare_circuit_capacity", &section.spare_circuit_capacity),
        item("distribution_board_warning_labels", section.distribution_board_warning_labels.answer, &section.distribution_board_warning_labels.notes),
        item("protection_devices_identified", section.protection_devices_identified.answer, &section.protection_devices_identified.notes),
        item("protection_matches_conductor_gauge", section.protection_matches_conductor_gauge.answer, &section.protection_matches_conductor_gauge.notes),
        item("has_neutral_and_earth_busbars", section.has_neutral_and_earth_busbars.answer, &section.has_neutral_and_earth_busbars.notes),
        item("terminals_match_conductor_gauge", section.terminals_match_conductor_gauge.answer, &section.terminals_match_conductor_gauge.notes),
        item("conductors_color_identified", section.conductors_color_identified.answer, &section.conductors_color_identified.notes),
        item("has_residual_current_device", section.has_residual_current_device.answer, &section.has_residual_current_device.notes),
        item("has_surge_protection_device", section.has_surge_protection_device.answer, &section.has_surge_protection_device.notes),
        item("has_safety_service_equipment", section.has_safety_service_equipment.answer, &section.has_safety_service_equipment.notes),
        choice("earthing_system_type", &section.earthing_system_type),
        item("has_backup_power_source", section.has_backup_power_source.answer, &section.has_backup_power_source.notes),
        item("has_safety_power_source", section.has_safety_power_source.answer, &section.has_safety_power_source.notes),
        item("has_source_paralleling_prevention", section.has_source_paralleling_prevention.answer, &section.has_source_paralleling_prevention.notes),
    ]);

    rows.push(vec![
        DERIVED.to_string(),
        "Espaço-reserva exigido (calculado)".to_string(),
        spare_circuit_required,
        DERIVED.to_string(),
        "6.5.4.7".to_string(),
    ]);
    rows.push(vec![
        DERIVED.to_string(),
        "Faixa declarada confere com os circuitos cadastrados".to_string(),
        declared_bracket_answer,
        declared_bracket_notes,
        "6.5.4.7".to_string(),
    ]);

    Table {
        caption: None,
        headers: vec![
            "Item",
            "Descrição do item",
            "Aspectos observados atendem à norma?",
            "Observações",
            "Item da norma NBR 5410",
        ],
        rows,
    }
}

/// Tabela 10 do modelo tem duas partes com colunas diferentes na mesma seção
/// — medições e ensaios — e é por isso que `Section` guarda `Vec<Table>` em
/// vez de uma grade só.
fn quantitative_assessment_tables(section: &QuantitativeAssessment) -> Vec<Table> {
    let measurements: [(&str, Decimal); 13] = [
        ("busbar_capacity_amps", section.busbar_capacity_amps),
        ("main_breaker_rating_amps", section.main_breaker_rating_amps),
        ("rcd_rating_amps", section.rcd_rating_amps),
        ("spd_rating_amps", section.spd_rating_amps),
        ("voltage_ab_volts", section.voltage_ab_volts),
        ("voltage_an_volts", section.voltage_an_volts),
        ("current_phase_a_amps", section.current_phase_a_amps),
        ("voltage_bc_volts", section.voltage_bc_volts),
        ("voltage_bn_volts", section.voltage_bn_volts),
        ("current_phase_b_amps", section.current_phase_b_amps),
        ("voltage_ca_volts", section.voltage_ca_volts),
        ("voltage_cn_volts", section.voltage_cn_volts),
        ("current_phase_c_amps", section.current_phase_c_amps),
    ];
    let measurement_rows = measurements
        .into_iter()
        .map(|(field, value)| {
            let (_, label, unit) =
                QUANTITATIVE_MEASUREMENTS.iter().find(|(f, _, _)| *f == field).unwrap();
            vec![label.to_string(), format!("{value} {unit}")]
        })
        .collect();

    // "Motivo" é o campo de observação do domínio (no template legado,
    // `{{ ensaio[1] }}`). A coluna "Observações" do modelo não é dado do
    // laudo: é o procedimento normativo fixo de cada ensaio, que vive em
    // docs/nbr-5410-tests.md e ainda não é carregado por código.
    let test_row = |field: &'static str, answer, notes: &str| {
        vec![
            labels::quantitative_test_clause(field).to_string(),
            labels::field_label(QUANTITATIVE_TESTS, field).to_string(),
            labels::binary_label(answer).to_string(),
            if notes.trim().is_empty() { DERIVED.to_string() } else { notes.to_string() },
        ]
    };

    vec![
        Table {
            caption: Some("Parte I — Medições"),
            headers: vec!["Grandeza", "Valor medido"],
            rows: measurement_rows,
        },
        Table {
            caption: Some("Parte II — Ensaios realizados"),
            headers: vec![
                "Item da norma NBR 5410",
                "Descrição do ensaio",
                "Realizado?",
                "Motivo",
            ],
            rows: vec![
                test_row("continuity_test", section.continuity_test.answer, &section.continuity_test.notes),
                test_row("insulation_resistance_test", section.insulation_resistance_test.answer, &section.insulation_resistance_test.notes),
                test_row("selv_pelv_separation_test", section.selv_pelv_separation_test.answer, &section.selv_pelv_separation_test.notes),
                test_row("equipotential_bonding_test", section.equipotential_bonding_test.answer, &section.equipotential_bonding_test.notes),
                test_row("applied_voltage_test", section.applied_voltage_test.answer, &section.applied_voltage_test.notes),
                test_row("functional_test", section.functional_test.answer, &section.functional_test.notes),
            ],
        },
    ]
}

/// Cabeçalhos verbatim da sub-tabela "Circuitos terminais" (Tabela 10,
/// Parte I). Literais, não `CIRCUIT_FIELDS`: aquela lista rotula campo de
/// dado e devolve `String` com fallback, o que não serve pra cabeçalho fixo.
fn circuits_table(input: &ReportInput) -> Table {
    Table {
        caption: None,
        headers: vec!["Circuito", "Fase", "Disjuntor", "Descrição", "Condutor", "Corrente"],
        rows: input
            .circuits
            .iter()
            .map(|circuit| {
                vec![
                    circuit.circuit_model.clone(),
                    circuit.phase.clone(),
                    circuit.breaker.clone(),
                    circuit.description.clone().unwrap_or_else(|| DERIVED.to_string()),
                    circuit.conductor.clone(),
                    format!("{} A", circuit.current),
                ]
            })
            .collect(),
    }
}

/// Monta as seções do laudo na ordem canônica, com os achados já agrupados
/// por `report_section` (ver src/domain/image.rs REPORT_SECTIONS). Seção sem
/// dado preenchido entra como `NotAssessed` — nunca omitida em silêncio, nem
/// pro modelo determinístico nem pro prompt da IA.
pub fn sections(input: &ReportInput) -> Vec<Section> {
    let mut result = Vec::with_capacity(5);

    result.push(
        match &input.inspection_planning {
            Some(section) => Section::new(
                "inspection_planning",
                vec![inspection_planning_table(section)],
                SectionState::Filled,
            ),
            None => Section::new("inspection_planning", Vec::new(), SectionState::NotAssessed),
        }
        .with_findings(&input.findings),
    );

    result.push(
        match &input.external_influences {
            Some(section) => Section::new(
                "external_influences",
                vec![external_influences_table(section)],
                SectionState::Filled,
            ),
            None => Section::new("external_influences", Vec::new(), SectionState::NotAssessed),
        }
        .with_findings(&input.findings),
    );

    result.push(
        match &input.qualitative_assessment {
            Some(section) => Section::new(
                "qualitative_assessment",
                vec![qualitative_assessment_table(
                    section,
                    input.required_spare_circuits,
                    input.circuits.len(),
                )],
                SectionState::Filled,
            ),
            None => Section::new("qualitative_assessment", Vec::new(), SectionState::NotAssessed),
        }
        .with_findings(&input.findings),
    );

    result.push(
        match &input.quantitative_assessment {
            Some(section) => Section::new(
                "quantitative_assessment",
                quantitative_assessment_tables(section),
                SectionState::Filled,
            ),
            None => Section::new("quantitative_assessment", Vec::new(), SectionState::NotAssessed),
        }
        .with_findings(&input.findings),
    );

    let circuits_state =
        if input.circuits.is_empty() { SectionState::NotAssessed } else { SectionState::Filled };
    let circuits_tables =
        if input.circuits.is_empty() { Vec::new() } else { vec![circuits_table(input)] };
    result.push(
        Section::new("circuits", circuits_tables, circuits_state).with_findings(&input.findings),
    );

    result
}

/// Achados sem `report_section` — vão no apêndice geral de imagens, ao final
/// do documento, exatamente como funcionava antes de a coluna existir.
pub fn appendix_findings(input: &ReportInput) -> Vec<Finding> {
    input
        .findings
        .iter()
        .filter(|f| f.report_section.is_none())
        .map(|f| Finding {
            category: f.category.clone(),
            description: f.description.clone(),
            report_section: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_input() -> ReportInput {
        ReportInput {
            inspection_planning: None,
            external_influences: None,
            qualitative_assessment: None,
            quantitative_assessment: None,
            circuits: Vec::new(),
            required_spare_circuits: None,
            findings: Vec::new(),
        }
    }

    #[test]
    fn secao_sem_dado_vira_nao_avaliada_nunca_omitida() {
        let sections = sections(&empty_input());

        assert_eq!(sections.len(), 5);
        assert!(sections.iter().all(|s| s.state == SectionState::NotAssessed));
        assert!(sections.iter().all(|s| s.tables.is_empty()));
    }

    #[test]
    fn achado_com_secao_entra_na_secao_e_nao_no_apendice() {
        let mut input = empty_input();
        input.findings.push(Finding {
            category: "exposed_live_conductors".to_string(),
            description: Some("Fiação exposta no quadro".to_string()),
            report_section: Some("quantitative_assessment".to_string()),
        });
        input.findings.push(Finding {
            category: "improvised_earthing".to_string(),
            description: None,
            report_section: None,
        });

        let sections = sections(&input);
        let quantitative = sections.iter().find(|s| s.key == "quantitative_assessment").unwrap();
        assert_eq!(quantitative.findings.len(), 1);
        assert_eq!(quantitative.findings[0].category, "exposed_live_conductors");

        let appendix = appendix_findings(&input);
        assert_eq!(appendix.len(), 1);
        assert_eq!(appendix[0].category, "improvised_earthing");
    }
}
