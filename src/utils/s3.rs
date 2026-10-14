use super::structs::{PGCliError, Result, S3Credentials};
use futures_util::stream;
use log::{info, warn};
use rusoto_core::{ByteStream, HttpClient, Region, RusotoError};
use rusoto_credential::StaticProvider;
use rusoto_s3::{
    AbortMultipartUploadRequest, CompleteMultipartUploadRequest, CompletedMultipartUpload,
    CompletedPart, CreateMultipartUploadRequest, GetObjectRequest, PutObjectRequest, S3, S3Client,
    UploadPartRequest,
};
use std::future::Future;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

const SMALL_OBJECT: u64 = 10 * 1024 * 1024;
const MAX_PART: u64 = 5 * 1024 * 1024 * 1024;

fn part_size(size: u64) -> Result<u64> {
    let size = size.div_ceil(10_000).max(SMALL_OBJECT);
    if size > MAX_PART {
        return Err(PGCliError::Other(
            "Backup exceeds the supported S3 multipart size/part-count limits".into(),
        ));
    }
    Ok(size)
}

fn retryable<E>(error: &RusotoError<E>) -> bool {
    match error {
        RusotoError::HttpDispatch(_) => true,
        RusotoError::Unknown(response) => {
            response.status.is_server_error() || response.status.as_u16() == 429
        }
        _ => false,
    }
}

async fn retry<T, E, F, Fut>(mut operation: F) -> Result<T>
where
    PGCliError: From<RusotoError<E>>,
    F: FnMut() -> Fut,
    Fut: Future<Output = std::result::Result<T, RusotoError<E>>>,
{
    for attempt in 0..3 {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) if attempt < 2 && retryable(&error) => {
                warn!(
                    "Retrying transient S3 request failure (attempt {})",
                    attempt + 1
                );
                tokio::time::sleep(Duration::from_millis(200 << attempt)).await;
            }
            Err(error) => return Err(error.into()),
        }
    }
    unreachable!("last attempt always returns")
}

async fn file_body(path: &Path, offset: u64, length: u64) -> std::io::Result<ByteStream> {
    let mut file = File::open(path).await?;
    file.seek(std::io::SeekFrom::Start(offset)).await?;
    let stream = stream::try_unfold((file, length), |(mut file, remaining)| async move {
        if remaining == 0 {
            return Ok::<_, std::io::Error>(None);
        }
        // Stream large parts with a bounded 64 KiB buffer. Retry reopens/seeks
        // the file, so it never needs an entire part in memory.
        let mut buffer = vec![0; remaining.min(64 * 1024) as usize];
        let count = file.read(&mut buffer).await?;
        if count == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Backup file changed during upload",
            ));
        }
        buffer.truncate(count);
        Ok(Some((buffer.into(), (file, remaining - count as u64))))
    });
    Ok(ByteStream::new_with_size(
        stream,
        usize::try_from(length)
            .map_err(|_| std::io::Error::other("S3 part is too large for this architecture"))?,
    ))
}

fn required<'a>(field: &'a Option<String>, name: &str) -> Result<&'a str> {
    field
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| PGCliError::Other(format!("S3 {name} is required")))
}

