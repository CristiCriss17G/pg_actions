use std::sync::Arc;

use super::db::{db_dump, init_pgpass, list_databases, postgres_connect};
use super::misc::{check_file_path_exists, validate_s3_credentials};
use super::structs::{PGCliError, PostgresCredentials, S3Credentials};
use clap::Args;
use log::{error, info};
use tokio::sync::Semaphore;
use tokio::task;

#[derive(Args)]
pub struct BackupArgs {
    /// database to backup
    #[arg(short, long)]
    database: String,
    /// output location
    /// chose between s3 or a file path, defaults to s3;
    /// when s3 is chosen, the output location is in the bucket specified by the S3_* environment variables with the name of postgresql-backup-<timestamp>.tar.xz;
    /// when a file path is chosen, the output location is the file path specified, but the extension is .tar.xz
    #[arg(short, long, env = "BACKUP_LOCATION", default_value = "s3")]
    output_location: String,
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
    info!("Backing up all databases to {}", data.output_location);

    match data.output_location.as_str() {
        "s3" => {
            validate_s3_credentials(s3_credentials)?;
        }
        _ => {
            if !check_file_path_exists(&data.output_location) {
                return Err(PGCliError::Other(
                    "Output location does not exist".to_string(),
                ));
            }
        }
    }

    init_pgpass(credentials).await?;

    let client = postgres_connect(credentials, None).await?;

    let databases = list_databases(&client).await?;

    let semaphore = Arc::new(Semaphore::new(data.jobs)); // Limit to 5 concurrent tasks

    let credentials = Arc::new(credentials.clone());

    let tasks: Vec<_> = databases
        .into_iter()
        .map(|database| {
            let permit = semaphore.clone().acquire_owned(); // Get a permit to execute the task
            let credentials = credentials.clone();

            task::spawn(async move {
                let _permit = permit.await.expect("Failed to acquire semaphore permit");
                let output_file = format!("/tmp/psql_backup/{}.bsql", database);
                match db_dump(&*credentials, &database, &output_file).await {
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

    Ok(())
}
