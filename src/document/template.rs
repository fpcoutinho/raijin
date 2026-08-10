use super::labels::finding_category_label;
use super::{Finding, Section, SectionState};

/// Legenda `(a)(b)(c)...` — padrão do apêndice órfão que
/// docs/findings-taxonomy.md documenta, em vez do "fotos soltas sem legenda"
/// do template ativo do legado.
fn item_letter(index: usize) -> char {
    (b'a' + (index % 26) as u8) as char
}

fn render_findings(out: &mut String, findings: &[Finding]) {
    for (index, finding) in findings.iter().enumerate() {
        out.push_str(&format!(
            "\n**({}) {}**",
            item_letter(index),
            finding_category_label(&finding.category)
        ));
        if let Some(description) = &finding.description {
            out.push_str(&format!(" — {description}"));
        }
        out.push('\n');
    }
}

fn render_section(out: &mut String, section: &Section) {
    out.push_str(&format!("\n## {}\n\n", section.title));

    match section.state {
        SectionState::NotAssessed => {
            out.push_str("Seção não avaliada neste laudo.\n");
        }
        SectionState::Filled => {
            for (label, value) in &section.entries {
                out.push_str(&format!("- **{label}:** {value}\n"));
            }
        }
    }

    if !section.findings.is_empty() {
        out.push_str("\n### Não conformidades registradas nesta seção\n");
        render_findings(out, &section.findings);
    }
}

/// Modelo determinístico — replace de dados nas seções do laudo, sem
/// provedor de IA. Estrutura vem de docs/report-template.md (extraída do
/// `template.docx` legado); prosa de composição nova onde o legado não tinha
/// texto fixo (ver aquele documento, seção "Consequência"). Cada seção
/// imprime, junto de si, os achados fotográficos que a ilustram — não jogados
/// todos no final, diferente do apêndice raso do template ativo.
pub fn render(sections: &[Section], appendix: &[Finding]) -> String {
    let mut out = String::new();

    for section in sections {
        render_section(&mut out, section);
    }

    if !appendix.is_empty() {
        out.push_str("\n## Imagens do Relatório\n");
        render_findings(&mut out, appendix);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::super::sections::{appendix_findings, sections};
    use super::*;
    use crate::domain::{InspectionPlanning, TernaryAnswer};
    use crate::document::ReportInput;

    fn sample_input() -> ReportInput {
        ReportInput {
            inspection_planning: Some(InspectionPlanning {
                professional_qualification: "Engenheiro Eletricista".to_string(),
                team_fit_for_work: true,
                safety_briefing_held: true,
                has_nr10_training: true,
                service_pre_checked: true,
                identified_hazards: vec!["Choque".to_string()],
                safety_equipment: vec!["Luva isolante".to_string()],
                requires_shutdown: false,
                signage_used: vec![],
                requires_area_delimitation: false,
                requires_utility_assistance: false,
                requires_voltage_check: true,
                requires_temporary_grounding: false,
                work_at_height: false,
                requires_safety_harness: false,
                safety_requirements_met: true,
                requires_reassessment: false,
            }),
            external_influences: None,
            qualitative_assessment: None,
            quantitative_assessment: None,
            circuits: Vec::new(),
            required_spare_circuits: None,
            findings: vec![crate::document::Finding {
                category: "exposed_live_conductors".to_string(),
                description: Some("Fiação exposta próxima ao jardim".to_string()),
                report_section: None,
            }],
        }
    }

    #[test]
    fn renderiza_secao_preenchida_secao_nao_avaliada_e_apendice() {
        let input = sample_input();
        let sections = sections(&input);
        let appendix = appendix_findings(&input);

        let text = render(&sections, &appendix);

        assert!(text.contains("## Avaliação e planejamento da execução"));
        assert!(text.contains("Engenheiro Eletricista"));
        assert!(text.contains("Sim"));

        assert!(text.contains("## Avaliação das influências externas da instalação elétrica"));
        assert!(text.contains("Seção não avaliada neste laudo."));

        assert!(text.contains("## Imagens do Relatório"));
        assert!(text.contains("Condutores energizados expostos e sem proteção"));
        assert!(text.contains("Fiação exposta próxima ao jardim"));

        // location_code e responsible_parties não existem em ReportInput —
        // não há como vazar o que não pode ser construído.
        assert!(!text.to_lowercase().contains("location_code"));
    }

    #[test]
    fn rotulo_ternario_em_pt_br() {
        assert_eq!(super::super::labels::ternary_label(TernaryAnswer::Partial), "Parcialmente");
        assert_eq!(super::super::labels::ternary_label(TernaryAnswer::Yes), "Sim");
        assert_eq!(super::super::labels::ternary_label(TernaryAnswer::No), "Não");
    }
}
