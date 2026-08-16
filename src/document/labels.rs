use crate::domain::{BinaryAnswer, TernaryAnswer};

// Rótulos pt-BR por nome de campo, de docs/domain-glossary.md (coluna "Rótulo
// pt-BR"). Duplicação consciente do glossário — único lugar do código que
// precisa desses ~82 rótulos, então fica aqui em vez de round-trip a um
// arquivo à parte. Códigos NBR (influências externas) NÃO entram aqui: são
// resolvidos em runtime por domain::label_of, fonte única em
// docs/nbr-5410-choices.json.

pub const INSPECTION_PLANNING: &[(&str, &str)] = &[
    ("professional_qualification", "Qual a qualificação profissional dos responsáveis pela inspeção?"),
    ("team_fit_for_work", "Os participantes da inspeção estão bem fisicamente e mentalmente?"),
    ("safety_briefing_held", "Houve diálogo de segurança?"),
    ("has_nr10_training", "Um ou mais executores da inspeção possui curso NR-10?"),
    ("service_pre_checked", "O serviço foi preliminarmente conferido?"),
    ("identified_hazards", "Quais riscos foram detectados?"),
    ("safety_equipment", "Quais equipamentos de segurança serão utilizados?"),
    ("requires_shutdown", "Este serviço requer desligamento ou bloqueio de equipamento ou rede?"),
    ("signage_used", "Este serviço requer sinalização?"),
    ("requires_area_delimitation", "Necessita delimitar a área de trabalho?"),
    ("requires_utility_assistance", "Necessita de auxílio de concessionária local?"),
    ("requires_voltage_check", "Necessário fazer verificação de tensão?"),
    ("requires_temporary_grounding", "A inspeção requer aterramento temporário?"),
    ("work_at_height", "A inspeção será realizada em altura?"),
    ("requires_safety_harness", "Será necessário se aprisionar à escada e utilização de cinto de segurança?"),
    ("safety_requirements_met", "Os requisitos de segurança foram atendidos por todos?"),
    ("requires_reassessment", "Houve necessidade de reavaliação das inspeções realizadas?"),
];

pub const EXTERNAL_INFLUENCES: &[(&str, &str)] = &[
    ("ambient_temperature_class", "Temperatura ambiente"),
    ("climatic_conditions_class", "Condições climáticas do ambiente"),
    ("altitude_class", "Altitude"),
    ("water_presence_class", "Presença de água"),
    ("solid_bodies_presence_class", "Presença de corpos sólidos"),
    ("corrosive_substances_class", "Presença de substâncias corrosivas ou poluentes"),
    ("mechanical_impact_class", "Impactos mecânicos"),
    ("vibration_class", "Vibrações"),
    ("flora_and_mold_class", "Presença de flora e mofo"),
    ("fauna_presence_class", "Presença de fauna"),
    ("electromagnetic_influence_class", "Influências eletromagnéticas, eletrostáticas ou ionizantes"),
    ("solar_radiation_class", "Radiação solar"),
    ("lightning_exposure_class", "Descargas atmosféricas"),
    ("air_movement_class", "Movimentação do ar"),
    ("wind_class", "Vento"),
    ("people_competence_class", "Competência das pessoas"),
    ("body_electrical_resistance_class", "Resistência elétrica do corpo humano no ambiente"),
    ("earth_potential_contact_class", "Contato das pessoas com o potencial da terra"),
    ("evacuation_conditions_class", "Condições de fuga das pessoas em emergências"),
    ("processed_materials_class", "Natureza dos materiais processados ou armazenados"),
    ("construction_materials_class", "Qual a natureza dos materiais de construção"),
    ("building_structure_class", "Qual a classificação da estrutura das edificações"),
];

pub const QUALITATIVE_ASSESSMENT: &[(&str, &str)] = &[
    ("has_installation_documentation", "Há documentação da instalação e esta inclui plantas, esquemas unifilares e outros, detalhes de montagem, memorial descritivo, especificações de componentes, parâmetros de projeto?"),
    ("renovation_documentation_updated", "O ambiente sofreu alguma reforma e a documentação foi atualizada ou acrescida de algum aditivo de projeto?"),
    ("inspected_before_commissioning", "A instalação foi inspecionada antes da entrada em funcionamento e existe algum documento atestando esse fato?"),
    ("wiring_allows_maintenance_access", "As linhas elétricas estão dispostas de modo a permitir verificações, ensaios, reparos ou modificação da instalação?"),
    ("components_selected_for_external_influences", "Os componentes da instalação foram selecionados e instalados levando-se em conta as influências externas?"),
    ("wiring_correctly_installed", "As linhas elétricas estão corretamente instaladas?"),
    ("outlets_comply_nbr14136", "As tomadas de força existentes atendem ao novo padrão nacional NBR 14136/2002?"),
    ("sufficient_outlet_count", "O ambiente apresenta tomadas de força em quantidade suficiente?"),
    ("distribution_board_accessible", "O quadro de distribuição está devidamente instalado em local de fácil acesso à manutenção, inspeção e ensaio?"),
    ("spare_circuit_capacity", "Há disponibilidade de criação de novos circuitos no quadro de distribuição?"),
    ("distribution_board_warning_labels", "Há indicações de advertência nos quadros de distribuição?"),
    ("protection_devices_identified", "Os dispositivos de proteção estão dispostos e identificados de forma fácil de reconhecer os respectivos circuitos protegidos?"),
    ("protection_matches_conductor_gauge", "A proteção dos circuitos é compatível com a bitola dos condutores?"),
    ("has_neutral_and_earth_busbars", "O Quadro de distribuição possui barramento de neutro e aterramento?"),
    ("terminals_match_conductor_gauge", "Todas as conexões estão com terminais apropriados para cada bitola utilizada?"),
    ("conductors_color_identified", "Os condutores estão identificados por cores ou conforme sua função?"),
    ("has_residual_current_device", "Existe disjuntor diferencial residual instalado no quadro de distribuição?"),
    ("has_surge_protection_device", "Existe dispositivo de proteção contra surtos de tensões?"),
    ("has_safety_service_equipment", "Há elementos para serviços de segurança a exemplo de iluminação de emergência, exaustores de fumaça, etc?"),
    ("earthing_system_type", "Qual o esquema de aterramento utilizado?"),
    ("has_backup_power_source", "Existe fonte alternativa ou de reserva de energia?"),
    ("has_safety_power_source", "Existe fonte de segurança de energia?"),
    ("has_source_paralleling_prevention", "Há mecanismos para evitar o paralelismo das fontes?"),
];

