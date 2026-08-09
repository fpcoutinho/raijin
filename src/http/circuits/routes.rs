use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use uuid::Uuid;

use crate::AppState;
use crate::domain::Circuit;
use crate::http::AuthUser;
use crate::http::error::ApiError;

use super::queries;
use super::schema::{CreateCircuitRequest, UpdateCircuitRequest};

// Todos os handlers começam por crate::http::reports::require_ownership com o
// report_id da URL: laudo de terceiro responde 404, e a checagem é do laudo
// pai, nunca do circuito.

pub async fn list_circuits(
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    user: AuthUser,
) -> Result<(StatusCode, Json<Vec<Circuit>>), ApiError> {
    crate::http::reports::require_ownership(&state, report_id, &user).await?;

    let circuits = queries::list_circuits(&state.db, report_id).await?;
    Ok((
        StatusCode::OK,
        Json(circuits),
    ))
}

pub async fn create_circuit(
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    user: AuthUser,
    Json(body): Json<CreateCircuitRequest>,
) -> Result<(StatusCode, Json<Circuit>), ApiError> {
    crate::http::reports::require_ownership(&state, report_id, &user).await?;

    body.validate()?;

    let circuit = queries::insert_circuit(
        &state.db,
        report_id,
        &body.circuit_model,
        &body.phase,
        &body.breaker,
        body.description.as_deref(),
        &body.conductor,
        body.current,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(circuit)))
}

pub async fn update_circuit(
    State(state): State<AppState>,
    Path((report_id, circuit_id)): Path<(Uuid, Uuid)>,
    user: AuthUser,
    Json(body): Json<UpdateCircuitRequest>,
) -> Result<Json<Circuit>, ApiError> {
    crate::http::reports::require_ownership(&state, report_id, &user).await?;

    body.validate()?;

    let circuit = queries::update_circuit(&state.db, report_id, circuit_id, &body)
        .await?
        .ok_or_else(|| ApiError::NotFound("Circuito não encontrado.".to_string()))?;

    Ok(Json(circuit))
}

pub async fn delete_circuit(
    State(state): State<AppState>,
    Path((report_id, circuit_id)): Path<(Uuid, Uuid)>,
    user: AuthUser,
) -> Result<StatusCode, ApiError> {
    crate::http::reports::require_ownership(&state, report_id, &user).await?;

    if queries::delete_circuit(&state.db, report_id, circuit_id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound("Circuito não encontrado.".to_string()))
    }
}
