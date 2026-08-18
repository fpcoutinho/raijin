use sqlx::PgPool;
use sqlx::types::Json;
use uuid::Uuid;

use crate::document::Finding;

use crate::domain::{
    ExternalInfluences, InspectionPlanning, QualitativeAssessment, QuantitativeAssessment, Report,
    ReportStatus,
};

use super::schema::{CreateReportRequest, ReportSortField, ReportSummary, UpdateReportRequest};

/// Existência e posse na mesma consulta. Separar em "existe?" + "é seu?" abriria
/// a porta pra um handler futuro checar só a primeira.
pub(crate) async fn report_belongs_to(
    pool: &PgPool,
    report_id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM reports WHERE id = $1 AND author_id = $2)",
        report_id,
        user_id,
    )
    .fetch_one(pool)
    .await
    .map(|exists| exists.unwrap_or(false))
}

/// Planejamento do laudo mais recente do mesmo autor no mesmo bloco, pro
/// auto-preenchimento do POST. O filtro por author_id não é só autorização:
/// planejamento de segurança de outra equipe não descreve esta inspeção.
pub async fn latest_planning_in_block(
    pool: &PgPool,
    author_id: Uuid,
    block_prefix: &str,
) -> Result<Option<Json<InspectionPlanning>>, sqlx::Error> {
    let row = sqlx::query_scalar!(
        r#"
        SELECT inspection_planning AS "inspection_planning: Json<InspectionPlanning>"
        FROM reports
        WHERE author_id = $1
          AND location_code LIKE $2 || '-%'
          AND inspection_planning IS NOT NULL
        ORDER BY created_at DESC
        LIMIT 1
        "#,
        author_id,
        block_prefix,
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.flatten())
}

pub async fn insert_report(
    pool: &PgPool,
    author_id: Uuid,
    body: &CreateReportRequest,
    inspection_planning: Option<Json<InspectionPlanning>>,
) -> Result<Report, sqlx::Error> {
    sqlx::query_as!(
        Report,
        r#"
        INSERT INTO reports (
            author_id, location_code, inspected_at, ambient_temperature_c,
            weather_conditions, responsible_parties, inspection_planning
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING
            id, author_id, location_code, inspected_at, ambient_temperature_c,
            weather_conditions, responsible_parties,
            status AS "status: ReportStatus",
            inspection_planning AS "inspection_planning: Json<InspectionPlanning>",
            external_influences AS "external_influences: Json<ExternalInfluences>",
            qualitative_assessment AS "qualitative_assessment: Json<QualitativeAssessment>",
            quantitative_assessment AS "quantitative_assessment: Json<QuantitativeAssessment>",
            document_content AS "document_content!: Json<serde_json::Value>",
            created_at, updated_at
        "#,
        author_id,
        body.location_code,
        body.inspected_at,
        body.ambient_temperature_c,
        body.weather_conditions,
        body.responsible_parties.as_deref().unwrap_or_default(),
        inspection_planning as Option<Json<InspectionPlanning>>,
    )
    .fetch_one(pool)
    .await
}

/// Filtros da listagem. Andam sempre juntos — a contagem precisa enxergar
/// exatamente o mesmo recorte que a página, senão o total mente.
pub struct ReportFilters<'a> {
    pub status: Option<ReportStatus>,
    pub location_prefix: Option<&'a str>,
    pub search: Option<&'a str>,
}

/// Escapa os curingas do `LIKE` no termo digitado. Sem isso um `%` no campo de
/// busca casaria qualquer coisa, e um `_` casaria qualquer caractere — o usuário
/// está procurando um texto, não escrevendo um padrão.
fn escape_like(term: &str) -> String {
    term.replace('\\', r"\\").replace('%', r"\%").replace('_', r"\_")
}

/// Total de laudos no recorte, ignorando `limit`/`offset`.
///
/// Consulta separada da página em vez de `COUNT(*) OVER ()`: a janela viaja
/// junto de cada linha e, mais importante, some quando a página sai vazia
/// (offset além do fim, ou filtro sem resultado) — justamente quando a UI mais
/// precisa do total pra saber que existe conteúdo em outra página.
pub async fn count_reports(
    pool: &PgPool,
    author_id: Uuid,
    filters: &ReportFilters<'_>,
) -> Result<i64, sqlx::Error> {
    let search = filters.search.map(escape_like);

    sqlx::query_scalar!(
        r#"
        SELECT COUNT(*)
        FROM reports
        WHERE author_id = $1
          AND ($2::report_status IS NULL OR status = $2)
          AND ($3::text IS NULL OR location_code LIKE $3 || '-%')
          AND (
            $4::text IS NULL
            OR location_code ILIKE '%' || $4 || '%' ESCAPE '\'
            OR EXISTS (
              SELECT 1 FROM unnest(responsible_parties) AS party
              WHERE party ILIKE '%' || $4 || '%' ESCAPE '\'
            )
          )
        "#,
        author_id,
        filters.status as Option<ReportStatus>,
        filters.location_prefix,
        search.as_deref(),
    )
    .fetch_one(pool)
    .await
    .map(|count| count.unwrap_or(0))
}

