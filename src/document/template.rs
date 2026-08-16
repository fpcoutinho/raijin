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
///
/// `pub(super)` porque `checkbox.rs` monta marcação e, por isso mesmo, é quem
/// tem de escapar o texto que vem do banco antes de embuti-la.
pub(super) fn escape_html(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Coluna de marcação passa reta; o resto é escapado pelo formato de destino.
fn cell_text(table: &Table, column: usize, cell: &str, escape: fn(&str) -> String) -> String {
    if table.markup_columns.contains(&column) { cell.to_string() } else { escape(cell) }
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
        for (column, cell) in row.iter().enumerate() {
            out.push_str(&format!("<td>{}</td>", cell_text(table, column, cell, escape_html)));
        }
        out.push_str("</tr>\n");
    }

    out.push_str("</tbody>\n</table>\n");
}

fn render_markdown_table(out: &mut String, table: &Table) {
    out.push_str(&format!("\n| {} |\n", table.headers.join(" | ")));
    out.push_str(&format!("|{}|\n", vec![" --- "; table.headers.len()].join("|")));

    for row in &table.rows {
        let cells: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(column, cell)| cell_text(table, column, cell, escape_markdown))
            .collect();
        out.push_str(&format!("| {} |\n", cells.join(" | ")));
    }
}

/// A legenda ABNT vem **antes** da grade e em parágrafo próprio — é assim que
/// o `itui` a reconhece para pendurar o bloco de contexto da inspeção embaixo
/// dela (ver `itui/src/domain/reportDocument.ts`). Mudar o formato daqui sem
/// mudar lá deixa o laudo sem o cabeçalho de data, local e responsáveis.
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

/// Um grupo de fotos como **uma figura ABNT**: as imagens lado a lado num
/// parágrafo só, e embaixo delas uma legenda numerada que descreve cada item
/// pela letra.
///
///     ![(a) …](image:…)![(b) …](image:…)
///
///     **Figura 3. Não conformidades — Avaliação qualitativa:** (a) Emendas
///     mal executadas — fita isolante no forro (b) Aterramento improvisado
///
/// Antes, cada achado saía como foto + rótulo próprio, um debaixo do outro:
/// três fotos viravam três blocos sem relação visível entre si e sem número
/// pelo qual o texto pudesse citá-las. A legenda única é o que transforma o
/// conjunto numa figura referenciável — e ela fica **acima** do parágrafo de
/// parecer, que o `/generate` escreve no fim da seção.
///
/// As imagens ficam todas na mesma linha de propósito: o parágrafo é o que o
/// `itui` usa para dispô-las em grade (`p:has(.report-image)`) e o que o
/// `.docx` converte numa linha de tabela de N células.
fn render_figure(
    out: &mut String,
    findings: &[Finding],
    images: bool,
    number: usize,
    title: &str,
) {
    if images {
        out.push('\n');
        for (index, finding) in findings.iter().enumerate() {
            out.push_str(&format!(
                "![({}) {}](image:{})",
                item_letter(index),
                finding_category_label(&finding.category),
                finding.image_id
            ));
        }
        out.push('\n');
    }

    out.push_str(&format!("\n**Figura {number}. {title}:**"));

    for (index, finding) in findings.iter().enumerate() {
        out.push_str(&format!(
            " ({}) {}",
            item_letter(index),
            escape_markdown(&finding_category_label(&finding.category))
        ));

        if let Some(description) = &finding.description {
            out.push_str(&format!(" — {}", escape_markdown(description)));
        }
    }

    out.push('\n');
}

fn render_section(out: &mut String, section: &Section, images: bool, figure: &mut usize) {
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
        *figure += 1;
        render_figure(
            out,
            &section.findings,
            images,
            *figure,
            &format!("Não conformidades — {}", section.title),
        );
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
    // Corrido pelo documento inteiro, e não reiniciado por seção: "Figura 3" só
    // é referência se houver uma única Figura 3 no laudo.
    let mut figure = 0;

    for section in sections {
        render_section(&mut out, section, images, &mut figure);
    }

    if !appendix.is_empty() {
        out.push_str("\n## Imagens do Relatório\n");
        figure += 1;
        render_figure(&mut out, appendix, images, figure, "Registro fotográfico complementar");
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
                markup_columns: &[],
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
    fn tabela_sai_com_legenda_abnt_numerada_antes_da_grade() {
        let input = sample_input();
        let text = render(&sections(&input), &appendix_findings(&input));

        let caption = "**Tabela 7. Avaliação e planejamento da execução**";
        let grid = "| Item | Descrição | Detalhamento | Observação |";

        assert!(text.contains(caption));
        assert!(text.find(caption) < text.find(grid));
    }

    #[test]
    fn questao_de_lista_reimprime_as_opcoes_com_a_escolhida_marcada() {
        let input = sample_input();
        let text = render(&sections(&input), &appendix_findings(&input));

        assert!(text.contains("[X] Engenheiro Eletricista"));
        assert!(text.contains("[ ] Técnico Eletrotécnico"));
        // Binária na horizontal, com a descartada visível.
        assert!(text.contains("Sim [X]"));
        assert!(text.contains("Não [ ]"));
    }

    #[test]
    fn coluna_de_marcacao_escapa_do_escape_de_markdown() {
        let input = sample_input();
        let text = render(&sections(&input), &appendix_findings(&input));

        // `<br>` inteiro na célula; a mesma tabela ainda escapa o resto.
        assert!(text.contains("<br>[ ] Eletricista"));
        assert!(!text.contains("&lt;br&gt;"));
    }

    #[test]
    fn achados_viram_uma_figura_numerada_com_legenda_por_letra() {
        let input = sample_input();
        let text = render(&sections(&input), &appendix_findings(&input));

        assert!(text.contains("**Figura 1. Registro fotográfico complementar:**"));
        assert!(text.contains("(a) Condutores energizados expostos e sem proteção"));
        // A legenda vem depois da imagem, não antes.
        assert!(text.find("![(a)") < text.find("**Figura 1."));
    }

    #[test]
    fn texto_do_engenheiro_nao_vira_marcacao_markdown() {
        let mut input = sample_input();
        input.findings[0].description = Some("Emenda 2*3mm perto do quadro_2018".to_string());

        let text = render(&sections(&input), &appendix_findings(&input));

        assert!(text.contains("Emenda 2\\*3mm perto do quadro\\_2018"));
    }

    #[test]
    fn resposta_ternaria_vira_a_letra_da_legenda_do_cabecalho() {
        assert_eq!(super::super::labels::ternary_letter(TernaryAnswer::Partial), "P");
        assert_eq!(super::super::labels::ternary_letter(TernaryAnswer::Yes), "S");
        assert_eq!(super::super::labels::ternary_letter(TernaryAnswer::No), "N");
    }
}
