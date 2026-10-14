use super::archive::{Compression, CompressionOptions, create_compressed_archive};
use super::db::database::list_databases;
use super::db::general::{db_dump, postgres_connect};
use super::misc::delete_file;
use super::structs::{
    DatabaseDetails, PGCliError, PGTools, PgDumpCompression, PostgresCredentials, Result,
    S3Credentials, SqlFileFormat,
};
use crate::utils::files::{dump_filename, finish_workspace, parent, workspace};
use crate::utils::sql::validate_identifier;
use clap::Args;
use log::{debug, error, info};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::{fs, task};

#[derive(Args, Debug, PartialEq)]
pub struct BackupArgs {
    /// database to backup
    /// defaults to all
    database: String,
    /// Output file path; defaults to S3 with a timestamped name.
    /// Archive suffixes are .tar.zst or .tar.xz according to --compression.
    #[arg(short, long, env = "BACKUP_LOCATION")]
    output_location: Option<PathBuf>,
    /// Concurrent database dumps (independent of compression threads)
    #[arg(short, long, env, default_value = "5", value_parser = clap::value_parser!(u32).range(1..))]
    jobs: u32,
    /// Format of the output file
    /// defaults to bsql
    #[arg(short, long, env, value_enum, default_value = "bsql")]
    format: SqlFileFormat,
    /// Create archive or just files,
    /// has no effect on `all` backups
    #[arg(long, env)]
    no_archive: bool,
    /// Keep private staging files on success or failure
    #[arg(long, env)]
    keep_temp: bool,
    /// Archive compression codec
    #[arg(long, env, value_enum, default_value = "zstd")]
    compression: Compression,
    /// Archive compression level: zstd 1-22 (default 3), xz 0-9 (default 9)
    #[arg(long, env)]
    compression_level: Option<u32>,
    /// Compression worker threads; 1 disables parallel compression.
    /// Parallel xz at high levels can require substantial memory.
    #[arg(long, env, default_value = "1", value_parser = clap::value_parser!(u32).range(1..))]
    compression_threads: u32,
    /// pg_dump compression: defaults to none for archives, default for raw files
    #[arg(long, env, value_enum)]
    pg_dump_compression: Option<PgDumpCompression>,
}

impl BackupArgs {
    fn creates_archive(&self) -> bool {
        !self.no_archive || self.database == "all"
    }

    fn dump_compression(&self) -> PgDumpCompression {
        self.pg_dump_compression
            .unwrap_or(if self.creates_archive() {
                PgDumpCompression::None
            } else {
                PgDumpCompression::Default
            })
    }

    fn output_path(&self, generated_name: &str) -> Result<PathBuf> {
        let path = self
            .output_location
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join(generated_name));
        if self.creates_archive() {
            self.compression.output_path(&path)
        } else {
            Ok(path)
        }
    }
}

