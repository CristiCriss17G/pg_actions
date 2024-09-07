use super::db::general::postgres_connect;
use super::db::user::{
    alter_role, create_role, delete_role, grant_privileges, list_roles, revoke_privileges,
};
use super::structs::{
    OutputFormat, PostgresCredentials, Result, SortingOrder, UserDetails, UserPrivileges,
};
use clap::{Args, Subcommand};
use csv::Writer;
use log::{debug, info, trace};
use prettytable::{row, Table};
use serde_json;
use std::collections::HashMap;

#[derive(Args, Debug, PartialEq)]
pub struct UserArgs {
    /// action subcommand
    #[command(subcommand)]
    subcommand: UserSubCommands,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum UserSubCommands {
    /// List users
    List {
        /// sort by username
        #[arg(long, value_enum)]
        sort: Option<SortingOrder>,
        ///quite mode
        #[arg(short, long, value_enum, default_value = "table")]
        output: OutputFormat,
        /// extra details
        #[arg(short, long)]
        extra: bool,
        /// query string, fuzzy search
        #[arg(short, long)]
        query: Option<String>,
    },
    /// Create a user
    Create {
        /// username
        username: String,
        /// password
        #[arg(short = 's', long)]
        password: Option<String>,
        /// user as superuser
        #[arg(long)]
        superuser: bool,
        /// createdb for user
        #[arg(long)]
        createdb: bool,
        /// role with no login
        /// defaults to false
        #[arg(short, long)]
        no_login: bool,
    },
    /// Delete a user
    Delete {
        /// username
        username: String,
    },
    /// Update a user
    Update {
        /// username
        username: String,
        /// password for user
        #[arg(short = 's', long)]
        password: Option<String>,
        /// user as superuser
        #[arg(long)]
        superuser: Option<bool>,
        /// createdb for user
        #[arg(long)]
        createdb: Option<bool>,
        /// role with no login
        #[arg(short, long)]
        no_login: Option<bool>,
    },
    /// Grant privileges to a user
    Grant {
        /// username
        username: String,
        /// database
        #[arg(short, long)]
        database: String,
        /// schema
        #[arg(long, default_value = "public")]
        schema: String,
        /// privileges
        /// full or read-only or read-update-only
        #[arg(short = 'g', long, value_enum)]
        privileges: UserPrivileges,
    },
    /// Revoke privileges from a user
    Revoke {
        /// username
        username: String,
        /// database
        #[arg(short, long)]
        database: String,
        /// schema
        #[arg(long, default_value = "public")]
        schema: String,
        /// privileges
        #[arg(short = 'g', long, value_enum)]
        privileges: UserPrivileges,
    },
}

pub async fn user(
    data: &UserArgs,
    credentials: &HashMap<String, PostgresCredentials>,
    pg_main_hostname: &String,
) -> Result<()> {
    let main_credentials = credentials.get(pg_main_hostname).unwrap();
    let client = postgres_connect(main_credentials, None).await?;
    match &data.subcommand {
        UserSubCommands::List {
            sort,
            output,
            extra,
            query,
        } => {
            debug!("List users, sort: {:?}, output: {:?}", sort, output);
            user_list(&client, sort, output, *extra, query).await?;
        }
        UserSubCommands::Create {
            username,
            password,
            superuser,
            createdb,
            no_login,
        } => {
            info!(
                "Create user: username: {}, superuser: {}, createdb: {}, no_login: {}",
                username, superuser, createdb, no_login
            );
            debug!(
                "Create user: username: {}, password: {:?}, superuser: {}, createdb: {}, no_login: {}",
                username, password, superuser, createdb, no_login
            );
            create_role(
                &client, username, password, *superuser, *createdb, *no_login,
            )
            .await?;
            info!("User created successfully")
        }
        UserSubCommands::Delete { username } => {
            info!("Delete user: username: {}", username);
            delete_role(&client, username).await?;
            info!("User deleted successfully")
        }
        UserSubCommands::Update {
            username,
            password,
            superuser,
            createdb,
            no_login,
        } => {
            info!(
                "Update user: username: {}, superuser: {:?}, createdb: {:?}, no_login: {:?}",
                username, superuser, createdb, no_login
            );
            debug!("Update user: username: {}, password: {:?}, superuser: {:?}, createdb: {:?}, no_login: {:?}", username, password, superuser, createdb, no_login);
            alter_role(&client, username, password, superuser, createdb, no_login).await?;
            info!("User updated successfully")
        }
        UserSubCommands::Grant {
            username,
            database,
            schema,
            privileges,
        } => {
            info!(
                "Grant privileges: username: {}, database: {}, schema: {}, privileges: {:?}",
                username, database, schema, privileges
            );
            debug!(
                "Grant privileges: username: {}, database: {}, schema: {}, privileges: {:?}",
                username, database, schema, privileges
            );
            grant_privileges(
                &client,
                main_credentials,
                username,
                database,
                schema,
                privileges,
            )
            .await?;
            info!("Privileges granted successfully")
        }
        UserSubCommands::Revoke {
            username,
            database,
            schema,
            privileges,
        } => {
            info!(
                "Revoke privileges: username: {}, database: {}, schema: {}, privileges: {:?}",
                username, database, schema, privileges
            );
            debug!(
                "Revoke privileges: username: {}, database: {}, schema: {}, privileges: {:?}",
                username, database, schema, privileges
            );
            revoke_privileges(
                &client,
                main_credentials,
                username,
                database,
                schema,
                privileges,
            )
            .await?;
            info!("Privileges revoked successfully")
        }
    }

    Ok(())
}

async fn user_list(
    client: &deadpool_postgres::Client,
    sort: &Option<SortingOrder>,
    output_format: &OutputFormat,
    extra: bool,
    query: &Option<String>,
) -> Result<()> {
    let roles = list_roles(client, sort, extra, query).await?;
    trace!("Roles: {:?}", roles);
    match output_format {
        OutputFormat::Simple => {
            for user in roles {
                match user {
                    UserDetails::Extra {
                        username,
                        createdb,
                        superuser,
                        login,
                        oid,
                    } => {
                        println!(
                            "{}\t{}\t{}\t{}\t{}",
                            username, createdb, superuser, login, oid
                        );
                    }
                    UserDetails::Username(username) => {
                        println!("{}", username);
                    }
                }
            }
        }
        OutputFormat::Table => {
            let mut table = Table::new();
            match extra {
                true => table.add_row(row![b =>
                    "IDX",
                    "Username",
                    "Createdb",
                    "Superuser",
                    "Login",
                    "OID"
                ]),
                false => table.add_row(row![b => "IDX", "Username"]),
            };
            for (i, user) in roles.iter().enumerate() {
                match user {
                    UserDetails::Extra {
                        username,
                        createdb,
                        superuser,
                        login,
                        oid,
                    } => {
                        table.add_row(row![i, username, createdb, superuser, login, oid]);
                    }
                    UserDetails::Username(username) => {
                        table.add_row(row![i, username]);
                    }
                }
            }
            table.printstd();
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&roles)?;
            println!("{}", json);
        }
        OutputFormat::JsonCompact => {
            let json = serde_json::to_string(&roles)?;
            println!("{}", json);
        }
        OutputFormat::JsonLines => {
            for user in roles {
                let json = serde_json::to_string(&user)?;
                println!("{}", json);
            }
        }
        OutputFormat::Csv => {
            let mut wtr = Writer::from_writer(vec![]);
            match extra {
                true => {
                    wtr.write_record(["IDX", "Username", "Createdb", "Superuser", "Login", "OID"])?
                }
                false => wtr.write_record(["IDX", "Username"])?,
            }
            for (i, user) in roles.iter().enumerate() {
                match user {
                    UserDetails::Extra {
                        username,
                        createdb,
                        superuser,
                        login,
                        oid,
                    } => {
                        wtr.write_record([
                            &i.to_string(),
                            &username.to_string(),
                            &createdb.to_string(),
                            &superuser.to_string(),
                            &login.to_string(),
                            &oid.to_string(),
                        ])?;
                    }
                    UserDetails::Username(username) => {
                        wtr.write_record([&i.to_string(), &username.to_string()])?;
                    }
                }
            }
            let data = String::from_utf8(wtr.into_inner().map_err(Box::new)?)?;
            println!("{}", data);
        }
    }
    Ok(())
}
