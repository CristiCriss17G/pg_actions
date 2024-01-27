mod utils;

use clap::{Parser, Subcommand};
use rpassword::prompt_password;
use tokio_postgres::Error;
use utils::clone;
use utils::structs::PostgresCredentials;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Sets Postgres connection URL
    #[arg(short = 'H', long, env, global = true)]
    hostname: Option<String>,

    /// Sets Postgres username
    #[arg(short = 'U', long, env, global = true)]
    superuser: Option<String>,

    /// Sets Postgres password
    #[arg(short = 'P', long = "Password", env = "PG_PASS", global = true)]
    password: Option<String>,

    /// S3/Minio endpoint
    #[arg(long, env, global = true)]
    s3_endpoint: Option<String>,

    /// S3/Minio access key
    #[arg(long, env, global = true)]
    s3_access_key: Option<String>,

    /// S3/Minio secret key
    #[arg(long, env, global = true)]
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
    #[arg(short='D', long, global = true, action = clap::ArgAction::Count)]
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
async fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    // You can check the value provided by positional arguments, or option arguments
    if let Some(hostname) = cli.hostname.as_deref() {
        println!("Value for hostname: {}", hostname);
    }

    if let Some(username) = cli.superuser.as_deref() {
        println!("Value for username: {}", username);
    }

    // check if password is defined, if not prompt for it
    let mut pg_password = cli.password.as_deref().unwrap_or("").to_string();

    if pg_password == "" {
        match prompt_password("Postgres password: ") {
            Ok(password) => pg_password = password,
            Err(e) => {
                eprintln!("Failed to read password: {}", e);
                std::process::exit(1);
            }
        }
    }

    // You can see how many times a particular flag or argument occurred
    // Note, only flags can have multiple occurrences
    match cli.debug {
        0 => println!("Debug mode is off"),
        1 => println!("Debug mode is kind of on"),
        2 => println!("Debug mode is on"),
        _ => println!("Don't be crazy"),
    }

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Some(Commands::Clone(clone_data)) => {
            clone::clone_db(
                clone_data,
                PostgresCredentials {
                    hostname: cli.hostname.as_deref().unwrap_or("localhost").to_string(),
                    pg_superuser: cli.superuser.as_deref().unwrap_or("postgres").to_string(),
                    pg_password,
                },
            )
            .await?;
        }
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
