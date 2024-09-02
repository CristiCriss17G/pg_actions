use super::general::{filter_entities, postgres_connect, validate_pg_names};
use crate::utils::structs::{DatabaseDetails, PGCliError, PostgresCredentials, SortingOrder};
use log::{debug, error, trace};
use tokio_postgres::{Client, Error};

pub async fn check_database_exists(client: &Client, db: &str) -> Result<bool, PGCliError> {
    trace!("Checking if database {} exists", db);
    let db_exists = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname=$1)",
            &[&db],
        )
        .await?
        .get::<_, bool>(0);
    Ok(db_exists)
}

pub async fn list_databases(
    client: &Client,
    sort: &Option<SortingOrder>,
    extra: bool,
    query: &Option<String>,
) -> Result<Vec<DatabaseDetails>, PGCliError> {
    let ignored_databases = "'template0','template1'";
    let fields = match extra {
        true => "d.datname, d.oid, pg_size_pretty(pg_database_size(d.datname)), r.rolname",
        false => "d.datname",
    };
    let rows = match sort {
        Some(SortingOrder::Ascending) => {
            client
                .query(
                    &format!(
                        "SELECT {fields} FROM pg_database d \
        LEFT JOIN pg_roles r ON d.datdba = r.oid \
        WHERE d.datistemplate = false \
        AND d.datname NOT IN ({ignored_databases}) \
        ORDER BY d.datname ASC"
                    ),
                    &[],
                )
                .await?
        }
        Some(SortingOrder::Descending) => {
            client
                .query(
                    &format!(
                        "SELECT {fields} FROM pg_database d \
        LEFT JOIN pg_roles r ON d.datdba = r.oid \
        WHERE d.datistemplate = false \
        AND d.datname NOT IN ({ignored_databases}) \
        ORDER BY d.datname DESC"
                    ),
                    &[],
                )
                .await?
        }
        None => {
            client
                .query(
                    &format!(
                        "SELECT {fields} FROM pg_database d \
        LEFT JOIN pg_roles r ON d.datdba = r.oid \
        WHERE d.datistemplate = false \
        AND d.datname NOT IN ({ignored_databases})"
                    ),
                    &[],
                )
                .await?
        }
    };

    let mut databases = Vec::new();
    for row in rows {
        match extra {
            true => {
                let name: String = row.get(0);
                let oid: u32 = row.get(1);
                let size_pretty: String = row.get(2);
                let owner: String = row.get(3);
                databases.push(DatabaseDetails::Extra {
                    name,
                    oid,
                    size_pretty,
                    owner,
                });
            }
            false => {
                let name: String = row.get(0);
                databases.push(DatabaseDetails::Name(name));
            }
        }
    }
    match query {
        Some(q) => Ok(filter_entities(databases, q)),
        None => Ok(databases),
    }
}

pub async fn create_db(client: &Client, db: &str, owner: &str) -> Result<u64, PGCliError> {
    trace!("Creating database {}", db);
    if !validate_pg_names(db) {
        error!("Invalid database name: {}", db);
        return Err(PGCliError::Other("Invalid database name".to_string()));
    }
    if !validate_pg_names(owner) {
        error!("Invalid owner name: {}", owner);
        return Err(PGCliError::Other("Invalid owner name".to_string()));
    }

    if check_database_exists(client, db).await? {
        error!("Database {} already exists", db);
        return Err(PGCliError::Other("Database already exists".to_string()));
    }

    let statement = format!("CREATE DATABASE \"{}\" WITH OWNER \"{}\"", db, owner);
    match client.execute(&statement, &[]).await {
        Ok(r) => Ok(r),
        Err(e) => Err(PGCliError::from(e)),
    }
}

pub async fn delete_db(client: &Client, db: &str) -> Result<u64, PGCliError> {
    trace!("Deleting database {}", db);
    if !validate_pg_names(db) {
        error!("Invalid database name: {}", db);
        return Err(PGCliError::Other("Invalid database name".to_string()));
    }

    if !check_database_exists(client, db).await? {
        error!("Database {} does not exist", db);
        return Err(PGCliError::Other("Database does not exist".to_string()));
    }

    let statement = format!("DROP DATABASE \"{}\"", db);
    match client.execute(&statement, &[]).await {
        Ok(r) => Ok(r),
        Err(e) => Err(PGCliError::from(e)),
    }
}

pub async fn kill_connections_to_db(client: &Client, db: &str) -> Result<u64, Error> {
    trace!("Killing connections to database {}", db);
    client
        .execute(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE pid <> pg_backend_pid() AND datname = $1",
            &[&db],
        )
        .await
}

