use super::structs::{DatabaseDetails, ExtensionDetails, OutputFormat, Result, UserDetails};
use csv::Writer;
use prettytable::{Cell, Row, Table};
use serde::Serialize;
use std::io::Write;

pub fn generated_password(username: &str, password: &str) -> Result<()> {
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(
        &mut output,
        &serde_json::json!({"username": username, "password": password}),
    )?;
    writeln!(output)?;
    Ok(())
}

pub trait Entity: Serialize {
    fn columns(extra: bool) -> &'static [&'static str];
    fn values(&self) -> Vec<String>;
}

impl Entity for DatabaseDetails {
    fn columns(extra: bool) -> &'static [&'static str] {
        if extra {
            &["IDX", "DB name", "Owner", "Size", "OID"]
        } else {
            &["IDX", "Name"]
        }
    }
    fn values(&self) -> Vec<String> {
        match self {
            Self::Name(name) => vec![name.clone()],
            Self::Extra {
                name,
                owner,
                size_pretty,
                oid,
            } => vec![
                name.clone(),
                owner.clone(),
                size_pretty.clone(),
                oid.to_string(),
            ],
        }
    }
}
impl Entity for UserDetails {
    fn columns(extra: bool) -> &'static [&'static str] {
        if extra {
            &["IDX", "Username", "Createdb", "Superuser", "Login", "OID"]
        } else {
            &["IDX", "Username"]
        }
    }
    fn values(&self) -> Vec<String> {
        match self {
            Self::Username(name) => vec![name.clone()],
            Self::Extra {
                username,
                createdb,
                superuser,
                login,
                oid,
            } => vec![
                username.clone(),
                createdb.to_string(),
                superuser.to_string(),
                login.to_string(),
                oid.to_string(),
            ],
        }
    }
}
impl Entity for ExtensionDetails {
    fn columns(extra: bool) -> &'static [&'static str] {
        if extra {
            &["IDX", "Name", "Owner", "Version", "Oid"]
        } else {
            &["IDX", "Name"]
        }
    }
    fn values(&self) -> Vec<String> {
        match self {
            Self::Name(name) => vec![name.clone()],
            Self::Extra {
                name,
                owner,
                version,
                oid,
            } => vec![
                name.clone(),
                owner.to_string(),
                version.clone(),
                oid.to_string(),
            ],
        }
    }
}

pub fn render<T: Entity>(
    entities: &[T],
    format: &OutputFormat,
    extra: bool,
    mut output: impl Write,
) -> Result<()> {
    match format {
        OutputFormat::Json => {
            serde_json::to_writer_pretty(&mut output, entities)?;
            writeln!(output)?;
        }
        OutputFormat::JsonCompact => {
            serde_json::to_writer(&mut output, entities)?;
            writeln!(output)?;
        }
        OutputFormat::JsonLines => {
            for entity in entities {
                serde_json::to_writer(&mut output, entity)?;
                writeln!(output)?;
            }
        }
        OutputFormat::Simple => {
            for entity in entities {
                writeln!(output, "{}", entity.values().join("\t"))?;
            }
        }
        OutputFormat::Csv => {
            let mut writer = Writer::from_writer(&mut output);
            writer.write_record(T::columns(extra))?;
            for (index, entity) in entities.iter().enumerate() {
                let mut values = vec![index.to_string()];
                values.extend(entity.values());
                writer.write_record(values)?;
            }
            writer.flush()?;
        }
        OutputFormat::Table => {
            let mut table = Table::new();
            table.add_row(Row::new(
                T::columns(extra)
                    .iter()
                    .map(|name| Cell::new(name))
                    .collect(),
            ));
            for (index, entity) in entities.iter().enumerate() {
                let mut values = vec![index.to_string()];
                values.extend(entity.values());
                table.add_row(Row::new(
                    values.iter().map(|value| Cell::new(value)).collect(),
                ));
            }
            table.print(&mut output)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_results_have_machine_readable_output() {
        for format in [OutputFormat::Json, OutputFormat::JsonCompact] {
            let mut bytes = Vec::new();
            render::<DatabaseDetails>(&[], &format, false, &mut bytes).unwrap();
            assert_eq!(String::from_utf8(bytes).unwrap(), "[]\n");
        }
        let mut bytes = Vec::new();
        render::<DatabaseDetails>(&[], &OutputFormat::Csv, false, &mut bytes).unwrap();
        assert_eq!(String::from_utf8(bytes).unwrap(), "IDX,Name\n");
    }
}
