use super::db::{db_dump, init_pgpass, list_databases, postgres_connect};
use super::misc::{
    check_file_directory_path_exists, create_compressed_archive, delete_directory,
    ensure_file_directory_path_exists,
};
use super::structs::{PGCliError, PostgresCredentials, S3Credentials};
use clap::Args;
use log::{debug, error, info};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task;

#[derive(Args)]
pub struct BackupArgs {
    /// database to backup
    /// defaults to all
    database: String,
    /// output location
    /// chose between s3 or a file path, defaults to s3;
    /// when s3 is chosen, the output location is in the bucket specified by the S3_* environment variables with the name of postgresql-backup-<timestamp>.tar.xz;
    /// when a file path is chosen, the output location is the file path specified, but the extension is .tar.xz
    #[arg(short, long, env = "BACKUP_LOCATION")]
    output_location: Option<PathBuf>,
    /// Parallel jobs to use
    /// defaults to 5
    #[arg(short, long, env, default_value = "5")]
    jobs: usize,
    /// owner password for db
    #[arg(short = 's', long)]
    password: Option<String>,
}

pub async fn backup_db(
    data: &BackupArgs,
    credentials: &PostgresCredentials,
    s3_credentials: &S3Credentials,
) -> Result<(), PGCliError> {
    if let Some(output_location) = &data.output_location {
        if !check_file_directory_path_exists(&output_location) {
            return Err(PGCliError::Other(
                "Output location does not exist".to_string(),
            ));
        }
        info!("Backing up to file: {}", output_location.to_str().unwrap());
    } else {
        info!("Backing up to S3");
        s3_credentials.validate_s3_credentials()?;
    }

    init_pgpass(credentials).await?;

    let client = postgres_connect(credentials, None).await?;

    let databases = match data.database.as_str() {
        "all" => list_databases(&client).await?,
        _ => vec![data.database.clone()],
    };
    info!("Databases to backup: {:?}", databases);

    debug!("Using {} parallel jobs", data.jobs);
    let semaphore = Arc::new(Semaphore::new(data.jobs)); // Limit to 5 concurrent tasks

    let temp_folder = format!(
        "/tmp/psql_backup/postgresql-backup-{}",
        chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S")
    );

    ensure_file_directory_path_exists(&temp_folder).await?;

    let tasks: Vec<_> = databases
        .into_iter()
        .map(|database| {
            let permit = semaphore.clone().acquire_owned(); // Get a permit to execute the task
            let credentials = credentials.clone();
            let temp_folder = temp_folder.clone();

            task::spawn(async move {
                let _permit = permit.await.expect("Failed to acquire semaphore permit");
                let output_file = format!("{}/{}.bsql", temp_folder, database);
                match db_dump(credentials, &database, &output_file).await {
                    Ok(_) => info!("Database {} dumped successfully", database),
                    Err(e) => error!("Failed to dump database {}: {}", database, e),
                }
            })
        })
        .collect();

    // Await all tasks to complete
    for task in tasks {
        let _ = task.await;
    }

    info!("All databases dumped successfully");

    let output_file = match &data.output_location {
        None => format!(
            "/tmp/psql_backup/postgresql-backup-{}.tar.xz",
            chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S")
        ),
        Some(output_location) => output_location.to_str().unwrap().to_string(),
    };

    info!("Creating compressed archive");
    create_compressed_archive(&temp_folder, &output_file).await?;
    info!("Compressed archive created successfully");

    debug!("Deleting temp folder");
    delete_directory(&temp_folder).await?;

    Ok(())
}
