use super::misc::{ensure_file_path_exists, open_file_path_in_write_mode};
use super::structs::{PGCliError, PostgresCredentials};
use log::{debug, error, info, trace};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tokio::io::AsyncWriteExt;
use tokio::task;
use tokio_postgres::{Client, Error, NoTls};

fn validate_pg_names(name: &str) -> bool {
    name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn compare_pgpass_credentials(
    credentials: &PostgresCredentials,
    pgpass: &PostgresCredentials,
) -> bool {
    credentials.pg_hostname == pgpass.pg_hostname
        && credentials.pg_port == pgpass.pg_port
        && credentials.pg_superuser == pgpass.pg_superuser
        && credentials.pg_password == pgpass.pg_password
}

/// Initialize the .pgpass file with the credentials
pub async fn init_pgpass(credentials: &PostgresCredentials) -> Result<(), PGCliError> {
    let home = dirs_next::home_dir().expect("Home directory not found");
    let pgpass_file = home.join(".pgpass");

    if pgpass_file.exists() {
        debug!("pgpass file already exists");
        let pgpass = try_read_pgpass().await;
        if let Some(pgpass) = pgpass {
            if compare_pgpass_credentials(credentials, &pgpass) {
                debug!("pgpass file already contains the credentials");
                return Ok(());
            }
        }
    }

    let mut file = open_file_path_in_write_mode(&pgpass_file, Some(0o600))
        .await
        .expect(format!("Failed to open file {}", pgpass_file.display()).as_str());

    let pgpass_line = format!(
        "{}:{}:{}:{}:{}",
        credentials.pg_hostname,
        credentials.pg_port,
        "*",
        credentials.pg_superuser,
        credentials.pg_password
    );

    match file.write_all(pgpass_line.as_bytes()).await {
        Ok(_) => debug!("pgpass file created successfully"),
        Err(e) => {
            error!("Failed to write to pgpass file: {}", e);
            return Err(PGCliError::Io(e));
        }
    }

    Ok(())
}

/// try to find a .pgpass file in the home directory and read its content
/// returns the content of the file as a PostgresCredentials struct
/// if the file is not found or is invalid, returns None
pub async fn try_read_pgpass() -> Option<PostgresCredentials> {
    let home = dirs_next::home_dir().expect("Home directory not found");
    let pgpass_file = home.join(".pgpass");

    if !pgpass_file.exists() {
        return None;
    }

    let file = match tokio::fs::read_to_string(&pgpass_file).await {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to read pgpass file: {}", e);
            return None;
        }
    };

    let mut parts = file.trim().split(':');
    let hostname = parts.next().expect("Failed to read hostname");
    let port = parts
        .next()
        .expect("Failed to read port")
        .parse::<u16>()
        .expect("Failed to parse port");
    let _ = parts.next().expect("Failed to read database");
    let username = parts.next().expect("Failed to read username");
    let password = parts.next().expect("Failed to read password");

    Some(PostgresCredentials {
        pg_hostname: hostname.to_string(),
        pg_port: port,
        pg_superuser: username.to_string(),
        pg_password: password.to_string(),
    })
}

pub async fn postgres_connect(
    credentials: &PostgresCredentials,
    db: Option<String>,
) -> Result<Client, PGCliError> {
    let (client, connection) = tokio_postgres::connect(
        &format!(
            "host={} port={} user={} password='{}' dbname={}",
            credentials.pg_hostname,
            credentials.pg_port,
            credentials.pg_superuser,
            credentials.pg_password,
            db.unwrap_or("postgres".to_string())
        ),
        NoTls,
    )
    .await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("Connection error: {}", e);
        }
    });

    Ok(client)
}

