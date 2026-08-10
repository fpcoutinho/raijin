use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use serde::Deserialize;

/// O JSON entra no binário: sob Lambda não há `docs/` no sistema de arquivos.
/// Continua fonte única — editar o arquivo muda o que a API aceita.
const CHOICES: &str = include_str!("../../docs/nbr-5410-choices.json");

/// As seções misturam campos com chaves de metadado (`_note`, `nbrClauses`,
/// `_defaultAnswerSet`), que não são objetos com `options` — daí `Value` aqui e
/// o filtro na montagem do mapa.
#[derive(Deserialize)]
struct ChoicesFile {
    #[serde(rename = "inspectionPlanning")]
    inspection_planning: HashMap<String, serde_json::Value>,
    #[serde(rename = "externalInfluences")]
    external_influences: HashMap<String, serde_json::Value>,
    #[serde(rename = "qualitativeAssessment")]
    qualitative_assessment: HashMap<String, serde_json::Value>,
}

fn options_of(entry: &serde_json::Value) -> Option<Vec<&str>> {
    let options = entry.get("options")?.as_array()?;
    Some(options.iter().filter_map(|o| o.as_str()).collect())
}

/// Nas influências externas o JSON guarda código + rótulo na mesma string, em
/// dois formatos ("AA4 - Temperado" e "AC1 Baixa"). A API trafega só o código,
/// que é o primeiro token nos dois casos.
fn code_of(option: &str) -> &str {
    option.split_whitespace().next().unwrap_or(option)
}

fn allowed() -> &'static HashMap<String, HashSet<String>> {
    static ALLOWED: OnceLock<HashMap<String, HashSet<String>>> = OnceLock::new();

    ALLOWED.get_or_init(|| {
        let file: ChoicesFile =
            serde_json::from_str(CHOICES).expect("nbr-5410-choices.json inválido");

        let mut map = HashMap::new();

        for (name, entry) in &file.external_influences {
            if let Some(options) = options_of(entry) {
                map.insert(
                    name.clone(),
                    options.into_iter().map(|o| code_of(o).to_string()).collect(),
                );
            }
        }

        let plain = file.inspection_planning.iter().chain(&file.qualitative_assessment);
        for (name, entry) in plain {
            if let Some(options) = options_of(entry) {
                map.insert(name.clone(), options.into_iter().map(str::to_string).collect());
            }
        }

        map
    })
}

/// Código → string completa da opção ("AA4" → "AA4 - Temperado (-5 ° a 40
/// °C)"), só para os campos de influências externas — é onde código e rótulo
/// vêm juntos na mesma string de origem. Mapa separado de `allowed()` porque
/// ali o valor já foi reduzido ao código (`code_of`) para a checagem de
/// pertencimento; aqui é o inverso, resolver o código de volta pro texto.
fn labels() -> &'static HashMap<String, HashMap<String, String>> {
    static LABELS: OnceLock<HashMap<String, HashMap<String, String>>> = OnceLock::new();

    LABELS.get_or_init(|| {
        let file: ChoicesFile =
            serde_json::from_str(CHOICES).expect("nbr-5410-choices.json inválido");

        file.external_influences
            .iter()
            .filter_map(|(name, entry)| {
                let options = options_of(entry)?;
                let by_code = options
                    .into_iter()
                    .map(|option| (code_of(option).to_string(), option.to_string()))
                    .collect();
                Some((name.clone(), by_code))
            })
            .collect()
    })
}

/// Cláusula da NBR 5410 associada a um campo, para o texto do laudo poder
/// citar número de item de verdade em vez de a IA inventar um (ver
/// src/llm/prompt.rs). Duas formas no JSON-fonte: `externalInfluences` guarda
/// `nbrClause` dentro do objeto de cada campo; `qualitativeAssessment` guarda
/// um mapa `nbrClauses` à parte, sibling dos campos. `None` tanto pra campo
/// sem lista normativa quanto pra campo cuja cláusula é `null` na fonte
/// (texto do template.docx ilegível/truncado nesse ponto).
fn clauses() -> &'static HashMap<String, String> {
    static CLAUSES: OnceLock<HashMap<String, String>> = OnceLock::new();

    CLAUSES.get_or_init(|| {
        let file: ChoicesFile =
            serde_json::from_str(CHOICES).expect("nbr-5410-choices.json inválido");

        let mut map = HashMap::new();

        for (name, entry) in &file.external_influences {
            if let Some(clause) = entry.get("nbrClause").and_then(|v| v.as_str()) {
                map.insert(name.clone(), clause.to_string());
            }
        }

        if let Some(clauses) = file.qualitative_assessment.get("nbrClauses").and_then(|v| v.as_object()) {
            for (name, clause) in clauses {
                if let Some(clause) = clause.as_str() {
                    map.insert(name.clone(), clause.to_string());
                }
            }
        }

        map
    })
}

