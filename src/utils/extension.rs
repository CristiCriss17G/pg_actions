use super::db::extension::{create_extension, drop_extension, list_extensions};
use super::db::general::postgres_connect;
use super::structs::{OutputFormat, PostgresCredentials, Result, SortingOrder};
use clap::{Args, Subcommand};
use deadpool_postgres::Client;
use log::{debug, info};

#[derive(Args, Debug, PartialEq)]
pub struct ExtensionArgs {
    /// Database name
    database: String,
    /// action subcommand
    #[command(subcommand)]
    subcommand: ExtensionSubCommands,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum ExtensionSubCommands {
    /// List extensions
    List {
        /// sort by extension name
        #[arg(long, value_enum)]
        sort: Option<SortingOrder>,
        /// quiet mode
        #[arg(short, long, value_enum, default_value = "table")]
        output: OutputFormat,
        /// extra details
        #[arg(short, long)]
        extra: bool,
    },
    /// Create an extension
    Create {
        /// extension name
        extension: String,
    },
    /// Delete an extension
    Delete {
        /// extension name
        extension: String,
    },
}

pub async fn extension(data: &ExtensionArgs, credentials: &PostgresCredentials) -> Result<()> {
    let client = postgres_connect(credentials, Some(data.database.clone())).await?;
    match &data.subcommand {
        ExtensionSubCommands::List {
            sort,
            output,
            extra,
        } => {
            debug!("Listing extensions");
            extension_list(&client, sort, output, *extra).await?;
        }
        ExtensionSubCommands::Create { extension } => {
            debug!("Creating extension: {}", extension);
            create_extension(&client, extension).await?;
            info!("Extension {} created", extension);
        }
        ExtensionSubCommands::Delete { extension } => {
            debug!("Deleting extension: {}", extension);
            drop_extension(&client, extension).await?;
            info!("Extension {} deleted", extension);
        }
    }
    Ok(())
}

async fn extension_list(
    client: &Client,
    sort: &Option<SortingOrder>,
    output_format: &OutputFormat,
    extra: bool,
) -> Result<()> {
    let extensions = list_extensions(client, sort, extra).await?;
    super::output::render(&extensions, output_format, extra, std::io::stdout().lock())
}
