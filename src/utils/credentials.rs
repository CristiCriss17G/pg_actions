use super::files::private_open;
use super::structs::{PGCliError, Result};
use clap::ValueEnum;
use log::warn;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum SslMode {
    /// No TLS; intended for a trusted local/test connection.
    Disable,
    /// Use TLS without certificate/hostname verification when available; otherwise connect without TLS.
    #[default]
    Prefer,
    /// Require encryption without verifying the server certificate/hostname.
    Require,
    /// Require encryption and verify certificate trust and hostname.
    VerifyFull,
}

impl SslMode {
    pub fn libpq(self) -> &'static str {
        match self {
            Self::Disable => "disable",
            Self::Prefer => "prefer",
            Self::Require => "require",
            Self::VerifyFull => "verify-full",
        }
    }
}

#[derive(Clone, Default)]
pub struct Passfile {
    entries: Vec<[String; 5]>,
}

impl Passfile {
    pub fn parse(contents: &str) -> Self {
        let mut entries = Vec::new();
        for (number, line) in contents.lines().enumerate() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = vec![String::new()];
            let mut escaped = false;
            for character in line.chars() {
                if escaped {
                    fields
                        .last_mut()
                        .expect("a field is always present")
                        .push(character);
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == ':' {
                    fields.push(String::new());
                } else {
                    fields
                        .last_mut()
                        .expect("a field is always present")
                        .push(character);
                }
            }
            if escaped
                || fields.len() != 5
                || (fields[1] != "*" && fields[1].parse::<u16>().is_err())
            {
                warn!("Ignoring invalid passfile entry at line {}", number + 1);
                continue;
            }
            // The parser never logs the input line, including on errors.
            if let Ok(fields) = fields.try_into() {
                entries.push(fields);
            }
        }
        Self { entries }
    }

    pub async fn read() -> Result<Self> {
        let explicit = std::env::var_os("PGPASSFILE");
        let path = if let Some(path) = &explicit {
            Some(PathBuf::from(path))
        } else {
            default_passfile()
        };
        let Some(path) = path else {
            return Ok(Self::default());
        };
        let metadata = match tokio::fs::metadata(&path).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && explicit.is_none() => {
                return Ok(Self::default());
            }
            Err(error) => return Err(error.into()),
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(PGCliError::Other(
                    "PostgreSQL passfile must have owner-only permissions (0600)".into(),
                ));
            }
        }
        #[cfg(not(unix))]
        let _ = metadata;
        Ok(Self::parse(&tokio::fs::read_to_string(path).await?))
    }

    fn matches(field: &str, value: &str) -> bool {
        field == "*" || field == value
    }

    pub fn password(&self, host: &str, port: u16, database: &str, user: &str) -> Option<&str> {
        let host = passfile_host(host);
        self.entries
            .iter()
            .find(|entry| {
                Self::matches(&entry[0], host)
                    && Self::matches(&entry[1], &port.to_string())
                    && Self::matches(&entry[2], database)
                    && Self::matches(&entry[3], user)
            })
            .map(|entry| entry[4].as_str())
    }

    pub fn resolve(
        self: &Arc<Self>,
        host: String,
        user: Option<String>,
        port: Option<u16>,
        password: Option<String>,
        database: &str,
    ) -> PostgresCredentials {
        let inferred = self.entries.iter().find(|entry| {
            Self::matches(&entry[0], passfile_host(&host))
                && Self::matches(&entry[2], database)
                && user.as_ref().is_none_or(|u| Self::matches(&entry[3], u))
                && port.is_none_or(|p| Self::matches(&entry[1], &p.to_string()))
        });
        let pg_superuser = user
            .or_else(|| inferred.filter(|e| e[3] != "*").map(|e| e[3].clone()))
            .unwrap_or_else(|| "postgres".into());
        let pg_port = port
            .or_else(|| inferred.and_then(|e| e[1].parse().ok()))
            .unwrap_or(5432);
        PostgresCredentials {
            pg_hostname: host,
            pg_superuser,
            pg_port,
            password,
            passfile: self.clone(),
            ssl_mode: SslMode::VerifyFull,
            ssl_root_cert: None,
        }
    }
}

fn default_passfile() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        dirs::config_dir().map(|dir| dir.join("postgresql/pgpass.conf"))
    }
    #[cfg(not(windows))]
    {
        dirs::home_dir().map(|dir| dir.join(".pgpass"))
    }
}

fn passfile_host(host: &str) -> &str {
    #[cfg(unix)]
    if host.starts_with('/') {
        return "localhost";
    }
    host
}

#[derive(Clone)]
pub struct PostgresCredentials {
    pub pg_hostname: String,
    pub pg_superuser: String,
    pub pg_port: u16,
    pub password: Option<String>,
    pub passfile: Arc<Passfile>,
    pub ssl_mode: SslMode,
    pub ssl_root_cert: Option<PathBuf>,
}

