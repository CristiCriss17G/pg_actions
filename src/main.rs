mod utils;

use clap::{parser::ValueSource, Command, CommandFactory, FromArgMatches, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use log::{info, warn, LevelFilter};
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::sync::Arc;
use utils::credentials::{Passfile, SslMode};
use utils::pg_tools::{PGTools, ToolKind, ToolPaths};
use utils::structs::{Result, S3Credentials, SqlFileFormat};
use utils::{backup, clone, database, logging::log_init, restore, user};

#[derive(Parser, PartialEq)]
#[command(name = "pg_actions", author, version, about, long_about = None)]
/// A simple CLI tool for managing Postgres databases
struct Cli {
    /// Sets Postgres connection URL
    #[arg(
        short = 'H',
        long,
        env = "PG_HOSTNAME",
        global = true,
        default_value = "localhost"
    )]
    pg_hostname: String,

    /// Sets Postgres username
    #[arg(short = 'U', long, env = "PG_SUPERUSER", global = true)]
    pg_superuser: Option<String>,

    /// Sets Postgres password
    #[arg(
        short = 'P',
        long,
        env = "PG_PASS",
        global = true,
        hide_env_values = true
    )]
    pg_password: Option<String>,

    /// Sets Postgres port
    #[arg(short = 'p', long, env = "PG_PORT", global = true)]
    pg_port: Option<u16>,

    /// Optional custom pg_dump path
    /// If not set, the default from PATH will be used
    #[arg(long, env, global = true)]
    pg_dump: Option<PathBuf>,

    /// Optional custom pg_restore path
    /// If not set, the default from PATH will be used
    #[arg(long, env, global = true)]
    pg_restore: Option<PathBuf>,

    /// Optional custom psql path
    /// If not set, the default from PATH will be used
    #[arg(long, env, global = true)]
    psql: Option<PathBuf>,

    /// PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3
    #[arg(
        long,
        env = "PG_SSL_MODE",
        global = true,
        value_enum,
        default_value = "prefer"
    )]
    pg_ssl_mode: SslMode,

    /// PEM CA certificate bundle for PostgreSQL TLS
    #[arg(long, env = "PG_SSL_ROOT_CERT", global = true)]
    pg_ssl_root_cert: Option<PathBuf>,

    /// Optional S3 session token for temporary credentials
    #[arg(long, env, global = true, hide_env_values = true)]
    s3_session_token: Option<String>,

    /// S3/RustFS endpoint
    #[arg(long, env, global = true)]
    s3_endpoint: Option<String>,

    /// S3/RustFS access key
    #[arg(long, env, global = true, hide_env_values = true)]
    s3_access_key: Option<String>,

    /// S3/RustFS secret key
    #[arg(long, env, global = true, hide_env_values = true)]
    s3_secret_key: Option<String>,

    /// S3/RustFS bucket
    #[arg(long, env, global = true)]
    s3_bucket: Option<String>,

    /// S3/RustFS region
    #[arg(long, env, global = true)]
    s3_region: Option<String>,

    /// S3/RustFS bucket prefix/folder
    #[arg(long, env, global = true)]
    s3_prefix: Option<String>,

    /// Turn debugging information on
    /// repetitive use increases verbosity, at most 2 times
    #[arg(short='v', long="verbose", global = true, action = clap::ArgAction::Count)]
    debug: u8,

    /// Show logging information as json
    #[arg(long, env, global = true)]
    use_json_logging: bool,

    /// Log file location
    #[arg(long, env, global = true)]
    log_file: Option<PathBuf>,

    /// Subcommands
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, PartialEq)]
enum Commands {
    /// Clone a database
    #[clap(visible_alias("cl"))]
    Clone(clone::CloneArgs),
    /// Backup operations
    Backup(backup::BackupArgs),
    /// Restore operations
    Restore(restore::RestoreArgs),
    /// User operations
    User(user::UserArgs),
    /// Database operations
    Database(database::DatabaseArgs),
    /// Generate shell completions
    Completions {
        /// The shell to generate the script for
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Do a basic check of the tools
    /// This is useful for CI/CD pipelines
    #[clap(visible_alias("ct"))]
    CheckTools,
}

fn print_completions<G: Generator>(generator: G, command: &mut Command) {
    generate(
        generator,
        command,
        command.get_name().to_string(),
        &mut io::stdout(),
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Cli::command().get_matches();
    let implicit_ssl_mode = matches.value_source("pg_ssl_mode") == Some(ValueSource::DefaultValue);
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());
    if let Commands::Completions { shell } = &cli.command {
        print_completions(*shell, &mut Cli::command());
        return Ok(());
    }
    let level = match cli.debug {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };
    log_init(level, cli.use_json_logging, cli.log_file.as_deref())?;
    let paths = ToolPaths {
        dump: cli.pg_dump.clone().unwrap_or_else(|| "pg_dump".into()),
        restore: cli
            .pg_restore
            .clone()
            .unwrap_or_else(|| "pg_restore".into()),
        psql: cli.psql.clone().unwrap_or_else(|| "psql".into()),
    };
    let required: &[ToolKind] = match &cli.command {
        Commands::CheckTools => &[ToolKind::Dump, ToolKind::Restore, ToolKind::Psql],
        Commands::Backup(_) => &[ToolKind::Dump],
        Commands::Clone(_) => &[ToolKind::Dump, ToolKind::Restore],
        Commands::Restore(data) if data.format() == SqlFileFormat::Sql => &[ToolKind::Psql],
        Commands::Restore(_) => &[ToolKind::Restore],
        _ => &[],
    };
    let tools = PGTools::check(&paths, required).await?;
    if matches!(cli.command, Commands::CheckTools) {
        info!("All requested tools are available");
        return Ok(());
    }
    if implicit_ssl_mode {
        warn!("PostgreSQL TLS defaults to prefer (no certificate/hostname verification; non-TLS fallback). In v3.0 the default will change to verify-full. Set --pg-ssl-mode or PG_SSL_MODE explicitly to select your policy.");
    }
    let passfile = Arc::new(Passfile::read().await?);
    let auth_database = match &cli.command {
        Commands::Clone(data) => data.source_database(),
        _ => "postgres",
    };
    let mut credentials = passfile.resolve(
        cli.pg_hostname.clone(),
        cli.pg_superuser.clone(),
        cli.pg_port,
        cli.pg_password
            .clone()
            .or_else(|| std::env::var("PGPASSWORD").ok()),
        auth_database,
    );
    credentials.ssl_mode = cli.pg_ssl_mode;
    credentials.ssl_root_cert = cli.pg_ssl_root_cert.clone();
    if credentials.password_for(auth_database).is_none() && io::stdin().is_terminal() {
        credentials.password = Some(rpassword::prompt_password("Postgres password: ")?);
    }
    let mut s3 = S3Credentials::new(
        cli.s3_endpoint,
        cli.s3_access_key,
        cli.s3_secret_key,
        cli.s3_bucket,
        cli.s3_region,
        cli.s3_prefix,
    );
    s3.s3_session_token = cli.s3_session_token;
    match &cli.command {
        Commands::Clone(data) => clone::clone_db(data, &credentials, &tools).await?,
        Commands::Backup(data) => backup::backup_db(data, &credentials, &s3, &tools).await?,
        Commands::Restore(data) => restore::restore_db(data, &credentials, &s3, &tools).await?,
        Commands::User(data) => user::user(data, &credentials).await?,
        Commands::Database(data) => database::database(data, &credentials).await?,
        Commands::CheckTools | Commands::Completions { .. } => {}
    }
    info!("Operation completed successfully");
    Ok(())
}
