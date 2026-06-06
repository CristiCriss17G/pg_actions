use crate::utils::misc::{ensure_file_path_exists, open_file_path_in_write_mode};
use crate::utils::structs::{PGCliError, PGTools, PostgresCredentials, Result, SqlFileFormat};
use deadpool_postgres::{
    Client, Config as DPConfig, ManagerConfig, Pool, RecyclingMethod, Runtime,
};
use log::{debug, error, info, trace};
use native_tls::TlsConnector;
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher,
};
use postgres_native_tls::MakeTlsConnector;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{Mutex, OnceCell};

pub(super) fn validate_pg_names(name: &str) -> bool {
    name.chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

fn compare_pgpass_credentials(
    credentials: &[&PostgresCredentials],
    pgpass: &PostgresCredentials,
) -> bool {
    credentials.contains(&pgpass)
}

/// Initialize the .pgpass file with the credentials
pub async fn init_pgpass(credentials: &[PostgresCredentials]) -> Result<()> {
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
            let pgpass_values: Vec<_> = pgpass.values().collect();
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
                return Err(PGCliError::TokioIo(e));
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

type PoolMap = HashMap<(PostgresCredentials, String), Pool>;

static POOL_MAP: OnceCell<Mutex<PoolMap>> = OnceCell::const_new();

async fn get_or_init_pool(credentials: &PostgresCredentials, db: Option<String>) -> Result<Pool> {
    // Initialize the global hashmap if it hasn't been initialized
    let pool_map = POOL_MAP
        .get_or_init(|| async { Mutex::new(HashMap::new()) })
        .await;

    // Database name, defaulting to "postgres"
    let dbname = db.unwrap_or("postgres".to_string());

    // Create a key using the credentials and database name
    let pool_key = (credentials.clone(), dbname.clone());

    {
        // Check if the pool already exists for the given credentials and db in the hashmap
        let pools = pool_map.lock().await;
        if let Some(pool) = pools.get(&pool_key) {
            trace!("Reusing existing pool for {:?}", pool_key);
            // Return a connection from the existing pool
            return Ok(pool.clone());
        }
    }

    let tls = TlsConnector::builder()
        .danger_accept_invalid_certs(true) // Allows self-signed certificates
        .build()
        .map_err(|e| PGCliError::ConnectionError(e.to_string()))?;
    let connector = MakeTlsConnector::new(tls);

    let mut cfg = DPConfig::new();
    cfg.host = Some(credentials.pg_hostname.clone());
    cfg.port = Some(credentials.pg_port);
    cfg.user = Some(credentials.pg_superuser.clone());
    cfg.password = Some(credentials.pg_password.clone());
    cfg.dbname = Some(dbname);
    cfg.application_name = Some("pg_actions".to_string());
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });

    trace!("Creating new pool for {:?}", pool_key);
    let pool = cfg.create_pool(Some(Runtime::Tokio1), connector)?;

    // let pool = Arc::new(pool);

    {
        // Store the newly created pool in the hashmap
        let mut pools = pool_map.lock().await;
        pools.insert(pool_key.clone(), pool.clone());
    }

    Ok(pool)
}

pub async fn postgres_connect(
    credentials: &PostgresCredentials,
    db: Option<String>,
) -> Result<Client> {
    // Get a pool from the global hashmap or create a new one
    let pool = get_or_init_pool(credentials, db).await?;

    // Return a connection from the newly created pool
    pool.get()
        .await
        .map_err(|e| PGCliError::PoolErrorTokio(e.to_string()))
}

pub async fn postgres_connect_pool(
    credentials: &PostgresCredentials,
    db: Option<String>,
) -> Result<Pool> {
    // Get a pool from the global hashmap or create a new one
    get_or_init_pool(credentials, db).await
}

pub async fn db_dump<P: AsRef<Path>>(
    credentials: PostgresCredentials,
    database_source: &str,
    dump_file: P,
    pg_tools: &PGTools,
    dump_format: SqlFileFormat,
) -> Result<String> {
    info!("Dumping database {}", database_source);

    // Clone the data before moving it into the closure
    // let credentials = credentials.clone();
    // Open the file inside the closure to ensure it's owned by the closure
    match ensure_file_path_exists(&dump_file).await {
        Ok(_) => (),
        Err(_) => {
            return Err(PGCliError::Other(
                "Failed to create dump file path".to_string(),
            ))
        }
    };

    let output_file = match fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&dump_file)
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

    let mut child = match Command::new(&pg_tools.pg_dump)
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
    let mut reader = BufReader::new(stderr).lines();

    // Process each line of the stderr
    loop {
        match reader.next_line().await {
            Ok(Some(ln)) => debug!("[{}]: {}", database_source, ln),
            Ok(None) => break, // End of stream
            Err(e) => {
                error!("Error reading stderr: {}", e);
                break;
            }
        }
    }

    match child.wait().await {
        Ok(status) if status.success() => Ok("Success".to_string()),
        Ok(_) => Err(PGCliError::Other("Failed to dump database".to_string())),
        Err(e) => Err(PGCliError::Other(format!("Failed to dump database: {}", e))),
    }
}

