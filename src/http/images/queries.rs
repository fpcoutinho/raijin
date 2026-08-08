use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{ImageUploadStatus, ReportImage};

pub async fn insert_pending_image(
    pool: &PgPool,
    image_id: Uuid,
    report_id: Uuid,
    storage_path: &str,
    finding_category: Option<String>,
    caption: Option<String>,
) -> Result<ReportImage, sqlx::Error> {
    sqlx::query_as!(
        ReportImage,
        r#"
        INSERT INTO report_images (id, report_id, storage_path, finding_category, caption, position)
        VALUES (
            $1, $2, $3, $4, $5,
            COALESCE((SELECT MAX(position) + 1 FROM report_images WHERE report_id = $2), 0)
        )
        RETURNING
            id, report_id, storage_path, finding_category,
            upload_status AS "upload_status: ImageUploadStatus",
            content_type, size_bytes, uploaded_at, caption, position,
            created_at, updated_at
        "#,
        image_id,
        report_id,
        storage_path,
        finding_category,
        caption,
    )
    .fetch_one(pool)
    .await
}

pub async fn list_images(pool: &PgPool, report_id: Uuid) -> Result<Vec<ReportImage>, sqlx::Error> {
    sqlx::query_as!(
        ReportImage,
        r#"
        SELECT
            id, report_id, storage_path, finding_category,
            upload_status AS "upload_status: ImageUploadStatus",
            content_type, size_bytes, uploaded_at, caption, position,
            created_at, updated_at
        FROM report_images
        WHERE report_id = $1
        ORDER BY position
        "#,
        report_id,
    )
    .fetch_all(pool)
    .await
}

pub async fn find_image(
    pool: &PgPool,
    report_id: Uuid,
    image_id: Uuid,
) -> Result<Option<ReportImage>, sqlx::Error> {
    sqlx::query_as!(
        ReportImage,
        r#"
        SELECT
            id, report_id, storage_path, finding_category,
            upload_status AS "upload_status: ImageUploadStatus",
            content_type, size_bytes, uploaded_at, caption, position,
            created_at, updated_at
        FROM report_images
        WHERE id = $1 AND report_id = $2
        "#,
        image_id,
        report_id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn mark_uploaded(
    pool: &PgPool,
    report_id: Uuid,
    image_id: Uuid,
    content_type: Option<String>,
    size_bytes: i64,
) -> Result<ReportImage, sqlx::Error> {
    sqlx::query_as!(
        ReportImage,
        r#"
        UPDATE report_images
        SET upload_status = 'uploaded', content_type = $3, size_bytes = $4, uploaded_at = now()
        WHERE id = $1 AND report_id = $2
        RETURNING
            id, report_id, storage_path, finding_category,
            upload_status AS "upload_status: ImageUploadStatus",
            content_type, size_bytes, uploaded_at, caption, position,
            created_at, updated_at
        "#,
        image_id,
        report_id,
        content_type,
        size_bytes,
    )
    .fetch_one(pool)
    .await
}
