use std::{io, path::Path};

use anyhow::{Result, bail};
use bytes::Bytes;
use futures_util::{TryStreamExt, stream::BoxStream};
use s3::{Bucket, Region, creds::Credentials, serde_types::ObjectIdentifier};
use slasha_db::s3_storage::S3Storage;
use tokio::fs;

/// Uploads a local file to the specified S3 storage key.
///
/// # Arguments
///
/// * `storage` - Target S3 storage configuration ([`S3Storage`]).
/// * `key` - Destination S3 object key string.
/// * `file_path` - Local path reference ([`Path`]) of source file.
///
/// # Returns
///
/// A [`Result`] indicating upload success.
pub async fn upload_file(storage: &S3Storage, key: &str, file_path: &Path) -> Result<()> {
    let bucket = get_bucket(storage)?;
    let mut file = fs::File::open(file_path).await?;
    let response = bucket.put_object_stream(&mut file, key).await?;
    if response.status_code() < 200 || response.status_code() >= 300 {
        bail!(
            "s3 upload failed with HTTP status {}",
            response.status_code()
        );
    }
    Ok(())
}

/// Downloads a remote S3 object and writes it to a destination path.
///
/// # Arguments
///
/// * `storage` - Target S3 storage configuration ([`S3Storage`]).
/// * `key` - S3 object key string.
/// * `dest_path` - Destination file path reference ([`Path`]).
///
/// # Returns
///
/// A [`Result`] indicating download success.
pub async fn download_file(storage: &S3Storage, key: &str, dest_path: &Path) -> Result<()> {
    let bucket = get_bucket(storage)?;
    let mut file = fs::File::create(dest_path).await?;
    let status_code = bucket.get_object_to_writer(key, &mut file).await?;
    if !(200..300).contains(&status_code) {
        bail!("s3 download failed with HTTP status {}", status_code);
    }
    Ok(())
}

/// Returns a byte stream for a remote S3 object.
///
/// # Arguments
///
/// * `storage` - Target S3 storage configuration ([`S3Storage`]).
/// * `key` - S3 object key string.
///
/// # Returns
///
/// A boxed byte stream of S3 object data.
pub async fn get_object_stream(
    storage: &S3Storage,
    key: &str,
) -> Result<BoxStream<'static, io::Result<Bytes>>> {
    let bucket = get_bucket(storage)?;
    let res = bucket.get_object_stream(key).await?;
    if !(200..300).contains(&res.status_code) {
        bail!("s3 download failed with HTTP status {}", res.status_code);
    }
    let stream = res.bytes.map_err(io::Error::other);
    Ok(Box::pin(stream))
}

/// Deletes an object from the S3 storage destination.
///
/// # Arguments
///
/// * `storage` - Target S3 storage configuration ([`S3Storage`]).
/// * `key` - S3 object key string.
///
/// # Returns
///
/// A [`Result`] indicating deletion success.
pub async fn delete_file(storage: &S3Storage, key: &str) -> Result<()> {
    let bucket = get_bucket(storage)?;
    let result = bucket
        .delete_objects(vec![ObjectIdentifier::new(key)])
        .await?;

    if let Some(err) = result.errors.first() {
        bail!("s3 delete failed: {} - {}", err.code, err.message);
    }

    Ok(())
}

/// Verifies connectivity and authorization to the configured S3 storage.
///
/// # Arguments
///
/// * `storage` - Target S3 storage configuration ([`S3Storage`]).
///
/// # Returns
///
/// A [`Result`] indicating connection test success.
pub async fn test_connection(storage: &S3Storage) -> Result<()> {
    let bucket = get_bucket(storage)?;
    bucket.list("".to_string(), Some("/".to_string())).await?;
    Ok(())
}

/// Constructs an S3 bucket client for a given storage configuration.
///
/// # Arguments
///
/// * `storage` - Target S3 storage configuration ([`S3Storage`]).
///
/// # Returns
///
/// A [`Result`] containing the configured [`Bucket`].
fn get_bucket(storage: &S3Storage) -> Result<Box<Bucket>> {
    let region = Region::Custom {
        region: storage.region.clone(),
        endpoint: storage.endpoint.clone(),
    };
    let credentials = Credentials::new(
        Some(&storage.access_key_id),
        Some(&storage.secret_access_key),
        None,
        None,
        None,
    )?;
    let bucket = Bucket::new(&storage.bucket, region, credentials)?;
    let bucket = if storage.force_path_style {
        bucket.with_path_style()
    } else {
        bucket
    };

    Ok(bucket)
}
