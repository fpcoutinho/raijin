use super::labels::finding_category_label;
use super::{Finding, Section, SectionState, Table};

/// Legenda `(a)(b)(c)...` — padrão do apêndice órfão que
/// docs/findings-taxonomy.md documenta, em vez do "fotos soltas sem legenda"
/// do template ativo do legado.
fn item_letter(index: usize) -> char {
    (b'a' + (index % 26) as u8) as char
}

/// Valor de campo é texto digitado pelo engenheiro: um `*` ou `_` solto numa
/// observação viraria ênfase e corromperia o nó na conversão pro TipTap, e um
/// `|` cortaria a linha da tabela em duas células.
fn escape_markdown(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(character, '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '~' | '|') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

/// `<` e `&` precisam virar entidade dentro de `<table>`; o escape de Markdown
/// não serve ali, e vice-versa.
fn escape_html(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Cabeçalho de dois níveis (Tabela 9, Parte II da Tabela 10) precisa de
/// `colspan`, que Markdown GFM não tem. HTML atravessa o conversor do `itui`
/// (`markdown-it` com `html: true`) e o TipTap entende `colspan` na célula.
fn render_html_table(out: &mut String, table: &Table) {
    out.push_str("\n<table>\n<thead>\n<tr>");
    for (label, span) in &table.header_groups {
        out.push_str(&format!("<th colspan=\"{span}\">{}</th>", escape_html(label)));
    }
    out.push_str("</tr>\n<tr>");
    for header in &table.headers {
        out.push_str(&format!("<th>{}</th>", escape_html(header)));
    }
    out.push_str("</tr>\n</thead>\n<tbody>\n");

    for row in &table.rows {
        out.push_str("<tr>");
        for cell in row {
            out.push_str(&format!("<td>{}</td>", escape_html(cell)));
        }
        out.push_str("</tr>\n");
    }

    out.push_str("</tbody>\n</table>\n");
}

fn render_markdown_table(out: &mut String, table: &Table) {
    out.push_str(&format!("\n| {} |\n", table.headers.join(" | ")));
    out.push_str(&format!("|{}|\n", vec![" --- "; table.headers.len()].join("|")));

    for row in &table.rows {
        let cells: Vec<String> = row.iter().map(|cell| escape_markdown(cell)).collect();
        out.push_str(&format!("| {} |\n", cells.join(" | ")));
    }
}

fn render_table(out: &mut String, table: &Table) {
    if let Some(caption) = table.caption {
        out.push_str(&format!("\n**{caption}**\n"));
    }

    if table.header_groups.is_empty() {
        render_markdown_table(out, table);
    } else {
        render_html_table(out, table);
    }
}

fn render_findings(out: &mut String, findings: &[Finding], images: bool) {
    for (index, finding) in findings.iter().enumerate() {
        let label = finding_category_label(&finding.category);
        if images {
            out.push_str(&format!("\n![{}](image:{})\n", label, finding.image_id));
        } else {
            out.push('\n');
        }
        out.push_str(&format!("**({}) {}**", item_letter(index), label));
        if let Some(description) = &finding.description {
            out.push_str(&format!(" — {}", escape_markdown(description)));
        }
        out.push('\n');
    }
}

fn render_section(out: &mut String, section: &Section, images: bool) {
    out.push_str(&format!("\n## {}\n", section.title));

    match section.state {
        SectionState::NotAssessed => {
            out.push_str("\nSeção não avaliada neste laudo.\n");
        }
        SectionState::Filled => {
            for table in &section.tables {
                render_table(out, table);
            }
        }
    }

    if !section.findings.is_empty() {
        out.push_str("\n### Não conformidades registradas nesta seção\n");
        render_findings(out, &section.findings, images);
    }
}

/// Modelo determinístico — replace de dados nas seções do laudo, sem
/// provedor de IA. Estrutura vem de docs/report-template.md (extraída do
/// `template.docx` legado); prosa de composição nova onde o legado não tinha
/// texto fixo (ver aquele documento, seção "Consequência"). Cada seção
/// imprime, junto de si, os achados fotográficos que a ilustram — não jogados
/// todos no final, diferente do apêndice raso do template ativo.
pub fn render(sections: &[Section], appendix: &[Finding]) -> String {
    render_with(sections, appendix, true)
}

/// Mesmo documento sem o marcador `![](image:<id>)` de cada achado: o `id`
/// não diz nada ao modelo e ainda arrisca ser copiado pra dentro da prosa.
pub fn render_for_prompt(sections: &[Section], appendix: &[Finding]) -> String {
    render_with(sections, appendix, false)
}

fn render_with(sections: &[Section], appendix: &[Finding], images: bool) -> String {
    let mut out = String::new();

    for section in sections {
        render_section(&mut out, section, images);
    }

    if !appendix.is_empty() {
        out.push_str("\n## Imagens do Relatório\n");
        render_findings(&mut out, appendix, images);
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
                image_id: uuid::Uuid::nil(),
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

        assert!(text.contains("(image:00000000-0000-0000-0000-000000000000)"));

        // location_code e responsible_parties não existem em ReportInput —
        // não há como vazar o que não pode ser construído.
        assert!(!text.to_lowercase().contains("location_code"));
    }

    #[test]
    fn cabecalho_agrupado_sai_em_html_com_colspan() {
        use crate::document::{Section, SectionState, Table};

        let section = Section {
            key: "qualitative_assessment",
            title: "Avaliação qualitativa",
            tables: vec![Table {
                caption: None,
                header_groups: vec![("", 1), ("Atendem à norma?", 2)],
                headers: vec!["Item", "Resposta", "Observações"],
                rows: vec![vec!["1".to_string(), "Sim".to_string(), "—".to_string()]],
            }],
            state: SectionState::Filled,
            findings: Vec::new(),
        };

        let text = render(&[section], &[]);

        assert!(text.contains("<th colspan=\"2\">Atendem à norma?</th>"));
        assert!(text.contains("<td>Sim</td>"));
        assert!(!text.contains("| Item |"));
    }

    #[test]
    fn texto_do_engenheiro_nao_vira_marcacao_markdown() {
        let mut input = sample_input();
        input.findings[0].description = Some("Emenda 2*3mm perto do quadro_2018".to_string());

        let text = render(&sections(&input), &appendix_findings(&input));

        assert!(text.contains("Emenda 2\\*3mm perto do quadro\\_2018"));
    }

    #[test]
    fn rotulo_ternario_em_pt_br() {
        assert_eq!(super::super::labels::ternary_label(TernaryAnswer::Partial), "Parcialmente");
        assert_eq!(super::super::labels::ternary_label(TernaryAnswer::Yes), "Sim");
        assert_eq!(super::super::labels::ternary_label(TernaryAnswer::No), "Não");
    }
}
