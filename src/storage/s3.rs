use std::time::Duration;

use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{BehaviorVersion, Region, RequestChecksumCalculation};
use aws_sdk_s3::presigning::PresigningConfig;

use super::{ObjectMetadata, ObjectStorage, StorageError};

pub struct S3CompatibleStorage {
    client: aws_sdk_s3::Client,
    bucket: String,
    upload_url_ttl: Duration,
    download_url_ttl: Duration,
}

impl S3CompatibleStorage {
    pub fn new(config: &crate::config::StorageConfig) -> Self {
        let credentials = Credentials::new(
            &config.access_key_id,
            &config.secret_access_key,
            None,
            None,
            "raijin-static",
        );

        let s3_config = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()))
            .endpoint_url(&config.endpoint)
            .credentials_provider(credentials)
            .force_path_style(true)
            // Sem isto o SDK inclui o header de checksum na assinatura, e o
            // PUT do navegador (que manda só Content-Type) falha a validação
            // de assinatura. O checksum é do SDK, não do protocolo.
            .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
            .build();

        Self {
            client: aws_sdk_s3::Client::from_conf(s3_config),
            bucket: config.bucket.clone(),
            upload_url_ttl: config.upload_url_ttl,
            download_url_ttl: config.download_url_ttl,
        }
    }
}

#[async_trait]
impl ObjectStorage for S3CompatibleStorage {
    async fn presigned_put(&self, key: &str, content_type: &str) -> Result<String, StorageError> {
        let presigning = PresigningConfig::expires_in(self.upload_url_ttl)
            .map_err(|error| StorageError::Presign(error.to_string()))?;

        let request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .presigned(presigning)
            .await
            .map_err(|error| StorageError::Presign(error.to_string()))?;

        Ok(request.uri().to_string())
    }

    async fn presigned_get(&self, key: &str) -> Result<String, StorageError> {
        let presigning = PresigningConfig::expires_in(self.download_url_ttl)
            .map_err(|error| StorageError::Presign(error.to_string()))?;

        let request = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presigning)
            .await
            .map_err(|error| StorageError::Presign(error.to_string()))?;

        Ok(request.uri().to_string())
    }

    async fn head(&self, key: &str) -> Result<Option<ObjectMetadata>, StorageError> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(output) => Ok(Some(ObjectMetadata {
                content_type: output.content_type().map(str::to_string),
                size_bytes: output.content_length().unwrap_or(0),
            })),
            Err(error) => {
                let service_error = error.into_service_error();
                if service_error.is_not_found() {
                    Ok(None)
                } else {
                    Err(StorageError::Head(service_error.to_string()))
                }
            }
        }
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|error| StorageError::Delete(error.to_string()))?;

        Ok(())
    }
}
