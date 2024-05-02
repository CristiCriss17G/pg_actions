use std::collections::HashMap;

use super::db::database::{
    change_whole_owner_of_db, create_db, delete_db, kill_connections_to_db, list_databases,
};
use super::db::general::postgres_connect;
use super::structs::{DatabaseDetails, PGCliError, PostgresCredentials, SortingOrder};
use clap::{Args, Subcommand};
use log::{debug, error, info, trace};
use prettytable::{row, Table};

#[derive(Args, Debug, PartialEq)]
pub struct DatabaseArgs {
    /// action subcommand
    #[command(subcommand)]
    subcommand: Option<DatabaseSubCommands>,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum DatabaseSubCommands {
    /// List databases
    List {
        /// sort by database name
        #[arg(long, value_enum)]
        sort: Option<SortingOrder>,
        /// quite mode
        #[arg(short, long)]
        quiet: bool,
        /// extra details
        #[arg(short, long)]
        extra: bool,
    },
    /// Create a database
    Create {
        /// database name
        database: String,
        /// owner user for db
        #[arg(short = 'o', long)]
        owner: Option<String>,
    },
    /// Delete a database
    Delete {
        /// database name
        database: String,
    },
    /// Update a database
    Update {
        /// database name
        database: String,
        /// new owner user for db
        #[arg(short = 'o', long)]
        new_owner: Option<String>,
    },
}

pub async fn database(
    data: &DatabaseArgs,
    credentials: &HashMap<String, PostgresCredentials>,
    pg_main_hostname: &String,
) -> Result<(), PGCliError> {
    let main_credentials = credentials.get(pg_main_hostname).unwrap();
    let client = postgres_connect(&main_credentials, None).await?;
    match &data.subcommand {
        Some(DatabaseSubCommands::List { sort, quiet, extra }) => {
            debug!("List databases");
            let databases = list_databases(&client, sort, *extra).await?;
            trace!("Databases: {:?}", databases);
            if databases.is_empty() {
                info!("No databases found");
            } else {
                match quiet {
                    true => {
                        for db in databases {
                            match db {
                                DatabaseDetails::Extra { name, oid, owner } => {
                                    println!("{}\t{}\t{}", name, owner, oid);
                                }
                                DatabaseDetails::Name(name) => {
                                    println!("{}", name);
                                }
                            }
                        }
                    }
                    false => {
                        let mut table = Table::new();
                        // apend rows with the index and the role name
                        match *extra {
                            true => {
                                table.add_row(row![b =>
                                    "IDX",
                                    "DB name",
                                    "Owner",
                                    "OID"
                                ]);
                                for (i, db) in databases.iter().enumerate() {
                                    match db {
                                        DatabaseDetails::Extra { name, oid, owner } => {
                                            table.add_row(row![i, name, owner, oid]);
                                        }
                                        DatabaseDetails::Name(name) => {
                                            table.add_row(row![i, name]);
                                        }
                                    }
                                }
                            }
                            false => {
                                table.add_row(row![b => "IDX", "Name"]);
                                for (i, db) in databases.iter().enumerate() {
                                    match db {
                                        DatabaseDetails::Extra { name, .. } => {
                                            table.add_row(row![i, name]);
                                        }
                                        DatabaseDetails::Name(name) => {
                                            table.add_row(row![i, name]);
                                        }
                                    }
                                }
                            }
                        }
                        table.printstd();
                    }
                }
            }
        }
        Some(DatabaseSubCommands::Create { database, owner }) => {
            debug!("Create database {}", database);
            let owner = owner.clone();
            create_db(
                &client,
                database,
                &owner.unwrap_or(main_credentials.pg_superuser.clone()),
            )
            .await?;
            info!("Database {} created", database);
        }
        Some(DatabaseSubCommands::Delete { database }) => {
            debug!("Delete database {}", database);
            kill_connections_to_db(&client, database).await?;
            delete_db(&client, database).await?;
            info!("Database {} deleted", database);
        }
        Some(DatabaseSubCommands::Update {
            database,
            new_owner,
        }) => {
            debug!("Update database {}", database);
            let new_owner = new_owner.clone();
            change_whole_owner_of_db(
                &client,
                main_credentials,
                database,
                &new_owner.unwrap_or(main_credentials.pg_superuser.clone()),
            )
            .await?;
            info!("Database {} updated", database);
        }
        None => {
            error!("No subcommand provided");
        }
    }
    Ok(())
}
