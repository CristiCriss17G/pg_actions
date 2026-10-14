use super::db::database::{
    change_whole_owner_of_db, create_db, delete_db, kill_connections_to_db, list_databases,
};
use super::db::general::postgres_connect;
use super::extension;
use super::structs::{OutputFormat, PostgresCredentials, Result, SortingOrder};
use clap::{Args, Subcommand};
use log::{debug, info};

#[derive(Args, Debug, PartialEq)]
pub struct DatabaseArgs {
    /// action subcommand
    #[command(subcommand)]
    subcommand: DatabaseSubCommands,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum DatabaseSubCommands {
    /// List databases
    List {
        /// sort by database name
        #[arg(long, value_enum)]
        sort: Option<SortingOrder>,
        /// quiet mode
        #[arg(short, long, value_enum, default_value = "table")]
        output: OutputFormat,
        /// extra details
        #[arg(short, long)]
        extra: bool,
        /// query string, fuzzy search
        #[arg(short, long)]
        query: Option<String>,
    },
    /// Create a database
    Create {
        /// database name
        database: String,
        /// owner user for db
        #[arg(short = 'o', long)]
        owner: Option<String>,
    },
    /// Delete a database
    Delete {
        /// database name
        database: String,
    },
    /// Update a database
    Update {
        /// database name
        database: String,
        /// new owner user for db
        #[arg(short = 'o', long, required = true)]
        new_owner: Option<String>,
    },
    /// Manipulate extensions on a database
    #[clap(visible_alias("ext"))]
    Extension(extension::ExtensionArgs),
}

pub async fn database(data: &DatabaseArgs, main_credentials: &PostgresCredentials) -> Result<()> {
    let client = postgres_connect(main_credentials, None).await?;
    match &data.subcommand {
        DatabaseSubCommands::List {
            sort,
            output,
            extra,
            query,
        } => {
            debug!("List databases");
            database_list(&client, sort, output, *extra, query).await?;
        }
        DatabaseSubCommands::Create { database, owner } => {
            debug!("Create database {}", database);
            let owner = owner.as_deref();
            create_db(&client, database, owner).await?;
            info!("Database {} created", database);
        }
        DatabaseSubCommands::Delete { database } => {
            debug!("Delete database {}", database);
            kill_connections_to_db(&client, database).await?;
            delete_db(&client, database).await?;
            info!("Database {} deleted", database);
        }
        DatabaseSubCommands::Update {
            database,
            new_owner,
        } => {
            debug!("Update database {}", database);
            let new_owner = new_owner.as_deref().ok_or_else(|| {
                super::structs::PGCliError::Other("Database update requires --new-owner".into())
            })?;
            change_whole_owner_of_db(&client, main_credentials, database, new_owner).await?;
            info!("Database {} updated", database);
        }
        DatabaseSubCommands::Extension(extension_data) => {
            extension::extension(extension_data, main_credentials).await?;
        }
    }
    Ok(())
}

async fn database_list(
    client: &deadpool_postgres::Client,
    sort: &Option<SortingOrder>,
    output_format: &OutputFormat,
    extra: bool,
    query: &Option<String>,
) -> Result<()> {
    let databases = list_databases(client, sort, extra, query).await?;
    super::output::render(&databases, output_format, extra, std::io::stdout().lock())
}
