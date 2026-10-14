use super::structs::{PGCliError, Result};
use log::info;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Dump,
    Restore,
    Psql,
}
impl ToolKind {
    fn name(self) -> &'static str {
        match self {
            Self::Dump => "pg_dump",
            Self::Restore => "pg_restore",
            Self::Psql => "psql",
        }
    }
}

#[derive(Debug)]
pub struct ToolPaths {
    pub dump: PathBuf,
    pub restore: PathBuf,
    pub psql: PathBuf,
}
impl ToolPaths {
    fn path(&self, kind: ToolKind) -> &Path {
        match kind {
            ToolKind::Dump => &self.dump,
            ToolKind::Restore => &self.restore,
            ToolKind::Psql => &self.psql,
        }
    }
}

#[derive(Debug, Clone)]
struct Tool {
    kind: ToolKind,
    path: PathBuf,
    major: u32,
}
#[derive(Debug, Clone, Default)]
pub struct PGTools {
    tools: Vec<Tool>,
}

fn resolve_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() || path.components().count() > 1 {
        return Ok(if path.is_absolute() {
            path.to_owned()
        } else {
            std::env::current_dir()?.join(path)
        });
    }
    let paths = std::env::var_os("PATH").unwrap_or_default();
    for directory in std::env::split_paths(&paths) {
        let candidate = directory.join(path);
        #[cfg(windows)]
        let candidate = if candidate.extension().is_none() {
            candidate.with_extension("exe")
        } else {
            candidate
        };
        if candidate.is_file() {
            // Retain symlink basename: Debian pg_wrapper dispatches by argv[0].
            return Ok(if candidate.is_absolute() {
                candidate
            } else {
                std::env::current_dir()?.join(candidate)
            });
        }
    }
    Err(PGCliError::PGToolsError(format!("{} was not found in PATH; install a compatible PostgreSQL client or supply its explicit path", path.display())))
}

fn parse_version(kind: ToolKind, text: &str) -> Option<u32> {
    let prefix = format!("{} (PostgreSQL) ", kind.name());
    text.trim()
        .strip_prefix(&prefix)?
        .split('.')
        .next()?
        .parse()
        .ok()
}

impl PGTools {
    pub async fn check(paths: &ToolPaths, required: &[ToolKind]) -> Result<Self> {
        let mut tools = Vec::new();
        for &kind in required {
            let path = resolve_path(paths.path(kind))?;
            let mut command = Command::new(&path);
            command
                .arg("--version")
                .stdin(Stdio::null())
                .kill_on_drop(true);
            let output = tokio::time::timeout(Duration::from_secs(10), command.output())
                .await
                .map_err(|_| {
                    PGCliError::PGToolsError(format!("{} --version timed out", path.display()))
                })??;
            if !output.status.success() {
                return Err(PGCliError::PGToolsError(format!(
                    "{} --version failed ({})",
                    path.display(),
                    output.status
                )));
            }
            let version = String::from_utf8_lossy(&output.stdout);
            let major = parse_version(kind, &version).ok_or_else(|| {
                PGCliError::PGToolsError(format!(
                    "{} returned an unrecognized PostgreSQL version",
                    path.display()
                ))
            })?;
            if major < 16 {
                return Err(PGCliError::PGToolsError(format!(
                    "{} major {major} is unsupported; minimum is 16",
                    path.display()
                )));
            }
            info!("{}: {} ({})", kind.name(), path.display(), version.trim());
            tools.push(Tool { kind, path, major });
        }
        if tools.windows(2).any(|pair| pair[0].major != pair[1].major) {
            return Err(PGCliError::PGToolsError("Required PostgreSQL clients have different major versions; select a coherent suite with explicit paths".into()));
        }
        Ok(Self { tools })
    }

    fn tool(&self, kind: ToolKind) -> Result<&Tool> {
        self.tools
            .iter()
            .find(|tool| tool.kind == kind)
            .ok_or_else(|| {
                PGCliError::PGToolsError(format!(
                    "{} was not checked for this operation",
                    kind.name()
                ))
            })
    }

    pub fn command(&self, kind: ToolKind) -> Result<Command> {
        Ok(Command::new(&self.tool(kind)?.path))
    }

    pub async fn check_dump_server(&self, client: &deadpool_postgres::Client) -> Result<()> {
        let version: String = client
            .query_one("SHOW server_version_num", &[])
            .await?
            .try_get(0)?;
        let server = version
            .parse::<u32>()
            .map_err(|_| PGCliError::Other("Invalid server version".into()))?
            / 10000;
        let dump = self.tool(ToolKind::Dump)?;
        if dump.major < server {
            return Err(PGCliError::PGToolsError(format!(
                "pg_dump {} cannot dump PostgreSQL {server}; select a client >= {server}",
                dump.major
            )));
        }
        Ok(())
    }

    pub async fn validate_archive(&self, path: &Path) -> Result<()> {
        let mut command = self.command(ToolKind::Restore)?;
        command
            .arg("--list")
            .arg(path)
            .stdin(Stdio::null())
            .kill_on_drop(true);
        let output = command.output().await?;
        if !output.status.success() {
            return Err(PGCliError::PGToolsError("pg_restore cannot read this archive; check the format, integrity and producing client version".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_parser_requires_the_requested_tool() {
        assert_eq!(
            parse_version(ToolKind::Dump, "pg_dump (PostgreSQL) 18.1 (Ubuntu)\n"),
            Some(18)
        );
        assert_eq!(
            parse_version(ToolKind::Restore, "pg_dump (PostgreSQL) 18.1"),
            None
        );
        assert_eq!(parse_version(ToolKind::Psql, "unrelated 18.1"), None);
    }
}
