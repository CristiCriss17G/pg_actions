use crate::utils::db::{
    change_owner_of_objects_in_db, create_db, create_user, db_dump, db_restore, delete_db,
    init_pgpass, kill_connections_to_db, postgres_connect,
};
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

    let dump_file = format!("/tmp/psql_backup/{}.bsql", data.new_database);
    match db_dump(credentials, &data.database, &dump_file).await {
        Ok(_) => info!("Database dumped successfully"),
        Err(e) => {
            error!("Failed to dump database: {}", e);
            return Err(e); // Convert String error to tokio_postgres::Error
        }
    }

    let client = postgres_connect(credentials, None).await?;

    // check if data.new_owner exists
    let new_owner_exists = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_roles WHERE rolname=$1)",
            &[&data.new_owner],
        )
        .await?
        .get::<_, bool>(0);

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
    let new_database_exists = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname=$1)",
            &[&data.new_database],
        )
        .await?
        .get::<_, bool>(0);

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
    match db_restore(credentials, &data.new_database, &dump_file).await {
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

    Ok(())
}
