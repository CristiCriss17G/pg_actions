use clap::ValueEnum;
use csv::{self, Writer};
use deadpool_postgres::{CreatePoolError, PoolError};
use log::SetLoggerError;
use rusoto_core::RusotoError;
use rusoto_s3::{CompleteMultipartUploadError, CreateMultipartUploadError, UploadPartError};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fmt;
use std::str::FromStr;
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
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

#[derive(Debug, Clone, Hash, PartialEq, Eq, Default)]
pub struct PGTools {
    pub pg_dump: String,
    pub pg_restore: String,
}

impl PGTools {
    pub fn new(pg_dump: String, pg_restore: String) -> PGTools {
        PGTools {
            pg_dump,
            pg_restore,
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
    #[error("Tokio I/O error: {0}")]
    TokioIo(#[from] tokio::io::Error),
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
    #[error("Serde Json Error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Serde CSV Error: {0}")]
    SerdeCsv(#[from] csv::Error),
    #[error("Serde CSV Writer Error: {0}")]
    SerdeCsvWriter(#[from] Box<csv::IntoInnerError<Writer<Vec<u8>>>>),
    #[error("String UTF8 Error: {0}")]
    StringUtf8(#[from] std::string::FromUtf8Error),
    #[error("Set Logger Error: {0}")]
    SetLoggerError(#[from] SetLoggerError),
    #[error("Pool Error: {0}")]
    PoolError(#[from] PoolError),
    #[error("Create Pool Error: {0}")]
    CreatePoolError(#[from] CreatePoolError),
    #[error("ConnectionError: {0}")]
    ConnectionError(String),
    #[error("PGToolsError: {0}")]
    PGToolsError(String),
    #[error("Pool Error Tokio: {0}")]
    PoolErrorTokio(String),
    #[error("Error: {0}")]
    Other(String),
}

/// A `Result` alias where the `Err` case is `ClodociError`.
pub type Result<T> = std::result::Result<T, PGCliError>;

#[derive(Debug, Clone, Deserialize, Serialize)]
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

impl AsRef<str> for UserDetails {
    fn as_ref(&self) -> &str {
        match self {
            UserDetails::Extra { username, .. } => username,
            UserDetails::Username(username) => username,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum DatabaseDetails {
    Extra {
        name: String,
        owner: String,
        size_pretty: String,
        oid: u32,
    },
    Name(String),
}

impl AsRef<str> for DatabaseDetails {
    fn as_ref(&self) -> &str {
        match self {
            DatabaseDetails::Extra { name, .. } => name,
            DatabaseDetails::Name(name) => name,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ExtensionDetails {
    Extra {
        name: String,
        owner: u32,
        version: String,
        oid: u32,
    },
    Name(String),
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum UserPrivileges {
    Full,
    ReadOnly,
    ReadUpdateOnly,
}

impl FromStr for UserPrivileges {
    type Err = PGCliError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "full" => Ok(UserPrivileges::Full),
            "read-only" => Ok(UserPrivileges::ReadOnly),
            "read-update-only" => Ok(UserPrivileges::ReadUpdateOnly),
            _ => Err(PGCliError::Other(format!("Invalid privilege: {}", s))),
        }
    }
}

impl fmt::Display for UserPrivileges {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UserPrivileges::Full => write!(f, "full"),
            UserPrivileges::ReadOnly => write!(f, "read-only"),
            UserPrivileges::ReadUpdateOnly => write!(f, "read-update-only"),
        }
    }
}

impl ValueEnum for UserPrivileges {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            UserPrivileges::Full,
            UserPrivileges::ReadOnly,
            UserPrivileges::ReadUpdateOnly,
        ]
    }

    fn to_possible_value<'a>(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            UserPrivileges::Full => clap::builder::PossibleValue::new("full"),
            UserPrivileges::ReadOnly => clap::builder::PossibleValue::new("read-only"),
            UserPrivileges::ReadUpdateOnly => clap::builder::PossibleValue::new("read-update-only"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum SortingOrder {
    Ascending,
    Descending,
}

impl FromStr for SortingOrder {
    type Err = PGCliError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "asc" => Ok(SortingOrder::Ascending),
            "desc" => Ok(SortingOrder::Descending),
            _ => Err(PGCliError::Other(format!("Invalid sorting order: {}", s))),
        }
    }
}

impl fmt::Display for SortingOrder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SortingOrder::Ascending => write!(f, "asc"),
            SortingOrder::Descending => write!(f, "desc"),
        }
    }
}

impl ValueEnum for SortingOrder {
    fn value_variants<'a>() -> &'a [Self] {
        &[SortingOrder::Ascending, SortingOrder::Descending]
    }

    fn to_possible_value<'a>(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            SortingOrder::Ascending => clap::builder::PossibleValue::new("asc"),
            SortingOrder::Descending => clap::builder::PossibleValue::new("desc"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum OutputFormat {
    Json,
    JsonCompact,
    JsonLines,
    Csv,
    Table,
    Simple,
}

impl FromStr for OutputFormat {
    type Err = PGCliError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "json" => Ok(OutputFormat::Json),
            "json-compact" => Ok(OutputFormat::JsonCompact),
            "json-lines" => Ok(OutputFormat::JsonLines),
            "csv" => Ok(OutputFormat::Csv),
            "table" => Ok(OutputFormat::Table),
            "simple" => Ok(OutputFormat::Simple),
            _ => Err(PGCliError::Other(format!("Invalid output format: {}", s))),
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::JsonCompact => write!(f, "json-compact"),
            OutputFormat::JsonLines => write!(f, "json-lines"),
            OutputFormat::Csv => write!(f, "csv"),
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Simple => write!(f, "simple"),
        }
    }
}

impl ValueEnum for OutputFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            OutputFormat::Json,
            OutputFormat::JsonCompact,
            OutputFormat::JsonLines,
            OutputFormat::Csv,
            OutputFormat::Table,
            OutputFormat::Simple,
        ]
    }

    fn to_possible_value<'a>(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            OutputFormat::Json => clap::builder::PossibleValue::new("json"),
            OutputFormat::JsonCompact => clap::builder::PossibleValue::new("json-compact"),
            OutputFormat::JsonLines => clap::builder::PossibleValue::new("json-lines"),
            OutputFormat::Csv => clap::builder::PossibleValue::new("csv"),
            OutputFormat::Table => clap::builder::PossibleValue::new("table"),
            OutputFormat::Simple => clap::builder::PossibleValue::new("simple"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum SqlFileFormat {
    Sql,
    Bsql,
}

impl FromStr for SqlFileFormat {
    type Err = PGCliError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "sql" => Ok(SqlFileFormat::Sql),
            "bsql" => Ok(SqlFileFormat::Bsql),
            _ => Err(PGCliError::Other(format!("Invalid sql file format: {}", s))),
        }
    }
}

impl fmt::Display for SqlFileFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SqlFileFormat::Sql => write!(f, "plain"),
            SqlFileFormat::Bsql => write!(f, "custom"),
        }
    }
}

impl ValueEnum for SqlFileFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[SqlFileFormat::Sql, SqlFileFormat::Bsql]
    }

    fn to_possible_value<'a>(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            SqlFileFormat::Sql => clap::builder::PossibleValue::new("sql"),
            SqlFileFormat::Bsql => clap::builder::PossibleValue::new("bsql"),
        })
    }
}
