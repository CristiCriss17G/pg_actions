use std::collections::HashMap;

use super::db::database::{
    change_whole_owner_of_db, create_db, delete_db, kill_connections_to_db, list_databases,
};
use super::db::general::postgres_connect;
use super::structs::{
    DatabaseDetails, OutputFormat, PGCliError, PostgresCredentials, SortingOrder,
};
use clap::{Args, Subcommand};
use csv::Writer;
use log::{debug, error, info, trace};
use prettytable::{row, Table};
use serde_json;

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
        #[arg(short, long, value_enum, default_value = "table")]
        output: OutputFormat,
        /// extra details
        #[arg(short, long)]
        extra: bool,
        /// query string, fuzzy search
        #[arg(short, long)]
        query: Option<String>,
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
    let client = postgres_connect(main_credentials, None).await?;
    match &data.subcommand {
        Some(DatabaseSubCommands::List {
            sort,
            output,
            extra,
            query,
        }) => {
            debug!("List databases");
            database_list(&client, sort, output, *extra, query).await?;
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

async fn database_list(
    client: &tokio_postgres::Client,
    sort: &Option<SortingOrder>,
    output_format: &OutputFormat,
    extra: bool,
    query: &Option<String>,
) -> Result<(), PGCliError> {
    let databases = list_databases(client, sort, extra, query).await?;
    trace!("Databases: {:?}", databases);
    if databases.is_empty() {
        info!("No databases found");
    } else {
        match output_format {
            OutputFormat::Simple => {
                for db in databases {
                    match db {
                        DatabaseDetails::Extra {
                            name,
                            oid,
                            size_pretty,
                            owner,
                        } => {
                            println!("{}\t{}\t{}\t{}", name, owner, size_pretty, oid);
                        }
                        DatabaseDetails::Name(name) => {
                            println!("{}", name);
                        }
                    }
                }
            }
            OutputFormat::Table => {
                let mut table = Table::new();
                match extra {
                    true => {
                        table.add_row(row![b =>
                            "IDX",
                            "DB name",
                            "Owner",
                            "Size",
                            "OID"
                        ]);
                        for (i, db) in databases.iter().enumerate() {
                            match db {
                                DatabaseDetails::Extra {
                                    name,
                                    oid,
                                    size_pretty,
                                    owner,
                                } => {
                                    table.add_row(row![i, name, owner, size_pretty, oid]);
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
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&databases)?;
                println!("{}", json);
            }
            OutputFormat::JsonCompact => {
                let json = serde_json::to_string(&databases)?;
                println!("{}", json);
            }
            OutputFormat::JsonLines => {
                for db in databases {
                    let json = serde_json::to_string(&db)?;
                    println!("{}", json);
                }
            }
            OutputFormat::Csv => {
                let mut wtr = Writer::from_writer(vec![]);
                if extra {
                    wtr.write_record(["IDX", "DB name", "Owner", "Size", "OID"])?;
                    for (i, db) in databases.iter().enumerate() {
                        match db {
                            DatabaseDetails::Extra {
                                name,
                                oid,
                                size_pretty,
                                owner,
                            } => {
                                wtr.write_record(&[
                                    i.to_string(),
                                    name.to_string(),
                                    owner.to_string(),
                                    size_pretty.to_string(),
                                    oid.to_string(),
                                ])?;
                            }
                            DatabaseDetails::Name(name) => {
                                wtr.write_record(&[i.to_string(), name.to_string()])?;
                            }
                        }
                    }
                } else {
                    wtr.write_record(["IDX", "Name"])?;
                    for (i, db) in databases.iter().enumerate() {
                        match db {
                            DatabaseDetails::Extra { name, .. } => {
                                wtr.write_record(&[i.to_string(), name.to_string()])?;
                            }
                            DatabaseDetails::Name(name) => {
                                wtr.write_record(&[i.to_string(), name.to_string()])?;
                            }
                        }
                    }
                }
                let data = String::from_utf8(wtr.into_inner()?)?;
                println!("{}", data);
            }
        }
    }
    Ok(())
}
