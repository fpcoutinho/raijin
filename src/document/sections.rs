use rust_decimal::Decimal;

use crate::domain::{ExternalInfluences, InspectionPlanning, QualitativeAssessment, QuantitativeAssessment};

use super::labels::{
    self, CIRCUIT_FIELDS, EXTERNAL_INFLUENCES, INSPECTION_PLANNING, QUALITATIVE_ASSESSMENT,
    QUANTITATIVE_MEASUREMENTS, QUANTITATIVE_TESTS,
};
use super::{Finding, ReportInput, Section, SectionState};

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
    fn new(key: &'static str, entries: Vec<(String, String)>, state: SectionState) -> Self {
        Section { key, title: title_of(key), entries, state, findings: Vec::new() }
    }

    fn with_findings(mut self, all: &[Finding]) -> Self {
        self.findings = findings_for(all, self.key);
        self
    }
}

fn inspection_planning_entries(section: &InspectionPlanning) -> Vec<(String, String)> {
    let label = |field| labels::field_label(INSPECTION_PLANNING, field).to_string();
    vec![
        (label("professional_qualification"), section.professional_qualification.clone()),
        (label("team_fit_for_work"), labels::bool_label(section.team_fit_for_work).to_string()),
        (label("safety_briefing_held"), labels::bool_label(section.safety_briefing_held).to_string()),
        (label("has_nr10_training"), labels::bool_label(section.has_nr10_training).to_string()),
        (label("service_pre_checked"), labels::bool_label(section.service_pre_checked).to_string()),
        (label("identified_hazards"), section.identified_hazards.join(", ")),
        (label("safety_equipment"), section.safety_equipment.join(", ")),
        (label("requires_shutdown"), labels::bool_label(section.requires_shutdown).to_string()),
        (label("signage_used"), section.signage_used.join(", ")),
        (label("requires_area_delimitation"), labels::bool_label(section.requires_area_delimitation).to_string()),
        (label("requires_utility_assistance"), labels::bool_label(section.requires_utility_assistance).to_string()),
        (label("requires_voltage_check"), labels::bool_label(section.requires_voltage_check).to_string()),
        (label("requires_temporary_grounding"), labels::bool_label(section.requires_temporary_grounding).to_string()),
        (label("work_at_height"), labels::bool_label(section.work_at_height).to_string()),
        (label("requires_safety_harness"), labels::bool_label(section.requires_safety_harness).to_string()),
        (label("safety_requirements_met"), labels::bool_label(section.safety_requirements_met).to_string()),
        (label("requires_reassessment"), labels::bool_label(section.requires_reassessment).to_string()),
    ]
}

fn external_influences_entries(section: &ExternalInfluences) -> Vec<(String, String)> {
    let entry = |field: &str, code: &str| {
        let label = format!(
            "{}{}",
            labels::field_label(EXTERNAL_INFLUENCES, field),
            labels::nbr_clause_suffix(field)
        );
        (label, labels::nbr_class_label(field, code))
    };
    vec![
        entry("ambient_temperature_class", &section.ambient_temperature_class),
        entry("climatic_conditions_class", &section.climatic_conditions_class),
        entry("altitude_class", &section.altitude_class),
        entry("water_presence_class", &section.water_presence_class),
        entry("solid_bodies_presence_class", &section.solid_bodies_presence_class),
        entry("corrosive_substances_class", &section.corrosive_substances_class),
        entry("mechanical_impact_class", &section.mechanical_impact_class),
        entry("vibration_class", &section.vibration_class),
        entry("flora_and_mold_class", &section.flora_and_mold_class),
        entry("fauna_presence_class", &section.fauna_presence_class),
        entry("electromagnetic_influence_class", &section.electromagnetic_influence_class),
        entry("solar_radiation_class", &section.solar_radiation_class),
        entry("lightning_exposure_class", &section.lightning_exposure_class),
        entry("air_movement_class", &section.air_movement_class),
        entry("wind_class", &section.wind_class),
        entry("people_competence_class", &section.people_competence_class),
        entry("body_electrical_resistance_class", &section.body_electrical_resistance_class),
        entry("earth_potential_contact_class", &section.earth_potential_contact_class),
        entry("evacuation_conditions_class", &section.evacuation_conditions_class),
        entry("processed_materials_class", &section.processed_materials_class),
        entry("construction_materials_class", &section.construction_materials_class),
        entry("building_structure_class", &section.building_structure_class),
    ]
}

