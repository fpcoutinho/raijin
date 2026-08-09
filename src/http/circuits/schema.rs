use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize};

use crate::domain::Circuit;
use crate::http::error::ApiError;

/// Só `description` é opcional — um circuito identifica um ramal real do quadro,
/// então sem modelo, fase, disjuntor, condutor e corrente não há o que registrar.
#[derive(Debug, Deserialize)]
pub struct CreateCircuitRequest {
    pub circuit_model: String,
    pub phase: String,
    pub breaker: String,
    pub description: Option<String>,
    pub conductor: String,
    pub current: Decimal,
}

impl CreateCircuitRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        let blank = [
            ("circuit_model", &self.circuit_model),
            ("phase", &self.phase),
            ("breaker", &self.breaker),
            ("conductor", &self.conductor),
        ]
        .into_iter()
        .find(|(_, value)| value.trim().is_empty());

        match blank {
            Some((field, _)) => Err(ApiError::Unprocessable(blank_field_message(field))),
            None => Ok(()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateCircuitRequest {
    pub circuit_model: Option<String>,
    pub phase: Option<String>,
    pub breaker: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub description: Option<Option<String>>,
    pub conductor: Option<String>,
    pub current: Option<Decimal>,
}

impl UpdateCircuitRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        let blank = [
            ("circuit_model", &self.circuit_model),
            ("phase", &self.phase),
            ("breaker", &self.breaker),
            ("conductor", &self.conductor),
        ]
        .into_iter()
        .find(|(_, value)| value.as_ref().is_some_and(|v| v.trim().is_empty()));

        match blank {
            Some((field, _)) => Err(ApiError::Unprocessable(blank_field_message(field))),
            None => Ok(()),
        }
    }
}

fn blank_field_message(field: &str) -> String {
    let label = match field {
        "circuit_model" => "o modelo do circuito",
        "phase" => "a fase",
        "breaker" => "o disjuntor",
        _ => "o condutor",
    };
    format!("Informe {label}.")
}

#[derive(Debug, Serialize)]
pub struct CircuitResponse {
    #[serde(flatten)]
    pub circuit: Circuit,
}

/// Sem isso o serde colapsaria `null` explícito em `None`, indistinguível de
/// campo ausente — e o PATCH perderia o "limpe este campo".
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::deserialize(deserializer).map(Some)
}