/// Uma página da listagem, filtrada e ordenada **no banco** — a UI pagina, então
/// ordenar em memória só classificaria a página carregada.
///
/// O `ORDER BY` é um leque de `CASE`, um par por coluna ordenável, e não SQL
/// concatenado: assim a query continua estática e verificada em compile-time
/// pela macro (ver CLAUDE.md). Os ramos não escolhidos avaliam para `NULL` em
/// todas as linhas e não desempatam nada.
pub async fn list_reports(
    pool: &PgPool,
    author_id: Uuid,
    filters: &ReportFilters<'_>,
    sort: ReportSortField,
    ascending: bool,
    limit: i64,
    offset: i64,
) -> Result<Vec<ReportSummary>, sqlx::Error> {
    let search = filters.search.map(escape_like);
    let sort = sort.as_str();

    sqlx::query_as!(
        ReportSummary,
        r#"
        SELECT id, location_code, inspected_at, status AS "status: ReportStatus",
               created_at, updated_at
        FROM reports
        WHERE author_id = $1
          AND ($2::report_status IS NULL OR status = $2)
          AND ($3::text IS NULL OR location_code LIKE $3 || '-%')
          AND (
            $4::text IS NULL
            OR location_code ILIKE '%' || $4 || '%' ESCAPE '\'
            OR EXISTS (
              SELECT 1 FROM unnest(responsible_parties) AS party
              WHERE party ILIKE '%' || $4 || '%' ESCAPE '\'
            )
          )
        ORDER BY
          CASE WHEN $5 = 'location_code' AND $6 THEN location_code END ASC,
          CASE WHEN $5 = 'location_code' AND NOT $6 THEN location_code END DESC,
          CASE WHEN $5 = 'inspected_at' AND $6 THEN inspected_at END ASC,
          CASE WHEN $5 = 'inspected_at' AND NOT $6 THEN inspected_at END DESC,
          CASE WHEN $5 = 'status' AND $6 THEN status END ASC,
          CASE WHEN $5 = 'status' AND NOT $6 THEN status END DESC,
          CASE WHEN $5 = 'created_at' AND $6 THEN created_at END ASC,
          CASE WHEN $5 = 'created_at' AND NOT $6 THEN created_at END DESC,
          CASE WHEN $5 = 'updated_at' AND $6 THEN updated_at END ASC,
          CASE WHEN $5 = 'updated_at' AND NOT $6 THEN updated_at END DESC,
          -- Desempate estável: sem ele, duas linhas com o mesmo valor podem
          -- trocar de lugar entre páginas e um laudo aparecer duas vezes.
          id
        LIMIT $7 OFFSET $8
        "#,
        author_id,
        filters.status as Option<ReportStatus>,
        filters.location_prefix,
        search.as_deref(),
        sort,
        ascending,
        limit,
        offset,
    )
    .fetch_all(pool)
    .await
}

/// A posse entra no WHERE junto com o id: laudo de outro autor devolve None,
/// indistinguível de inexistente.
pub async fn find_report(
    pool: &PgPool,
    report_id: Uuid,
    author_id: Uuid,
) -> Result<Option<Report>, sqlx::Error> {
    sqlx::query_as!(
        Report,
        r#"
        SELECT
            id, author_id, location_code, inspected_at, ambient_temperature_c,
            weather_conditions, responsible_parties,
            status AS "status: ReportStatus",
            inspection_planning AS "inspection_planning: Json<InspectionPlanning>",
            external_influences AS "external_influences: Json<ExternalInfluences>",
            qualitative_assessment AS "qualitative_assessment: Json<QualitativeAssessment>",
            quantitative_assessment AS "quantitative_assessment: Json<QuantitativeAssessment>",
            document_content AS "document_content!: Json<serde_json::Value>",
            created_at, updated_at
        FROM reports
        WHERE id = $1 AND author_id = $2
        "#,
        report_id,
        author_id,
    )
    .fetch_optional(pool)
    .await
}