fn qualitative_assessment_entries(
    section: &QualitativeAssessment,
    required_spare_circuits: Option<u32>,
) -> Vec<(String, String)> {
    let label = |field| format!("{}{}", labels::field_label(QUALITATIVE_ASSESSMENT, field), labels::nbr_clause_suffix(field));
    let answer_notes = |answer, notes: &str| {
        let base = labels::ternary_label(answer).to_string();
        if notes.trim().is_empty() { base } else { format!("{base} — {notes}") }
    };

    // spare_circuit_capacity é a única conta real do domínio (NBR 5410
    // 6.5.4.7): a faixa que o engenheiro escolheu entra como resposta, e o
    // espaço-reserva exigido de verdade (derivado do número real de
    // circuitos, não congelado) entra como fato separado — nunca é a IA
    // quem calcula isso.
    let spare_circuit_required = match required_spare_circuits {
        Some(required) => format!("{required} circuito(s), conforme NBR 5410 6.5.4.7"),
        None => "não calculado — nenhum circuito cadastrado".to_string(),
    };

    vec![
        (label("has_installation_documentation"), answer_notes(section.has_installation_documentation.answer, &section.has_installation_documentation.notes)),
        (label("renovation_documentation_updated"), answer_notes(section.renovation_documentation_updated.answer, &section.renovation_documentation_updated.notes)),
        (label("inspected_before_commissioning"), answer_notes(section.inspected_before_commissioning.answer, &section.inspected_before_commissioning.notes)),
        (label("wiring_allows_maintenance_access"), answer_notes(section.wiring_allows_maintenance_access.answer, &section.wiring_allows_maintenance_access.notes)),
        (label("components_selected_for_external_influences"), answer_notes(section.components_selected_for_external_influences.answer, &section.components_selected_for_external_influences.notes)),
        (label("wiring_correctly_installed"), answer_notes(section.wiring_correctly_installed.answer, &section.wiring_correctly_installed.notes)),
        (label("outlets_comply_nbr14136"), answer_notes(section.outlets_comply_nbr14136.answer, &section.outlets_comply_nbr14136.notes)),
        (label("sufficient_outlet_count"), answer_notes(section.sufficient_outlet_count.answer, &section.sufficient_outlet_count.notes)),
        (label("distribution_board_accessible"), answer_notes(section.distribution_board_accessible.answer, &section.distribution_board_accessible.notes)),
        (label("spare_circuit_capacity"), section.spare_circuit_capacity.clone()),
        ("Espaço-reserva exigido (calculado)".to_string(), spare_circuit_required),
        (label("distribution_board_warning_labels"), answer_notes(section.distribution_board_warning_labels.answer, &section.distribution_board_warning_labels.notes)),
        (label("protection_devices_identified"), answer_notes(section.protection_devices_identified.answer, &section.protection_devices_identified.notes)),
        (label("protection_matches_conductor_gauge"), answer_notes(section.protection_matches_conductor_gauge.answer, &section.protection_matches_conductor_gauge.notes)),
        (label("has_neutral_and_earth_busbars"), answer_notes(section.has_neutral_and_earth_busbars.answer, &section.has_neutral_and_earth_busbars.notes)),
        (label("terminals_match_conductor_gauge"), answer_notes(section.terminals_match_conductor_gauge.answer, &section.terminals_match_conductor_gauge.notes)),
        (label("conductors_color_identified"), answer_notes(section.conductors_color_identified.answer, &section.conductors_color_identified.notes)),
        (label("has_residual_current_device"), answer_notes(section.has_residual_current_device.answer, &section.has_residual_current_device.notes)),
        (label("has_surge_protection_device"), answer_notes(section.has_surge_protection_device.answer, &section.has_surge_protection_device.notes)),
        (label("has_safety_service_equipment"), answer_notes(section.has_safety_service_equipment.answer, &section.has_safety_service_equipment.notes)),
        (label("earthing_system_type"), section.earthing_system_type.clone()),
        (label("has_backup_power_source"), answer_notes(section.has_backup_power_source.answer, &section.has_backup_power_source.notes)),
        (label("has_safety_power_source"), answer_notes(section.has_safety_power_source.answer, &section.has_safety_power_source.notes)),
        (label("has_source_paralleling_prevention"), answer_notes(section.has_source_paralleling_prevention.answer, &section.has_source_paralleling_prevention.notes)),
    ]
}

