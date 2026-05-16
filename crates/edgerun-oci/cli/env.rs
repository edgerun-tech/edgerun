use crate::prelude::*;
use std::io;
use std::path::Path;

pub(crate) fn parse_env_assignment(value: &str) -> io::Result<String> {
    parse_env_pair(value)?;
    Ok(value.to_string())
}

pub(crate) fn parse_env_pair(value: &str) -> io::Result<(&str, &str)> {
    let (key, value) = value.split_once('=').ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "environment entries must be KEY=VALUE",
        )
    })?;
    validate_env_key(key)?;
    Ok((key, value))
}

pub(crate) fn read_env_file(path: &Path) -> io::Result<Vec<String>> {
    let data = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for (index, line) in data.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let parsed = parse_env_assignment(line).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("{}:{}: {}", path.display(), index + 1, error),
            )
        })?;
        out.push(parsed);
    }
    Ok(out)
}

pub(crate) fn upsert_env(env: &mut Vec<String>, key: &str, value: &str) {
    let prefix = format!("{key}=");
    env.retain(|entry| !entry.starts_with(&prefix));
    env.push(format!("{key}={value}"));
}

fn validate_env_key(key: &str) -> io::Result<()> {
    let mut bytes = key.bytes();
    let Some(first) = bytes.next() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "environment key must not be empty",
        ));
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "environment key must start with a letter or '_'",
        ));
    }
    if !bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "environment key must contain only letters, digits, and '_'",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_validates_env_assignments() {
        assert_eq!(parse_env_assignment("A_1=value").unwrap(), "A_1=value");
        assert!(parse_env_assignment("A_1").is_err());
        assert!(parse_env_assignment("1A=value").is_err());
    }

    #[test]
    fn upsert_env_replaces_existing_key() {
        let mut env = vec!["A=old".into(), "B=2".into()];
        upsert_env(&mut env, "A", "new");
        assert_eq!(env, vec!["B=2", "A=new"]);
    }
}
