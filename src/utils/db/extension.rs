use crate::utils::{
    db::general::validate_pg_names,
    structs::{ExtensionDetails, PGCliError, SortingOrder},
};
use deadpool_postgres::Client;
use log::{error, trace};

pub async fn list_extensions(
    client: &Client,
    sort: &Option<SortingOrder>,
    extra: bool,
) -> Result<Vec<ExtensionDetails>, PGCliError> {
    let fields = match extra {
        true => "extname, extowner, extversion, oid",
        false => "extname",
    };
    let rows = match sort {
        Some(SortingOrder::Ascending) => {
            client
                .query(
                    &format!(
                        "SELECT {fields} FROM pg_extension \
        ORDER BY extname ASC"
                    ),
                    &[],
                )
                .await?
        }
        Some(SortingOrder::Descending) => {
            client
                .query(
                    &format!(
                        "SELECT {fields} FROM pg_extension \
        ORDER BY extname DESC"
                    ),
                    &[],
                )
                .await?
        }
        None => {
            client
                .query(&format!("SELECT {fields} FROM pg_extension"), &[])
                .await?
        }
    };

    let mut extensions = Vec::new();
    for row in rows {
        match extra {
            true => {
                let name: String = row.get(0);
                let owner: u32 = row.get(1);
                let version: String = row.get(2);
                let oid: u32 = row.get(3);
                extensions.push(ExtensionDetails::Extra {
                    name,
                    owner,
                    version,
                    oid,
                });
            }
            false => {
                let name: String = row.get(0);
                extensions.push(ExtensionDetails::Name(name));
            }
        }
    }
    Ok(extensions)
}

pub async fn create_extension(client: &Client, extension: &str) -> Result<u64, PGCliError> {
    trace!("Creating extension {}", extension);
    if !validate_pg_names(extension) {
        error!("Invalid extension name: {}", extension);
        return Err(PGCliError::Other("Invalid extension name".to_string()));
    }
    let statement = format!("CREATE EXTENSION IF NOT EXISTS \"{}\"", extension);
    match client.execute(&statement, &[]).await {
        Ok(r) => Ok(r),
        Err(e) => Err(PGCliError::from(e)),
    }
}

pub async fn drop_extension(client: &Client, extension: &str) -> Result<u64, PGCliError> {
    trace!("Dropping extension {}", extension);
    if !validate_pg_names(extension) {
        error!("Invalid extension name: {}", extension);
        return Err(PGCliError::Other("Invalid extension name".to_string()));
    }
    let statement = format!("DROP EXTENSION IF EXISTS \"{}\"", extension);
    match client.execute(&statement, &[]).await {
        Ok(r) => Ok(r),
        Err(e) => Err(PGCliError::from(e)),
    }
}
