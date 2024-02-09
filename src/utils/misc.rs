use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::fs::{self, OpenOptions, Permissions};
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub fn generate_random_string(length: usize) -> String {
    let mut rng = thread_rng();
    let random_string: String = (&mut rng)
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    random_string
}

pub fn open_file_in_write_mode(path: &str, permissions: Option<u32>) -> io::Result<fs::File> {
    let path = Path::new(path);
    let file = open_file_path_in_write_mode(path, permissions).expect("Failed to open file");
    Ok(file)
}

pub fn open_file_path_in_write_mode(path: &Path, permissions: Option<u32>) -> io::Result<fs::File> {
    if let Some(dir) = path.parent() {
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
    }

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;

    // Set the permissions for the file to 644 (owner: read-write, group: read, others: read)
    let permissions = Permissions::from_mode(permissions.unwrap_or(0o644));
    fs::set_permissions(path, permissions)?;

    Ok(file)
}

pub fn delete_file_path(path: &Path) -> io::Result<()> {
    fs::remove_file(path)?;
    Ok(())
}

pub fn delete_file(path: &str) -> io::Result<()> {
    let path = Path::new(path);
    delete_file_path(path)
}
