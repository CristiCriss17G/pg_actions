use super::general::validate_pg_names;
use crate::utils::structs::{PGCliError, UserDetails};
use log::{error, trace};
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
    sort: &Option<String>,
    extra: bool,
) -> Result<Vec<UserDetails>, PGCliError> {
    let ignored_roles= "'pg_checkpoint','pg_create_subscription','pg_database_owner','pg_execute_server_program','pg_monitor','pg_read_all_data','pg_read_all_settings','pg_read_all_stats','pg_read_server_files','pg_stat_scan_tables','pg_use_reserved_connections','pg_write_all_data','pg_write_server_files','pg_signal_backend'";
    let fields = match extra {
        true => "rolname, rolsuper, rolcreatedb, rolcanlogin, oid",
        false => "rolname",
    };
    let rows = match sort {
        Some(sort) => {
            match sort.as_str() {
                "asc" => client.query(&format!("SELECT {fields} FROM pg_roles WHERE rolname NOT IN ({ignored_roles}) ORDER BY rolname ASC"), &[]).await?,
                "desc" => client.query(&format!("SELECT {fields} FROM pg_roles WHERE rolname NOT IN ({ignored_roles}) ORDER BY rolname DESC"), &[]).await?,
                _ => client.query(&format!("SELECT {fields} FROM pg_roles WHERE rolname NOT IN ({ignored_roles})"), &[]).await?,
            }
        },
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
    let mut statement = format!("CREATE ROLE {} WITH PASSWORD '{}'", role, password);
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

    let statement = format!("DROP ROLE {}", role);
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
