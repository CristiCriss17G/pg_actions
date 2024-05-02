use super::general::validate_pg_names;
use crate::utils::{
    db::general::postgres_connect,
    structs::{PGCliError, PostgresCredentials, SortingOrder, UserDetails, UserPrivileges},
};
use log::{debug, error, trace};
use tokio_postgres::Client;

pub async fn check_user_exists(client: &Client, user: &str) -> Result<bool, PGCliError> {
    let user_exists = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_roles WHERE rolname=$1)",
            &[&user],
        )
        .await?
        .get::<_, bool>(0);
    Ok(user_exists)
}

pub async fn list_roles(
    client: &Client,
    sort: &Option<SortingOrder>,
    extra: bool,
) -> Result<Vec<UserDetails>, PGCliError> {
    let ignored_roles= "'pg_checkpoint','pg_create_subscription','pg_database_owner','pg_execute_server_program',\
    'pg_monitor','pg_read_all_data','pg_read_all_settings','pg_read_all_stats','pg_read_server_files',\
    'pg_stat_scan_tables','pg_use_reserved_connections','pg_write_all_data','pg_write_server_files',\
    'pg_signal_backend'";
    let fields = match extra {
        true => "rolname, rolsuper, rolcreatedb, rolcanlogin, oid",
        false => "rolname",
    };
    let rows = match sort {
        Some(SortingOrder::Ascending) => client.query(&format!("SELECT {fields} FROM pg_roles WHERE rolname NOT IN ({ignored_roles}) ORDER BY rolname ASC"), &[]).await?,
        Some(SortingOrder::Descending) => client.query(&format!("SELECT {fields} FROM pg_roles WHERE rolname NOT IN ({ignored_roles}) ORDER BY rolname DESC"), &[]).await?,
        None => client.query(&format!("SELECT {fields} FROM pg_roles WHERE rolname NOT IN ({ignored_roles})"), &[]).await?,
    };

    let mut roles = Vec::new();
    for row in rows {
        match extra {
            true => {
                let username: String = row.get(0);
                let superuser: bool = row.get(1);
                let createdb: bool = row.get(2);
                let login: bool = row.get(3);
                let oid: u32 = row.get(4);
                roles.push(UserDetails::Extra {
                    username,
                    createdb,
                    superuser,
                    login,
                    oid,
                });
            }
            false => {
                let username: String = row.get(0);
                roles.push(UserDetails::Username(username));
            }
        }
    }
    Ok(roles)
}

pub async fn create_user(client: &Client, user: &str, password: &str) -> Result<u64, PGCliError> {
    trace!("Creating user {}", user);
    if !validate_pg_names(user) {
        error!("Invalid user name: {}", user);
        return Err(PGCliError::Other("Invalid user name".to_string()));
    }
    if !validate_pg_names(password) {
        error!("Invalid password");
        return Err(PGCliError::Other("Invalid password".to_string()));
    }

    create_role(&client, user, password, false, false, false).await
}

pub async fn create_role(
    client: &Client,
    role: &str,
    password: &str,
    superuser: bool,
    createdb: bool,
    no_login: bool,
) -> Result<u64, PGCliError> {
    trace!("Creating role {}", role);
    if !validate_pg_names(role) {
        error!("Invalid role name: {}", role);
        return Err(PGCliError::Other("Invalid role name".to_string()));
    }
    if !validate_pg_names(password) {
        error!("Invalid password");
        return Err(PGCliError::Other("Invalid password".to_string()));
    }

    // check if the role already exists
    if check_user_exists(&client, role).await? {
        error!("Role {} already exists", role);
        return Err(PGCliError::Other("Role already exists".to_string()));
    }

    trace!(
        "Creating role {}, superuser: {}, createdb: {}, no_login: {}",
        role,
        superuser,
        createdb,
        no_login
    );
    let mut statement = format!("CREATE ROLE \"{}\" WITH PASSWORD '{}'", role, password);
    if superuser {
        statement.push_str(" SUPERUSER");
    }
    if createdb {
        statement.push_str(" CREATEDB");
    }
    if !no_login {
        statement.push_str(" LOGIN");
    }

    match client.execute(&statement, &[]).await {
        Ok(r) => Ok(r),
        Err(e) => Err(PGCliError::from(e)),
    }
}

