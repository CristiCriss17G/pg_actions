use super::db::database::{
    change_owner_of_objects_in_db, check_database_exists, create_db, delete_db,
    kill_connections_to_db,
};
use super::db::general::{db_dump, db_restore, init_pgpass, postgres_connect};
use super::db::user::{check_user_exists, create_user};
use crate::utils::misc::{delete_file, generate_random_string};
use crate::utils::structs::PostgresCredentials;
use clap::Args;
use log::{error, info};

use super::structs::PGCliError;

#[derive(Args)]
pub struct CloneArgs {
    /// database to clone
    #[arg(short, long)]
    database: String,
    /// new destination database
    #[arg(short, long)]
    new_database: String,
    /// overwrite new database if it exists
    #[arg(long)]
    overwrite: bool,
    /// owner user for db
    #[arg(short = 'o', long)]
    new_owner: String,
    /// create the new owner user
    #[arg(short, long)]
    create_owner: bool,
    /// new password for db
    /// provide if the user does not exist
    #[arg(short = 's', long)]
    new_password: Option<String>,
    /// keep the dump file
    /// defaults to false
    #[arg(short, long, env)]
    keep_dump: bool,
}

pub async fn clone_db(
    data: &CloneArgs,
    credentials: &PostgresCredentials,
) -> Result<(), PGCliError> {
    info!(
        "Cloning database {} to {}",
        data.database, data.new_database
    );

    init_pgpass(credentials).await?;

    let client = postgres_connect(credentials, None).await?;
    if !check_database_exists(&client, &data.database).await? {
        error!("Database {} does not exist", &data.database);
        return Err(PGCliError::Other(format!(
            "Database {} does not exist",
            &data.database
        )));
    }

    let dump_file = format!(
        "/tmp/psql_backup/{}-{}.bsql",
        data.new_database,
        generate_random_string(6)
    );

    match db_dump(credentials.clone(), &data.database, &dump_file).await {
        Ok(_) => info!("Database dumped successfully"),
        Err(e) => {
            error!("Failed to dump database: {}", e);
            return Err(e);
        }
    }

    // check if data.new_owner exists
    let new_owner_exists = check_user_exists(&client, &data.new_owner).await?;

    if !new_owner_exists && data.create_owner {
        if let Some(password) = &data.new_password {
            info!("Creating new owner {}", &data.new_owner);
            match create_user(&client, &data.new_owner, &password).await {
                Ok(_) => info!("New owner created successfully"),
                Err(e) => {
                    error!("Failed to create new owner: {}", e);
                    return Err(PGCliError::from(e));
                }
            }
        } else {
            error!("New owner does not exist and no password provided");
            return Err(PGCliError::Other(
                "New owner does not exist and no password provided".to_string(),
            ));
        }
    } else if !new_owner_exists && !data.create_owner {
        error!("New owner does not exist and create_owner is not set");
        return Err(PGCliError::Other(
            "New owner does not exist and create_owner is not set".to_string(),
        ));
    }

    // check if data.new_database exists
    let new_database_exists = check_database_exists(&client, &data.new_database).await?;

    if new_database_exists && !data.overwrite {
        error!("New database already exists and overwrite is not set");
        return Err(PGCliError::Other(
            "New database already exists and overwrite is not set".to_string(),
        ));
    } else if new_database_exists && data.overwrite {
        info!("Dropping database {}", &data.new_database);
        info!("Killing connections to database {}", &data.new_database);
        match kill_connections_to_db(&client, &data.new_database).await {
            Ok(_) => info!("Connections killed successfully"),
            Err(e) => {
                error!("Failed to kill connections: {}", e);
                return Err(PGCliError::from(e));
            }
        }
        match delete_db(&client, &data.new_database).await {
            Ok(_) => info!("Database dropped successfully"),
            Err(e) => {
                error!("Failed to drop database: {}", e);
                return Err(PGCliError::from(e));
            }
        }
    }

    info!("Creating database {}", &data.new_database);
    match create_db(&client, &data.new_database, &data.new_owner).await {
        Ok(_) => info!("Database created successfully"),
        Err(e) => {
            error!("Failed to create database: {}", e);
            return Err(PGCliError::from(e));
        }
    }

    info!("Restoring database {}", &data.new_database);
    match db_restore(credentials.clone(), &data.new_database, &dump_file).await {
        Ok(_) => info!("Database restored successfully"),
        Err(e) => {
            error!("Failed to restore database: {}", e);
            return Err(e);
        }
    }

    info!(
        "Changing owner of objects in database {}",
        &data.new_database
    );
    match change_owner_of_objects_in_db(credentials, &data.new_database, &data.new_owner).await {
        Ok(_) => info!("Owner of objects changed successfully"),
        Err(e) => {
            error!("Failed to change owner of objects: {}", e);
            return Err(e);
        }
    }

    if !data.keep_dump {
        info!("Deleting dump file {}", &dump_file);
        match delete_file(&dump_file).await {
            Ok(_) => info!("Dump file deleted successfully"),
            Err(e) => {
                error!("Failed to delete dump file: {}", e);
                return Err(PGCliError::from(e));
            }
        }
    } else {
        info!("Dump file kept at {}", &dump_file);
    }

    Ok(())
}
