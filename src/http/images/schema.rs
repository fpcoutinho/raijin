use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::ReportImage;

#[derive(Debug, Deserialize)]
pub struct CreateImageUploadRequest {
    pub content_type: String,
    pub finding_category: Option<String>,
    pub caption: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateImageUploadResponse {
    pub image_id: Uuid,
    pub upload_url: String,
    pub required_content_type: String,
}

#[derive(Debug, Serialize)]
pub struct ConfirmedImage {
    #[serde(flatten)]
    pub image: ReportImage,
    /// URL de leitura de vida curta pro frontend exibir a imagem sem o bucket
    /// ser público. Vence rápido — o frontend pede outra ao recarregar.
    pub view_url: String,
}

#[derive(Debug, Serialize)]
pub struct ListedImage {
    #[serde(flatten)]
    pub image: ReportImage,
    /// `None` enquanto o upload não foi confirmado — ainda não existe objeto
    /// no bucket pra assinar uma URL de leitura.
    pub view_url: Option<String>,
}