pub async fn delete_role(client: &Client, role: &str) -> Result<u64, PGCliError> {
    trace!("Deleting role {}", role);
    if !validate_pg_names(role) {
        error!("Invalid role name: {}", role);
        return Err(PGCliError::Other("Invalid role name".to_string()));
    }

    if !check_user_exists(&client, role).await? {
        error!("Role {} does not exist", role);
        return Err(PGCliError::Other("Role does not exist".to_string()));
    }

    let statement = format!("DROP ROLE \"{}\"", role);
    match client.execute(&statement, &[]).await {
        Ok(r) => Ok(r),
        Err(e) => Err(PGCliError::from(e)),
    }
}

pub async fn alter_role(
    client: &Client,
    role: &str,
    password: &Option<String>,
    superuser: &Option<bool>,
    createdb: &Option<bool>,
    no_login: &Option<bool>,
) -> Result<u64, PGCliError> {
    trace!("Altering role {}", role);
    if !validate_pg_names(role) {
        error!("Invalid role name: {}", role);
        return Err(PGCliError::Other("Invalid role name".to_string()));
    }

    if !check_user_exists(&client, role).await? {
        error!("Role {} does not exist", role);
        return Err(PGCliError::Other("Role does not exist".to_string()));
    }

    // If the password is not provided, we don't alter it
    let mut statement = format!("ALTER ROLE {}", role);
    if let Some(password) = password {
        if !validate_pg_names(&password) {
            error!("Invalid password");
            return Err(PGCliError::Other("Invalid password".to_string()));
        }
        statement.push_str(&format!(" WITH PASSWORD '{}'", password));
    }

    if let Some(superuser_flag) = superuser {
        if *superuser_flag {
            statement.push_str(" SUPERUSER");
        } else {
            statement.push_str(" NOSUPERUSER");
        }
    }

    if let Some(createdb_flag) = createdb {
        if *createdb_flag {
            statement.push_str(" CREATEDB");
        } else {
            statement.push_str(" NOCREATEDB");
        }
    }

    if let Some(no_login_flag) = no_login {
        if !*no_login_flag {
            statement.push_str(" LOGIN");
        } else {
            statement.push_str(" NOLOGIN");
        }
    }

    match client.execute(&statement, &[]).await {
        Ok(r) => Ok(r),
        Err(e) => Err(PGCliError::from(e)),
    }
}

pub async fn grant_privileges(
    client: &Client,
    credentials: &PostgresCredentials,
    role: &str,
    database: &str,
    schema: &str,
    privileges: &UserPrivileges,
) -> Result<u64, PGCliError> {
    trace!("Granting privileges to role {}", role);
    if !validate_pg_names(role) {
        error!("Invalid role name: {}", role);
        return Err(PGCliError::Other("Invalid role name".to_string()));
    }

    if !check_user_exists(&client, role).await? {
        error!("Role {} does not exist", role);
        return Err(PGCliError::Other("Role does not exist".to_string()));
    }
    let mut res: u64 = 0;

    match privileges {
        UserPrivileges::Full => {
            let statement = format!(
                "GRANT ALL PRIVILEGES ON DATABASE \"{}\" TO \"{}\"",
                database, role
            );

            match client.execute(&statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };

            let db_client = postgres_connect(&credentials, Some(database.to_string())).await?;

            let schema_statement = format!(
                "GRANT ALL PRIVILEGES ON SCHEMA \"{}\" TO \"{}\"",
                schema, role
            );
            let table_statement = format!(
                "GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA \"{}\" TO \"{}\"",
                schema, role
            );
            let sequence_statement = format!(
                "GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA \"{}\" TO \"{}\"",
                schema, role
            );

            match db_client.execute(&schema_statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };

            match db_client.execute(&table_statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };

            match db_client.execute(&sequence_statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };
        }
        UserPrivileges::ReadOnly => {
            let statement = format!("GRANT CONNECT ON DATABASE \"{}\" TO \"{}\"", database, role);

            match client.execute(&statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };

            let db_client = postgres_connect(&credentials, Some(database.to_string())).await?;

            let schema_statement = format!("GRANT USAGE ON SCHEMA \"{}\" TO \"{}\"", schema, role);
            let table_statement = format!(
                "GRANT SELECT ON ALL TABLES IN SCHEMA \"{}\" TO \"{}\"",
                schema, role
            );
            let sequence_statement = format!(
                "GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA \"{}\" TO \"{}\"",
                schema, role
            );

            let default_privileges_statement = format!(
                "ALTER DEFAULT PRIVILEGES IN SCHEMA \"{}\" GRANT SELECT ON TABLES TO \"{}\"",
                schema, role
            );

            match db_client.execute(&schema_statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };

            match db_client.execute(&table_statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };

            match db_client.execute(&sequence_statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };

            match db_client.execute(&default_privileges_statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };
        }
    }

    debug!("Privilege {} granted to role {}", privileges, role);

    Ok(res)
}

