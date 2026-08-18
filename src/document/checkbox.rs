//! O laudo com cara de **formulário de inspeção**, não de despejo de variável.
//!
//! O template legado imprimia a lista inteira de opções em cada questão e
//! marcava a escolhida — quem lê o laudo assinado vê o que *não* foi respondido
//! tanto quanto o que foi, e é isso que distingue um formulário preenchido de
//! uma frase solta numa célula. `docs/report-template.md` registra a grade; a
//! lista de opções vem de `docs/nbr-5410-choices.json`, mesma fonte que valida
//! o `PATCH` da seção — nunca redigitada aqui.
//!
//! A saída é **marcação**, não texto: as opções são separadas por `<br>` porque
//! célula de tabela Markdown não aceita quebra de linha. Por isso a coluna que
//! recebe estas strings entra em `Table::markup_columns` e escapa do escape do
//! renderizador — o que obriga este módulo a escapar por conta própria tudo que
//! veio do banco (ver `escape_html`).

use super::template::escape_html;

const CHECKED: &str = "[X]";
const UNCHECKED: &str = "[ ]";

/// Espaço que sobrevive ao colapso do HTML. Separar `Sim [X]` de `Não [ ]` com
/// espaço comum deixa os dois grudados na renderização. Caractere literal, e
/// não a entidade `&#160;`: assim a mesma string serve à célula de marcação e
/// ao cabeçalho, que passa pelo escape de HTML e viraria `&amp;#160;`.
const GAP: &str = "\u{a0}\u{a0}";

fn mark(selected: bool) -> &'static str {
    if selected { CHECKED } else { UNCHECKED }
}

/// Uma opção por linha, marcador **antes** do rótulo — a leitura vertical de
/// um formulário depende de os marcadores ficarem alinhados na mesma coluna.
fn lines(entries: impl IntoIterator<Item = (bool, String)>) -> String {
    entries
        .into_iter()
        .map(|(selected, label)| format!("{} {}", mark(selected), label))
        .collect::<Vec<_>>()
        .join("<br>")
}

/// Resposta binária na horizontal: `Sim [X]  Não [ ]`.
///
/// Na horizontal, e não em duas linhas como as listas de opção, porque são
/// doze das dezessete questões da Tabela 7 — empilhá-las dobraria a altura da
/// tabela sem acrescentar informação nenhuma.
pub fn yes_no(value: bool) -> String {
    format!("Sim {}{GAP}Não {}", mark(value), mark(!value))
}

/// A lista normativa do campo com as escolhidas marcadas.
///
/// Vale para escolha única (`selected` com um item) e múltipla — a diferença
/// entre as duas é do domínio, não do desenho da célula.
///
/// **Valor fora da lista não é descartado**: entra como linha extra marcada. A
/// alternativa seria um laudo que perde silenciosamente uma resposta dada em
/// campo porque a norma foi atualizada e o JSON ainda não — o pior resultado
/// possível num documento de responsabilidade técnica.
///
/// `None` quando o campo não tem lista normativa: o chamador cai no texto
/// corrido, que é o comportamento antigo.
pub fn option_list(field: &str, selected: &[String]) -> Option<String> {
    let options = crate::domain::field_options(field)?;

    let known = options.iter().map(|option| {
        (
            selected.iter().any(|value| value == option),
            escape_html(option),
        )
    });

    let unknown = selected
        .iter()
        .filter(|value| !options.iter().any(|option| option == *value))
        .map(|value| (true, escape_html(value)));

    Some(lines(known.chain(unknown)))
}

/// Igual a `option_list`, mas para o campo de escolha única guardado como
/// `String` solta.
pub fn single_choice(field: &str, value: &str) -> String {
    option_list(field, std::slice::from_ref(&value.to_string()))
        .unwrap_or_else(|| escape_html(value))
}

/// Lista sem fonte normativa (hoje nenhuma, mas `signage_used` pode aceitar
/// texto livre no futuro): sem opções a marcar, sobra a enumeração do que foi
/// selecionado — ainda com marcador, para não quebrar a coluna.
pub fn selected_only(values: &[String]) -> String {
    if values.is_empty() {
        return format!("{UNCHECKED} Nenhum");
    }

    lines(values.iter().map(|value| (true, escape_html(value))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marca_a_opcao_escolhida_e_deixa_as_outras_em_branco() {
        let cell = single_choice("professional_qualification", "Técnico Eletrotécnico");

        assert!(cell.contains("[ ] Engenheiro Eletricista"));
        assert!(cell.contains("[X] Técnico Eletrotécnico"));
        assert!(cell.contains("[ ] Eletricista"));
        assert!(cell.contains("<br>"));
    }

    #[test]
    fn escolha_multipla_marca_todas_as_selecionadas() {
        let cell = option_list(
            "identified_hazards",
            &["Choque".to_string(), "Queda".to_string()],
        )
        .unwrap();

        assert!(cell.contains("[X] Queda"));
        assert!(cell.contains("[X] Choque"));
        assert!(cell.contains("[ ] Explosão"));
    }

    #[test]
    fn valor_fora_da_lista_normativa_entra_marcado_em_vez_de_sumir() {
        let cell = single_choice("professional_qualification", "Engenheiro de Segurança");

        assert!(cell.contains("[X] Engenheiro de Segurança"));
        assert!(cell.contains("[ ] Engenheiro Eletricista"));
    }

    #[test]
    fn texto_do_banco_nao_injeta_marcacao_na_celula() {
        let cell = single_choice("professional_qualification", "<b>Eletricista</b>");

        assert!(cell.contains("[X] &lt;b&gt;Eletricista&lt;/b&gt;"));
        assert!(!cell.contains("<b>"));
    }

    #[test]
    fn binaria_sai_na_horizontal_com_a_resposta_marcada() {
        assert!(yes_no(true).starts_with("Sim [X]"));
        assert!(yes_no(true).ends_with("Não [ ]"));
        assert!(yes_no(false).starts_with("Sim [ ]"));
        assert!(yes_no(false).ends_with("Não [X]"));
    }

    #[test]
    fn campo_sem_lista_normativa_devolve_none() {
        assert!(option_list("weather_conditions", &[]).is_none());
    }
}