pub async fn backup_db(
    data: &BackupArgs,
    main_credentials: &PostgresCredentials,
    s3_credentials: &S3Credentials,
    pg_tools: &PGTools,
) -> Result<()> {
    let compression_options = if data.creates_archive() {
        Some(CompressionOptions::new(
            data.compression,
            data.compression_level,
            data.compression_threads,
        )?)
    } else {
        None
    };
    let database_name = if data.database == "all" {
        "postgresql-backup"
    } else {
        &data.database
    };
    let suffix = if data.creates_archive() {
        data.compression.extension()
    } else {
        data.format.file_extension()
    };
    let output_file_name = format!(
        "{}-{}.{}",
        database_name,
        chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S"),
        suffix
    );
    let mut output_file = data.output_path(&output_file_name)?;
    if let Some(output_location) = &data.output_location {
        if !parent(output_location).is_dir() {
            return Err(PGCliError::Other(
                "Output location does not exist".to_string(),
            ));
        }
        info!("Backing up to file: {}", output_file.display());
    } else {
        info!("Backing up to S3");
        s3_credentials.validate_s3_credentials()?;
    }

    if data.database != "all" {
        validate_identifier(&data.database)?;
    }
    let client = postgres_connect(main_credentials, None).await?;
    pg_tools.check_dump_server(&client).await?;

    let databases: Vec<String> = match data.database.as_str() {
        "all" => list_databases(&client, &None, false, &None)
            .await?
            .iter()
            .map(|x| match &x {
                DatabaseDetails::Name(name) => name.clone(),
                DatabaseDetails::Extra { name, .. } => name.clone(),
            })
            .collect(),
        _ => vec![data.database.clone()],
    };
    info!("Databases to backup: {:?}", databases);

    debug!("Using {} parallel jobs", data.jobs);
    let semaphore = Arc::new(Semaphore::new(data.jobs as usize));

    let temp_directory = workspace("postgresql-backup-")?;
    let temp_folder = temp_directory.path().to_path_buf();
    if data.output_location.is_none() {
        output_file = temp_folder.join("upload-output");
    }
    let result = async {
        let dump_folder = temp_folder.join("dumps");
        fs::create_dir(&dump_folder).await?;
        let pg_tools = Arc::new(pg_tools.clone()); // Wrap pg_tools in an Arc
        let backup_format = Arc::new(data.format);
        let dump_compression = data.dump_compression();
        debug!("pg_dump compression: {:?}", dump_compression);

        let file_map: Vec<_> = databases.iter().map(|name| serde_json::json!({"database": name, "file": dump_filename(name, backup_format.file_extension())})).collect();
    fs::write(dump_folder.join("files.json"), serde_json::to_vec_pretty(&file_map)?).await?;
    let tasks: Vec<_> = databases
            .into_iter()
            .map(|database| {
                let permit = semaphore.clone(); // Get a permit to execute the task
                let credentials = main_credentials.clone();
                let temp_folder = dump_folder.clone();
                let pg_tools = pg_tools.clone(); // Clone the Arc, not the data
                let backup_format = backup_format.clone();

                task::spawn(async move {
                    let _permit = permit
                        .acquire()
                        .await
                        .map_err(|_| PGCliError::Other("Backup task semaphore closed".into()))?;
                    let output_file =
                        temp_folder.join(dump_filename(&database, backup_format.file_extension()));
                    db_dump(
                        credentials,
                        &database,
                        &output_file,
                        &pg_tools,
                        *backup_format,
                        dump_compression,
                    )
                    .await
                    .map_err(|error| {
                        PGCliError::Other(format!(
                            "Failed to dump database {}: {}",
                            database, error
                        ))
                    })?;
                    info!("Database {} dumped successfully", database);
                    Ok::<(), PGCliError>(())
                })
            })
            .collect();

        // Wait for every dump before returning an error or removing temporary files.
        let mut first_error = None;
        for task in tasks {
            let result = match task.await {
                Ok(result) => result,
                Err(error) => Err(PGCliError::TaskJoin(error)),
            };
            if let Err(error) = result {
                error!("{}", error);
                first_error.get_or_insert(error);
            }
        }
        if let Some(error) = first_error {
            return Err(error);
        }

        info!("All databases dumped successfully");

        if let Some(options) = compression_options {
            info!(
                "Creating {:?} archive at level {} with {} compression thread(s)",
                options.codec, options.level, options.threads
            );
            create_compressed_archive(&dump_folder, &output_file, options).await?;
            info!("Compressed archive created successfully");
        }

        if !data.creates_archive() {
            let output_location =
                dump_folder.join(dump_filename(&data.database, data.format.file_extension()));
            info!(
                "Copying file from {} to {}",
                output_location.display(),
                output_file.display()
            );
            let temporary = tempfile::NamedTempFile::new_in(parent(&output_file))?;
            fs::copy(output_location, temporary.path()).await?;
            temporary
                .persist(&output_file)
                .map_err(|error| PGCliError::TokioIo(error.error))?;
            info!("File copied successfully");
        }

        if data.output_location.is_none() {
            info!("Uploading to S3");
            s3_credentials
                .multipart_upload(&output_file, &output_file_name)
                .await?;
            info!("Uploaded to S3 successfully");
            debug!("Deleting local file");
            delete_file(&output_file).await?;
        }

        debug!("Deleting temp folder");

        debug!("Backup completed successfully");

        Ok(())
    }
    .await;
    finish_workspace(temp_directory, data.keep_temp, result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct BackupCli {
        #[command(flatten)]
        args: BackupArgs,
    }

    fn args(arguments: &[&str]) -> BackupArgs {
        BackupCli::try_parse_from(std::iter::once("backup").chain(arguments.iter().copied()))
            .unwrap()
            .args
    }

    #[test]
    fn compression_defaults_follow_archive_behavior() {
        let archived = args(&["example"]);
        assert_eq!(archived.compression, Compression::Zstd);
        assert_eq!(archived.compression_threads, 1);
        assert_eq!(archived.dump_compression(), PgDumpCompression::None);
        let raw = args(&["example", "--no-archive"]);
        assert!(!raw.creates_archive());
        assert_eq!(raw.dump_compression(), PgDumpCompression::Default);
        let all = args(&["all", "--no-archive"]);
        assert!(all.creates_archive());
        assert_eq!(all.dump_compression(), PgDumpCompression::None);
        assert_eq!(
            all.output_path("all.tar.zst").unwrap(),
            std::env::temp_dir().join("all.tar.zst")
        );
        assert_eq!(
            args(&["example", "--pg-dump-compression", "default"]).dump_compression(),
            PgDumpCompression::Default
        );
        assert_eq!(
            args(&["example", "--no-archive", "--pg-dump-compression", "none"]).dump_compression(),
            PgDumpCompression::None
        );
    }

    #[test]
    fn rejects_zero_jobs_and_compression_threads() {
        for option in ["--jobs", "--compression-threads"] {
            assert!(BackupCli::try_parse_from(["backup", "example", option, "0"]).is_err());
        }
    }
}
