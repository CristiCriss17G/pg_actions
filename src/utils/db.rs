use crate::utils::structs::PostgresCredentials;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::{
    path::Path,
    process::{Command, Stdio},
};
use tokio_postgres::{Client, Error, NoTls};

fn open_file_in_write_mode(path: &str) -> io::Result<fs::File> {
    let path = Path::new(path);

    let file = open_file_path_in_write_mode(path).expect("Failed to open file");
    Ok(file)
}

fn open_file_path_in_write_mode(path: &Path) -> io::Result<fs::File> {
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

    Ok(file)
}

pub async fn init_pgpass(credentials: &PostgresCredentials) -> Result<(), Error> {
    let home = dirs_next::home_dir().expect("Home directory not found");
    let pgpass_file = home.join(".pgpass");

    if !pgpass_file.exists() {
        let mut file = open_file_path_in_write_mode(&pgpass_file).expect("Failed to open file");

        let pgpass_line = format!(
            "{}:{}:{}:{}:{}",
            credentials.pg_hostname,
            credentials.pg_port,
            "*",
            credentials.pg_superuser,
            credentials.pg_password
        );

        match file.write_all(pgpass_line.as_bytes()) {
            Ok(_) => println!("pgpass file created successfully"),
            Err(e) => {
                eprintln!("Failed to write to pgpass file: {}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

pub async fn postgres_connect(credentials: &PostgresCredentials) -> Result<Client, Error> {
    let (client, connection) = tokio_postgres::connect(
        &format!(
            "host={} port={} user={} password={}",
            credentials.pg_hostname,
            credentials.pg_port,
            credentials.pg_superuser,
            credentials.pg_password
        ),
        NoTls,
    )
    .await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    Ok(client)
}

pub async fn db_dump(
    credentials: &PostgresCredentials,
    database_source: &str,
    dump_file: &str,
) -> Result<String, String> {
    let output_file = open_file_in_write_mode(dump_file).expect("Failed to open dump file");

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
            Ok(ln) => println!("[{}]: {}", database_source, ln),
            Err(e) => eprintln!("Error reading stderr: {}", e),
        }
    }

    let status = child.wait().expect("Failed to wait on child");

    if status.success() {
        Ok("Success".to_string())
    } else {
        Err("Failed".to_string())
    }
}

pub async fn create_user(client: &Client, user: &str, password: &str) -> Result<u64, Error> {
    client
        .execute("CREATE USER $1 WITH PASSWORD $2", &[&user, &password])
        .await
}

pub async fn create_db(client: &Client, db: &str, owner: &str) -> Result<u64, Error> {
    client
        .execute("CREATE DATABASE $1 WITH OWNER $2", &[&db, &owner])
        .await
}

pub async fn delete_db(client: &Client, db: &str) -> Result<u64, Error> {
    client.execute("DROP DATABASE $1", &[&db]).await
}
