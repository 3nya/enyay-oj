#![allow(dead_code)]

use std::{env, fmt, str::FromStr};

use s3::{
    Bucket, Region,
    creds::{Credentials, error::CredentialsError},
    error::S3Error,
};

#[derive(Clone)]
pub struct Storage {
    bucket: Box<Bucket>,
}

#[derive(Debug)]
pub enum StorageError {
    MissingConfig(&'static str),
    InvalidRegion(String),
    Credentials(CredentialsError),
    S3(S3Error),
    UnexpectedStatus {
        operation: &'static str,
        status: u16,
    },
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingConfig(name) => write!(f, "missing storage config: {name}"),
            Self::InvalidRegion(region) => write!(f, "invalid S3 region: {region}"),
            Self::Credentials(error) => write!(f, "S3 credentials failed: {error}"),
            Self::S3(error) => write!(f, "S3 request failed: {error}"),
            Self::UnexpectedStatus { operation, status } => {
                write!(f, "S3 {operation} returned unexpected status {status}")
            }
        }
    }
}

impl std::error::Error for StorageError {}

impl From<S3Error> for StorageError {
    fn from(error: S3Error) -> Self {
        Self::S3(error)
    }
}

impl From<CredentialsError> for StorageError {
    fn from(error: CredentialsError) -> Self {
        Self::Credentials(error)
    }
}

impl Storage {
    pub fn from_env() -> Result<Self, StorageError> {
        let bucket_name = required_env("S3_BUCKET")?;
        let region_name = env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
        let region = match env::var("S3_ENDPOINT") {
            Ok(endpoint) => Region::Custom {
                region: region_name,
                endpoint,
            },
            Err(_) => Region::from_str(&region_name)
                .map_err(|_| StorageError::InvalidRegion(region_name.clone()))?,
        };

        let credentials = Credentials::default()?;
        let bucket = Bucket::new(&bucket_name, region, credentials)?;
        let bucket = if env_bool("S3_PATH_STYLE") {
            bucket.with_path_style()
        } else {
            bucket
        };

        Ok(Self { bucket })
    }

    pub async fn get_object(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let response = self.bucket.get_object(normalize_key(key)).await?;
        expect_status("GET", response.status_code(), &[200])?;
        Ok(response.as_slice().to_vec())
    }

    pub async fn put_object(
        &self,
        key: &str,
        bytes: &[u8],
        content_type: &str,
    ) -> Result<(), StorageError> {
        let response = self
            .bucket
            .put_object_with_content_type(normalize_key(key), bytes, content_type)
            .await?;
        expect_status("PUT", response.status_code(), &[200])?;
        Ok(())
    }

    pub async fn post_object(
        &self,
        key: &str,
        bytes: &[u8],
        content_type: &str,
    ) -> Result<(), StorageError> {
        self.put_object(key, bytes, content_type).await
    }

    pub async fn delete_object(&self, key: &str) -> Result<(), StorageError> {
        let response = self.bucket.delete_object(normalize_key(key)).await?;
        expect_status("DELETE", response.status_code(), &[200, 202, 204])?;
        Ok(())
    }
}

fn required_env(name: &'static str) -> Result<String, StorageError> {
    env::var(name)
        .map(|value| value.trim().to_string())
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or(StorageError::MissingConfig(name))
}

fn env_bool(name: &str) -> bool {
    matches!(
        env::var(name).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

fn normalize_key(key: &str) -> &str {
    key.trim_start_matches('/')
}

fn expect_status(
    operation: &'static str,
    status: u16,
    expected: &[u16],
) -> Result<(), StorageError> {
    if expected.contains(&status) {
        Ok(())
    } else {
        Err(StorageError::UnexpectedStatus { operation, status })
    }
}
