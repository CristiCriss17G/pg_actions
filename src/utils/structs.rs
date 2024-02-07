use thiserror::Error;

pub struct PostgresCredentials {
    pub pg_hostname: String,
    pub pg_superuser: String,
    pub pg_password: String,
    pub pg_port: u16,
}

#[derive(Error, Debug)]
pub enum PGCliError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Postgres error: {0}")]
    Postgres(#[from] tokio_postgres::Error),
    #[error("Error: {0}")]
    Other(String),
}
