use super::structs::{PGCliError, Result};

pub fn validate_identifier(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 63 || value.contains('\0') {
        return Err(PGCliError::Other(
            "SQL identifiers must contain 1–63 bytes and no NUL".into(),
        ));
    }
    Ok(())
}

pub fn identifier(value: &str) -> Result<String> {
    validate_identifier(value)?;
    Ok(format!("\"{}\"", value.replace('"', "\"\"")))
}

pub fn literal(value: &str) -> Result<String> {
    if value.contains('\0') {
        return Err(PGCliError::Other("SQL values cannot contain NUL".into()));
    }
    Ok(format!(
        "E'{}'",
        value.replace('\\', "\\\\").replace('\'', "''")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_quoted_identifiers_and_literal_punctuation() {
        assert_eq!(identifier("A-\"name").unwrap(), "\"A-\"\"name\"");
        assert_eq!(literal("x'\\;--").unwrap(), "E'x''\\\\;--'");
        for name in ["", "a\0b", &"x".repeat(64)] {
            assert!(identifier(name).is_err());
        }
        assert!(literal("x\0y").is_err());
    }
}