/// `COALESCE` resolve os campos que não aceitam null. `ambient_temperature_c` e
/// `weather_conditions` aceitam, então precisam da flag: `COALESCE` não
/// distingue "não mandou" de "mandou null pra limpar".
pub async fn update_report(
    pool: &PgPool,
    report_id: Uuid,
    author_id: Uuid,
    body: &UpdateReportRequest,
) -> Result<Option<Report>, sqlx::Error> {
    let (set_ambient, ambient_temperature_c) = match body.ambient_temperature_c {
        Some(value) => (true, value),
        None => (false, None),
    };
    let (set_weather, weather_conditions) = match &body.weather_conditions {
        Some(value) => (true, value.as_deref()),
        None => (false, None),
    };

    sqlx::query_as!(
        Report,
        r#"
        UPDATE reports
        SET location_code = COALESCE($3::text, location_code),
            inspected_at = COALESCE($4::timestamptz, inspected_at),
            responsible_parties = COALESCE($5::text[], responsible_parties),
            status = COALESCE($6::report_status, status),
            ambient_temperature_c = CASE WHEN $7::bool THEN $8::int4 ELSE ambient_temperature_c END,
            weather_conditions = CASE WHEN $9::bool THEN $10::text ELSE weather_conditions END
        WHERE id = $1 AND author_id = $2
        RETURNING
            id, author_id, location_code, inspected_at, ambient_temperature_c,
            weather_conditions, responsible_parties,
            status AS "status: ReportStatus",
            inspection_planning AS "inspection_planning: Json<InspectionPlanning>",
            external_influences AS "external_influences: Json<ExternalInfluences>",
            qualitative_assessment AS "qualitative_assessment: Json<QualitativeAssessment>",
            quantitative_assessment AS "quantitative_assessment: Json<QuantitativeAssessment>",
            document_content AS "document_content!: Json<serde_json::Value>",
            created_at, updated_at
        "#,
        report_id,
        author_id,
        body.location_code.as_deref(),
        body.inspected_at,
        body.responsible_parties.as_deref(),
        body.status as Option<ReportStatus>,
        set_ambient,
        ambient_temperature_c,
        set_weather,
        weather_conditions,
    )
    .fetch_optional(pool)
    .await
}

