use super::structs::{PGCliError, S3Credentials};
use log::{debug, info, trace};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rusoto_core::{HttpClient, Region};
use rusoto_credential::StaticProvider;
use rusoto_s3::{
    CompleteMultipartUploadRequest, CompletedMultipartUpload, CompletedPart,
    CreateMultipartUploadRequest, S3Client, UploadPartRequest, S3,
};
use std::fs::File;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::str::FromStr;
use tar::Builder;
use tokio::fs::{self, File as AFile, OpenOptions};
use tokio::io::AsyncReadExt;
use tokio::{io, task};
use xz2::stream::{Check, Stream};
use xz2::write::XzEncoder;

fn bytes_to_human_readable(size: u64) -> String {
    let units = ["bytes", "KB", "MB", "GB", "TB", "PB"];
    let mut size = size as f64;
    let mut index = 0;

    while size >= 1024.0 && index < units.len() - 1 {
        size /= 1024.0;
        index += 1;
    }

    format!("{:.2} {}", size, units[index])
}

pub fn generate_random_string(length: usize) -> String {
    let mut rng = thread_rng();
    let random_string: String = (&mut rng)
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    random_string
}

pub fn check_file_path_exists<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref(); // Convert P to &Path
    path.exists()
}

pub fn check_file_directory_path_exists<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref(); // Convert P to &Path
    if let Some(dir) = path.parent() {
        return dir.exists();
    }
    false
}

pub async fn ensure_file_path_exists<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref(); // Convert P to &Path
    if let Some(dir) = path.parent() {
        ensure_file_directory_path_exists(dir).await?;
    }
    Ok(())
}

pub async fn ensure_file_directory_path_exists<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref(); // Convert P to &Path
    if !path.exists() {
        fs::create_dir_all(path).await?;
    }
    Ok(())
}

pub async fn open_file_path_in_write_mode(
    path: &Path,
    permissions: Option<u32>,
) -> io::Result<fs::File> {
    ensure_file_path_exists(path).await?;

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .await?;

    // Set the permissions for the file to 644 (owner: read-write, group: read, others: read)
    let permissions = Permissions::from_mode(permissions.unwrap_or(0o644));
    fs::set_permissions(path, permissions).await?;

    Ok(file)
}

pub async fn delete_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref(); // Convert P to &Path
    fs::remove_file(path).await?;
    Ok(())
}

pub async fn delete_directory<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref(); // Convert P to &Path
    fs::remove_dir_all(path).await?;
    Ok(())
}

pub async fn create_compressed_archive(
    input_dir: &str,
    output_file: &str,
) -> Result<(), PGCliError> {
    debug!("Creating compressed archive");
    // clone the input_dir and output_file to move into the blocking task
    let input_dir = input_dir.to_string();
    let mut output_file = output_file.to_string();

    if !check_file_directory_path_exists(&input_dir) {
        return Err(PGCliError::Other(
            "Input directory does not exist".to_string(),
        ));
    }

    // if the output file does not end with .tar.xz, append it
    if !output_file.ends_with(".tar.xz") {
        output_file.push_str(".tar.xz");
    }

    //if the output file exists, delete it
    if check_file_path_exists(&output_file) {
        delete_file(&output_file).await?;
        trace!("Deleted existing file {:?}", output_file);
    }

    // spawn a blocking task to create the compressed archive
    let output = task::spawn_blocking(move || {
        // Create a file for the output
        let file = File::create(&output_file)?;
        let stream = Stream::new_easy_encoder(9, Check::Crc64)?;
        let xz_encoder = XzEncoder::new_stream(file, stream);

        let mut archive = Builder::new(xz_encoder);

        debug!("Adding files to the archive");
        // Recursively add files from input_dir to the archive.
        // This part is simplified; you might want to add error handling and async file reads.
        for entry in walkdir::WalkDir::new(&input_dir) {
            let entry = entry?;
            let path = entry.path();
            debug!("Adding {:?}", path);
            if path.is_file() {
                trace!("Adding file {:?}", path);
                archive.append_path_with_name(
                    path,
                    path.strip_prefix(&input_dir)?.to_str().unwrap(),
                )?;
            }
        }

        // Ensure all data is flushed and the archive is finished
        archive.finish()?;

        Ok(())
    })
    .await?;

    output
}

impl S3Credentials {
    pub fn validate_s3_credentials(&self) -> Result<&S3Credentials, PGCliError> {
        let s3_endpoint = self.s3_endpoint.as_deref().unwrap_or("").to_string();
        let s3_access_key = self.s3_access_key.as_deref().unwrap_or("").to_string();
        let s3_secret_key = self.s3_secret_key.as_deref().unwrap_or("").to_string();
        let s3_bucket = self.s3_bucket.as_deref().unwrap_or("").to_string();
        let s3_region = self.s3_region.as_deref().unwrap_or("").to_string();
        let s3_prefix = self.s3_prefix.as_deref().unwrap_or("").to_string();

        if s3_endpoint.is_empty() {
            return Err(PGCliError::Other("S3 endpoint is required".to_string()));
        }
        if s3_access_key.is_empty() {
            return Err(PGCliError::Other("S3 access key is required".to_string()));
        }
        if s3_secret_key.is_empty() {
            return Err(PGCliError::Other("S3 secret key is required".to_string()));
        }
        if s3_bucket.is_empty() {
            return Err(PGCliError::Other("S3 bucket is required".to_string()));
        }
        if s3_region.is_empty() {
            return Err(PGCliError::Other("S3 region is required".to_string()));
        }
        if s3_prefix.is_empty()
            || !s3_prefix
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '/' || c == '-')
        {
            return Err(PGCliError::Other(
                "S3 prefix is required or invalid".to_string(),
            ));
        }