pub async fn revoke_privileges(
    client: &Client,
    credentials: &PostgresCredentials,
    role: &str,
    database: &str,
    schema: &str,
    privileges: &UserPrivileges,
) -> Result<u64, PGCliError> {
    trace!("Revoking privileges from role {}", role);
    if !validate_pg_names(role) {
        error!("Invalid role name: {}", role);
        return Err(PGCliError::Other("Invalid role name".to_string()));
    }

    if !check_user_exists(&client, role).await? {
        error!("Role {} does not exist", role);
        return Err(PGCliError::Other("Role does not exist".to_string()));
    }
    let mut res: u64 = 0;

    match privileges {
        UserPrivileges::Full => {
            let statement = format!(
                "REVOKE ALL PRIVILEGES ON DATABASE \"{}\" FROM \"{}\"",
                database, role
            );

            match client.execute(&statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };

            let db_client = postgres_connect(&credentials, Some(database.to_string())).await?;

            let schema_statement = format!(
                "REVOKE ALL PRIVILEGES ON SCHEMA \"{}\" FROM \"{}\"",
                schema, role
            );

            let table_statement = format!(
                "REVOKE ALL PRIVILEGES ON ALL TABLES IN SCHEMA \"{}\" FROM \"{}\"",
                schema, role
            );

            let sequence_statement = format!(
                "REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA \"{}\" FROM \"{}\"",
                schema, role
            );

            match db_client.execute(&schema_statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };

            match db_client.execute(&table_statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };

            match db_client.execute(&sequence_statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };
        }
        UserPrivileges::ReadOnly => {
            let statement = format!(
                "REVOKE CONNECT ON DATABASE \"{}\" FROM \"{}\"",
                database, role
            );

            match client.execute(&statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };

            let db_client = postgres_connect(&credentials, Some(database.to_string())).await?;

            let schema_statement =
                format!("REVOKE USAGE ON SCHEMA \"{}\" FROM \"{}\"", schema, role);

            let table_statement = format!(
                "REVOKE SELECT ON ALL TABLES IN SCHEMA \"{}\" FROM \"{}\"",
                schema, role
            );

            let sequence_statement = format!(
                "REVOKE USAGE, SELECT ON ALL SEQUENCES IN SCHEMA \"{}\" FROM \"{}\"",
                schema, role
            );

            let default_privileges_statement = format!(
                "ALTER DEFAULT PRIVILEGES IN SCHEMA \"{}\" REVOKE SELECT ON TABLES FROM \"{}\"",
                schema, role
            );

            match db_client.execute(&schema_statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };

            match db_client.execute(&table_statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };

            match db_client.execute(&sequence_statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };

            match db_client.execute(&default_privileges_statement, &[]).await {
                Ok(r) => res += r,
                Err(e) => return Err(PGCliError::from(e)),
            };
        }
    }

    debug!("Privilege {} revoked from role {}", privileges, role);

    Ok(res)
}
