mod utils;

use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use env_logger::{Builder, Env};
use log::{debug, error, info, trace, LevelFilter};
use rpassword::prompt_password;
use std::collections::HashMap;
use std::io;
use std::io::Write;
use utils::db::general::try_read_pgpass;
use utils::structs::{PGCliError, PostgresCredentials, S3Credentials};
use utils::{backup, clone, database, user};

#[derive(Parser, Debug, PartialEq)]
#[command(author, version, about, long_about = None)]
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
    #[arg(
        short = 'U',
        long,
        env = "PG_SUPERUSER",
        global = true,
        default_value = "postgres"
    )]
    pg_superuser: Option<String>,

    /// Sets Postgres password
    #[arg(
        short = 'P',
        long,
        env = "PG_PASS",
        global = true,
        hide_env_values = true,
        default_value = "postgres"
    )]
    pg_password: Option<String>,

    /// Sets Postgres port
    #[arg(
        short = 'p',
        long,
        env = "PG_PORT",
        global = true,
        default_value = "5432"
    )]
    pg_port: Option<u16>,

    /// S3/Minio endpoint
    #[arg(long, env, global = true)]
    s3_endpoint: Option<String>,

    /// S3/Minio access key
    #[arg(long, env, global = true)]
    s3_access_key: Option<String>,

    /// S3/Minio secret key
    #[arg(long, env, global = true, hide_env_values = true)]
    s3_secret_key: Option<String>,

    /// S3/Minio bucket
    #[arg(long, env, global = true)]
    s3_bucket: Option<String>,

    /// S3/Minio region
    #[arg(long, env, global = true)]
    s3_region: Option<String>,

    /// S3/Minio bucket prefix/folder
    #[arg(long, env, global = true)]
    s3_prefix: Option<String>,

    /// Turn debugging information on
    /// repetitive use increases verbosity, at most 2 times
    #[arg(short='v', long="verbose", global = true, action = clap::ArgAction::Count)]
    debug: u8,

    /// Subcommands
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug, PartialEq)]
enum Commands {
    /// Clone a database
    Clone(clone::CloneArgs),
    /// Backup operations
    Backup(backup::BackupArgs),
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
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

#[tokio::main]
async fn main() -> Result<(), PGCliError> {
    let cli = Cli::parse();

    // Initialize the logger
    let log_level = match cli.debug {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    if cfg!(debug_assertions) {
        Builder::from_env(Env::default())
            .format(|buf, record| {
                writeln!(
                    buf,
                    "{} [{}:{}:{}] - {}",
                    chrono::Local::now().format("%+"),
                    record.level(),
                    record.target(),
                    record.line().unwrap_or(0),
                    record.args()
                )
            })
            .filter_level(log_level)
            .init();
    } else {
        Builder::from_env(Env::default())
            .format(|buf, record| {
                writeln!(
                    buf,
                    "{} [{}] - {}",
                    chrono::Local::now().format("%+"),
                    record.level(),
                    record.args()
                )
            })
            .filter_level(log_level)
            .init();
    };

    info!("Starting up...");
    if log_level > LevelFilter::Info {
        info!("Debugging enabled to level {}", log_level);
    }

    let pg_main_hostname = cli.pg_hostname.clone();

    let mut pgpass = {
        let pgpass = try_read_pgpass().await;
        if !pgpass.is_empty() && pgpass.contains_key(&pg_main_hostname) {
            pgpass
        } else {
            let mut pgpassword = cli.pg_password.as_deref().unwrap_or("").to_string();
            if pgpassword == "" {
                match prompt_password("Postgres password: ") {
                    Ok(password) => pgpassword = password,
                    Err(e) => {
                        error!("Failed to read password: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            let mut new_pgpass = HashMap::from([(
                cli.pg_hostname.clone(),
                PostgresCredentials::new(
                    cli.pg_hostname.to_string(),
                    cli.pg_superuser
                        .as_deref()
                        .unwrap_or("postgres")
                        .to_string(),
                    pgpassword,
                    cli.pg_port.unwrap_or(5432),
                ),
            )]);
            if !pgpass.is_empty() {
                new_pgpass.extend(pgpass);
            }
            new_pgpass
        }
    };

    let s3_credentials = S3Credentials::new(
        cli.s3_endpoint,
        cli.s3_access_key,
        cli.s3_secret_key,
        cli.s3_bucket,
        cli.s3_region,
        cli.s3_prefix,
    );

    trace!("Using pgpass: {:?}", pgpass);

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Some(Commands::Clone(clone_data)) => {
            match clone::clone_db(clone_data, &mut pgpass, &pg_main_hostname).await {
                Ok(_) => info!("Database cloned successfully"),
                Err(e) => {
                    error!("Failed to clone database: {}", e);
                    return Err(e);
                }
            }
        }
        Some(Commands::Backup(backup_data)) => {
            match backup::backup_db(backup_data, &pgpass, &pg_main_hostname, &s3_credentials).await
            {
                Ok(_) => info!("Database backed up successfully"),
                Err(e) => {
                    error!("Failed to backup database: {}", e);
                    return Err(e);
                }
            }
        }
        Some(Commands::User(user_data)) => {
            match user::user(user_data, &pgpass, &pg_main_hostname).await {
                Ok(_) => info!("User operation completed successfully"),
                Err(e) => {
                    error!("Failed to perform user operation: {}", e);
                    return Err(e);
                }
            }
        }
        Some(Commands::Database(database_data)) => {
            match database::database(database_data, &pgpass, &pg_main_hostname).await {
                Ok(_) => info!("Database operation completed successfully"),
                Err(e) => {
                    error!("Failed to perform database operation: {}", e);
                    return Err(e);
                }
            }
        }
        Some(Commands::Completions { shell }) => {
            debug!("Generating completions for {:?}", shell);
            print_completions(*shell, &mut Cli::command());
        }
        None => {}
    }

    Ok(())
}