impl fmt::Debug for PostgresCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PostgresCredentials")
            .field("host", &self.pg_hostname)
            .field("user", &self.pg_superuser)
            .field("port", &self.pg_port)
            .field("password", &"[REDACTED]")
            .field("ssl_mode", &self.ssl_mode)
            .finish()
    }
}

impl PostgresCredentials {
    pub fn password_for(&self, database: &str) -> Option<&str> {
        self.password.as_deref().or_else(|| {
            self.passfile.password(
                passfile_host(&self.pg_hostname),
                self.pg_port,
                database,
                &self.pg_superuser,
            )
        })
    }

    pub fn child_passfile(&self, database: &str) -> Result<tempfile::TempDir> {
        let directory = super::files::workspace("pg-actions-auth-")?;
        let path = directory.path().join("pgpass");
        let mut file = private_open(&path, false)?;
        if let Some(password) = self.password_for(database) {
            let fields = [
                passfile_host(&self.pg_hostname),
                &self.pg_port.to_string(),
                database,
                &self.pg_superuser,
                password,
            ];
            if fields
                .iter()
                .any(|value| value.contains(['\n', '\r', '\0']))
            {
                return Err(PGCliError::Other(
                    "Connection credentials cannot contain newline or NUL in a passfile".into(),
                ));
            }
            let line = fields
                .map(|value| value.replace('\\', "\\\\").replace(':', "\\:"))
                .join(":");
            writeln!(file, "{line}")?;
        }
        Ok(directory)
    }

    pub fn configure_child(&self, command: &mut Command, database: &str, passfile: &Path) {
        // Never let inherited libpq settings override the resolved connection.
        for name in [
            "PGPASSWORD",
            "PGSERVICE",
            "PGSERVICEFILE",
            "PGHOSTADDR",
            "PGOPTIONS",
            "PGSSLCERT",
            "PGSSLKEY",
            "PGSSLROOTCERT",
            "PGSSLCRL",
            "PGSSLCRLDIR",
            "PGSSLMODE",
            "PGREQUIRESSL",
        ] {
            command.env_remove(name);
        }
        command
            .arg("--no-password")
            .arg("--host")
            .arg(&self.pg_hostname)
            .arg("--port")
            .arg(self.pg_port.to_string())
            .arg("--username")
            .arg(&self.pg_superuser)
            // An explicit dbname makes pg_restore connect rather than emit SQL.
            // Quote it as a conninfo value so names containing '=' or URI syntax
            // cannot replace the resolved host/user/password.
            .arg("--dbname")
            .arg(format!(
                "dbname='{}'",
                database.replace('\\', "\\\\").replace('\'', "\\'")
            ))
            .env("PGDATABASE", database)
            .env("PGPASSFILE", passfile)
            .env("PGSSLMODE", self.ssl_mode.libpq())
            // libpq otherwise prefers GSS encryption over TLS when available.
            .env("PGGSSENCMODE", "disable")
            .env("PGAPPNAME", "pg_actions");
        if let Some(certificate) = &self.ssl_root_cert {
            command.env("PGSSLROOTCERT", certificate);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_matching_escaping_and_explicit_precedence() {
        let passfile = Arc::new(Passfile::parse(
            "#comment\nhost:5433:db:other:other-pass\nhost:*:db:user:colon\\:and\\\\slash\nhost:5432:*:user:last\ninvalid-secret\nhost:broken:*:user:secret\n",
        ));
        assert_eq!(
            passfile.password("host", 5432, "db", "user"),
            Some("colon:and\\slash")
        );
        assert_eq!(
            passfile.password("host", 5432, "otherdb", "user"),
            Some("last")
        );
        let credentials = passfile.resolve(
            "host".into(),
            Some("user".into()),
            Some(5432),
            Some("explicit".into()),
            "db",
        );
        assert_eq!(credentials.password_for("db"), Some("explicit"));
        assert!(!format!("{credentials:?}").contains("explicit"));
        let inferred = passfile.resolve("host".into(), None, None, None, "db");
        assert_eq!(inferred.pg_superuser, "other");
        assert_eq!(inferred.pg_port, 5433);
    }

    #[test]
    fn temporary_passfile_round_trips_without_writing_user_file() {
        let credentials = Arc::new(Passfile::default()).resolve(
            "host".into(),
            Some("user".into()),
            Some(5432),
            Some("p:a\\ss".into()),
            "db",
        );
        let directory = credentials.child_passfile("db").unwrap();
        let path = directory.path().join("pgpass");
        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            Passfile::parse(&contents).password("host", 5432, "db", "user"),
            Some("p:a\\ss")
        );
        drop(directory);
        assert!(!path.exists());
    }
}
