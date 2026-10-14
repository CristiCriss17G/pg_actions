use clap::ValueEnum;
use csv::{self, Writer};
use deadpool_postgres::{CreatePoolError, PoolError};
use log::SetLoggerError;
use rusoto_core::RusotoError;
use rusoto_s3::{
    AbortMultipartUploadError, CompleteMultipartUploadError, CreateMultipartUploadError,
    GetObjectError, PutObjectError, UploadPartError,
};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fmt;
use std::str::FromStr;
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PgDumpCompression {
    /// Let pg_dump use its normal compression settings.
    Default,
    /// Disable pg_dump compression; the outer archive handles compression.
    None,
}

pub use super::credentials::PostgresCredentials;
pub use super::pg_tools::PGTools;

pub struct S3Credentials {
    pub s3_session_token: Option<String>,
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
            s3_session_token: None,
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
    RusotoCreateMultipartUploadError(#[source] Box<RusotoError<CreateMultipartUploadError>>),
    #[error("Rusoto UploadPartError Error: {0}")]
    RusotoUploadPartError(#[source] Box<RusotoError<UploadPartError>>),
    #[error("Rusoto CompleteMultipartUploadError Error: {0}")]
    RusotoCompleteMultipartUploadError(#[source] Box<RusotoError<CompleteMultipartUploadError>>),
    #[error("S3 abort error: {0}")]
    S3Abort(#[source] Box<RusotoError<AbortMultipartUploadError>>),
    #[error("S3 upload error: {0}")]
    S3Put(#[source] Box<RusotoError<PutObjectError>>),
    #[error("S3 download error: {0}")]
    S3Get(#[source] Box<RusotoError<GetObjectError>>),
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
    #[error("Error: {0}")]
    Other(String),
}

impl From<RusotoError<CreateMultipartUploadError>> for PGCliError {
    fn from(error: RusotoError<CreateMultipartUploadError>) -> Self {
        Self::RusotoCreateMultipartUploadError(Box::new(error))
    }
}
impl From<RusotoError<UploadPartError>> for PGCliError {
    fn from(error: RusotoError<UploadPartError>) -> Self {
        Self::RusotoUploadPartError(Box::new(error))
    }
}
impl From<RusotoError<CompleteMultipartUploadError>> for PGCliError {
    fn from(error: RusotoError<CompleteMultipartUploadError>) -> Self {
        Self::RusotoCompleteMultipartUploadError(Box::new(error))
    }
}

impl From<RusotoError<PutObjectError>> for PGCliError {
    fn from(error: RusotoError<PutObjectError>) -> Self {
        Self::S3Put(Box::new(error))
    }
}
impl From<RusotoError<GetObjectError>> for PGCliError {
    fn from(error: RusotoError<GetObjectError>) -> Self {
        Self::S3Get(Box::new(error))
    }
}

impl From<RusotoError<AbortMultipartUploadError>> for PGCliError {
    fn from(error: RusotoError<AbortMultipartUploadError>) -> Self {
        Self::S3Abort(Box::new(error))
    }
}

/// A `Result` alias where the `Err` case is `PGCliError`.
pub type Result<T> = std::result::Result<T, PGCliError>;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
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

// impl PartialEq for UserDetails with String
impl PartialEq<String> for UserDetails {
    fn eq(&self, other: &String) -> bool {
        match self {
            UserDetails::Extra { username, .. } => username == other,
            UserDetails::Username(username) => username == other,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
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

// impl PartialEq for DatabaseDetails with String
impl PartialEq<String> for DatabaseDetails {
    fn eq(&self, other: &String) -> bool {
        match self {
            DatabaseDetails::Extra { name, .. } => name == other,
            DatabaseDetails::Name(name) => name == other,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum ExtensionDetails {
    Extra {
        name: String,
        owner: u32,
        version: String,
        oid: u32,
    },
    Name(String),
}

impl AsRef<str> for ExtensionDetails {
    fn as_ref(&self) -> &str {
        match self {
            ExtensionDetails::Extra { name, .. } => name,
            ExtensionDetails::Name(name) => name,
        }
    }
}

// impl PartialEq for ExtensionDetails with String
impl PartialEq<String> for ExtensionDetails {
    fn eq(&self, other: &String) -> bool {
        match self {
            ExtensionDetails::Extra { name, .. } => name == other,
            ExtensionDetails::Name(name) => name == other,
        }
    }
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

impl SqlFileFormat {
    pub fn file_extension(&self) -> &str {
        match self {
            SqlFileFormat::Sql => "sql",
            SqlFileFormat::Bsql => "bsql",
        }
    }
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

#[cfg(test)]
mod error_tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn boxed_s3_errors_preserve_conversion_display_and_source() {
        let errors = [
            PGCliError::from(RusotoError::<CreateMultipartUploadError>::Validation(
                "create failure".into(),
            )),
            PGCliError::from(RusotoError::<UploadPartError>::Validation(
                "part failure".into(),
            )),
            PGCliError::from(RusotoError::<CompleteMultipartUploadError>::Validation(
                "complete failure".into(),
            )),
        ];
        for (error, expected) in
            errors
                .iter()
                .zip(["create failure", "part failure", "complete failure"])
        {
            assert!(error.to_string().contains(expected));
            assert_eq!(error.source().unwrap().to_string(), expected);
        }
        assert!(std::mem::size_of::<PGCliError>() <= 128);
    }
}
