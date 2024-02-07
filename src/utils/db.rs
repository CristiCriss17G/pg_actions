use crate::utils::structs::PGCliError;
use crate::utils::structs::PostgresCredentials;
use log::{debug, error, info, trace};
use std::fs::{self, OpenOptions, Permissions};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::{
    path::Path,
    process::{Command, Stdio},
};
use tokio_postgres::{Client, Error, NoTls};

fn open_file_in_write_mode(path: &str, permissions: Option<u32>) -> io::Result<fs::File> {
    let path = Path::new(path);
    let file = open_file_path_in_write_mode(path, permissions).expect("Failed to open file");
    Ok(file)
}

fn open_file_path_in_write_mode(path: &Path, permissions: Option<u32>) -> io::Result<fs::File> {
    if let Some(dir) = path.parent() {
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
    }

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;

    // Set the permissions for the file to 644 (owner: read-write, group: read, others: read)
    let permissions = Permissions::from_mode(permissions.unwrap_or(0o644));
    fs::set_permissions(path, permissions)?;

    Ok(file)
}

/// Initialize the .pgpass file with the credentials
pub async fn init_pgpass(credentials: &PostgresCredentials) -> Result<(), PGCliError> {
    let home = dirs_next::home_dir().expect("Home directory not found");
    let pgpass_file = home.join(".pgpass");

    if pgpass_file.exists() {
        debug!("pgpass file already exists");
        return Ok(());
    }

    let mut file = open_file_path_in_write_mode(&pgpass_file, Some(0o600))
        .expect(format!("Failed to open file {}", pgpass_file.display()).as_str());

    let pgpass_line = format!(
        "{}:{}:{}:{}:{}",
        credentials.pg_hostname,
        credentials.pg_port,
        "*",
        credentials.pg_superuser,
        credentials.pg_password
    );

    match file.write_all(pgpass_line.as_bytes()) {
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
pub fn try_read_pgpass() -> Option<PostgresCredentials> {
    let home = dirs_next::home_dir().expect("Home directory not found");
    let pgpass_file = home.join(".pgpass");

    if !pgpass_file.exists() {
        return None;
    }

    let file = match fs::read_to_string(&pgpass_file) {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to read pgpass file: {}", e);
            return None;
        }
    };

    let mut parts = file.split(':');
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
    credentials: &PostgresCredentials,
    database_source: &str,
    dump_file: &str,
) -> Result<String, PGCliError> {
    let output_file = open_file_in_write_mode(dump_file, None).expect("Failed to open dump file");

    info!("Dumping database {}", database_source);

    let mut child = Command::new("pg_dump")
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
        .expect("Failed to spawn pg_dump command");

    let stderr = child.stderr.take().expect("Failed to take stderr");
    let reader = BufReader::new(stderr);

    // Process each line of the stderr
    for line in reader.lines() {
        match line {
            Ok(ln) => debug!("[{}]: {}", database_source, ln),
            Err(e) => error!("Error reading stderr: {}", e),
        }
    }

    let status = child.wait().expect("Failed to wait on child");

    if status.success() {
        Ok("Success".to_string())
    } else {
        Err(PGCliError::Other("Failed to dump database".to_string()))
    }
}

pub async fn db_restore(
    credentials: &PostgresCredentials,
    database_target: &str,
    dump_file: &str,
) -> Result<String, PGCliError> {
    info!("Restoring database {}", database_target);

    let input_file = fs::File::open(dump_file).expect("Failed to open dump file");

    let mut child = Command::new("pg_restore")
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
        .expect("Failed to spawn pg_restore command");

    let stderr = child.stderr.take().expect("Failed to take stderr");
    let reader = BufReader::new(stderr);

    // Process each line of the stderr
    for line in reader.lines() {
        match line {
            Ok(ln) => debug!("[{}]: {}", database_target, ln),
            Err(e) => error!("Error reading stderr: {}", e),
        }
    }

    let status = child.wait().expect("Failed to wait on child");

    if status.success() {
        Ok("Success".to_string())
    } else {
        Err(PGCliError::Other("Failed to restore database".to_string()))
    }
}

pub async fn create_user(client: &Client, user: &str, password: &str) -> Result<u64, Error> {
    trace!("Creating user {}", user);
    client
        .execute("CREATE USER $1 WITH PASSWORD $2", &[&user, &password])
        .await
}

pub async fn create_db(client: &Client, db: &str, owner: &str) -> Result<u64, Error> {
    trace!("Creating database {}", db);
    client
        .execute("CREATE DATABASE $1 WITH OWNER $2", &[&db, &owner])
        .await
}

pub async fn delete_db(client: &Client, db: &str) -> Result<u64, Error> {
    trace!("Deleting database {}", db);
    client.execute("DROP DATABASE $1", &[&db]).await
}

pub async fn change_owner_of_objects_in_db(
    credentials: &PostgresCredentials,
    db: &str,
    new_owner: &str,
) -> Result<(), PGCliError> {
    trace!("Changing owner of database {} to {}", db, new_owner);
    debug!("Connecting to postgres to change owner of database {}", db);
    let client = postgres_connect(credentials, Some(db.to_string())).await?;

    // Prepare the REASSIGN OWNED BY command string
    let reassign_command = format!(
        "REASSIGN OWNED BY {} TO {};",
        credentials.pg_superuser, new_owner
    );

    // Execute the REASSIGN OWNED BY command
    match client.batch_execute(&reassign_command).await {
        Ok(_) => {
            debug!(
                "Ownership of all objects successfully reassigned from {} to {}.",
                credentials.pg_superuser, new_owner
            );
            Ok(())
        }
        Err(e) => {
            error!("Failed to reassign ownership of objects: {}", e);
            Err(PGCliError::Postgres(e))
        }
    }
}
