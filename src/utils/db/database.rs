use super::general::{filter_entities, postgres_connect};
use crate::utils::sql::{identifier, validate_identifier};
use crate::utils::structs::{
    DatabaseDetails, PGCliError, PostgresCredentials, Result, SortingOrder,
};
use deadpool_postgres::Client;
use log::{error, trace};

pub async fn check_database_exists(client: &Client, db: &str) -> Result<bool> {
    trace!("Checking if database {} exists", db);
    let database_exists = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname=$1)",
            &[&db],
        )
        .await?
        .get::<_, bool>(0);
    Ok(database_exists)
}

pub async fn get_database_owner(client: &Client, db: &str) -> Result<String> {
    trace!("Getting owner of database {}", db);
    let owner = client
        .query_one(
            "SELECT r.rolname FROM pg_database d \
        LEFT JOIN pg_roles r ON d.datdba = r.oid \
        WHERE d.datname = $1",
            &[&db],
        )
        .await?
        .get::<_, String>(0);
    Ok(owner)
}

pub async fn list_databases(
    client: &Client,
    sort: &Option<SortingOrder>,
    extra: bool,
    query: &Option<String>,
) -> Result<Vec<DatabaseDetails>> {
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

pub async fn create_db(client: &Client, db: &str, owner: Option<&str>) -> Result<u64> {
    trace!("Creating database {}", db);
    if validate_identifier(db).is_err() {
        error!("Invalid database name: {}", db);
        return Err(PGCliError::Other("Invalid database name".to_string()));
    }
    if let Some(owner) = owner
        && validate_identifier(owner).is_err()
    {
        error!("Invalid owner name: {}", owner);
        return Err(PGCliError::Other("Invalid owner name".to_string()));
    }

    if check_database_exists(client, db).await? {
        error!("Database {} already exists", db);
        return Err(PGCliError::Other("Database already exists".to_string()));
    }

    let statement = match owner {
        Some(owner) => format!(
            "CREATE DATABASE {} WITH OWNER {}",
            identifier(db)?,
            identifier(owner)?
        ),
        None => format!("CREATE DATABASE {}", identifier(db)?),
    };

    match client.execute(&statement, &[]).await {
        Ok(r) => Ok(r),
        Err(e) => Err(PGCliError::from(e)),
    }
}

pub async fn delete_db(client: &Client, db: &str) -> Result<u64> {
    trace!("Deleting database {}", db);
    if validate_identifier(db).is_err() {
        error!("Invalid database name: {}", db);
        return Err(PGCliError::Other("Invalid database name".to_string()));
    }

    if !check_database_exists(client, db).await? {
        error!("Database {} does not exist", db);
        return Err(PGCliError::Other("Database does not exist".to_string()));
    }

    let statement = format!("DROP DATABASE {}", identifier(db)?);
    match client.execute(&statement, &[]).await {
        Ok(r) => Ok(r),
        Err(e) => Err(PGCliError::from(e)),
    }
}

pub async fn kill_connections_to_db(client: &Client, db: &str) -> Result<u64> {
    trace!("Killing connections to database {}", db);
    client
        .execute(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE pid <> pg_backend_pid() AND datname = $1",
            &[&db],
        )
        .await.map_err(PGCliError::from)
}

pub async fn change_whole_owner_of_db(
    client: &Client,
    credentials: &PostgresCredentials,
    db: &str,
    new_owner: &str,
) -> Result<()> {
    trace!(
        "Changing owner on the whole database {} with objects to {}",
        db, new_owner
    );

    if validate_identifier(new_owner).is_err() {
        error!("Invalid new owner name: {}", new_owner);
        return Err(PGCliError::Other("Invalid new owner name".to_string()));
    }

    if validate_identifier(db).is_err() {
        error!("Invalid database name: {}", db);
        return Err(PGCliError::Other("Invalid database name".to_string()));
    }

    if !check_database_exists(client, db).await? {
        error!("Database {} does not exist", db);
        return Err(PGCliError::Other("Database does not exist".to_string()));
    }

    change_objects_owner(credentials, db, new_owner).await
}

/// Reassign application objects transactionally, with no detached tasks.
/// Extension members and system schemas retain their ownership.
pub async fn change_objects_owner(
    credentials: &PostgresCredentials,
    db: &str,
    new_owner: &str,
) -> Result<()> {
    let owner = identifier(new_owner)?;
    let mut client = postgres_connect(credentials, Some(db.to_owned())).await?;
    let transaction = client.transaction().await?;
    let schemas = transaction.query("SELECT n.oid, n.nspname FROM pg_namespace n WHERE n.nspname !~ '^pg_' AND n.nspname <> 'information_schema' AND NOT EXISTS (SELECT 1 FROM pg_depend d WHERE d.classid='pg_namespace'::regclass AND d.objid=n.oid AND d.deptype='e') ORDER BY n.nspname", &[]).await?;
    for schema in schemas {
        let namespace: u32 = schema.try_get(0)?;
        let schema_name: String = schema.try_get(1)?;
        let quoted_schema = identifier(&schema_name)?;
        let relations = transaction.query("SELECT c.oid, c.relname, c.relkind::text FROM pg_class c WHERE c.relnamespace=$1 AND c.relkind IN ('r','p','v','m','S','f') AND NOT EXISTS (SELECT 1 FROM pg_depend d WHERE d.classid='pg_class'::regclass AND d.objid=c.oid AND d.deptype='e') ORDER BY CASE WHEN c.relkind='S' THEN 1 ELSE 0 END, c.oid", &[&namespace]).await?;
        for relation in relations {
            let name: String = relation.try_get(1)?;
            let kind: String = relation.try_get(2)?;
            // Owned sequences follow their table automatically. ALTER SEQUENCE
            // OWNER on an owned sequence can fail; handle standalone ones only.
            if kind == "S" {
                let oid: u32 = relation.try_get(0)?;
                let owned: bool = transaction.query_one("SELECT EXISTS(SELECT 1 FROM pg_depend WHERE classid='pg_class'::regclass AND objid=$1 AND deptype IN ('a','i'))", &[&oid]).await?.try_get(0)?;
                if owned {
                    continue;
                }
            }
            let keyword = match kind.as_str() {
                "v" => "VIEW",
                "m" => "MATERIALIZED VIEW",
                "S" => "SEQUENCE",
                "f" => "FOREIGN TABLE",
                _ => "TABLE",
            };
            transaction
                .execute(
                    &format!(
                        "ALTER {keyword} {quoted_schema}.{} OWNER TO {owner}",
                        identifier(&name)?
                    ),
                    &[],
                )
                .await?;
        }
        let types = transaction.query("SELECT t.typname, t.typtype::text FROM pg_type t LEFT JOIN pg_class c ON c.oid=t.typrelid WHERE t.typnamespace=$1 AND t.typtype IN ('b','e','d','c','r') AND t.typcategory <> 'A' AND (t.typrelid=0 OR c.relkind='c') AND NOT EXISTS (SELECT 1 FROM pg_depend d WHERE d.classid='pg_type'::regclass AND d.objid=t.oid AND d.deptype IN ('e','i'))", &[&namespace]).await?;
        for row in types {
            let name: String = row.try_get(0)?;
            let kind: String = row.try_get(1)?;
            let keyword = if kind == "d" { "DOMAIN" } else { "TYPE" };
            transaction
                .execute(
                    &format!(
                        "ALTER {keyword} {quoted_schema}.{} OWNER TO {owner}",
                        identifier(&name)?
                    ),
                    &[],
                )
                .await?;
        }
        let routines = transaction.query("SELECT p.proname, p.prokind::text, pg_get_function_identity_arguments(p.oid) FROM pg_proc p WHERE p.pronamespace=$1 AND NOT EXISTS (SELECT 1 FROM pg_depend d WHERE d.classid='pg_proc'::regclass AND d.objid=p.oid AND d.deptype='e')", &[&namespace]).await?;
        for row in routines {
            let name: String = row.try_get(0)?;
            let kind: String = row.try_get(1)?;
            let arguments: String = row.try_get(2)?;
            // pg_get_function_identity_arguments emits server-quoted SQL syntax.
            let keyword = match kind.as_str() {
                "p" => "PROCEDURE",
                "a" => "AGGREGATE",
                _ => "FUNCTION",
            };
            transaction
                .execute(
                    &format!(
                        "ALTER {keyword} {quoted_schema}.{}({arguments}) OWNER TO {owner}",
                        identifier(&name)?
                    ),
                    &[],
                )
                .await?;
        }
        transaction
            .execute(
                &format!("ALTER SCHEMA {quoted_schema} OWNER TO {owner}"),
                &[],
            )
            .await?;
    }
    let large_objects = transaction
        .query("SELECT oid FROM pg_largeobject_metadata", &[])
        .await?;
    for row in large_objects {
        let oid: u32 = row.try_get(0)?;
        transaction
            .execute(&format!("ALTER LARGE OBJECT {oid} OWNER TO {owner}"), &[])
            .await?;
    }
    transaction
        .execute(
            &format!("ALTER DATABASE {} OWNER TO {owner}", identifier(db)?),
            &[],
        )
        .await?;
    transaction.commit().await?;
    Ok(())
}
