use super::db::database::list_databases;
use super::db::general::{db_restore, init_pgpass, postgres_connect};
use super::misc::{
    check_file_path_exists, create_compressed_archive, delete_directory, delete_file,
    ensure_file_directory_path_exists, extract_compressed_archive, search_file_in_directory,
};
use super::structs::{
    PGCliError, PGTools, PostgresCredentials, Result, S3Credentials, SqlFileFormat,
};
use clap::Args;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

const MAX_RETRIES: u8 = 3;

#[derive(Args, Debug, PartialEq)]
pub struct RestoreArgs {
    /// database to restore
    database: String,
    /// input location
    /// chose between s3 or a file path, defaults to s3;
    /// when s3 is chosen, the input location is in the bucket specified by the S3_* environment variables with the name of postgresql-backup-<timestamp>.tar.xz;
    /// when a file path is chosen, the input location is the file path specified, but the extension is .tar.xz
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
    /// overwrite the database if it exists
    #[arg(long, env)]
    overwrite: bool,
}

pub async fn restore_db(
    data: &RestoreArgs,
    credentials: &HashMap<String, PostgresCredentials>,
    pg_main_hostname: &String,
    s3_credentials: &S3Credentials,
    pg_tools: &PGTools,
) -> Result<()> {
    // check if the input location exists and is a local file or a remote one (start with s3://)
    if !check_file_path_exists(&data.input_location)
        && !data
            .input_location
            .to_str()
            .unwrap_or("")
            .starts_with("s3://")
    {
        return Err(PGCliError::Other(
            "Input location does not exist".to_string(),
        ));
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

    // get the list of databases
    let databases = list_databases(&client, &None, false, &None).await?;

    // Convert databases to a Vec of Strings
    let database_names: Vec<&str> = databases.iter().map(|db| db.as_ref()).collect();

    // check if the database exists
    if database_names.contains(&data.database.as_str()) && !data.overwrite {
        return Err(PGCliError::Other(
            "Database already exists, use --overwrite to overwrite".to_string(),
        ));
    }

    // if the input location is from s3, download the file to a temporary location
    let file_location = if data
        .input_location
        .to_str()
        .unwrap_or("")
        .starts_with("s3://")
    {
        info!("Restoring from S3");
        s3_credentials.validate_s3_credentials()?;
        // strip s3:// from the input location
        let input_location = data.input_location.to_str().unwrap().replace("s3://", "");
        // if the input location contains the bucket name, strip it
        let input_location = if let Some(ref bucket) = s3_credentials.s3_bucket {
            input_location.replace(&format!("{}/", bucket), "")
        } else {
            input_location
        };
        // if the input location contains the prefix, strip it
        let input_location = if let Some(ref prefix) = s3_credentials.s3_prefix {
            input_location.replace(&format!("{}/", prefix), "")
        } else {
            input_location
        };
        // get the last part of the input location as the file name
        let temp_file_name = PathBuf::from(input_location.split('/').last().unwrap());
        let temp_dir = PathBuf::from(r"/tmp/psql_backup/");
        let temp_file = temp_dir.join(&temp_file_name);

        debug!("Downloading file to: {:?}", temp_file);
        // Attempt to download the file with retry logic
        let mut attempt = 0;

        loop {
            attempt += 1;
            match s3_credentials
                .download_file(&input_location, &temp_file)
                .await
            {
                Ok(_) => {
                    info!("File downloaded successfully!");
                    break;
                }
                Err(e) if attempt < MAX_RETRIES => {
                    warn!("Attempt {} failed: {}. Retrying...", attempt, e);
                    sleep(Duration::from_secs(5)).await;
                }
                Err(e) => {
                    error!("Failed to download file: {}. Retrying...", e);
                    return Err(e);
                }
            }
        }
        temp_file
    }
    // if the input location is a local file, use it as is
    else {
        data.input_location.clone()
    };

    debug!("Restoring from file: {:?}", file_location);

    let mut is_temp_file = false;

    // if the files are compressed, extract them
    let restore_file = if file_location.to_str().unwrap().ends_with(".tar.xz") {
        is_temp_file = true;
        let temp_folder_name = file_location
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .replace(".tar", "");
        let temp_folder = PathBuf::from(r"/tmp/psql_backup/").join(temp_folder_name);
        ensure_file_directory_path_exists(&temp_folder).await?;
        extract_compressed_archive(&file_location, &temp_folder).await?;
        // serach for a file in the extracted folder
        // with the name of the database and extension of the format
        let file_name = format!("{}.{}", data.database, data.format);
        search_file_in_directory(&temp_folder, &file_name)
            .await?
            .ok_or(PGCliError::Other(
                "No file found in the extracted folder".to_string(),
            ))?
    } else {
        file_location
    };

    info!("Restoring database: {}", data.database);

    Ok(())
}