pub const QUANTITATIVE_MEASUREMENTS: &[(&str, &str, &str)] = &[
    ("busbar_capacity_amps", "Capacidade de barramento", "A"),
    ("main_breaker_rating_amps", "Proteção Geral Disjuntor", "A"),
    ("rcd_rating_amps", "Proteção DR", "A"),
    ("spd_rating_amps", "Proteção DPS", "A"),
    ("voltage_ab_volts", "Vab", "V"),
    ("voltage_an_volts", "Van", "V"),
    ("current_phase_a_amps", "Ia", "A"),
    ("voltage_bc_volts", "Vbc", "V"),
    ("voltage_bn_volts", "Vbn", "V"),
    ("current_phase_b_amps", "Ib", "A"),
    ("voltage_ca_volts", "Vca", "V"),
    ("voltage_cn_volts", "Vcn", "V"),
    ("current_phase_c_amps", "Ic", "A"),
];

pub const QUANTITATIVE_TESTS: &[(&str, &str)] = &[
    ("continuity_test", "Continuidade dos condutores de proteção e das eqüipotencializações principal e suplementar?"),
    ("insulation_resistance_test", "Resistência de isolamento da instalação elétrica?"),
    ("selv_pelv_separation_test", "Resistência de isolamento aplicável a SELV, PELV e separação elétrica?"),
    ("equipotential_bonding_test", "Verificação das condições de proteção por eqüipotencialização e seccionamento automático da alimentação?"),
    ("applied_voltage_test", "Ensaio de tensão aplicada?"),
    ("functional_test", "Ensaio de funcionamento?"),
];

/// Rótulos pt-BR das 5 categorias de achado — docs/findings-taxonomy.md
/// §"Identificadores canônicos". Slug desconhecido mostra como veio, não some.
pub const FINDING_CATEGORIES: &[(&str, &str)] = &[
    ("exposed_live_conductors", "Condutores energizados expostos e sem proteção"),
    ("improvised_earthing", "Aterramentos improvisados"),
    ("splice_conditions", "Condições das emendas"),
    ("poorly_installed_wiring", "Linhas elétricas mal instaladas ou afixadas"),
    ("short_circuit_or_hotspot_signs", "Sinais de ocorrência de curtos ou pontos quentes"),
];

pub fn finding_category_label(category: &str) -> String {
    FINDING_CATEGORIES
        .iter()
        .find(|(slug, _)| *slug == category)
        .map(|(_, label)| label.to_string())
        .unwrap_or_else(|| category.to_string())
}

pub fn field_label(table: &[(&str, &str)], field: &str) -> String {
    table.iter().find(|(name, _)| *name == field).map(|(_, label)| label.to_string()).unwrap_or_else(|| field.to_string())
}

/// A resposta como **letra**, que é o que a coluna de conformidade das Tabelas
/// 9 e 11 recebe: o cabeçalho já traz a legenda "(S) SIM (N) NÃO (P)
/// PARCIALMENTE", e repetir a palavra inteira em 23 linhas empurra a coluna de
/// observações para fora da folha.
///
/// Vale também para o prompt da IA, que consome as mesmas tabelas (ver
/// `Section`): o cabeçalho com a legenda vai junto, então a letra continua
/// decodificável do outro lado.
pub fn ternary_letter(answer: TernaryAnswer) -> &'static str {
    match answer {
        TernaryAnswer::Yes => "S",
        TernaryAnswer::No => "N",
        TernaryAnswer::Partial => "P",
    }
}

pub fn binary_letter(answer: BinaryAnswer) -> &'static str {
    match answer {
        BinaryAnswer::Yes => "S",
        BinaryAnswer::No => "N",
    }
}

/// Rótulo completo do código NBR de influências externas (`domain::label_of`);
/// se o código não estiver na lista normativa, mostra como veio — não some
/// nem inventa texto, só deixa de traduzir.
pub fn nbr_class_label(field: &str, code: &str) -> String {
    crate::domain::label_of(field, code).map(str::to_string).unwrap_or_else(|| code.to_string())
}

/// Só o tipo, sem o código — a coluna "Classificação" do modelo guarda `AA5`
/// e a coluna "Tipo" guarda o resto. Separadas na origem porque juntá-las
/// obrigaria o `itui` a desfazer a concatenação pra montar a tabela.
pub fn nbr_class_type(field: &str, code: &str) -> String {
    let full = nbr_class_label(field, code);
    full.strip_prefix(code)
        .map(|rest| rest.trim_start_matches([' ', '-']).trim().to_string())
        .filter(|rest| !rest.is_empty())
        .unwrap_or(full)
}

pub fn quantitative_test_clause(field: &str) -> &'static str {
    crate::domain::test_clause(field).unwrap_or("—")
}

