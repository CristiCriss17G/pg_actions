use thiserror::Error;
use tokio::task::JoinError;
#[derive(Debug, Clone)]
pub struct PostgresCredentials {
    pub pg_hostname: String,
    pub pg_superuser: String,
    pub pg_password: String,
    pub pg_port: u16,
}

pub struct S3Credentials {
    pub s3_endpoint: Option<String>,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
    pub s3_prefix: Option<String>,
}

#[derive(Error, Debug)]
pub enum PGCliError {
    #[error("I/O error: {0}")]
    Io(#[from] tokio::io::Error),
    #[error("Postgres error: {0}")]
    Postgres(#[from] tokio_postgres::Error),
    #[error("Task join error: {0}")]
    TaskJoin(#[from] JoinError),
    #[error("XZ Error: {0}")]
    Xz(#[from] xz2::stream::Error),
    #[error("Walkdir Error: {0}")]
    Walkdir(#[from] walkdir::Error),
    #[error("Path StripPrefixError Error: {0}")]
    StripPrefixError(#[from] std::path::StripPrefixError),
    #[error("Error: {0}")]
    Other(String),
}
