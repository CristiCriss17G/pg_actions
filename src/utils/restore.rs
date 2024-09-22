use super::db::database::{
    change_owner_of_objects_in_db, change_owner_of_tables_in_db, check_database_exists, create_db,
    delete_db, get_database_owner, kill_connections_to_db,
};
use super::db::general::{db_restore, init_pgpass, postgres_connect};
use super::misc::{
    check_file_path_exists, delete_directory, delete_file, ensure_file_directory_path_exists,
    extract_compressed_archive, search_file_in_directory,
};
use super::structs::{
    PGCliError, PGTools, PostgresCredentials, Result, S3Credentials, SqlFileFormat,
};
use clap::Args;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::sleep;

const MAX_RETRIES: u8 = 3;

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

    let database_exists = check_database_exists(&client, &data.database).await?;

    let database_owner = if let Some(ref owner) = &data.owner {
        owner.clone()
    } else if database_exists {
        get_database_owner(&client, &data.database).await?
    } else {
        main_credentials.pg_superuser.clone()
    };

    if database_exists && !data.overwrite {
        error!("Database already exists and overwrite is not set");
        return Err(PGCliError::Other(
            "Database already exists and overwrite is not set".to_string(),
        ));
    } else if database_exists && data.overwrite {
        info!("Dropping database {}", &data.database);
        info!("Killing connections to database {}", &data.database);
        match kill_connections_to_db(&client, &data.database).await {
            Ok(_) => info!("Connections killed successfully"),
            Err(e) => {
                error!("Failed to kill connections: {}", e);
                return Err(e);
            }
        }
        match delete_db(&client, &data.database).await {
            Ok(_) => info!("Database dropped successfully"),
            Err(e) => {
                error!("Failed to drop database: {}", e);
                return Err(e);
            }
        }
    }

    let mut temp_archive: Option<PathBuf> = None;
    let mut temp_folder_extracted: Option<PathBuf> = None;

    // if the input location is from s3, download the file to a temporary location
    let file_location =
        match get_file_location(&data.input_location, s3_credentials, &mut temp_archive).await {
            Ok(file) => file,
            Err(e) => {
                error!("Failed to get file location: {}", e);
                cleanup(data.keep_temp, &temp_archive, &temp_folder_extracted).await?;
                return Err(e);
            }
        };

    debug!("Restoring from file: {:?}", file_location);

    // if the files are compressed, extract them
    let restore_file = match get_restore_file_location(
        &file_location,
        &data.database,
        &data.format,
        &data.file_name,
        &mut temp_folder_extracted,
    )
    .await
    {
        Ok(file) => file,
        Err(e) => {
            error!("Failed to get restore file location: {}", e);
            cleanup(data.keep_temp, &temp_archive, &temp_folder_extracted).await?;
            return Err(e);
        }
    };

    info!("Creating database {}", &data.database);
    match create_db(&client, &data.database, &database_owner).await {
        Ok(_) => info!("Database created successfully"),
        Err(e) => {
            error!("Failed to create database: {}", e);
            return Err(e);
        }
    }

    info!("Restoring database {}", &data.database);
    match db_restore(
        main_credentials.clone(),
        &data.database,
        &restore_file,
        pg_tools,
        data.format,
    )
    .await
    {
        Ok(_) => info!("Database restored successfully"),
        Err(e) => {
            error!("Failed to restore database: {}", e);
            // cleanup
            cleanup(data.keep_temp, &temp_archive, &temp_folder_extracted).await?;
            return Err(e);
        }
    }

    info!("Changing owner of tables in database {}", &data.database);
    match change_owner_of_tables_in_db(main_credentials, &data.database, &database_owner).await {
        Ok(_) => info!("Owner of tables changed successfully"),
        Err(e) => {
            error!("Failed to change owner of tables: {}", e);
            return Err(e);
        }
    }

    info!("Changing owner of objects in database {}", &data.database);
    match change_owner_of_objects_in_db(main_credentials, &data.database, &database_owner).await {
        Ok(_) => info!("Owner of objects changed successfully"),
        Err(e) => {
            error!("Failed to change owner of objects: {}", e);
            return Err(e);
        }
    }

    cleanup(data.keep_temp, &temp_archive, &temp_folder_extracted).await?;

    Ok(())
}

async fn get_file_location<P: AsRef<Path>>(
    input_location: P,
    s3_credentials: &S3Credentials,
    temp_archive: &mut Option<PathBuf>,
) -> Result<PathBuf> {
    if input_location
        .as_ref()
        .to_str()
        .unwrap_or("")
        .starts_with("s3://")
    {
        info!("Restoring from S3");
        s3_credentials.validate_s3_credentials()?;
        // strip s3:// from the input location
        let input_location = input_location
            .as_ref()
            .to_str()
            .unwrap()
            .replace("s3://", "");
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
                    sleep(Duration::from_secs(3)).await;
                }
                Err(e) => {
                    error!("Failed to download file: {}. Retrying...", e);
                    return Err(e);
                }
            }
        }
        *temp_archive = Some(temp_file.clone());
        Ok(temp_file)
    }
    // if the input location is a local file, use it as is
    else {
        Ok(input_location.as_ref().to_path_buf())
    }
}

async fn get_restore_file_location<P: AsRef<Path>>(
    file_location: P,
    database: &str,
    format: &SqlFileFormat,
    file_name: &Option<String>,
    temp_folder_extracted: &mut Option<PathBuf>,
) -> Result<PathBuf> {
    if file_location
        .as_ref()
        .to_str()
        .unwrap()
        .ends_with(".tar.xz")
    {
        let temp_folder_name = file_location
            .as_ref()
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .replace(".tar", "");
        let temp_folder = PathBuf::from(r"/tmp/psql_backup/").join(temp_folder_name);
        ensure_file_directory_path_exists(&temp_folder).await?;
        extract_compressed_archive(file_location.as_ref(), &temp_folder).await?;
        // serach for a file in the extracted folder
        // with the name of the database and extension of the format
        let file_name = if let Some(file) = &file_name {
            format!("{}.{}", file, format.file_extension())
        } else {
            format!("{}.{}", database, format.file_extension())
        };
        *temp_folder_extracted = Some(temp_folder.clone());
        let file_location = search_file_in_directory(&temp_folder, &file_name)
            .await?
            .ok_or_else(|| {
                PGCliError::Other(format!(
                    "Database file `{}` not found in the extracted folder",
                    file_name
                ))
            })?;
        Ok(file_location)
    } else {
        Ok(file_location.as_ref().to_path_buf())
    }
}

async fn cleanup(
    keep: bool,
    temp_archive: &Option<PathBuf>,
    temp_folder_extracted: &Option<PathBuf>,
) -> Result<()> {
    if !keep {
        // cleanup
        info!("Cleaning up");
        if let Some(temp_archive) = temp_archive {
            debug!("Deleting temporary archive: {:?}", temp_archive);
            delete_file(&temp_archive).await?;
        }
        if let Some(temp_folder_extracted) = temp_folder_extracted {
            debug!("Deleting temporary folder: {:?}", temp_folder_extracted);
            delete_directory(&temp_folder_extracted).await?;
        }
    } else {
        info!("Temporary files kept at: /tmp/psql_backup/");
    }
    Ok(())
}
