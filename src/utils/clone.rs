use std::collections::HashMap;

use super::db::database::{
    change_owner_of_objects_in_db, change_owner_of_tables_in_db, check_database_exists, create_db,
    delete_db, kill_connections_to_db,
};
use super::db::general::{db_dump, db_restore, init_pgpass, postgres_connect};
use super::db::user::{check_user_exists, create_user};
use crate::utils::misc::{delete_file, generate_random_string};
use crate::utils::structs::{PGTools, PostgresCredentials};
use clap::Args;
use log::{debug, error, info};

use super::structs::PGCliError;

#[derive(Args, Debug, PartialEq)]
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
    #[arg(short = 'o', long, default_value = "postgres")]
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
    /// Optional new host
    #[arg(long, env)]
    pg_hostname2: Option<String>,
    /// Optional new port
    #[arg(long, env, default_value = "5432")]
    pg_port2: Option<u16>,
    /// Optional user for new host
    #[arg(long, env)]
    pg_superuser2: Option<String>,
    /// Optional password for new host
    #[arg(long, env, hide_env_values = true)]
    pg_password2: Option<String>,
}

pub async fn clone_db(
    data: &CloneArgs,
    credentials: &mut HashMap<String, PostgresCredentials>,
    pg_main_hostname: &String,
    pg_tools: &PGTools,
) -> Result<(), PGCliError> {
    info!(
        "Cloning database {} to {}",
        data.database, data.new_database
    );

    let source_credentials = credentials.get(pg_main_hostname).unwrap().clone();
    let mut destination_credentials = source_credentials.clone();

    let mut all_credentials = credentials
        .values()
        .cloned()
        .collect::<Vec<PostgresCredentials>>();

    if let Some(pg_hostname2) = &data.pg_hostname2 {
        // check if the new host is in the credentials
        if !credentials.contains_key(pg_hostname2) {
            debug!("New host not found in pgpass credentials");
            // if any other value is not provided, give error
            if data.pg_port2.is_none() {
                error!("New port not provided");
                return Err(PGCliError::Other("New port not provided".to_string()));
            }
            if data.pg_superuser2.is_none() {
                error!("New superuser not provided");
                return Err(PGCliError::Other("New superuser not provided".to_string()));
            }
            if data.pg_password2.is_none() {
                error!("New password not provided");
                return Err(PGCliError::Other("New password not provided".to_string()));
            }

            let new_credentials = PostgresCredentials::new(
                pg_hostname2.to_string(),
                data.pg_superuser2.clone().unwrap(),
                data.pg_password2.clone().unwrap(),
                data.pg_port2.unwrap(),
            );
            all_credentials.push(new_credentials.clone());
            credentials.insert(pg_hostname2.to_string(), new_credentials);
        }
        destination_credentials = credentials.get(pg_hostname2).unwrap().clone();
        info!(
            "Cloning from {} to {}",
            &source_credentials.pg_hostname, &destination_credentials.pg_hostname
        );
    }

    init_pgpass(&all_credentials).await?;

    let source_client = postgres_connect(&source_credentials, None).await?;
    if !check_database_exists(&source_client, &data.database).await? {
        error!("Database {} does not exist", &data.database);
        return Err(PGCliError::Other(format!(
            "Database {} does not exist",
            &data.database
        )));
    }

    let dump_file = format!(
        "/tmp/psql_backup/{}-{}.bsql",
        data.new_database,
        generate_random_string(6, true)
    );

    match db_dump(
        source_credentials.clone(),
        &data.database,
        &dump_file,
        pg_tools,
    )
    .await
    {
        Ok(_) => info!("Database dumped successfully"),
        Err(e) => {
            error!("Failed to dump database: {}", e);
            return Err(e);
        }
    }

    let destination_client = postgres_connect(&destination_credentials, None).await?;

    // check if data.new_owner exists
    let new_owner_exists = check_user_exists(&destination_client, &data.new_owner).await?;

    if !new_owner_exists && data.create_owner {
        info!("Creating new owner {}", &data.new_owner);
        match create_user(&destination_client, &data.new_owner, &data.new_password).await {
            Ok(_) => info!("New owner created successfully"),
            Err(e) => {
                error!("Failed to create new owner: {}", e);
                return Err(PGCliError::from(e));
            }
        }
    } else if !new_owner_exists && !data.create_owner {
        error!("New owner does not exist and create_owner is not set");
        return Err(PGCliError::Other(
            "New owner does not exist and create_owner is not set".to_string(),
        ));
    }

    // check if data.new_database exists
    let new_database_exists =
        check_database_exists(&destination_client, &data.new_database).await?;

    if new_database_exists && !data.overwrite {
        error!("New database already exists and overwrite is not set");
        return Err(PGCliError::Other(
            "New database already exists and overwrite is not set".to_string(),
        ));
    } else if new_database_exists && data.overwrite {
        info!("Dropping database {}", &data.new_database);
        info!("Killing connections to database {}", &data.new_database);
        match kill_connections_to_db(&destination_client, &data.new_database).await {
            Ok(_) => info!("Connections killed successfully"),
            Err(e) => {
                error!("Failed to kill connections: {}", e);
                return Err(PGCliError::from(e));
            }
        }
        match delete_db(&destination_client, &data.new_database).await {
            Ok(_) => info!("Database dropped successfully"),
            Err(e) => {
                error!("Failed to drop database: {}", e);
                return Err(PGCliError::from(e));
            }
        }
    }

    info!("Creating database {}", &data.new_database);
    match create_db(&destination_client, &data.new_database, &data.new_owner).await {
        Ok(_) => info!("Database created successfully"),
        Err(e) => {
            error!("Failed to create database: {}", e);
            return Err(PGCliError::from(e));
        }
    }

    info!("Restoring database {}", &data.new_database);
    match db_restore(
        destination_credentials.clone(),
        &data.new_database,
        &dump_file,
        pg_tools,
    )
    .await
    {
        Ok(_) => info!("Database restored successfully"),
        Err(e) => {
            error!("Failed to restore database: {}", e);
            return Err(e);
        }
    }

    info!(
        "Changing owner of tables in database {}",
        &data.new_database
    );
    match change_owner_of_tables_in_db(
        &destination_credentials,
        &data.new_database,
        &data.new_owner,
    )
    .await
    {
        Ok(_) => info!("Owner of tables changed successfully"),
        Err(e) => {
            error!("Failed to change owner of tables: {}", e);
            return Err(e);
        }
    }

    info!(
        "Changing owner of objects in database {}",
        &data.new_database
    );
    match change_owner_of_objects_in_db(
        &destination_credentials,
        &data.new_database,
        &data.new_owner,
    )
    .await
    {
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
