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

#[cfg(test)]
mod tests {
    use super::{code_of, is_allowed};

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
}