fn decimal_entry(label: &str, unit: &str, value: Decimal) -> (String, String) {
    (label.to_string(), format!("{value} {unit}"))
}

fn quantitative_assessment_entries(section: &QuantitativeAssessment) -> Vec<(String, String)> {
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
    let mut entries = Vec::with_capacity(19);
    for (field, value) in measurements {
        let (_, label, unit) =
            QUANTITATIVE_MEASUREMENTS.iter().find(|(f, _, _)| *f == field).unwrap();
        entries.push(decimal_entry(label, unit, value));
    }

    let binary_notes = |answer, notes: &str| {
        let base = labels::binary_label(answer).to_string();
        if notes.trim().is_empty() { base } else { format!("{base} — {notes}") }
    };
    let test_label = |field| {
        format!("{}{}", labels::field_label(QUANTITATIVE_TESTS, field), labels::quantitative_test_clause_suffix(field))
    };

    entries.push((test_label("continuity_test"), binary_notes(section.continuity_test.answer, &section.continuity_test.notes)));
    entries.push((test_label("insulation_resistance_test"), binary_notes(section.insulation_resistance_test.answer, &section.insulation_resistance_test.notes)));
    entries.push((test_label("selv_pelv_separation_test"), binary_notes(section.selv_pelv_separation_test.answer, &section.selv_pelv_separation_test.notes)));
    entries.push((test_label("equipotential_bonding_test"), binary_notes(section.equipotential_bonding_test.answer, &section.equipotential_bonding_test.notes)));
    entries.push((test_label("applied_voltage_test"), binary_notes(section.applied_voltage_test.answer, &section.applied_voltage_test.notes)));
    entries.push((test_label("functional_test"), binary_notes(section.functional_test.answer, &section.functional_test.notes)));

    entries
}

fn circuits_entries(input: &ReportInput) -> Vec<(String, String)> {
    let field = |name| labels::field_label(CIRCUIT_FIELDS, name);
    input
        .circuits
        .iter()
        .enumerate()
        .map(|(index, circuit)| {
            let value = format!(
                "{}: {} | {}: {} | {}: {} | {}: {} | {}: {} | {}: {}A",
                field("circuit_model"),
                circuit.circuit_model,
                field("phase"),
                circuit.phase,
                field("breaker"),
                circuit.breaker,
                field("description"),
                circuit.description.as_deref().unwrap_or("—"),
                field("conductor"),
                circuit.conductor,
                field("current"),
                circuit.current,
            );
            (format!("Circuito {}", index + 1), value)
        })
        .collect()
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
                inspection_planning_entries(section),
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
                external_influences_entries(section),
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
                qualitative_assessment_entries(section, input.required_spare_circuits),
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
                quantitative_assessment_entries(section),
                SectionState::Filled,
            ),
            None => Section::new("quantitative_assessment", Vec::new(), SectionState::NotAssessed),
        }
        .with_findings(&input.findings),
    );

    let circuits_state =
        if input.circuits.is_empty() { SectionState::NotAssessed } else { SectionState::Filled };
    result.push(
        Section::new("circuits", circuits_entries(input), circuits_state)
            .with_findings(&input.findings),
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
        assert!(sections.iter().all(|s| s.entries.is_empty()));
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
