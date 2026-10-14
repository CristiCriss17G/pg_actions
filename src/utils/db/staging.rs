use super::database::{
    change_objects_owner, check_database_exists, create_db, delete_db, get_database_owner,
    kill_connections_to_db,
};
use super::general::{db_restore, postgres_connect};
use super::user::check_user_exists;
use crate::utils::misc::generate_random_string;
use crate::utils::sql::identifier;
use crate::utils::structs::{PGCliError, PGTools, PostgresCredentials, Result, SqlFileFormat};
use log::{info, warn};
use std::path::Path;

pub async fn preflight(
    client: &deadpool_postgres::Client,
    target: &str,
    owner: Option<&str>,
    overwrite: bool,
) -> Result<Option<String>> {
    identifier(target)?;
    if matches!(target, "postgres" | "template0" | "template1") {
        return Err(PGCliError::Other(
            "Staged restore cannot replace PostgreSQL maintenance/template databases".into(),
        ));
    }
    let existing = check_database_exists(client, target).await?;
    if existing && !overwrite {
        return Err(PGCliError::Other(
            "Database already exists and overwrite is not set".into(),
        ));
    }
    let capabilities = client
        .query_one(
            "SELECT rolsuper, rolcreatedb FROM pg_roles WHERE rolname=current_user",
            &[],
        )
        .await?;
    let superuser: bool = capabilities.try_get(0)?;
    let createdb: bool = capabilities.try_get(1)?;
    if !superuser && !createdb {
        return Err(PGCliError::Other("Staged restore requires CREATEDB".into()));
    }
    let effective_owner = if let Some(owner) = owner {
        identifier(owner)?;
        if !check_user_exists(client, owner).await? {
            return Err(PGCliError::Other("Requested owner does not exist".into()));
        }
        Some(owner.to_owned())
    } else if existing {
        Some(get_database_owner(client, target).await?)
    } else {
        None
    };
    if !superuser {
        if existing {
            let allowed: bool = client.query_one("SELECT datdba=(SELECT oid FROM pg_roles WHERE rolname=current_user) FROM pg_database WHERE datname=$1", &[&target]).await?.try_get(0)?;
            if !allowed {
                return Err(PGCliError::Other(
                    "Only the database owner or a superuser can replace it".into(),
                ));
            }
        }
        if let Some(owner) = &effective_owner {
            let allowed: bool = client
                .query_one(
                    "SELECT pg_has_role(current_user, $1::text, 'SET')",
                    &[owner],
                )
                .await?
                .try_get(0)?;
            if !allowed {
                return Err(PGCliError::Other(
                    "Current user cannot SET ROLE to the requested owner".into(),
                ));
            }
        }
    }
    Ok(effective_owner)
}

pub async fn staged_restore(
    credentials: &PostgresCredentials,
    target: &str,
    input: &Path,
    tools: &PGTools,
    format: SqlFileFormat,
    owner: Option<&str>,
    overwrite: bool,
) -> Result<()> {
    let mut client = postgres_connect(credentials, None).await?;
    let owner = preflight(&client, target, owner, overwrite).await?;
    if format == SqlFileFormat::Bsql {
        tools.validate_archive(input).await?;
    }
    let stage = format!("pg_actions_stage_{}", generate_random_string(16, true));
    create_db(&client, &stage, None).await?;
    let mut staging_credentials = credentials.clone();
    staging_credentials.password = credentials
        .password_for(target)
        .or_else(|| credentials.password_for("postgres"))
        .map(str::to_owned);
    let result = async {
        db_restore(staging_credentials.clone(), &stage, input, tools, format).await?;
        if let Some(owner) = &owner {
            change_objects_owner(&staging_credentials, &stage, owner).await?;
        }
        let previous = format!("pg_actions_previous_{}", generate_random_string(16, true));
        let exists = check_database_exists(&client, target).await?;
        if exists && !overwrite {
            return Err(PGCliError::Other(
                "Destination appeared during restore; overwrite was not requested".into(),
            ));
        }
        kill_connections_to_db(&client, &stage).await?;
        if exists {
            kill_connections_to_db(&client, target).await?;
        }
        // Database renames are transactional. A failed second rename rolls back
        // the first, leaving the original name and data intact.
        let transaction = client.transaction().await?;
        if exists {
            transaction
                .execute(
                    &format!(
                        "ALTER DATABASE {} RENAME TO {}",
                        identifier(target)?,
                        identifier(&previous)?
                    ),
                    &[],
                )
                .await?;
        }
        transaction
            .execute(
                &format!(
                    "ALTER DATABASE {} RENAME TO {}",
                    identifier(&stage)?,
                    identifier(target)?
                ),
                &[],
            )
            .await?;
        transaction.commit().await?;
        info!("Restored database published as {target}");
        if let Err(error) = if exists {
            delete_db(&client, &previous).await
        } else {
            Ok(0)
        } {
            // Publication succeeded. Never fail it or remove the new destination
            // because cleanup of the old database failed.
            warn!("Previous database retained as {previous}; cleanup failed: {error}");
        }
        Ok(())
    }
    .await;
    if result.is_err() {
        if let Err(error) = kill_connections_to_db(&client, &stage).await {
            warn!("Staging disconnect failed: {error}");
        }
        if let Err(error) = delete_db(&client, &stage).await {
            warn!("Staging database {stage} cleanup failed: {error}");
        }
    }
    result
}