pub async fn db_dump(
    credentials: PostgresCredentials,
    database_source: &str,
    dump_file: &str,
) -> Result<String, PGCliError> {
    info!("Dumping database {}", database_source);

    // Clone the data before moving it into the closure
    // let credentials = credentials.clone();
    let database_source = database_source.to_string();
    let dump_file_path = dump_file.to_string();
    // Open the file inside the closure to ensure it's owned by the closure
    match ensure_file_path_exists(&dump_file_path).await {
        Ok(_) => (),
        Err(_) => {
            return Err(PGCliError::Other(
                "Failed to create dump file path".to_string(),
            ))
        }
    };

    let output = task::spawn_blocking(move || {
        let output_file = match std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&dump_file_path)
        {
            Ok(file) => file,
            Err(e) => {
                return Err(PGCliError::Other(format!(
                    "Failed to create dump file: {}",
                    e
                )))
            }
        };

        let mut child = match Command::new("pg_dump")
            .arg("-v") // Enable verbose mode
            .arg(format!("--host={}", credentials.pg_hostname))
            .arg(format!("--username={}", credentials.pg_superuser))
            .arg(format!("--port={}", credentials.pg_port))
            .arg(format!("--dbname={}", database_source))
            .arg("--format=custom")
            .arg("--no-owner")
            .arg("--no-privileges")
            .stdout(Stdio::from(output_file)) // Redirect standard output to file
            .stderr(Stdio::piped()) // Capture the standard error
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                return Err(PGCliError::Other(format!(
                    "Failed to spawn pg_dump command: {}",
                    e
                )))
            }
        };

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| PGCliError::Other("Failed to take stderr".to_string()))?;
        let reader = BufReader::new(stderr);

        // Process each line of the stderr
        for line in reader.lines() {
            match line {
                Ok(ln) => debug!("[{}]: {}", database_source, ln),
                Err(e) => error!("Error reading stderr: {}", e),
            }
        }

        match child.wait() {
            Ok(status) if status.success() => Ok("Success".to_string()),
            Ok(_) => Err(PGCliError::Other("Failed to dump database".to_string())),
            Err(e) => Err(PGCliError::Other(format!("Failed to dump database: {}", e))),
        }
    })
    .await?;

    output
}

pub async fn db_restore(
    credentials: PostgresCredentials,
    database_target: &str,
    dump_file: &str,
) -> Result<String, PGCliError> {
    info!("Restoring database {}", database_target);

    let database_target = database_target.to_string();
    let dump_file_path = dump_file.to_string();

    let output = task::spawn_blocking(move || {
        // Open the file inside the closure to ensure it's owned by the closure

        let input_file = match std::fs::File::open(&dump_file_path) {
            Ok(file) => file,
            Err(_) => return Err(PGCliError::Other("Failed to open dump file".to_string())),
        };

        let mut child = match Command::new("pg_restore")
            .arg("-v") // Enable verbose mode
            .arg(format!("--host={}", credentials.pg_hostname))
            .arg(format!("--username={}", credentials.pg_superuser))
            .arg(format!("--port={}", credentials.pg_port))
            .arg(format!("--dbname={}", database_target))
            .arg("--format=custom")
            .arg("--no-owner")
            .arg("--no-privileges")
            .stdin(Stdio::from(input_file)) // Redirect standard input from file
            .stderr(Stdio::piped()) // Capture the standard error
            .spawn()
        {
            Ok(child) => child,
            Err(_) => {
                return Err(PGCliError::Other(
                    "Failed to spawn pg_restore command".to_string(),
                ))
            }
        };

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| PGCliError::Other("Failed to take stderr".to_string()))?;
        let reader = BufReader::new(stderr);

        // Process each line of the stderr
        for line in reader.lines() {
            match line {
                Ok(ln) => debug!("[{}]: {}", database_target, ln),
                Err(e) => error!("Error reading stderr: {}", e),
            }
        }

        match child.wait() {
            Ok(status) if status.success() => Ok("Success".to_string()),
            Ok(_) => Err(PGCliError::Other("Failed to restore database".to_string())),
            Err(e) => Err(PGCliError::Other(format!(
                "Failed to restore database: {}",
                e
            ))),
        }
    })
    .await?;

    output
}

