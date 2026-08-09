use axum::extract::{Path, State};
use axum::response::Json;
use uuid::Uuid;

use crate::AppState;
use crate::domain::{FINDING_CATEGORIES, ImageUploadStatus};
use crate::http::error::ApiError;
use crate::http::AuthUser;
use crate::http::reports::require_ownership;

use super::schema::{ConfirmedImage, CreateImageUploadRequest, CreateImageUploadResponse, ListedImage};
use super::queries;

/// Extensões aceitas, mapeadas a partir do Content-Type declarado pelo cliente.
fn extension_for(content_type: &str) -> Option<&'static str> {
    match content_type {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        "image/heic" => Some("heic"),
        _ => None,
    }
}

pub async fn create_upload(
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    user: AuthUser,
    Json(body): Json<CreateImageUploadRequest>,
) -> Result<Json<CreateImageUploadResponse>, ApiError> {
    let Some(extension) = extension_for(&body.content_type) else {
        return Err(ApiError::Unprocessable(format!(
            "Tipo de imagem não suportado: {}. Use JPEG, PNG, WEBP ou HEIC.",
            body.content_type
        )));
    };

    if let Some(category) = &body.finding_category
        && !FINDING_CATEGORIES.contains(&category.as_str())
    {
        return Err(ApiError::Unprocessable(format!(
            "Categoria de achado desconhecida: {category}."
        )));
    }

    require_ownership(&state, report_id, &user).await?;

    let image_id = Uuid::new_v4();
    let storage_path = format!("reports/{report_id}/{image_id}.{extension}");

    let image = queries::insert_pending_image(
        &state.db,
        image_id,
        report_id,
        &storage_path,
        body.finding_category,
        body.caption,
    )
    .await?;

    let upload_url = state
        .storage
        .presigned_put(&image.storage_path, &body.content_type)
        .await?;

    Ok(Json(CreateImageUploadResponse {
        image_id: image.id,
        upload_url,
        required_content_type: body.content_type,
    }))
}

pub async fn list_images(
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    user: AuthUser,
) -> Result<Json<Vec<ListedImage>>, ApiError> {
    require_ownership(&state, report_id, &user).await?;

    let images = queries::list_images(&state.db, report_id).await?;

    let mut listed = Vec::with_capacity(images.len());
    for image in images {
        let view_url = if image.upload_status == ImageUploadStatus::Uploaded {
            Some(state.storage.presigned_get(&image.storage_path).await?)
        } else {
            None
        };
        listed.push(ListedImage { image, view_url });
    }

    Ok(Json(listed))
}

pub async fn confirm_upload(
    State(state): State<AppState>,
    Path((report_id, image_id)): Path<(Uuid, Uuid)>,
    user: AuthUser,
) -> Result<Json<ConfirmedImage>, ApiError> {
    require_ownership(&state, report_id, &user).await?;

    let image = queries::find_image(&state.db, report_id, image_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Imagem não encontrada para este laudo.".to_string()))?;

    if image.upload_status == ImageUploadStatus::Uploaded {
        let view_url = state.storage.presigned_get(&image.storage_path).await?;
        return Ok(Json(ConfirmedImage { image, view_url }));
    }

    let Some(metadata) = state.storage.head(&image.storage_path).await? else {
        return Err(ApiError::Unprocessable(
            "Upload ainda não chegou ao armazenamento. Tente novamente em instantes.".to_string(),
        ));
    };

    let updated = queries::mark_uploaded(
        &state.db,
        report_id,
        image_id,
        metadata.content_type,
        metadata.size_bytes,
    )
    .await?;

    let view_url = state.storage.presigned_get(&updated.storage_path).await?;
    Ok(Json(ConfirmedImage { image: updated, view_url }))
}
