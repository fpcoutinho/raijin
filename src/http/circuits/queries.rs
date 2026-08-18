use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::Circuit;

use super::schema::{CreateCircuitRequest, UpdateCircuitRequest};

pub async fn list_circuits(pool: &PgPool, report_id: Uuid) -> Result<Vec<Circuit>, sqlx::Error> {
    sqlx::query_as!(
        Circuit,
        r#"
        SELECT id, report_id, circuit_model, phase, breaker, description,
               conductor, current, created_at, updated_at
        FROM circuits WHERE report_id = $1 ORDER BY created_at
        "#,
        report_id,
    )
    .fetch_all(pool)
    .await
}

pub async fn insert_circuit(
    pool: &PgPool,
    report_id: Uuid,
    body: &CreateCircuitRequest,
) -> Result<Circuit, sqlx::Error> {
    sqlx::query_as!(
        Circuit,
        r#"
        INSERT INTO circuits (report_id, circuit_model, phase, breaker, description, conductor, current)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, report_id, circuit_model, phase, breaker, description,
                  conductor, current, created_at, updated_at
        "#,
        report_id,
        body.circuit_model,
        body.phase,
        body.breaker,
        body.description.as_deref(),
        body.conductor,
        body.current,
    )
    .fetch_one(pool)
    .await
}

pub async fn update_circuit(
    pool: &PgPool,
    report_id: Uuid,
    circuit_id: Uuid,
    body: &UpdateCircuitRequest,
) -> Result<Option<Circuit>, sqlx::Error> {
    let (set_description, description) = match &body.description {
        Some(value) => (true, value.as_deref()),
        None => (false, None),
    };

    sqlx::query_as!(
        Circuit,
        r#"
        UPDATE circuits
        SET circuit_model = COALESCE($3::text, circuit_model),
            phase         = COALESCE($4::text, phase),
            breaker       = COALESCE($5::text, breaker),
            conductor     = COALESCE($6::text, conductor),
            current       = COALESCE($7::numeric, current),
            description   = CASE WHEN $8::bool THEN $9::text ELSE description END
        WHERE id = $1 AND report_id = $2
        RETURNING id, report_id, circuit_model, phase, breaker, description,
                  conductor, current, created_at, updated_at
        "#,
        circuit_id,
        report_id,
        body.circuit_model.as_deref(),
        body.phase.as_deref(),
        body.breaker.as_deref(),
        body.conductor.as_deref(),
        body.current,
        set_description,
        description,
    )
    .fetch_optional(pool)
    .await
}

pub async fn delete_circuit(
    pool: &PgPool,
    report_id: Uuid,
    circuit_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        DELETE FROM circuits
        WHERE id = $1 AND report_id = $2
        "#,
        circuit_id,
        report_id,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}