/// `circuits` e `report_images` caem por ON DELETE CASCADE; os objetos no bucket
/// não são apagados junto.
pub async fn delete_report(
    pool: &PgPool,
    report_id: Uuid,
    author_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM reports WHERE id = $1 AND author_id = $2",
        report_id,
        author_id,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

// Uma função por seção: o nome da coluna não pode ser parâmetro de bind, e o
// RETURNING precisa ser literal pra macro verificar em compile-time.

pub async fn update_inspection_planning(
    pool: &PgPool,
    report_id: Uuid,
    author_id: Uuid,
    section: Json<InspectionPlanning>,
) -> Result<Option<Report>, sqlx::Error> {
    sqlx::query_as!(
        Report,
        r#"
        UPDATE reports SET inspection_planning = $3
        WHERE id = $1 AND author_id = $2
        RETURNING
            id, author_id, location_code, inspected_at, ambient_temperature_c,
            weather_conditions, responsible_parties,
            status AS "status: ReportStatus",
            inspection_planning AS "inspection_planning: Json<InspectionPlanning>",
            external_influences AS "external_influences: Json<ExternalInfluences>",
            qualitative_assessment AS "qualitative_assessment: Json<QualitativeAssessment>",
            quantitative_assessment AS "quantitative_assessment: Json<QuantitativeAssessment>",
            document_content AS "document_content!: Json<serde_json::Value>",
            created_at, updated_at
        "#,
        report_id,
        author_id,
        section as Json<InspectionPlanning>,
    )
    .fetch_optional(pool)
    .await
}

pub async fn update_external_influences(
    pool: &PgPool,
    report_id: Uuid,
    author_id: Uuid,
    section: Json<ExternalInfluences>,
) -> Result<Option<Report>, sqlx::Error> {
    sqlx::query_as!(
        Report,
        r#"
        UPDATE reports SET external_influences = $3
        WHERE id = $1 AND author_id = $2
        RETURNING
            id, author_id, location_code, inspected_at, ambient_temperature_c,
            weather_conditions, responsible_parties,
            status AS "status: ReportStatus",
            inspection_planning AS "inspection_planning: Json<InspectionPlanning>",
            external_influences AS "external_influences: Json<ExternalInfluences>",
            qualitative_assessment AS "qualitative_assessment: Json<QualitativeAssessment>",
            quantitative_assessment AS "quantitative_assessment: Json<QuantitativeAssessment>",
            document_content AS "document_content!: Json<serde_json::Value>",
            created_at, updated_at
        "#,
        report_id,
        author_id,
        section as Json<ExternalInfluences>,
    )
    .fetch_optional(pool)
    .await
}

pub async fn update_qualitative_assessment(
    pool: &PgPool,
    report_id: Uuid,
    author_id: Uuid,
    section: Json<QualitativeAssessment>,
) -> Result<Option<Report>, sqlx::Error> {
    sqlx::query_as!(
        Report,
        r#"
        UPDATE reports SET qualitative_assessment = $3
        WHERE id = $1 AND author_id = $2
        RETURNING
            id, author_id, location_code, inspected_at, ambient_temperature_c,
            weather_conditions, responsible_parties,
            status AS "status: ReportStatus",
            inspection_planning AS "inspection_planning: Json<InspectionPlanning>",
            external_influences AS "external_influences: Json<ExternalInfluences>",
            qualitative_assessment AS "qualitative_assessment: Json<QualitativeAssessment>",
            quantitative_assessment AS "quantitative_assessment: Json<QuantitativeAssessment>",
            document_content AS "document_content!: Json<serde_json::Value>",
            created_at, updated_at
        "#,
        report_id,
        author_id,
        section as Json<QualitativeAssessment>,
    )
    .fetch_optional(pool)
    .await
}

pub async fn update_quantitative_assessment(
    pool: &PgPool,
    report_id: Uuid,
    author_id: Uuid,
    section: Json<QuantitativeAssessment>,
) -> Result<Option<Report>, sqlx::Error> {
    sqlx::query_as!(
        Report,
        r#"
        UPDATE reports SET quantitative_assessment = $3
        WHERE id = $1 AND author_id = $2
        RETURNING
            id, author_id, location_code, inspected_at, ambient_temperature_c,
            weather_conditions, responsible_parties,
            status AS "status: ReportStatus",
            inspection_planning AS "inspection_planning: Json<InspectionPlanning>",
            external_influences AS "external_influences: Json<ExternalInfluences>",
            qualitative_assessment AS "qualitative_assessment: Json<QualitativeAssessment>",
            quantitative_assessment AS "quantitative_assessment: Json<QuantitativeAssessment>",
            document_content AS "document_content!: Json<serde_json::Value>",
            created_at, updated_at
        "#,
        report_id,
        author_id,
        section as Json<QuantitativeAssessment>,
    )
    .fetch_optional(pool)
    .await
}

/// Achados prontos pro modelo determinístico e pro prompt da IA — quatro
/// colunas só, de propósito: é aqui que a regra de privacidade fica
/// verificável por leitura (nem `storage_path`, nem qualquer coluna que
/// identifique a edificação; o `id` é chave nossa, e não sobe pro prompt). `image_ids = None` considera todas as imagens
/// confirmadas com achado; o agrupamento por seção é feito depois, em
/// `document::sections`, não aqui.
pub async fn list_findings(
    pool: &PgPool,
    report_id: Uuid,
    image_ids: Option<&[Uuid]>,
) -> Result<Vec<Finding>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT id, finding_category, report_section, caption
        FROM report_images
        WHERE report_id = $1
          AND upload_status = 'uploaded'
          AND (finding_category IS NOT NULL OR report_section IS NOT NULL)
          AND ($2::uuid[] IS NULL OR id = ANY($2))
        ORDER BY position
        "#,
        report_id,
        image_ids as Option<&[Uuid]>,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            row.finding_category.map(|category| Finding {
                image_id: row.id,
                category,
                description: row.caption,
                report_section: row.report_section,
            })
        })
        .collect())
}

pub async fn update_document_content(
    pool: &PgPool,
    report_id: Uuid,
    author_id: Uuid,
    content: Json<serde_json::Value>,
) -> Result<Option<Report>, sqlx::Error> {
    sqlx::query_as!(
        Report,
        r#"
        UPDATE reports SET document_content = $3
        WHERE id = $1 AND author_id = $2
        RETURNING
            id, author_id, location_code, inspected_at, ambient_temperature_c,
            weather_conditions, responsible_parties,
            status AS "status: ReportStatus",
            inspection_planning AS "inspection_planning: Json<InspectionPlanning>",
            external_influences AS "external_influences: Json<ExternalInfluences>",
            qualitative_assessment AS "qualitative_assessment: Json<QualitativeAssessment>",
            quantitative_assessment AS "quantitative_assessment: Json<QuantitativeAssessment>",
            document_content AS "document_content!: Json<serde_json::Value>",
            created_at, updated_at
        "#,
        report_id,
        author_id,
        content as Json<serde_json::Value>,
    )
    .fetch_optional(pool)
    .await
}