pub async fn change_whole_owner_of_db(
    client: &Client,
    credentials: &PostgresCredentials,
    db: &str,
    new_owner: &str,
) -> Result<(), PGCliError> {
    trace!(
        "Changing owner on the whole database {} with objects to {}",
        db,
        new_owner
    );

    if !validate_pg_names(new_owner) {
        error!("Invalid new owner name: {}", new_owner);
        return Err(PGCliError::Other("Invalid new owner name".to_string()));
    }

    if !validate_pg_names(db) {
        error!("Invalid database name: {}", db);
        return Err(PGCliError::Other("Invalid database name".to_string()));
    }

    if !check_database_exists(client, db).await? {
        error!("Database {} does not exist", db);
        return Err(PGCliError::Other("Database does not exist".to_string()));
    }

    trace!("Changing owner of database {} to {}", db, new_owner);
    change_owner_of_db(client, db, new_owner).await?;
    trace!(
        "Changing owner of tables in database {} to {}",
        db,
        new_owner
    );
    change_owner_of_tables_in_db(credentials, db, new_owner).await?;
    trace!(
        "Changing owner of objects in database {} to {}",
        db,
        new_owner
    );
    change_owner_of_objects_in_db(credentials, db, new_owner).await
}

pub async fn change_owner_of_db(
    client: &Client,
    db: &str,
    new_owner: &str,
) -> Result<u64, PGCliError> {
    trace!("Changing owner of database {} to {}", db, new_owner);
    if !validate_pg_names(new_owner) {
        error!("Invalid new owner name: {}", new_owner);
        return Err(PGCliError::Other("Invalid new owner name".to_string()));
    }
    if !validate_pg_names(db) {
        error!("Invalid database name: {}", db);
        return Err(PGCliError::Other("Invalid database name".to_string()));
    }
    let statement = format!("ALTER DATABASE \"{}\" OWNER TO \"{}\"", db, new_owner);
    match client.execute(&statement, &[]).await {
        Ok(r) => Ok(r),
        Err(e) => Err(PGCliError::from(e)),
    }
}

pub async fn change_owner_of_tables_in_db(
    credentials: &PostgresCredentials,
    db: &str,
    new_owner: &str,
) -> Result<(), PGCliError> {
    trace!(
        "Changing owner of tables in database {} to {}",
        db,
        new_owner
    );
    debug!("Connecting to postgres to change owner of database {}", db);
    if !validate_pg_names(new_owner) {
        error!("Invalid new owner name: {}", new_owner);
        return Err(PGCliError::Other("Invalid new owner name".to_string()));
    }
    if !validate_pg_names(db) {
        error!("Invalid database name: {}", db);
        return Err(PGCliError::Other("Invalid database name".to_string()));
    }

    let client = postgres_connect(credentials, Some(db.to_string())).await?;

    // Query to select table names
    debug!("Querying tables to change owner");
    let rows = client
        .query(
            "SELECT tablename FROM pg_tables WHERE schemaname = 'public'",
            &[],
        )
        .await?;
    trace!("Tables to change owner: {:?} ({})", rows, rows.len());

    // Loop through each table and change its owner
    for row in rows {
        let tablename: &str = row.get(0);
        debug!("Changing owner of table: {} to {}", tablename, new_owner);

        // Dynamically build the ALTER TABLE command
        let statement = format!(
            "ALTER TABLE \"public\".\"{}\" OWNER TO {}",
            tablename, new_owner
        );

        // Execute the ALTER TABLE command
        client.execute(&statement, &[]).await?;
    }

    Ok(())
}

pub async fn change_owner_of_objects_in_db(
    credentials: &PostgresCredentials,
    db: &str,
    new_owner: &str,
) -> Result<(), PGCliError> {
    trace!("Changing owner of database {} to {}", db, new_owner);
    debug!("Connecting to postgres to change owner of database {}", db);

    // Sanitize the new owner name or validate it here
    // For example, ensure it matches a strict pattern (alphanumeric + underscore)
    if !validate_pg_names(new_owner) {
        error!("Invalid new owner name: {}", new_owner);
        return Err(PGCliError::Other("Invalid new owner name".to_string()));
    }
    if !validate_pg_names(db) {
        error!("Invalid database name: {}", db);
        return Err(PGCliError::Other("Invalid database name".to_string()));
    }

    let client = postgres_connect(credentials, Some(db.to_string())).await?;

    // Query to select type names
    debug!("Querying types to change owner");
    let rows = client.query(
        "SELECT typname FROM pg_type WHERE typtype IN ('b', 'e') AND typcategory != 'A' AND typnamespace IN (SELECT oid FROM pg_namespace WHERE nspname NOT IN ('pg_catalog', 'information_schema'))",
        &[],
    ).await?;
    trace!("Types to change owner: {:?} ({})", rows, rows.len());

    // Loop through each type and change its owner
    for row in rows {
        let typname: &str = row.get(0);
        debug!("Changing owner of type: {} to {}", typname, new_owner);

        // Dynamically build the ALTER TYPE command
        let statement = format!("ALTER TYPE \"{}\" OWNER TO \"{}\"", typname, new_owner);

        // Execute the ALTER TYPE command
        client.execute(&statement, &[]).await?;
    }

    Ok(())
}
