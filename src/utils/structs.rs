use rusoto_core::RusotoError;
use rusoto_s3::{CompleteMultipartUploadError, CreateMultipartUploadError, UploadPartError};
use thiserror::Error;
use tokio::task::JoinError;
#[derive(Debug, Clone)]
pub struct PostgresCredentials {
    pub pg_hostname: String,
    pub pg_superuser: String,
    pub pg_password: String,
    pub pg_port: u16,
}

impl PostgresCredentials {
    pub fn new(
        pg_hostname: String,
        pg_superuser: String,
        pg_password: String,
        pg_port: u16,
    ) -> PostgresCredentials {
        PostgresCredentials {
            pg_hostname,
            pg_superuser,
            pg_password,
            pg_port,
        }
    }
}

pub struct S3Credentials {
    pub s3_endpoint: Option<String>,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
    pub s3_prefix: Option<String>,
}

impl S3Credentials {
    pub fn new(
        s3_endpoint: Option<String>,
        s3_access_key: Option<String>,
        s3_secret_key: Option<String>,
        s3_bucket: Option<String>,
        s3_region: Option<String>,
        s3_prefix: Option<String>,
    ) -> S3Credentials {
        S3Credentials {
            s3_endpoint,
            s3_access_key,
            s3_secret_key,
            s3_bucket,
            s3_region,
            s3_prefix,
        }
    }
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
    #[error("Rusoto TLS Error: {0}")]
    RusotoTls(#[from] rusoto_core::request::TlsError),
    #[error("Rusoto CreateMultipartUploadError Error: {0}")]
    RusotoCreateMultipartUploadError(#[from] RusotoError<CreateMultipartUploadError>),
    #[error("Rusoto UploadPartError Error: {0}")]
    RusotoUploadPartError(#[from] RusotoError<UploadPartError>),
    #[error("Rusoto CompleteMultipartUploadError Error: {0}")]
    RusotoCompleteMultipartUploadError(#[from] RusotoError<CompleteMultipartUploadError>),
    #[error("Error: {0}")]
    Other(String),
}

#[derive(Debug, Clone)]
pub enum UserDetails {
    Extra {
        username: String,
        createdb: bool,
        superuser: bool,
        login: bool,
        oid: u32,
    },
    Username(String),
}

#[derive(Debug, Clone)]
pub enum DatabaseDetails {
    Extra {
        name: String,
        owner: String,
        oid: u32,
    },
    Name(String),
}
