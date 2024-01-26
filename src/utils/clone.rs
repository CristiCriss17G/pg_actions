use clap::Args;

#[derive(Args)]
pub struct CloneArgs {
    /// database to clone
    #[arg(short, long)]
    database: String,
    /// new destination database
    #[arg(short, long)]
    new_database: String,
    /// owner user for db
    #[arg(short = 'o', long)]
    new_owner: String,
    /// create the new owner user
    #[arg(short, long)]
    create_owner: bool,
    /// new password for db
    /// provide if the user does not exist
    #[arg(short = 'p', long)]
    new_password: Option<String>,
}

pub struct CloneExternalData {
    pub hostname: String,
    pub pg_superuser: String,
    pub pg_password: String,
}

pub fn clone_db(data: &CloneArgs, external_data: CloneExternalData) {
    println!(
        "Cloning database {} to {}",
        data.database, data.new_database
    );
}
