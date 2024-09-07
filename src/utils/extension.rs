use super::db::extension::{create_extension, drop_extension, list_extensions};
use super::db::general::postgres_connect;
use super::structs::{ExtensionDetails, OutputFormat, PostgresCredentials, Result, SortingOrder};
use clap::{Args, Subcommand};
use csv::Writer;
use deadpool_postgres::Client;
use log::{debug, info, trace};
use prettytable::{row, Table};
use serde_json;

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
        /// quite mode
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
    trace!("Extensions: {:?}", extensions);
    match output_format {
        OutputFormat::Simple => {
            for extension in extensions {
                match extension {
                    ExtensionDetails::Extra {
                        name,
                        owner,
                        version,
                        oid,
                    } => {
                        println!("{}\t{}\t{}\t{}", name, owner, version, oid);
                    }
                    ExtensionDetails::Name(name) => {
                        println!("{}", name);
                    }
                }
            }
        }
        OutputFormat::Table => {
            let mut table = Table::new();
            match extra {
                true => table.add_row(row!["IDX", "Name", "Owner", "Version", "Oid"]),
                false => table.add_row(row!["IDX", "Name"]),
            };
            for (i, extension) in extensions.iter().enumerate() {
                match extension {
                    ExtensionDetails::Extra {
                        name,
                        owner,
                        version,
                        oid,
                    } => {
                        table.add_row(row![i, name, owner, version, oid]);
                    }
                    ExtensionDetails::Name(name) => {
                        table.add_row(row![i, name]);
                    }
                }
            }
            table.printstd();
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&extensions)?;
            println!("{}", json);
        }
        OutputFormat::JsonCompact => {
            let json = serde_json::to_string(&extensions)?;
            println!("{}", json);
        }
        OutputFormat::JsonLines => {
            for extension in extensions {
                let json = serde_json::to_string(&extension)?;
                println!("{}", json);
            }
        }
        OutputFormat::Csv => {
            let mut wtr = Writer::from_writer(vec![]);
            match extra {
                true => wtr.write_record(["IDX", "Name", "Owner", "Version", "Oid"])?,
                false => wtr.write_record(["IDX", "Name"])?,
            };
            for (i, extension) in extensions.iter().enumerate() {
                match extension {
                    ExtensionDetails::Extra {
                        name,
                        owner,
                        version,
                        oid,
                    } => {
                        wtr.write_record([
                            &i.to_string(),
                            &name.to_string(),
                            &owner.to_string(),
                            &version.to_string(),
                            &oid.to_string(),
                        ])?;
                    }
                    ExtensionDetails::Name(name) => {
                        wtr.write_record([&i.to_string(), &name.to_string()])?;
                    }
                }
            }
            let data = String::from_utf8(wtr.into_inner().map_err(Box::new)?)?;
            println!("{}", data);
        }
    }

    Ok(())
}