        Ok(self)
    }

    pub async fn multipart_upload<P: AsRef<Path>>(
        &self,
        file_path: P,
        file_key: &str,
    ) -> Result<(), PGCliError> {
        debug!("Starting multipart upload");
        let file_path = file_path.as_ref();
        let region = match self.s3_endpoint {
            Some(ref endpoint) => Region::Custom {
                name: self
                    .s3_region
                    .clone()
                    .unwrap_or_else(|| "us-east-1".to_owned()),
                endpoint: endpoint.clone(),
            },
            None => self
                .s3_region
                .clone()
                .map_or(Region::UsEast1, |region_name| {
                    Region::from_str(&region_name).unwrap_or(Region::UsEast1)
                }),
        };

        let s3_client = S3Client::new_with(
            HttpClient::new()?,
            StaticProvider::new_minimal(
                self.s3_access_key.clone().unwrap(),
                self.s3_secret_key.clone().unwrap(),
            ),
            region,
        );

        let file_size = tokio::fs::metadata(file_path)
            .await
            .expect("it exists I swear")
            .len();

        debug!(
            "File size: {} bytes, {}",
            file_size,
            bytes_to_human_readable(file_size)
        );

        let file_key_prepared = format!(
            "{}/{}",
            self.s3_prefix.clone().unwrap_or_else(|| "".to_owned()),
            file_key
        );

        let create_multipart_req = CreateMultipartUploadRequest {
            bucket: self
                .s3_bucket
                .clone()
                .ok_or_else(|| PGCliError::Other("S3 bucket is required".to_string()))?,
            key: file_key_prepared.clone(),
            ..Default::default()
        };

        info!("Creating multipart upload for {}", file_key_prepared);

        let upload_id = s3_client
            .create_multipart_upload(create_multipart_req)
            .await?
            .upload_id
            .ok_or_else(|| PGCliError::Other("Failed to get upload ID".to_string()))?;

        trace!("Upload ID: {}", upload_id);

        let mut file = AFile::open(file_path).await?;
        let mut parts = Vec::new();
        let mut part_number = 1;
        let mut buffer;
        let parts_number = (file_size as f64 / 10_000_000.0).ceil() as usize;

        if file_size > 10 * 1024 * 1024 {
            debug!("File size is greater than 10MB, using 10MB buffer");
            buffer = vec![0; 10 * 1024 * 1024]; // 10MB buffer
            trace!("Buffer size: {}", buffer.len());
        } else {
            debug!("File size is less than 10MB, using file size * 1.5 as buffer");
            buffer = vec![0; (file_size as f64 * 1.5) as usize];
            trace!("Buffer size: {}", buffer.len());
        }

        loop {
            let mut total_bytes_read = 0;

            loop {
                if total_bytes_read >= buffer.len() {
                    break; // Exit if the buffer is full.
                }

                match file.read(&mut buffer[total_bytes_read..]).await {
                    Ok(0) => break, // EOF reached.
                    Ok(bytes_read) => {
                        trace!("Bytes read: {}", bytes_read);
                        total_bytes_read += bytes_read;
                        if total_bytes_read == buffer.len() {
                            break; // Buffer is fully utilized.
                        }
                    }
                    Err(e) => return Err(e.into()), // Handle errors appropriately.
                }
            }

            trace!("Total bytes read: {}", total_bytes_read);
            if total_bytes_read == 0 {
                break;
            }

            trace!("Uploading part {} of {}", part_number, parts_number);
            trace!("Upload ID: {}", upload_id);
            let upload_part_req = UploadPartRequest {
                bucket: self
                    .s3_bucket
                    .clone()
                    .ok_or_else(|| PGCliError::Other("S3 bucket is required".to_string()))?,
                key: file_key_prepared.clone(),
                upload_id: upload_id.clone(),
                part_number: part_number as i64,
                body: Some(buffer[..total_bytes_read].to_vec().into()),
                ..Default::default()
            };

            let part_result = s3_client.upload_part(upload_part_req).await?;
            parts.push(CompletedPart {
                e_tag: part_result.e_tag,
                part_number: Some(part_number as i64),
            });

            part_number += 1;
        }

        debug!("Uploaded all parts");

        let complete_req = CompleteMultipartUploadRequest {
            bucket: self.s3_bucket.clone().unwrap(),
            key: file_key_prepared.clone(),
            upload_id,
            multipart_upload: Some(CompletedMultipartUpload { parts: Some(parts) }),
            ..Default::default()
        };

        debug!("Completing multipart upload");
        s3_client.complete_multipart_upload(complete_req).await?;

        Ok(())
    }
}
