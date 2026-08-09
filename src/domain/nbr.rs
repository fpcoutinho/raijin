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

/// `None` quando o campo não tem lista normativa — texto livre, nada a validar.
pub fn is_allowed(field: &str, value: &str) -> Option<bool> {
    allowed().get(field).map(|values| values.contains(value))
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
    use super::{code_of, is_allowed, required_spare_circuits};

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
