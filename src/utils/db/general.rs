use crate::utils::misc::{ensure_file_path_exists, open_file_path_in_write_mode};
use crate::utils::structs::{PGCliError, PGTools, PostgresCredentials, SqlFileFormat};
use log::{debug, error, info, trace};
use native_tls::TlsConnector;
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher,
};
use postgres_native_tls::MakeTlsConnector;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tokio::io::AsyncWriteExt;
use tokio::task;
use tokio_postgres::Client;

pub(super) fn validate_pg_names(name: &str) -> bool {
    name.chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

fn compare_pgpass_credentials(
    credentials: &Vec<&PostgresCredentials>,
    pgpass: &PostgresCredentials,
) -> bool {
    credentials.iter().any(|c| {
        c.pg_hostname == pgpass.pg_hostname
            && c.pg_port == pgpass.pg_port
            && c.pg_superuser == pgpass.pg_superuser
            && c.pg_password == pgpass.pg_password
    })
}

/// Initialize the .pgpass file with the credentials
pub async fn init_pgpass(credentials: &[PostgresCredentials]) -> Result<(), PGCliError> {
    let home = dirs::home_dir().expect("Home directory not found");
    let pgpass_file = home.join(".pgpass");

    let mut credentials = credentials.to_owned();

    if pgpass_file.exists() {
        debug!("pgpass file already exists");
        let pgpass = try_read_pgpass().await;

        if pgpass.is_empty() {
            debug!("pgpass file is empty");
        } else {
            let mut valid = true;
            let pgpass_values = pgpass.values().collect();
            for value in credentials.iter() {
                trace!("Checking pgpass line: {:?}", value);
                if !compare_pgpass_credentials(&pgpass_values, value) {
                    debug!("pgpass does not contain all the good credentials");
                    valid = false;
                    break;
                }
            }
            if valid {
                debug!("pgpass file already contains the credentials");
                return Ok(());
            }
            let mut credentials_set: HashSet<_> = credentials.iter().collect();
            credentials_set.extend(pgpass_values);
            trace!("Merging credentials {:?}", credentials_set);
            credentials = credentials_set.into_iter().cloned().collect();
        }
    }

    let mut file = open_file_path_in_write_mode(&pgpass_file, Some(0o600))
        .await
        .unwrap_or_else(|_| panic!("Failed to open file {}", pgpass_file.display()));

    for (i, value) in credentials.iter().enumerate() {
        let pgpass_line = format!(
            "{}:{}:{}:{}:{}\n",
            value.pg_hostname, value.pg_port, "*", value.pg_superuser, value.pg_password
        );

        match file.write_all(pgpass_line.as_bytes()).await {
            Ok(_) => trace!("pgpass written line {}", i),
            Err(e) => {
                error!("Failed to write to pgpass file: {}", e);
                return Err(PGCliError::Io(e));
            }
        }
    }
    debug!("pgpass file written");

    Ok(())
}

/// try to find a .pgpass file in the home directory and read its content
/// returns the content of the file as a PostgresCredentials struct
/// if the file is not found or is invalid, returns None
pub async fn try_read_pgpass() -> HashMap<String, PostgresCredentials> {
    let home = dirs::home_dir().expect("Home directory not found");
    let pgpass_file = home.join(".pgpass");

    if !pgpass_file.exists() {
        return HashMap::new();
    }

    let file = match tokio::fs::read_to_string(&pgpass_file).await {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to read pgpass file: {}", e);
            return HashMap::new();
        }
    };

    let mut result = HashMap::new();

    let lines = file.lines();

    for line in lines {
        let mut parts = line.trim().split(':');
        if parts.clone().count() != 5 {
            error!("Invalid .pgpass line: {}", line);
            continue;
        }
        let hostname = parts.next().expect("Failed to read hostname");
        let port = parts
            .next()
            .expect("Failed to read port")
            .parse::<u16>()
            .expect("Failed to parse port");
        let _ = parts.next().expect("Failed to read database");
        let username = parts.next().expect("Failed to read username");
        let password = parts.next().expect("Failed to read password");

        result.insert(
            hostname.to_string(),
            PostgresCredentials::new(
                hostname.to_string(),
                username.to_string(),
                password.to_string(),
                port,
            ),
        );
    }

    result
}

pub async fn postgres_connect(
    credentials: &PostgresCredentials,
    db: Option<String>,
) -> Result<Client, PGCliError> {
    let tls = TlsConnector::builder()
        .danger_accept_invalid_certs(true) // Allows self-signed certificates
        .build()
        .map_err(|e| PGCliError::ConnectionError(e.to_string()))?;
    let connector = MakeTlsConnector::new(tls);

    let (client, connection) = tokio_postgres::connect(
        &format!(
            "host={} port={} user={} password='{}' dbname={}",
            credentials.pg_hostname,
            credentials.pg_port,
            credentials.pg_superuser,
            credentials.pg_password,
            db.unwrap_or("postgres".to_string())
        ),
        connector,
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
    pg_tools: &PGTools,
    dump_format: SqlFileFormat,
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

    let pg_dump = pg_tools.pg_dump.clone();

    task::spawn_blocking(move || {
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

        trace!("Dumping format: {}", dump_format);

        let mut child = match Command::new(pg_dump)
            .arg("-v") // Enable verbose mode
            .arg(format!("--host={}", credentials.pg_hostname))
            .arg(format!("--username={}", credentials.pg_superuser))
            .arg(format!("--port={}", credentials.pg_port))
            .arg(format!("--dbname={}", database_source))
            .arg(format!("--format={}", dump_format))
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
    .await?
}

pub async fn db_restore(
    credentials: PostgresCredentials,
    database_target: &str,
    dump_file: &str,
    pg_tools: &PGTools,
    restore_format: SqlFileFormat,
) -> Result<String, PGCliError> {
    info!("Restoring database {}", database_target);

    let database_target = database_target.to_string();
    let dump_file_path = dump_file.to_string();

    let pg_restore = pg_tools.pg_restore.clone();

    task::spawn_blocking(move || {
        // Open the file inside the closure to ensure it's owned by the closure

        let input_file = match std::fs::File::open(&dump_file_path) {
            Ok(file) => file,
            Err(_) => return Err(PGCliError::Other("Failed to open dump file".to_string())),
        };

        trace!("Restoring format: {}", restore_format);

        let mut child = match Command::new(pg_restore)
            .arg("-v") // Enable verbose mode
            .arg(format!("--host={}", credentials.pg_hostname))
            .arg(format!("--username={}", credentials.pg_superuser))
            .arg(format!("--port={}", credentials.pg_port))
            .arg(format!("--dbname={}", database_target))
            .arg(format!("--format={}", restore_format))
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
    .await?
}

pub(super) fn filter_entities<T: AsRef<str>>(
    entities: impl IntoIterator<Item = T>,
    query: &str,
) -> Vec<T> {
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut matches = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart)
        .match_list(entities, &mut matcher);
    matches.sort_by(|a, b| a.1.cmp(&b.1));
    matches.into_iter().map(|(db, _)| db).collect()
}