impl S3Credentials {
    pub fn validate_s3_credentials(&self) -> Result<&Self> {
        for (value, name) in [
            (&self.s3_endpoint, "endpoint"),
            (&self.s3_access_key, "access key"),
            (&self.s3_secret_key, "secret key"),
            (&self.s3_bucket, "bucket"),
            (&self.s3_region, "region"),
        ] {
            required(value, name)?;
        }
        let prefix = required(&self.s3_prefix, "prefix")?;
        if !prefix
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '/' | '-'))
        {
            return Err(PGCliError::Other("S3 prefix is invalid".into()));
        }
        Ok(self)
    }

    fn client(&self) -> Result<S3Client> {
        self.validate_s3_credentials()?;
        let region = if let Some(endpoint) = &self.s3_endpoint {
            Region::Custom {
                name: required(&self.s3_region, "region")?.into(),
                endpoint: endpoint.clone(),
            }
        } else {
            Region::from_str(required(&self.s3_region, "region")?)
                .map_err(|_| PGCliError::Other("Invalid S3 region".into()))?
        };
        Ok(S3Client::new_with(
            HttpClient::new()?,
            StaticProvider::new(
                required(&self.s3_access_key, "access key")?.into(),
                required(&self.s3_secret_key, "secret key")?.into(),
                self.s3_session_token.clone(),
                None,
            ),
            region,
        ))
    }

    fn key(&self, file_key: &str) -> String {
        match &self.s3_prefix {
            Some(prefix) => format!("{prefix}/{file_key}"),
            None => file_key.to_owned(),
        }
    }

    pub async fn multipart_upload<P: AsRef<Path>>(
        &self,
        file_path: P,
        file_key: &str,
    ) -> Result<()> {
        let client = self.client()?;
        self.upload_with(&client, file_path.as_ref(), file_key)
            .await
    }

    async fn upload_with<C: S3>(&self, client: &C, path: &Path, file_key: &str) -> Result<()> {
        let size = tokio::fs::metadata(path).await?.len();
        let part_size = part_size(size)?;
        let bucket = required(&self.s3_bucket, "bucket")?.to_owned();
        let key = self.key(file_key);
        if size <= SMALL_OBJECT {
            retry(|| async {
                client
                    .put_object(PutObjectRequest {
                        bucket: bucket.clone(),
                        key: key.clone(),
                        content_length: Some(size as i64),
                        body: Some(file_body(path, 0, size).await?),
                        ..Default::default()
                    })
                    .await
            })
            .await?;
            return Ok(());
        }
        let upload_id = client
            .create_multipart_upload(CreateMultipartUploadRequest {
                bucket: bucket.clone(),
                key: key.clone(),
                ..Default::default()
            })
            .await?
            .upload_id
            .ok_or_else(|| PGCliError::Other("S3 did not return a multipart upload ID".into()))?;
        let result = async {
            let mut parts = Vec::new();
            for (index, offset) in (0..size)
                .step_by(usize::try_from(part_size).map_err(|_| {
                    PGCliError::Other("S3 part is too large for this architecture".into())
                })?)
                .enumerate()
            {
                let length = part_size.min(size - offset);
                let part_number = (index + 1) as i64;
                let response = retry(|| async {
                    client
                        .upload_part(UploadPartRequest {
                            bucket: bucket.clone(),
                            key: key.clone(),
                            upload_id: upload_id.clone(),
                            part_number,
                            content_length: Some(length as i64),
                            body: Some(file_body(path, offset, length).await?),
                            ..Default::default()
                        })
                        .await
                })
                .await?;
                let etag = response.e_tag.ok_or_else(|| {
                    PGCliError::Other("S3 upload part did not return an ETag".into())
                })?;
                parts.push(CompletedPart {
                    e_tag: Some(etag),
                    part_number: Some(part_number),
                });
                info!(
                    "Uploaded S3 part {} of {}",
                    part_number,
                    size.div_ceil(part_size)
                );
            }
            let request = CompleteMultipartUploadRequest {
                bucket: bucket.clone(),
                key: key.clone(),
                upload_id: upload_id.clone(),
                multipart_upload: Some(CompletedMultipartUpload { parts: Some(parts) }),
                ..Default::default()
            };
            retry(|| client.complete_multipart_upload(request.clone())).await?;
            Ok(())
        }
        .await;
        if result.is_err() {
            let abort = AbortMultipartUploadRequest {
                bucket,
                key,
                upload_id,
                ..Default::default()
            };
            if retry(|| client.abort_multipart_upload(abort.clone()))
                .await
                .is_err()
            {
                warn!(
                    "Failed to abort incomplete S3 upload; configure an incomplete-upload lifecycle rule"
                );
            }
        }
        result
    }

    pub async fn download_file<P: AsRef<Path>>(&self, file_key: &str, path: P) -> Result<()> {
        let client = self.client()?;
        let request = GetObjectRequest {
            bucket: required(&self.s3_bucket, "bucket")?.into(),
            key: self.key(file_key),
            ..Default::default()
        };
        let response = client.get_object(request).await?;
        let mut reader = response
            .body
            .ok_or_else(|| PGCliError::Other("S3 response has no object body".into()))?
            .into_async_read();
        let path = path.as_ref();
        let temporary = tempfile::NamedTempFile::new_in(super::files::parent(path))?;
        let mut file = File::from_std(temporary.reopen()?);
        let copied = tokio::io::copy(&mut reader, &mut file).await?;
        if response
            .content_length
            .is_some_and(|length| length < 0 || length as u64 != copied)
        {
            return Err(PGCliError::Other(
                "S3 object download was incomplete".into(),
            ));
        }
        file.flush().await?;
        file.sync_all().await?;
        drop(file);
        temporary
            .persist(path)
            .map_err(|error| PGCliError::TokioIo(error.error))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn part_sizing_covers_large_backups_and_enforces_limits() {
        for size in [
            0,
            1,
            SMALL_OBJECT,
            100 * 1024 * 1024 * 1024,
            5 * 1024_u64.pow(4),
        ] {
            let part = part_size(size).unwrap();
            assert!(size.div_ceil(part) <= 10_000);
            assert!((SMALL_OBJECT..=MAX_PART).contains(&part));
        }
        assert!(part_size(MAX_PART * 10_000 + 1).is_err());
    }

    #[tokio::test]
    async fn empty_upload_stream_and_truncated_file_are_handled() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut empty = file_body(file.path(), 0, 0)
            .await
            .unwrap()
            .into_async_read();
        let mut bytes = Vec::new();
        assert_eq!(empty.read_to_end(&mut bytes).await.unwrap(), 0);
        std::fs::write(file.path(), b"part").unwrap();
        let mut truncated = file_body(file.path(), 0, 8)
            .await
            .unwrap()
            .into_async_read();
        assert_eq!(
            truncated.read_to_end(&mut bytes).await.unwrap_err().kind(),
            std::io::ErrorKind::UnexpectedEof
        );
    }
}
