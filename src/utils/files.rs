use super::structs::{PGCliError, Result};
use log::{info, warn};
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub fn parent(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

pub fn private_open(path: &Path, append: bool) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create(true)
        .append(append)
        .truncate(!append);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Do not follow an existing symlink while opening a sensitive output.
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(file)
}

pub fn dump_filename(database: &str, extension: &str) -> String {
    // Preserve historical names where they are portable. Otherwise encode every
    // UTF-8 byte; the prefix keeps encoded names distinct from ordinary names.
    let reserved = matches!(
        database.to_ascii_uppercase().as_str(),
        "CON" | "PRN" | "AUX" | "NUL"
    ) || (1..=9).any(|i| {
        database.eq_ignore_ascii_case(&format!("COM{i}"))
            || database.eq_ignore_ascii_case(&format!("LPT{i}"))
    });
    let safe = !database.is_empty()
        && !reserved
        && !database.starts_with("pghex-")
        && database
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
    let name = if safe {
        database.to_owned()
    } else {
        let hex: String = database
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        format!("pghex-{hex}")
    };
    format!("{name}.{extension}")
}

pub fn workspace(prefix: &str) -> Result<TempDir> {
    Ok(tempfile::Builder::new().prefix(prefix).tempdir()?)
}

pub fn finish_workspace<T>(directory: TempDir, keep: bool, result: Result<T>) -> Result<T> {
    if keep {
        info!("Temporary files kept at {}", directory.keep().display());
        return result;
    }
    match directory.close() {
        Ok(()) => result,
        Err(error) => match result {
            Err(primary) => {
                warn!("Temporary cleanup also failed: {error}");
                Err(primary)
            }
            Ok(_) => Err(error.into()),
        },
    }
}

pub fn safe_existing_file(directory: &Path, name: &str) -> Result<Option<PathBuf>> {
    for entry in walkdir::WalkDir::new(directory).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_file() && entry.file_name() == name {
            let path = entry.path().canonicalize()?;
            if !path.starts_with(directory.canonicalize()?) {
                return Err(PGCliError::Other(
                    "Archive file is outside the extraction directory".into(),
                ));
            }
            return Ok(Some(path));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filenames_are_single_portable_components() {
        for name in ["../escape", "C:/evil", "CON", "a\"b", "pghex-6162", ""] {
            let filename = dump_filename(name, "bsql");
            assert_eq!(Path::new(&filename).components().count(), 1);
            assert!(!filename.contains(['/', '\\', ':', '"']));
        }
        assert_eq!(dump_filename("sample_db", "sql"), "sample_db.sql");
        assert_eq!(parent(Path::new("backup.sql")), Path::new("."));
        assert_ne!(dump_filename("Case", "sql"), dump_filename("case", "sql"));
    }

    #[cfg(unix)]
    #[test]
    fn failed_cleanup_preserves_the_primary_error() {
        use std::os::unix::fs::PermissionsExt;
        let directory = workspace("cleanup-test-").unwrap();
        let path = directory.path().to_path_buf();
        std::fs::write(path.join("file"), b"fixture").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o500)).unwrap();
        let result: Result<()> = finish_workspace(
            directory,
            false,
            Err(PGCliError::Other("primary failure".into())),
        );
        assert_eq!(result.unwrap_err().to_string(), "Error: primary failure");
        // The owner can restore permissions even when removal failed.
        if path.exists() {
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            std::fs::remove_dir_all(path).unwrap();
        }
    }
    #[cfg(unix)]
    #[test]
    fn private_output_rejects_symlinks_and_is_owner_only() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file");
        private_open(&file, false).unwrap();
        assert_eq!(
            std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let link = dir.path().join("link");
        symlink(&file, &link).unwrap();
        assert!(private_open(&link, false).is_err());
    }
}
