use super::archive::{detect_compression, extract_compressed_archive};
use super::db::general::postgres_connect;
use super::db::staging::{preflight, staged_restore};
use super::files::{dump_filename, finish_workspace, safe_existing_file, workspace};
use super::structs::{
    PGCliError, PGTools, PostgresCredentials, Result, S3Credentials, SqlFileFormat,
};
use clap::Args;
use log::{info, warn};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Args, Debug, PartialEq)]
pub struct RestoreArgs {
    /// database to restore
    database: String,
    /// input location
    /// chose between s3 or a file path
    /// when s3 is chosen, the input location should be in the format s3://bucket_name/file_path
    /// you can also specify all the S3_* args/environment variables to use S3, the file url will have priority
    #[arg(short, long, env = "RESTORE_LOCATION")]
    input_location: PathBuf,
    /// Parallel jobs to use
    /// defaults to 5
    #[arg(short, long, env, default_value = "5")]
    jobs: usize,
    /// Format of the input file
    /// defaults to bsql
    #[arg(short, long, env, value_enum, default_value = "bsql")]
    format: SqlFileFormat,
    /// File name, without extension,
    /// in case of archive extraction and the file name is different from the database name
    #[arg(long = "file", env)]
    file_name: Option<String>,
    /// Owner user for db
    #[arg(short = 'o', long)]
    owner: Option<String>,
    /// overwrite the database if it exists
    #[arg(long, env)]
    overwrite: bool,
    /// keep the temp files
    /// defaults to false
    #[arg(short, long, env)]
    keep_temp: bool,
}

impl RestoreArgs {
    pub fn format(&self) -> SqlFileFormat {
        self.format
    }
}

pub async fn restore_db(
    data: &RestoreArgs,
    credentials: &PostgresCredentials,
    s3: &S3Credentials,
    tools: &PGTools,
) -> Result<()> {
    let client = postgres_connect(credentials, None).await?;
    preflight(
        &client,
        &data.database,
        data.owner.as_deref(),
        data.overwrite,
    )
    .await?;
    let directory = workspace("postgresql-restore-")?;
    let result = async {
        let file = get_file_location(&data.input_location, s3, directory.path()).await?;
        let restore_file = if detect_compression(&file)?.is_some() {
            let extracted = directory.path().join("extracted");
            tokio::fs::create_dir(&extracted).await?;
            extract_compressed_archive(&file, &extracted).await?;
            let source = data.file_name.as_deref().unwrap_or(&data.database);
            let filename = dump_filename(source, data.format.file_extension());
            // Encoded names protect new archives; plain names retain old archives.
            let matched = safe_existing_file(&extracted, &filename)?;
            match matched {
                Some(path) => path,
                None => safe_existing_file(
                    &extracted,
                    &format!("{source}.{}", data.format.file_extension()),
                )?
                .ok_or_else(|| {
                    PGCliError::Other("Requested database dump was not found in archive".into())
                })?,
            }
        } else {
            file
        };
        staged_restore(
            credentials,
            &data.database,
            &restore_file,
            tools,
            data.format,
            data.owner.as_deref(),
            data.overwrite,
        )
        .await
    }
    .await;
    finish_workspace(directory, data.keep_temp, result)
}

async fn get_file_location(input: &Path, s3: &S3Credentials, workspace: &Path) -> Result<PathBuf> {
    let Some(remote) = input.to_str().filter(|value| value.starts_with("s3://")) else {
        if !input.is_file() {
            return Err(PGCliError::Other("Input location is not a file".into()));
        }
        return Ok(input.to_path_buf());
    };
    // Keep existing URL/prefix semantics here; A11 is explicitly deferred.
    s3.validate_s3_credentials()?;
    let mut key = remote.replace("s3://", "");
    if let Some(bucket) = &s3.s3_bucket {
        key = key.replace(&format!("{bucket}/"), "");
    }
    if let Some(prefix) = &s3.s3_prefix {
        key = key.replace(&format!("{prefix}/"), "");
    }
    let suffix = if key.ends_with(".tar.xz") {
        "tar.xz"
    } else if key.ends_with(".tar.zst") {
        "tar.zst"
    } else {
        "dump"
    };
    let file = workspace.join(format!("download.{suffix}"));
    for attempt in 1..=3 {
        match s3.download_file(&key, &file).await {
            Ok(()) => {
                info!("Downloaded restore input");
                return Ok(file);
            }
            Err(error) if attempt < 3 => {
                warn!("Download attempt {attempt} failed: {error}");
                tokio::time::sleep(Duration::from_secs(1 << (attempt - 1))).await;
            }
            Err(error) => return Err(error),
        }
    }
    Err(PGCliError::Other("Download did not complete".into()))
}
