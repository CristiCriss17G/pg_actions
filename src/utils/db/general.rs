use crate::utils::credentials::SslMode;
use crate::utils::files::{parent, private_open};
use crate::utils::pg_tools::ToolKind;
use crate::utils::structs::{
    PGCliError, PGTools, PgDumpCompression, PostgresCredentials, Result, SqlFileFormat,
};
use deadpool_postgres::{Client, Config as DPConfig, ManagerConfig, RecyclingMethod, Runtime};
use native_tls::{Certificate, TlsConnector};
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher,
};
use postgres_native_tls::MakeTlsConnector;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

pub async fn postgres_connect(
    credentials: &PostgresCredentials,
    db: Option<String>,
) -> Result<Client> {
    let database = db.as_deref().unwrap_or("postgres");
    let mut builder = TlsConnector::builder();
    if matches!(credentials.ssl_mode, SslMode::Prefer | SslMode::Require) {
        builder
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true);
    }
    if let Some(path) = &credentials.ssl_root_cert {
        let pem = tokio::fs::read(path).await?;
        // Parse complete PEM certificate blocks, allowing the comments/blank
        // lines commonly present in CA bundles.
        let bundle = std::str::from_utf8(&pem)
            .map_err(|_| PGCliError::ConnectionError("Invalid PEM certificate bundle".into()))?;
        let mut count = 0;
        for block in bundle.split_inclusive("-----END CERTIFICATE-----") {
            if let Some(start) = block.find("-----BEGIN CERTIFICATE-----") {
                let certificate =
                    Certificate::from_pem(&block.as_bytes()[start..]).map_err(|_| {
                        PGCliError::ConnectionError(
                            "Invalid PEM certificate in --pg-ssl-root-cert".into(),
                        )
                    })?;
                builder.add_root_certificate(certificate);
                count += 1;
            }
        }
        if count == 0 {
            return Err(PGCliError::ConnectionError(
                "CA bundle contains no PEM certificates".into(),
            ));
        }
    }
    let connector =
        MakeTlsConnector::new(builder.build().map_err(|_| {
            PGCliError::ConnectionError("Unable to initialize PostgreSQL TLS".into())
        })?);
    let mut cfg = DPConfig::new();
    cfg.host = Some(credentials.pg_hostname.clone());
    cfg.port = Some(credentials.pg_port);
    cfg.user = Some(credentials.pg_superuser.clone());
    cfg.password = credentials.password_for(database).map(str::to_owned);
    cfg.dbname = Some(database.to_owned());
    cfg.application_name = Some("pg_actions".into());
    cfg.ssl_mode = Some(match credentials.ssl_mode {
        SslMode::Disable => deadpool_postgres::SslMode::Disable,
        SslMode::Prefer => deadpool_postgres::SslMode::Prefer,
        _ => deadpool_postgres::SslMode::Require,
    });
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    let pool = cfg.create_pool(Some(Runtime::Tokio1), connector)?;
    Ok(pool.get().await?)
}

async fn run_child(mut command: Command, operation: &str) -> Result<String> {
    // Output is deliberately not echoed: PostgreSQL diagnostics may include
    // passwords, SQL values and restored data. Exit status still propagates.
    let output = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .status()
        .await?;
    if output.success() {
        Ok("Success".into())
    } else {
        Err(PGCliError::Other(format!(
            "{operation} failed ({output}); database changes were not published"
        )))
    }
}

pub async fn db_dump<P: AsRef<Path>>(
    credentials: PostgresCredentials,
    database: &str,
    dump_file: P,
    tools: &PGTools,
    format: SqlFileFormat,
    compression: PgDumpCompression,
) -> Result<String> {
    let path = dump_file.as_ref();
    tokio::fs::create_dir_all(parent(path)).await?;
    let passfile = credentials.child_passfile(database)?;
    let mut command = tools.command(ToolKind::Dump)?;
    credentials.configure_child(&mut command, database, &passfile.path().join("pgpass"));
    command
        .arg(format!("--format={format}"))
        .arg("--no-owner")
        .arg("--no-privileges");
    if compression == PgDumpCompression::None {
        command.arg("--compress=0");
    }
    // Use a private temporary output and publish only after a successful dump.
    let temporary = tempfile::NamedTempFile::new_in(parent(path))?;
    let file = private_open(temporary.path(), false)?;
    let status = command
        .stdin(Stdio::null())
        .stdout(Stdio::from(file))
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .status()
        .await?;
    if !status.success() {
        return Err(PGCliError::Other(format!("pg_dump failed ({status})")));
    }
    temporary
        .persist(path)
        .map_err(|error| PGCliError::TokioIo(error.error))?;
    Ok("Success".into())
}

pub async fn db_restore<P: AsRef<Path>>(
    credentials: PostgresCredentials,
    database: &str,
    dump_file: P,
    tools: &PGTools,
    format: SqlFileFormat,
) -> Result<String> {
    let passfile = credentials.child_passfile(database)?;
    let mut command = tools.command(match format {
        SqlFileFormat::Sql => ToolKind::Psql,
        SqlFileFormat::Bsql => ToolKind::Restore,
    })?;
    credentials.configure_child(&mut command, database, &passfile.path().join("pgpass"));
    match format {
        SqlFileFormat::Sql => {
            command
                .arg("-X")
                .arg("--set=ON_ERROR_STOP=1")
                .arg("--file")
                .arg(dump_file.as_ref());
        }
        SqlFileFormat::Bsql => {
            command
                .arg("--exit-on-error")
                .arg("--no-owner")
                .arg("--no-privileges")
                .arg(dump_file.as_ref());
        }
    }
    run_child(command, "restore").await
}

pub(super) fn filter_entities<T: AsRef<str>>(
    entities: impl IntoIterator<Item = T>,
    query: &str,
) -> Vec<T> {
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut matches = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart)
        .match_list(entities, &mut matcher);
    matches.sort_by_key(|a| std::cmp::Reverse(a.1));
    matches.into_iter().map(|(db, _)| db).collect()
}
