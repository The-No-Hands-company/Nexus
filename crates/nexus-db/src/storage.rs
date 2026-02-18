//! MinIO / S3-compatible object storage client.
//!
//! Wraps `aws-sdk-s3` to provide upload, presigned URL generation,
//! and deletion for attachments, avatars, and custom emoji.

use anyhow::{Context, Result};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::{
    config::{Builder as S3Builder, Credentials, Region},
    primitives::ByteStream,
    Client,
};
use std::time::Duration;

/// Storage configuration (loaded from app config).
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// MinIO / S3 endpoint URL (e.g. `http://localhost:9000`)
    pub endpoint: String,
    /// Access key
    pub access_key: String,
    /// Secret key
    pub secret_key: String,
    /// Bucket name
    pub bucket: String,
    /// Region (use `us-east-1` for MinIO)
    pub region: String,
    /// Public CDN base URL for direct asset links (optional).
    /// If set, public presigned URLs will be rewritten to this base.
    pub public_url: Option<String>,
}

/// S3/MinIO storage client — wraps the AWS SDK.
#[derive(Clone)]
pub struct StorageClient {
    inner: Client,
    bucket: String,
    public_url: Option<String>,
}

impl StorageClient {
    /// Initialise client from config.
    pub fn new(cfg: &StorageConfig) -> Result<Self> {
        let creds = Credentials::new(
            &cfg.access_key,
            &cfg.secret_key,
            None, // session token
            None, // expiry
            "nexus-storage",
        );

        let s3_cfg = S3Builder::new()
            .endpoint_url(&cfg.endpoint)
            .credentials_provider(creds)
            .region(Region::new(cfg.region.clone()))
            // Force path-style URLs (required for MinIO)
            .force_path_style(true)
            .build();

        Ok(Self {
            inner: Client::from_conf(s3_cfg),
            bucket: cfg.bucket.clone(),
            public_url: cfg.public_url.clone(),
        })
    }

    // ------------------------------------------------------------------
    // Core upload
    // ------------------------------------------------------------------

    /// Upload bytes to the given key.
    ///
    /// Returns the storage key (same as `key` param) on success.
    pub async fn put_object(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> Result<String> {
        let stream = ByteStream::from(data);

        self.inner
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .body(stream)
            .send()
            .await
            .with_context(|| format!("Failed to upload {key} to object storage"))?;

        Ok(key.to_string())
    }

    // ------------------------------------------------------------------
    // URL generation
    // ------------------------------------------------------------------

    /// Generate a presigned GET URL valid for `expiry` seconds.
    pub async fn presigned_get_url(&self, key: &str, expiry_secs: u64) -> Result<String> {
        // If we have a public CDN URL, just concat — no presigning needed.
        if let Some(ref base) = self.public_url {
            return Ok(format!("{}/{}/{}", base.trim_end_matches('/'), &self.bucket, key));
        }

        let presigning_cfg = PresigningConfig::expires_in(Duration::from_secs(expiry_secs))
            .context("Failed to build presigning config")?;

        let req = self
            .inner
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presigning_cfg)
            .await
            .with_context(|| format!("Failed to create presigned URL for {key}"))?;

        Ok(req.uri().to_string())
    }

    /// Build a permanent public URL (only use when bucket is public).
    pub fn public_url(&self, key: &str) -> Option<String> {
        self.public_url
            .as_ref()
            .map(|base| format!("{}/{}/{}", base.trim_end_matches('/'), &self.bucket, key))
    }

    // ------------------------------------------------------------------
    // Deletion
    // ------------------------------------------------------------------

    /// Delete an object by its storage key.
    pub async fn delete_object(&self, key: &str) -> Result<()> {
        self.inner
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .with_context(|| format!("Failed to delete {key} from object storage"))?;

        Ok(())
    }

    // ------------------------------------------------------------------
    // Multipart helpers (for large files)
    // ------------------------------------------------------------------

    /// Upload a file in a single request (up to 5 GiB in one shot via SDK).
    /// For files ≥ 100 MiB the SDK transparently uses multipart under the hood.
    pub async fn upload_file(
        &self,
        key: &str,
        path: &std::path::Path,
        content_type: &str,
    ) -> Result<String> {
        let stream = ByteStream::from_path(path)
            .await
            .context("Failed to open file for upload")?;

        self.inner
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .body(stream)
            .send()
            .await
            .with_context(|| format!("Failed to upload file {key}"))?;

        Ok(key.to_string())
    }

    // ------------------------------------------------------------------
    // Bucket management (admin / startup helpers)
    // ------------------------------------------------------------------

    /// Ensure the bucket exists; create it if absent.
    pub async fn ensure_bucket(&self) -> Result<()> {
        match self.inner.head_bucket().bucket(&self.bucket).send().await {
            Ok(_) => {
                tracing::debug!(bucket = %self.bucket, "Bucket already exists");
                Ok(())
            }
            Err(_) => {
                tracing::info!(bucket = %self.bucket, "Bucket does not exist, creating");
                self.inner
                    .create_bucket()
                    .bucket(&self.bucket)
                    .send()
                    .await
                    .context("Failed to create object storage bucket")?;
                Ok(())
            }
        }
    }
}
