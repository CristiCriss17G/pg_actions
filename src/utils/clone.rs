use super::credentials::PostgresCredentials;
use super::db::database::check_database_exists;
use super::db::general::{db_dump, postgres_connect};
use super::db::staging::{preflight, staged_restore};
use super::db::user::{check_user_exists, create_user};
use super::files::{finish_workspace, workspace};
use super::sql::validate_identifier;
use super::structs::{PGCliError, PGTools, Result, SqlFileFormat};
use clap::Args;
use log::info;

#[derive(Args, PartialEq)]
pub struct CloneArgs {
    /// database to clone
    #[arg(short, long)]
    database: String,
    /// new destination database
    #[arg(short, long)]
    new_database: String,
    /// overwrite new database if it exists
    #[arg(long)]
    overwrite: bool,
    /// owner user for db
    #[arg(short = 'o', long)]
    new_owner: Option<String>,
    /// create the new owner user
    #[arg(short, long)]
    create_owner: bool,
    /// new password for db
    /// provide if the user does not exist
    #[arg(short = 's', long)]
    new_password: Option<String>,
    /// keep the dump file
    /// defaults to false
    #[arg(short, long, env)]
    keep_dump: bool,
    /// Optional new host
    #[arg(long, env)]
    pg_hostname2: Option<String>,
    /// Optional new port
    #[arg(long, env)]
    pg_port2: Option<u16>,
    /// Optional user for new host
    #[arg(long, env)]
    pg_superuser2: Option<String>,
    /// Optional password for new host
    #[arg(long, env, hide_env_values = true)]
    pg_password2: Option<String>,
}

impl CloneArgs {
    pub fn source_database(&self) -> &str {
        &self.database
    }
    pub fn destination_credentials(&self, source: &PostgresCredentials) -> PostgresCredentials {
        if self.pg_hostname2.is_none()
            && self.pg_port2.is_none()
            && self.pg_superuser2.is_none()
            && self.pg_password2.is_none()
        {
            return source.clone();
        }
        let mut destination = source.passfile.resolve(
            self.pg_hostname2
                .clone()
                .unwrap_or_else(|| source.pg_hostname.clone()),
            self.pg_superuser2.clone(),
            self.pg_port2,
            self.pg_password2.clone(),
            "postgres",
        );
        destination.ssl_mode = source.ssl_mode;
        destination.ssl_root_cert = source.ssl_root_cert.clone();
        destination
    }
}

pub async fn clone_db(
    data: &CloneArgs,
    source: &PostgresCredentials,
    tools: &PGTools,
) -> Result<()> {
    validate_identifier(&data.database)?;
    validate_identifier(&data.new_database)?;
    if let Some(owner) = &data.new_owner {
        validate_identifier(owner)?;
    }
    let destination = data.destination_credentials(source);
    let source_client = postgres_connect(source, None).await?;
    let destination_client = postgres_connect(&destination, None).await?;
    if data.database == data.new_database
        && same_server(source, &destination, &source_client, &destination_client).await
    {
        return Err(PGCliError::Other(
            "Source and destination identify the same database".into(),
        ));
    }
    if !check_database_exists(&source_client, &data.database).await? {
        return Err(PGCliError::Other("Source database does not exist".into()));
    }
    tools.check_dump_server(&source_client).await?;
    // Check destination existence/permissions before creating a requested role.
    preflight(
        &destination_client,
        &data.new_database,
        None,
        data.overwrite,
    )
    .await?;
    if let Some(owner) = &data.new_owner {
        if !check_user_exists(&destination_client, owner).await? {
            if !data.create_owner {
                return Err(PGCliError::Other(
                    "Owner does not exist; use --create-owner".into(),
                ));
            }
            // Validate the password before any role or database mutation.
            if let Some(password) = &data.new_password {
                super::sql::literal(password)?;
            }
            if let Some(password) =
                create_user(&destination_client, owner, &data.new_password).await?
            {
                super::output::generated_password(owner, &password)?;
            }
        }
    }
    preflight(
        &destination_client,
        &data.new_database,
        data.new_owner.as_deref(),
        data.overwrite,
    )
    .await?;
    let directory = workspace("postgresql-clone-")?;
    let dump = directory.path().join("clone.bsql");
    let result = async {
        db_dump(
            source.clone(),
            &data.database,
            &dump,
            tools,
            SqlFileFormat::Bsql,
            super::structs::PgDumpCompression::Default,
        )
        .await?;
        staged_restore(
            &destination,
            &data.new_database,
            &dump,
            tools,
            SqlFileFormat::Bsql,
            data.new_owner.as_deref(),
            data.overwrite,
        )
        .await?;
        info!("Database {} cloned to {}", data.database, data.new_database);
        Ok(())
    }
    .await;
    finish_workspace(directory, data.keep_dump, result)
}

// Backend IPs can coincide across separate VPCs/containers. Compare a cluster
// identifier and instance start time when permitted; otherwise compare the
// resolved client endpoints rather than private backend addresses.
async fn same_server(
    source: &PostgresCredentials,
    destination: &PostgresCredentials,
    source_client: &deadpool_postgres::Client,
    destination_client: &deadpool_postgres::Client,
) -> bool {
    if source.pg_hostname == destination.pg_hostname && source.pg_port == destination.pg_port {
        return true;
    }
    let query = "SELECT system_identifier::text, extract(epoch FROM pg_catalog.pg_postmaster_start_time())::text FROM pg_catalog.pg_control_system()";
    let source_identity = source_client
        .query_one(query, &[])
        .await
        .ok()
        .and_then(|row| {
            Some((
                row.try_get::<_, String>(0).ok()?,
                row.try_get::<_, String>(1).ok()?,
            ))
        });
    let destination_identity = destination_client
        .query_one(query, &[])
        .await
        .ok()
        .and_then(|row| {
            Some((
                row.try_get::<_, String>(0).ok()?,
                row.try_get::<_, String>(1).ok()?,
            ))
        });
    if let (Some(source), Some(destination)) = (source_identity, destination_identity) {
        return source == destination;
    }
    let source_addresses =
        tokio::net::lookup_host((source.pg_hostname.as_str(), source.pg_port)).await;
    let destination_addresses =
        tokio::net::lookup_host((destination.pg_hostname.as_str(), destination.pg_port)).await;
    if let (Ok(source), Ok(destination)) = (source_addresses, destination_addresses) {
        let source: Vec<_> = source.collect();
        return destination
            .into_iter()
            .any(|address| source.contains(&address));
    }
    false
}
