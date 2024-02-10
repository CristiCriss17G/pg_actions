use crate::utils::structs::S3Credentials;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::fs::{self, OpenOptions, Permissions};
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use super::structs::PGCliError;

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

pub fn check_file_path_exists(path: &str) -> bool {
    let path = Path::new(path);
    if let Some(dir) = path.parent() {
        return dir.exists();
    }
    false
}

pub fn open_file_in_write_mode(path: &str, permissions: Option<u32>) -> io::Result<fs::File> {
    let path = Path::new(path);
    let file = open_file_path_in_write_mode(path, permissions).expect("Failed to open file");
    Ok(file)
}

pub fn open_file_path_in_write_mode(path: &Path, permissions: Option<u32>) -> io::Result<fs::File> {
    if let Some(dir) = path.parent() {
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
    }

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;

    // Set the permissions for the file to 644 (owner: read-write, group: read, others: read)
    let permissions = Permissions::from_mode(permissions.unwrap_or(0o644));
    fs::set_permissions(path, permissions)?;

    Ok(file)
}

pub fn delete_file_path(path: &Path) -> io::Result<()> {
    fs::remove_file(path)?;
    Ok(())
}

pub fn delete_file(path: &str) -> io::Result<()> {
    let path = Path::new(path);
    delete_file_path(path)
}

pub fn validate_s3_credentials(credentials: &S3Credentials) -> Result<S3Credentials, PGCliError> {
    let s3_endpoint = credentials.s3_endpoint.as_deref().unwrap_or("").to_string();
    let s3_access_key = credentials
        .s3_access_key
        .as_deref()
        .unwrap_or("")
        .to_string();
    let s3_secret_key = credentials
        .s3_secret_key
        .as_deref()
        .unwrap_or("")
        .to_string();
    let s3_bucket = credentials.s3_bucket.as_deref().unwrap_or("").to_string();
    let s3_region = credentials.s3_region.as_deref().unwrap_or("").to_string();
    let s3_prefix = credentials.s3_prefix.as_deref().unwrap_or("").to_string();

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

    Ok(S3Credentials {
        s3_endpoint: Some(s3_endpoint),
        s3_access_key: Some(s3_access_key),
        s3_secret_key: Some(s3_secret_key),
        s3_bucket: Some(s3_bucket),
        s3_region: Some(s3_region),
        s3_prefix: Some(s3_prefix),
    })
}
