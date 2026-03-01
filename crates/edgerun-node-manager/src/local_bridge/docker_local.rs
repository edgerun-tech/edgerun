// SPDX-License-Identifier: Apache-2.0
use anyhow::{anyhow, Result};

pub(crate) fn docker_container_selector_from(raw: &str) -> Result<String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(anyhow!("container selector is required"));
    }
    if value.len() > 128 {
        return Err(anyhow!("container selector is too long"));
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    {
        return Err(anyhow!("container selector contains invalid characters"));
    }
    Ok(value.to_string())
}

pub(crate) fn docker_container_action_from(raw: &str) -> Result<String> {
    let action = raw.trim().to_ascii_lowercase();
    match action.as_str() {
        "start" | "stop" | "restart" => Ok(action),
        _ => Err(anyhow!("action must be start, stop, or restart")),
    }
}
