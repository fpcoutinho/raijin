use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use uuid::Uuid;

use crate::AppState;
use crate::domain::{
    block_prefix, ExternalInfluences, InspectionPlanning, QualitativeAssessment,
    QuantitativeAssessment, Report,
};
use crate::http::AuthUser;
use crate::http::error::ApiError;

use super::queries;
use super::schema::{
    CreateReportRequest, CreatedReport, ListReportsQuery, ReportDetail, ReportSummary,
    UpdateReportRequest, validate_external_influences, validate_inspection_planning,
    validate_qualitative_assessment,
};

fn not_found() -> ApiError {
    ApiError::NotFound("Laudo não encontrado.".to_string())
}

pub(crate) async fn require_ownership(
    state: &AppState,
    report_id: Uuid,
    user: &AuthUser,
) -> Result<(), ApiError> {
    if queries::report_belongs_to(&state.db, report_id, user.id).await? {
        Ok(())
    } else {
        Err(ApiError::NotFound("Laudo não encontrado.".to_string()))
    }
}

pub async fn create_report(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateReportRequest>,
) -> Result<(StatusCode, Json<CreatedReport>), ApiError> {
    body.validate()?;

    let inspection_planning = match block_prefix(&body.location_code) {
        Some(prefix) => queries::latest_planning_in_block(&state.db, user.id, prefix).await?,
        None => None,
    };
    let planning_autofilled = inspection_planning.is_some();

    let report = queries::insert_report(
        &state.db,
        user.id,
        &body.location_code,
        body.inspected_at,
        body.ambient_temperature_c,
        body.weather_conditions,
        &body.responsible_parties.unwrap_or_default(),
        inspection_planning,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(CreatedReport {
            report,
            planning_autofilled,
        }),
    ))
}

// ============================================================
// A implementar (Step 4, manual) — contrato em docs/api-contract.md
// ============================================================

pub async fn list_reports(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ListReportsQuery>,
) -> Result<Json<Vec<ReportSummary>>, ApiError> {
    let reports = queries::list_reports(
        &state.db,
        user.id,
        query.status,
        query.location_prefix.as_deref(),
        query.limit(),
        query.offset(),
    )
    .await?;

    Ok(Json(reports))
}

pub async fn get_report(
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    user: AuthUser,
) -> Result<Json<ReportDetail>, ApiError> {
    let report = queries::find_report(&state.db, report_id, user.id)
        .await?
        .ok_or_else(not_found)?;

    let circuits = crate::http::circuits::queries::list_circuits(&state.db, report_id).await?;

    Ok(Json(ReportDetail { report, circuits }))
}

pub async fn update_report(
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    user: AuthUser,
    Json(body): Json<UpdateReportRequest>,
) -> Result<Json<Report>, ApiError> {
    body.validate()?;

    let report = queries::update_report(&state.db, report_id, user.id, &body)
        .await?
        .ok_or_else(not_found)?;

    Ok(Json(report))
}

pub async fn delete_report(
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    user: AuthUser,
) -> Result<StatusCode, ApiError> {
    if queries::delete_report(&state.db, report_id, user.id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found())
    }
}

// PATCH de seção: o corpo é a struct de domínio inteira — a seção é a unidade
// de validação, não o campo. Validar os códigos NBR contra
// docs/nbr-5410-choices.json é parte da implementação destes handlers.

pub async fn update_inspection_planning(
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    user: AuthUser,
    Json(body): Json<InspectionPlanning>,
) -> Result<Json<Report>, ApiError> {
    if state.nbr_validation {
        validate_inspection_planning(&body)?;
    }

    let report =
        queries::update_inspection_planning(&state.db, report_id, user.id, sqlx::types::Json(body))
            .await?
            .ok_or_else(not_found)?;

    Ok(Json(report))
}

pub async fn update_external_influences(
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    user: AuthUser,
    Json(body): Json<ExternalInfluences>,
) -> Result<Json<Report>, ApiError> {
    if state.nbr_validation {
        validate_external_influences(&body)?;
    }

    let report =
        queries::update_external_influences(&state.db, report_id, user.id, sqlx::types::Json(body))
            .await?
            .ok_or_else(not_found)?;

    Ok(Json(report))
}

pub async fn update_qualitative_assessment(
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    user: AuthUser,
    Json(body): Json<QualitativeAssessment>,
) -> Result<Json<Report>, ApiError> {
    if state.nbr_validation {
        validate_qualitative_assessment(&body)?;
    }

    let report = queries::update_qualitative_assessment(
        &state.db,
        report_id,
        user.id,
        sqlx::types::Json(body),
    )
    .await?
    .ok_or_else(not_found)?;

    Ok(Json(report))
}

pub async fn update_quantitative_assessment(
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    user: AuthUser,
    Json(body): Json<QuantitativeAssessment>,
) -> Result<Json<Report>, ApiError> {
    let report = queries::update_quantitative_assessment(
        &state.db,
        report_id,
        user.id,
        sqlx::types::Json(body),
    )
    .await?
    .ok_or_else(not_found)?;

    Ok(Json(report))
}

/// Árvore do editor TipTap — JSON livre, sem struct de domínio.
pub async fn update_document_content(
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    user: AuthUser,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<Report>, ApiError> {
    let report =
        queries::update_document_content(&state.db, report_id, user.id, sqlx::types::Json(body))
            .await?
            .ok_or_else(not_found)?;

    Ok(Json(report))
}
