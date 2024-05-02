use std::collections::HashMap;

use super::db::general::postgres_connect;
use super::db::user::{
    alter_role, create_role, delete_role, grant_privileges, list_roles, revoke_privileges,
};
use super::structs::{PGCliError, PostgresCredentials, SortingOrder, UserDetails, UserPrivileges};
use clap::{Args, Subcommand};
use log::{debug, error, info, trace};
use prettytable::{row, Table};

#[derive(Args, Debug, PartialEq)]
pub struct UserArgs {
    /// action subcommand
    #[command(subcommand)]
    subcommand: Option<UserSubCommands>,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum UserSubCommands {
    /// List users
    List {
        /// sort by username
        #[arg(long, value_enum)]
        sort: Option<SortingOrder>,
        ///quite mode
        #[arg(short, long)]
        quiet: bool,
        /// extra details
        #[arg(short, long)]
        extra: bool,
    },
    /// Create a user
    Create {
        /// username
        username: String,
        /// password
        #[arg(short = 's', long)]
        password: String,
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
        #[arg(long)]
        schema: String,
        /// privileges
        /// full or read-only
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
        #[arg(long)]
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
) -> Result<(), PGCliError> {
    let main_credentials = credentials.get(pg_main_hostname).unwrap();
    let client = postgres_connect(main_credentials, None).await?;
    match &data.subcommand {
        Some(UserSubCommands::List { sort, quiet, extra }) => {
            debug!("List users, sort: {:?}, quiet: {:?}", sort, quiet);
            let roles = list_roles(&client, sort, *extra).await?;
            trace!("Roles: {:?}", roles);
            match quiet {
                true => {
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
                false => {
                    let mut table = Table::new();
                    // apend rows with the index and the role name
                    match *extra {
                        true => {
                            table.add_row(row![b =>
                                "IDX",
                                "Username",
                                "Createdb",
                                "Superuser",
                                "Login",
                                "OID"
                            ]);
                            for (i, user) in roles.iter().enumerate() {
                                match user {
                                    UserDetails::Extra {
                                        username,
                                        createdb,
                                        superuser,
                                        login,
                                        oid,
                                    } => {
                                        table.add_row(row![
                                            i, username, createdb, superuser, login, oid
                                        ]);
                                    }
                                    UserDetails::Username(username) => {
                                        table.add_row(row![i, username]);
                                    }
                                }
                            }
                        }
                        false => {
                            table.add_row(row![b => "IDX", "Username"]);
                            for (i, user) in roles.iter().enumerate() {
                                match user {
                                    UserDetails::Extra { username, .. } => {
                                        table.add_row(row![i, username]);
                                    }
                                    UserDetails::Username(username) => {
                                        table.add_row(row![i, username]);
                                    }
                                }
                            }
                        }
                    }
                    table.printstd();
                }
            }
        }
        Some(UserSubCommands::Create {
            username,
            password,
            superuser,
            createdb,
            no_login,
        }) => {
            info!(
                "Create user: username: {}, superuser: {}, createdb: {}, no_login: {}",
                username, superuser, createdb, no_login
            );
            debug!(
                "Create user: username: {}, password: {}, superuser: {}, createdb: {}, no_login: {}",
                username, password, superuser, createdb, no_login
            );
            create_role(
                &client, username, password, *superuser, *createdb, *no_login,
            )
            .await?;
            info!("User created successfully")
        }
        Some(UserSubCommands::Delete { username }) => {
            info!("Delete user: username: {}", username);
            delete_role(&client, username).await?;
            info!("User deleted successfully")
        }
        Some(UserSubCommands::Update {
            username,
            password,
            superuser,
            createdb,
            no_login,
        }) => {
            info!(
                "Update user: username: {}, superuser: {:?}, createdb: {:?}, no_login: {:?}",
                username, superuser, createdb, no_login
            );
            debug!("Update user: username: {}, password: {:?}, superuser: {:?}, createdb: {:?}, no_login: {:?}", username, password, superuser, createdb, no_login);
            alter_role(&client, username, password, superuser, createdb, no_login).await?;
            info!("User updated successfully")
        }
        None => {
            error!("No subcommand provided");
        }
        Some(UserSubCommands::Grant {
            username,
            database,
            schema,
            privileges,
        }) => {
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
        Some(UserSubCommands::Revoke {
            username,
            database,
            schema,
            privileges,
        }) => {
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
