use super::db::database::list_databases;
use super::db::general::{db_dump, init_pgpass, postgres_connect};
use super::misc::{
    check_file_directory_path_exists, create_compressed_archive, delete_directory, delete_file,
    ensure_file_directory_path_exists,
};
use super::structs::{
    DatabaseDetails, PGCliError, PGTools, PostgresCredentials, S3Credentials, SqlFileFormat,
};
use clap::Args;
use log::{debug, error, info};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::{fs, task};

#[derive(Args, Debug, PartialEq)]
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
    /// Format of the output file
    /// defaults to bsql
    #[arg(short, long, env, value_enum, default_value = "bsql")]
    format: SqlFileFormat,
    /// Create archive or just files,
    /// has no effect on `all` backups
    #[arg(long, env)]
    no_archive: bool,
    // owner password for db
    // #[arg(short = 's', long)]
    // password: Option<String>,
}

pub async fn backup_db(
    data: &BackupArgs,
    credentials: &HashMap<String, PostgresCredentials>,
    pg_main_hostname: &String,
    s3_credentials: &S3Credentials,
    pg_tools: &PGTools,
) -> Result<(), PGCliError> {
    if let Some(output_location) = &data.output_location {
        if !check_file_directory_path_exists(output_location) {
            return Err(PGCliError::Other(
                "Output location does not exist".to_string(),
            ));
        }
        info!("Backing up to file: {}", output_location.to_str().unwrap());
    } else {
        info!("Backing up to S3");
        s3_credentials.validate_s3_credentials()?;
    }

    init_pgpass(
        &credentials
            .values()
            .cloned()
            .collect::<Vec<PostgresCredentials>>(),
    )
    .await?;

    let main_credentials = credentials.get(pg_main_hostname).unwrap();

    let client = postgres_connect(main_credentials, None).await?;

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
    let semaphore = Arc::new(Semaphore::new(data.jobs)); // Limit to 5 concurrent tasks

    let temp_folder = format!(
        "/tmp/psql_backup/postgresql-backup-{}",
        chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S")
    );

    ensure_file_directory_path_exists(&temp_folder).await?;

    let pg_tools = Arc::new(pg_tools.clone()); // Wrap pg_tools in an Arc
    let backup_format = Arc::new(data.format);

    let tasks: Vec<_> = databases
        .into_iter()
        .map(|database| {
            let permit = semaphore.clone(); // Get a permit to execute the task
            let credentials = main_credentials.clone();
            let temp_folder = temp_folder.clone();
            let pg_tools = pg_tools.clone(); // Clone the Arc, not the data
            let backup_format = backup_format.clone();

            task::spawn(async move {
                let _permit = permit
                    .acquire()
                    .await
                    .expect("Failed to acquire semaphore permit");
                let output_file = format!(
                    "{}/{}.{}",
                    temp_folder,
                    database,
                    match *backup_format {
                        SqlFileFormat::Bsql => "bsql",
                        SqlFileFormat::Sql => "sql",
                    }
                );
                match db_dump(
                    credentials,
                    &database,
                    &output_file,
                    &pg_tools,
                    *backup_format,
                )
                .await
                {
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

    let output_file_name = {
        let database_name = match data.database.as_str() {
            "all" => "postgresql-backup",
            _ => &data.database,
        };
        let file_suffix = if data.no_archive {
            match *backup_format {
                SqlFileFormat::Bsql => "bsql",
                SqlFileFormat::Sql => "sql",
            }
        } else {
            "tar.xz"
        };
        format!(
            "{}-{}.{}",
            database_name,
            chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S"),
            file_suffix
        )
    };

    let output_file = match &data.output_location {
        None => {
            format!("/tmp/psql_backup/{}", output_file_name)
        }
        Some(output_location) => output_location.to_str().unwrap().to_string(),
    };

    if !data.no_archive || data.database == "all" {
        info!("Creating compressed archive");
        create_compressed_archive(&temp_folder, &output_file).await?;
        info!("Compressed archive created successfully");
    }

    if data.database != "all" && data.no_archive {
        let output_location_str = format!(
            "{}/{}.{}",
            temp_folder,
            data.database,
            match *backup_format {
                SqlFileFormat::Bsql => "bsql",
                SqlFileFormat::Sql => "sql",
            }
        );
        info!(
            "Moving file from {} to {}",
            output_location_str, output_file
        );
        match fs::copy(output_location_str, &output_file).await {
            Ok(_) => info!("File moved successfully"),
            Err(e) => error!("Failed to move file: {}", e),
        }
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
    delete_directory(&temp_folder).await?;

    debug!("Backup completed successfully");

    Ok(())
}
