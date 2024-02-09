mod utils;

use clap::{Parser, Subcommand};
use env_logger::{Builder, Env};
use log::{error, info, trace, LevelFilter};
use rpassword::prompt_password;
use std::io::Write;
use utils::clone;
use utils::db::try_read_pgpass;
use utils::structs::{PGCliError, PostgresCredentials};

#[derive(Parser)]
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
    hostname: Option<String>,

    /// Sets Postgres username
    #[arg(
        short = 'U',
        long,
        env = "PG_SUPERUSER",
        global = true,
        default_value = "postgres"
    )]
    superuser: Option<String>,

    /// Sets Postgres password
    #[arg(
        short = 'P',
        long = "Password",
        env = "PG_PASS",
        global = true,
        hide_env_values = true,
        default_value = "postgres"
    )]
    password: Option<String>,

    /// Sets Postgres port
    #[arg(
        short = 'p',
        long,
        env = "PG_PORT",
        global = true,
        default_value = "5432"
    )]
    port: Option<u16>,

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

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Clone a database
    Clone(clone::CloneArgs),
    /// Backup operations
    Backup {
        /// target database
        /// default: all
        target_database: Option<String>,
    },
    /// User operations
    User {
        /// action subcommand
        #[command(subcommand)]
        subcommand: Option<UserSubCommands>,
    },
}

#[derive(Subcommand)]
enum UserSubCommands {
    /// List users
    List,
    /// Create a user
    Create {
        /// username
        #[arg(short, long)]
        username: String,
        /// password
        #[arg(short, long)]
        password: String,
        /// superuser
        #[arg(long)]
        superuser: bool,
        /// createdb
        #[arg(long)]
        createdb: bool,
        /// createrole
        #[arg(long)]
        createrole: bool,
        /// login
        #[arg(short, long)]
        login: bool,
    },
    /// Delete a user
    Delete {
        /// username
        #[arg(short, long)]
        username: String,
    },
    /// Update a user
    Update {
        /// username
        #[arg(short, long)]
        username: String,
        /// password
        #[arg(short, long)]
        password: String,
        /// superuser
        #[arg(long)]
        superuser: bool,
        /// createdb
        #[arg(long)]
        createdb: bool,
        /// createrole
        #[arg(long)]
        createrole: bool,
        /// login
        #[arg(short, long)]
        login: bool,
    },
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

    // try to read the pgpass file if Some is returned store tha values in a variable, else read them from the cli, and promt for password if not provided
    let mut pgpass: PostgresCredentials = try_read_pgpass().unwrap_or(PostgresCredentials {
        pg_hostname: cli.hostname.as_deref().unwrap_or("localhost").to_string(),
        pg_superuser: cli.superuser.as_deref().unwrap_or("postgres").to_string(),
        pg_password: cli.password.as_deref().unwrap_or("").to_string(),
        pg_port: cli.port.unwrap_or(5432),
    });

    if pgpass.pg_password == "" {
        match prompt_password("Postgres password: ") {
            Ok(password) => pgpass.pg_password = password,
            Err(e) => {
                eprintln!("Failed to read password: {}", e);
                std::process::exit(1);
            }
        }
    }

    trace!("Using pgpass: {:?}", pgpass);

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Some(Commands::Clone(clone_data)) => match clone::clone_db(clone_data, &pgpass).await {
            Ok(_) => info!("Database cloned successfully"),
            Err(e) => {
                error!("Failed to clone database: {}", e);
                return Err(e);
            }
        },
        Some(Commands::Backup { target_database }) => {
            println!(
                "Backing up database {}",
                target_database.as_deref().unwrap_or("all").to_string()
            );
        }
        Some(Commands::User { subcommand }) => match subcommand {
            Some(UserSubCommands::List) => {
                println!("Listing users...");
            }
            Some(UserSubCommands::Create {
                username,
                password: _,
                superuser,
                createdb,
                createrole,
                login,
            }) => {
                println!("Creating user {}...", username);
                println!("Superuser: {}", superuser);
                println!("Createdb: {}", createdb);
                println!("Createrole: {}", createrole);
                println!("Login: {}", login);
            }
            Some(UserSubCommands::Delete { username }) => {
                println!("Deleting user {}...", username);
            }
            Some(UserSubCommands::Update {
                username,
                password: _,
                superuser,
                createdb,
                createrole,
                login,
            }) => {
                println!("Updating user {}...", username);
                println!("Superuser: {}", superuser);
                println!("Createdb: {}", createdb);
                println!("Createrole: {}", createrole);
                println!("Login: {}", login);
            }
            None => {}
        },
        None => {}
    }

    Ok(())

    // Continued program logic goes here...
}
