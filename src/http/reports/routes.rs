use std::convert::Infallible;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::Json;
use futures::StreamExt;
use serde_json::json;
use sqlx::types::Json as SqlxJson;
use uuid::Uuid;

use crate::AppState;
use crate::document::{self, ReportInput};
use crate::domain::{
    block_prefix, required_spare_circuits, ExternalInfluences, InspectionPlanning,
    QualitativeAssessment, QuantitativeAssessment, Report,
};
use crate::http::AuthUser;
use crate::http::error::ApiError;
use crate::llm::{prompt, GenerationEvent};

use super::queries;
use super::schema::{
    CreateReportRequest, CreatedReport, DraftQuery, DraftResponse, GenerateRequest,
    ListReportsQuery, ReportDetail, ReportSummary, SpareCircuits, UpdateReportRequest,
    validate_external_influences, validate_inspection_planning, validate_qualitative_assessment,
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

    let spare_circuits = SpareCircuits::of(circuits.len());

    Ok(Json(ReportDetail { report, circuits, spare_circuits }))
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

// ============================================================
// Redação do relatório — modelo determinístico (/draft) e por IA
// (/generate). Ver docs/api-contract.md e CLAUDE.md, "O que a IA faz".
// ============================================================

/// Reúne o que os dois caminhos de redação precisam, já sem `location_code`
/// nem `responsible_parties` — a exclusão é do tipo `ReportInput`, não desta
/// função (ver src/document/mod.rs).
async fn collect_input(
    state: &AppState,
    report_id: Uuid,
    author_id: Uuid,
    image_ids: Option<&[Uuid]>,
) -> Result<ReportInput, ApiError> {
    let report = queries::find_report(&state.db, report_id, author_id).await?.ok_or_else(not_found)?;
    let circuits = crate::http::circuits::queries::list_circuits(&state.db, report_id).await?;
    let findings = queries::list_findings(&state.db, report_id, image_ids).await?;
    let required_spare_circuits = required_spare_circuits(circuits.len());

    Ok(ReportInput {
        inspection_planning: report.inspection_planning.map(|SqlxJson(value)| value),
        external_influences: report.external_influences.map(|SqlxJson(value)| value),
        qualitative_assessment: report.qualitative_assessment.map(|SqlxJson(value)| value),
        quantitative_assessment: report.quantitative_assessment.map(|SqlxJson(value)| value),
        circuits,
        required_spare_circuits,
        findings,
    })
}

fn has_content(input: &ReportInput) -> bool {
    input.inspection_planning.is_some()
        || input.external_influences.is_some()
        || input.qualitative_assessment.is_some()
        || input.quantitative_assessment.is_some()
        || !input.circuits.is_empty()
        || !input.findings.is_empty()
}

fn empty_report_error() -> ApiError {
    ApiError::Unprocessable("Preencha ao menos uma seção do laudo antes de gerar o texto.".to_string())
}

/// Modelo padrão determinístico — sem provedor externo, sem `503` possível.
/// Continua respondendo mesmo se a IA estiver fora do ar; é o piso do
/// sistema (ver CLAUDE.md, "Dois caminhos de redação").
pub async fn draft(
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    user: AuthUser,
    Query(query): Query<DraftQuery>,
) -> Result<Json<DraftResponse>, ApiError> {
    require_ownership(&state, report_id, &user).await?;

    let image_ids = query.image_ids();
    let input = collect_input(&state, report_id, user.id, image_ids.as_deref()).await?;
    if !has_content(&input) {
        return Err(empty_report_error());
    }

    let sections = document::sections(&input);
    let appendix = document::appendix_findings(&input);
    let text = document::template::render(&sections, &appendix);

    Ok(Json(DraftResponse { text }))
}

fn token_event((section, text): (String, String)) -> Result<Event, axum::Error> {
    Event::default().event("token").json_data(json!({ "section": section, "text": text }))
}

/// Redação por IA, em streaming SSE — três eventos (`token`/`done`/`error`),
/// contrato em docs/api-contract.md. O stream carrega **só a prosa**: as
/// tabelas são do `/draft`, que o `itui` já carregou antes de abrir este
/// stream. Cada `token` vai etiquetado com a seção a que pertence, para o
/// front encaixar o texto no lugar certo (ver llm::prompt::SectionSplitter).
///
/// Falha antes do primeiro byte (laudo de outro autor, laudo vazio, provedor
/// fora do ar na abertura) é resposta HTTP normal via `ApiError`; falha depois
/// vira `event: error` e encerra o stream — o status HTTP já foi enviado.
pub async fn generate(
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    user: AuthUser,
    Json(body): Json<GenerateRequest>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    require_ownership(&state, report_id, &user).await?;

    let image_ids = body.image_ids;
    let input = collect_input(&state, report_id, user.id, image_ids.as_deref()).await?;
    if !has_content(&input) {
        return Err(empty_report_error());
    }

    let sections = document::sections(&input);
    let appendix = document::appendix_findings(&input);
    let request = prompt::build_request(&sections, &appendix);

    let generation = state.llm.generate_stream(request).await?;

    let mut splitter = prompt::SectionSplitter::new();

    let events = generation
        .flat_map(move |result| {
            let events = match result {
                Ok(GenerationEvent::Token { text }) => {
                    splitter.push(&text).into_iter().map(token_event).collect()
                }
                Ok(GenerationEvent::Done { finish_reason, total_tokens }) => {
                    let mut events: Vec<_> =
                        splitter.flush().into_iter().map(token_event).collect();
                    events.push(Event::default().event("done").json_data(
                        json!({ "finish_reason": finish_reason, "total_tokens": total_tokens }),
                    ));
                    events
                }
                Err(error) => {
                    tracing::error!(%error, "erro no provedor de IA durante o stream");
                    vec![Event::default().event("error").json_data(
                        json!({ "error": "Provedor de IA indisponível. Tente novamente." }),
                    )]
                }
            };
            futures::stream::iter(events)
        })
        .map(|event| Ok(event.unwrap_or_else(|_| Event::default().event("error"))));

    Ok(Sse::new(events).keep_alive(KeepAlive::default()))
}
