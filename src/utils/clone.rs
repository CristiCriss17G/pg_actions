use crate::utils::db::{create_db, create_user, db_dump, delete_db, init_pgpass, postgres_connect};
use crate::utils::structs::PostgresCredentials;
use clap::Args;
use tokio_postgres::Error;

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
    #[arg(short = 'p', long)]
    new_password: Option<String>,
    /// keep the dump file
    /// defaults to false
    #[arg(short, long, env)]
    keep_dump: bool,
}

pub async fn clone_db(data: &CloneArgs, credentials: PostgresCredentials) -> Result<(), Error> {
    println!(
        "Cloning database {} to {}",
        data.database, data.new_database
    );

    init_pgpass(&credentials).await?;

    let dump_file = format!("/tmp/psql_backup/{}.bsql", data.new_database);
    match db_dump(&credentials, &data.database, &dump_file).await {
        Ok(_) => println!("Database dumped successfully"),
        Err(e) => {
            eprintln!("Failed to dump database: {}", e);
            std::process::exit(1);
        }
    }

    let client = postgres_connect(&credentials).await?;

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
            println!("Creating new owner {}", &data.new_owner);
            match create_user(&client, &data.new_owner, &password).await {
                Ok(_) => println!("New owner created successfully"),
                Err(e) => {
                    eprintln!("Failed to create new owner: {}", e);
                    std::process::exit(1);
                }
            }
        } else {
            eprintln!("New owner does not exist and no password provided");
            std::process::exit(1);
        }
    } else if !new_owner_exists && !data.create_owner {
        eprintln!("New owner does not exist and create_owner is not set");
        std::process::exit(1);
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
        eprintln!("New database already exists and overwrite is not set");
        std::process::exit(1);
    } else if new_database_exists && data.overwrite {
        println!("Dropping database {}", &data.new_database);
        match delete_db(&client, &data.new_database).await {
            Ok(_) => println!("Database dropped successfully"),
            Err(e) => {
                eprintln!("Failed to drop database: {}", e);
                std::process::exit(1);
            }
        }
    }

    println!("Creating database {}", &data.new_database);
    match create_db(&client, &data.new_database, &data.new_owner).await {
        Ok(_) => println!("Database created successfully"),
        Err(e) => {
            eprintln!("Failed to create database: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