pub async fn db_restore<P: AsRef<Path>>(
    credentials: PostgresCredentials,
    database_target: &str,
    dump_file: P,
    pg_tools: &PGTools,
    restore_format: SqlFileFormat,
) -> Result<String> {
    info!("Restoring database {}", database_target);

    trace!("Restoring format: {}", restore_format);

    if restore_format == SqlFileFormat::Sql {
        return db_restore_from_sql(credentials, database_target, dump_file, pg_tools).await;
    }

    let input_file = match std::fs::File::open(&dump_file) {
        Ok(file) => file,
        Err(_) => return Err(PGCliError::Other("Failed to open dump file".to_string())),
    };

    let mut child = match Command::new(&pg_tools.pg_restore)
        .arg("-v") // Enable verbose mode
        .arg(format!("--host={}", credentials.pg_hostname))
        .arg(format!("--username={}", credentials.pg_superuser))
        .arg(format!("--port={}", credentials.pg_port))
        .arg(format!("--dbname={}", database_target))
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
    let mut reader = BufReader::new(stderr).lines();

    // Process each line of the stderr
    loop {
        match reader.next_line().await {
            Ok(Some(ln)) => debug!("[{}]: {}", database_target, ln),
            Ok(None) => break, // End of stream
            Err(e) => {
                error!("Error reading stderr: {}", e);
                break;
            }
        }
    }

    match child.wait().await {
        Ok(status) if status.success() => Ok("Success".to_string()),
        Ok(_) => Err(PGCliError::Other("Failed to restore database".to_string())),
        Err(e) => Err(PGCliError::Other(format!(
            "Failed to restore database: {}",
            e
        ))),
    }
}

async fn db_restore_from_sql<P: AsRef<Path>>(
    credentials: PostgresCredentials,
    database_target: &str,
    dump_file: P,
    pg_tools: &PGTools,
) -> Result<String> {
    debug!("Restoring database {} from SQL file", database_target);

    let mut child = match Command::new(&pg_tools.psql)
        .arg("-e") // Enable verbose mode
        .arg(format!("--host={}", credentials.pg_hostname))
        .arg(format!("--username={}", credentials.pg_superuser))
        .arg(format!("--port={}", credentials.pg_port))
        .arg(format!("--dbname={}", database_target))
        .arg(format!("--file={}", dump_file.as_ref().to_str().unwrap()))
        .stdin(Stdio::null()) // Redirect standard input from null
        .stdout(Stdio::piped()) // Capture the standard output
        .stderr(Stdio::piped()) // Capture the standard error
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            return Err(PGCliError::Other(
                "Failed to spawn psql command for SQL file".to_string(),
            ))
        }
    };

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| PGCliError::Other("Failed to take stdout".to_string()))?;

    let database_target_clone = database_target.to_string();
    let stdouth = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();

        // Process each line of the stdout
        loop {
            match reader.next_line().await {
                Ok(Some(ln)) => debug!("[{}:stdout]: {}", database_target_clone, ln),
                Ok(None) => break, // End of stream
                Err(e) => {
                    error!("Error reading stdout: {}", e);
                    break;
                }
            }
        }
    });

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| PGCliError::Other("Failed to take stderr".to_string()))?;

    let mut reader = BufReader::new(stderr).lines();

    let database_target = database_target.to_string();
    // Process each line of the stderr
    loop {
        match reader.next_line().await {
            Ok(Some(ln)) => debug!("[{}:stderr]: {}", database_target, ln),
            Ok(None) => break, // End of stream
            Err(e) => {
                error!("Error reading stderr: {}", e);
                break;
            }
        }
    }

    stdouth.await?;

    match child.wait().await {
        Ok(status) if status.success() => Ok("Success".to_string()),
        Ok(_) => Err(PGCliError::Other("Failed to restore database".to_string())),
        Err(e) => Err(PGCliError::Other(format!(
            "Failed to restore database: {}",
            e
        ))),
    }
}

pub(super) fn filter_entities<T: AsRef<str>>(
    entities: impl IntoIterator<Item = T>,
    query: &str,
) -> Vec<T> {
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut matches = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart)
        .match_list(entities, &mut matcher);
    matches.sort_by_key(|a| a.1);
    matches.into_iter().map(|(db, _)| db).collect()
}
