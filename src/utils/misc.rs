use super::structs::{PGCliError, S3Credentials};
use log::{debug, trace};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::fs::File;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tar::Builder;
use tokio::fs::{self, OpenOptions};
use tokio::{io, task};
use xz2::stream::{Check, Stream};
use xz2::write::XzEncoder;

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