pub fn clause_of(field: &str) -> Option<&'static str> {
    clauses().get(field).map(String::as_str)
}

/// `None` quando o campo não tem lista normativa — texto livre, nada a validar.
pub fn is_allowed(field: &str, value: &str) -> Option<bool> {
    allowed().get(field).map(|values| values.contains(value))
}

/// Rótulo completo (com descrição) do código NBR escolhido, para renderizar o
/// laudo em pt-BR sem duplicar as listas normativas em Rust (ver `src/document/`).
/// `None` sem lista normativa para o campo, ou código não encontrado nela.
pub fn label_of(field: &str, code: &str) -> Option<&'static str> {
    labels().get(field)?.get(code).map(String::as_str)
}

/// Espaço de reserva exigido no quadro de distribuição (NBR 5410 6.5.4.7), a
/// partir do número real de circuitos. O legado só guardava a faixa escolhida
/// pelo engenheiro e descartava esta saída — ver docs/nbr-5410-tests.md.
///
/// Acima de 30 a norma pede 0,15 × N; a conta é inteira (`ceil`) pra não passar
/// por float, mesma razão de `Decimal` nas medições.
///
/// `None` sem circuito cadastrado: a faixa "até 6" da tabela pressupõe um quadro
/// levantado, e laudo vazio é wizard incompleto, não exigência de 2 reservas.
pub fn required_spare_circuits(circuit_count: usize) -> Option<u32> {
    let required = match circuit_count {
        0 => return None,
        1..=6 => 2,
        7..=12 => 3,
        13..=30 => 4,
        n => ((15 * n as u64).div_ceil(100)) as u32,
    };

    Some(required)
}

#[cfg(test)]
mod tests {
    use super::{code_of, is_allowed, label_of, required_spare_circuits};

    #[test]
    fn extrai_codigo_dos_dois_formatos_do_json() {
        assert_eq!(code_of("AA4 - Temperado (-5 ° a 40 °C)"), "AA4");
        assert_eq!(code_of("AC1 Baixa ( ≤ 2000 m )"), "AC1");
        assert_eq!(code_of("AM8-1 Campos magnéticos radiados nível médio"), "AM8-1");
    }

    #[test]
    fn aceita_codigo_normativo_e_recusa_inventado() {
        assert_eq!(is_allowed("ambient_temperature_class", "AA4"), Some(true));
        assert_eq!(is_allowed("ambient_temperature_class", "ZZ9"), Some(false));
        assert_eq!(is_allowed("earthing_system_type", "TN-S"), Some(true));
        assert_eq!(is_allowed("professional_qualification", "Engenheiro Eletricista"), Some(true));
    }

    #[test]
    fn campo_sem_lista_normativa_nao_valida() {
        assert_eq!(is_allowed("weather_conditions", "Ensolarado"), None);
    }

    #[test]
    fn resolve_rotulo_completo_do_codigo() {
        assert_eq!(
            label_of("ambient_temperature_class", "AA4"),
            Some("AA4 - Temperado (-5 ° a 40 °C)")
        );
        assert_eq!(label_of("ambient_temperature_class", "ZZ9"), None);
        assert_eq!(label_of("weather_conditions", "Ensolarado"), None);
    }

    #[test]
    fn sem_circuito_nao_calcula() {
        assert_eq!(required_spare_circuits(0), None);
    }

    /// As bordas de cada faixa da tabela 6.5.4.7.
    #[test]
    fn espaco_reserva_por_faixa() {
        assert_eq!(required_spare_circuits(1), Some(2));
        assert_eq!(required_spare_circuits(6), Some(2));
        assert_eq!(required_spare_circuits(7), Some(3));
        assert_eq!(required_spare_circuits(12), Some(3));
        assert_eq!(required_spare_circuits(13), Some(4));
        assert_eq!(required_spare_circuits(30), Some(4));
    }

    #[test]
    fn acima_de_30_arredonda_para_cima() {
        assert_eq!(required_spare_circuits(31), Some(5)); // 4,65
        assert_eq!(required_spare_circuits(40), Some(6)); // 6,0 exato
        assert_eq!(required_spare_circuits(100), Some(15)); // 15,0 exato
        assert_eq!(required_spare_circuits(101), Some(16)); // 15,15
    }
}
