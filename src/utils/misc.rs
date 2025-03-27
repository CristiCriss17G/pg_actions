use super::structs::{PGCliError, PGTools, S3Credentials};
use log::{debug, info, trace, warn};
use rand::distr::Alphanumeric;
use rand::{rng, Rng};
use rusoto_core::{HttpClient, Region, RusotoError};
use rusoto_credential::StaticProvider;
use rusoto_s3::{
    CompleteMultipartUploadRequest, CompletedMultipartUpload, CompletedPart,
    CreateMultipartUploadRequest, GetObjectError, S3Client, UploadPartRequest, S3,
};
use std::collections::HashMap;
use std::fs::File;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::str::FromStr;
use tar::{Archive, Builder};
use tokio::fs::{self, File as AFile, OpenOptions};
use tokio::io::AsyncReadExt;
use tokio::{io, task};
use xz2::read::XzDecoder;
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

pub fn generate_random_string(length: usize, to_lowercase: bool) -> String {
    let mut rng = rng();
    let random_string: String = (&mut rng)
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .map(|c| {
            if to_lowercase {
                c.to_ascii_lowercase()
            } else {
                c
            }
        })
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

pub async fn search_file_in_directory(
    directory: &Path,
    file_name: &str,
) -> Result<Option<PathBuf>, PGCliError> {
    debug!(
        "Searching for file '{}' in directory '{}'",
        file_name,
        directory.display()
    );

    if !check_file_directory_path_exists(directory) {
        return Err(PGCliError::Other("Directory does not exist".to_string()));
    }

    // Spawn a blocking task to search the directory
    let directory = directory.to_path_buf();
    let file_name = file_name.to_string();
    task::spawn_blocking(move || {
        for entry in walkdir::WalkDir::new(&directory) {
            let entry = entry?;
            let path = entry.path();

            // Check if the file matches the search query
            if path.is_file() && *path.file_name().unwrap_or_default() == *file_name {
                trace!("Found file: {:?}", path);
                return Ok(Some(path.to_path_buf()));
            }
        }
        Ok(None) // Return None if the file is not found
    })
    .await?
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
    task::spawn_blocking(move || {
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
    .await?
}

pub async fn extract_compressed_archive(
    input_file: &Path,
    output_dir: &Path,
) -> Result<(), PGCliError> {
    debug!("Extracting compressed archive");

    // clone the input_file and output_dir to move into the blocking task
    let input_file = input_file.to_path_buf();
    let output_dir = output_dir.to_path_buf();

    if !check_file_path_exists(&input_file) {
        return Err(PGCliError::Other("Input file does not exist".to_string()));
    }

    // if the output directory does not exist, create it
    if !check_file_directory_path_exists(&output_dir) {
        ensure_file_directory_path_exists(&output_dir).await?;
        trace!("Created output directory {:?}", output_dir);
    }

    // spawn a blocking task to extract the compressed archive
    task::spawn_blocking(move || {
        // Open the compressed file
        let file = File::open(&input_file)?;
        let decoder = XzDecoder::new(file);

        let mut archive = Archive::new(decoder);

        debug!("Extracting files from the archive");
        // Extract all files from the archive to the output directory
        archive.unpack(&output_dir)?;

        Ok(())
    })
    .await?
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

    pub async fn download_file<P: AsRef<Path>>(
        &self,
        file_key: &str,
        file_path: P,
    ) -> Result<(), PGCliError> {
        debug!("Downloading file from S3");
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

        let file_key_prepared = match self.s3_prefix {
            Some(ref prefix) => format!("{}/{}", prefix, file_key),
            None => file_key.to_string(),
        };

        let get_req = rusoto_s3::GetObjectRequest {
            bucket: self
                .s3_bucket
                .clone()
                .ok_or_else(|| PGCliError::Other("S3 bucket is required".to_string()))?,
            key: file_key_prepared.clone(),
            ..Default::default()
        };

        match s3_client.get_object(get_req).await {
            Ok(response) => {
                let mut stream = response
                    .body
                    .ok_or(PGCliError::Other(
                        "Failed to get object body from S3 response".to_string(),
                    ))?
                    .into_async_read();
                let mut file = open_file_path_in_write_mode(file_path, None).await?;
                io::copy(&mut stream, &mut file).await?;
            }
            Err(RusotoError::Service(GetObjectError::NoSuchKey(_))) => {
                // Handle the case where the file does not exist in the S3 bucket
                return Err(PGCliError::Other(format!(
                    "File {} does not exist in the S3 bucket",
                    file_key_prepared
                )));
            }
            Err(e) => {
                // Handle any other errors (network issues, permissions, etc.)
                return Err(PGCliError::Other(format!("Failed to download file: {}", e)));
            }
        };

        debug!("Downloaded file to {:?}", file_path);

        Ok(())
    }
}

fn check_command_availability(command: &str) -> Result<Output, io::Error> {
    Command::new(command)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
}

fn parse_version(version_output: &str) -> Option<u32> {
    // Split by whitespace and find the part that starts with the version number
    version_output
        .split_whitespace()
        .find(|&part| part.chars().next().unwrap_or(' ').is_ascii_digit())
        .and_then(|version| version.split('.').next())
        .and_then(|major| major.parse::<u32>().ok())
}

fn pg_tools_error_messages(
    pg_dump_version: Option<u32>,
    pg_restore_version: Option<u32>,
    psql_version: Option<u32>,
    min_version: u32,
) -> Option<String> {
    let mut error_message = String::new();

    let format_error = |tool_name: &str, version: Option<u32>| {
        if let Some(v) = version {
            if v < min_version {
                format!(
                    "{} version {} is not supported. Minimum supported version is {}.",
                    tool_name, v, min_version
                )
            } else {
                String::new()
            }
        } else {
            format!("{} tool not found.", tool_name)
        }
    };

    error_message.push_str(&format_error("pg_dump", pg_dump_version));
    error_message.push_str(&format_error("pg_restore", pg_restore_version));
    error_message.push_str(&format_error("psql", psql_version));

    if !error_message.is_empty() {
        error_message.push_str(&format!("\nPlease install {} {} or later.\nFor more information, visit: https://www.postgresql.org/download/", "PostgreSQL", min_version));
        Some(error_message)
    } else {
        None
    }
}

pub fn check_pg_tools_version(
    commands: &HashMap<String, String>,
    min_version: u32,
    warn: bool,
) -> Result<PGTools, PGCliError> {
    let pg_dump = match commands.get("pg_dump") {
        Some(path) => {
            debug!("Using pg_dump from: {}", path);
            path
        }
        _ => "pg_dump",
    };
    let pg_restore = match commands.get("pg_restore") {
        Some(path) => {
            debug!("Using pg_restore from: {}", path);
            path
        }
        _ => "pg_restore",
    };
    let psql = match commands.get("psql") {
        Some(path) => {
            debug!("Using psql from: {}", path);
            path
        }
        _ => "psql",
    };

    let pg_dump_version = check_command_availability(pg_dump)
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
        .map(|version_output| parse_version(&version_output))
        .unwrap_or(None);

    let pg_restore_version = check_command_availability(pg_restore)
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
        .map(|version_output| parse_version(&version_output))
        .unwrap_or(None);

    let psql_version = check_command_availability(psql)
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
        .map(|version_output| parse_version(&version_output))
        .unwrap_or(None);

    if let Some(error_message) = pg_tools_error_messages(
        pg_dump_version,
        pg_restore_version,
        psql_version,
        min_version,
    ) {
        if warn {
            warn!("{}", error_message);
        }
        Err(PGCliError::PGToolsError(error_message))
    } else {
        Ok(PGTools::new(
            pg_dump.to_string(),
            pg_restore.to_string(),
            psql.to_string(),
        ))
    }
}
