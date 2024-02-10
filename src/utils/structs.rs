use thiserror::Error;

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
    Io(#[from] std::io::Error),
    #[error("Postgres error: {0}")]
    Postgres(#[from] tokio_postgres::Error),
    #[error("Error: {0}")]
    Other(String),
}