pub async fn list_databases(client: &Client) -> Result<Vec<String>, PGCliError> {
    let rows = client.query("SELECT datname FROM pg_database WHERE datistemplate = false AND datname NOT IN ('postgres', 'template0', 'template1')", &[]).await?;
    let mut databases = Vec::new();
    for row in rows {
        let db: String = row.get(0);
        databases.push(db);
    }
    Ok(databases)
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

    let statement = format!("CREATE USER {} WITH PASSWORD '{}'", user, password);
    match client.execute(&statement, &[]).await {
        Ok(r) => Ok(r),
        Err(e) => Err(PGCliError::from(e)),
    }
}

pub async fn create_db(client: &Client, db: &str, owner: &str) -> Result<u64, PGCliError> {
    trace!("Creating database {}", db);
    if !validate_pg_names(db) {
        error!("Invalid database name: {}", db);
        return Err(PGCliError::Other("Invalid database name".to_string()));
    }
    if !validate_pg_names(owner) {
        error!("Invalid owner name: {}", owner);
        return Err(PGCliError::Other("Invalid owner name".to_string()));
    }

    let statement = format!("CREATE DATABASE {} WITH OWNER {}", db, owner);
    match client.execute(&statement, &[]).await {
        Ok(r) => Ok(r),
        Err(e) => Err(PGCliError::from(e)),
    }
}

pub async fn delete_db(client: &Client, db: &str) -> Result<u64, PGCliError> {
    trace!("Deleting database {}", db);
    if !validate_pg_names(db) {
        error!("Invalid database name: {}", db);
        return Err(PGCliError::Other("Invalid database name".to_string()));
    }
    let statement = format!("DROP DATABASE {}", db);
    match client.execute(&statement, &[]).await {
        Ok(r) => Ok(r),
        Err(e) => Err(PGCliError::from(e)),
    }
}

pub async fn kill_connections_to_db(client: &Client, db: &str) -> Result<u64, Error> {
    trace!("Killing connections to database {}", db);
    client
        .execute(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE pid <> pg_backend_pid() AND datname = $1",
            &[&db],
        )
        .await
}

pub async fn change_owner_of_db(
    client: &Client,
    db: &str,
    new_owner: &str,
) -> Result<u64, PGCliError> {
    trace!("Changing owner of database {} to {}", db, new_owner);
    if !validate_pg_names(new_owner) {
        error!("Invalid new owner name: {}", new_owner);
        return Err(PGCliError::Other("Invalid new owner name".to_string()));
    }
    match client
        .execute("ALTER DATABASE $1 OWNER TO $2", &[&db, &new_owner])
        .await
    {
        Ok(r) => Ok(r),
        Err(e) => Err(PGCliError::from(e)),
    }
}

pub async fn change_owner_of_objects_in_db(
    credentials: &PostgresCredentials,
    db: &str,
    new_owner: &str,
) -> Result<(), PGCliError> {
    trace!("Changing owner of database {} to {}", db, new_owner);
    debug!("Connecting to postgres to change owner of database {}", db);

    // Sanitize the new owner name or validate it here
    // For example, ensure it matches a strict pattern (alphanumeric + underscore)
    if !validate_pg_names(new_owner) {
        error!("Invalid new owner name: {}", new_owner);
        return Err(PGCliError::Other("Invalid new owner name".to_string()));
    }

    let client = postgres_connect(credentials, Some(db.to_string())).await?;

    // Query to select type names
    debug!("Querying types to change owner");
    let rows = client.query(
        "SELECT typname FROM pg_type WHERE typtype IN ('b', 'e') AND typcategory != 'A' AND typnamespace IN (SELECT oid FROM pg_namespace WHERE nspname NOT IN ('pg_catalog', 'information_schema'))",
        &[],
    ).await?;
    trace!("Types to change owner: {:?} ({})", rows, rows.len());

    // Loop through each type and change its owner
    for row in rows {
        let typname: &str = row.get(0);
        debug!("Changing owner of type: {} to {}", typname, new_owner);

        // Dynamically build the ALTER TYPE command
        let statement = format!("ALTER TYPE \"{}\" OWNER TO {}", typname, new_owner);

        // Execute the ALTER TYPE command
        client.execute(&statement, &[]).await?;
    }

    Ok(())
}
