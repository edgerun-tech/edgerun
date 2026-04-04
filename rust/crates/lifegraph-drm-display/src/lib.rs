use lifegraph_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use lifegraph_display::{
    default_display_descriptor, DisplayDevice, DisplayInfo, DisplayMode, DisplayUpdateRequest,
};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrmConnectorInfo {
    pub sysfs_path: PathBuf,
    pub connector_name: String,
    pub enabled: bool,
    pub connected: bool,
    pub current_mode: Option<DisplayMode>,
    pub modes: Vec<DisplayMode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrmDisplayBackend {
    pub sysfs_root: PathBuf,
    pub connector: DrmConnectorInfo,
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn parse_mode_line(line: &str) -> Option<DisplayMode> {
    let mut parts = line.split('x');
    let width = parts.next()?.trim().parse().ok()?;
    let rest = parts.next()?;
    let mut rest_parts = rest.split(['i', 'p', '@']);
    let height: u32 = rest_parts.next()?.trim().parse().ok()?;
    let refresh = line
        .split('@')
        .nth(1)
        .and_then(|v| {
            v.trim_end_matches('H')
                .trim_end_matches('z')
                .parse::<f32>()
                .ok()
        })
        .map(|hz| (hz * 1000.0) as u32)
        .unwrap_or(60_000);
    Some(DisplayMode {
        width,
        height,
        refresh_millihz: refresh,
    })
}

fn parse_modes(contents: &str) -> Vec<DisplayMode> {
    contents.lines().filter_map(parse_mode_line).collect()
}

pub fn discover_drm_connectors() -> Result<Vec<DrmConnectorInfo>, CapabilityError> {
    discover_drm_connectors_in(Path::new("/sys/class/drm"))
}

pub fn discover_drm_connectors_in(root: &Path) -> Result<Vec<DrmConnectorInfo>, CapabilityError> {
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(root).map_err(|e| CapabilityError::Provider(e.to_string()))? {
        let entry = entry.map_err(|e| CapabilityError::Provider(e.to_string()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.contains('-') || name.starts_with("version") {
            continue;
        }
        let status = read_trimmed(&path.join("status")).unwrap_or_default();
        let enabled = read_trimmed(&path.join("enabled")).is_some_and(|v| v == "enabled");
        let modes = fs::read_to_string(path.join("modes"))
            .ok()
            .map(|s| parse_modes(&s))
            .unwrap_or_default();
        let current_mode = read_trimmed(&path.join("mode")).and_then(|line| parse_mode_line(&line));
        out.push(DrmConnectorInfo {
            sysfs_path: path,
            connector_name: name,
            enabled,
            connected: status == "connected",
            current_mode,
            modes,
        });
    }
    out.sort_by(|a, b| a.connector_name.cmp(&b.connector_name));
    Ok(out)
}

impl CapabilityProvider for DrmDisplayBackend {
    fn descriptor(&self) -> CapabilityDescriptor {
        default_display_descriptor("drm-sysfs", &self.connector.connector_name)
    }
}

impl DisplayDevice for DrmDisplayBackend {
    fn display_info(&self) -> Result<DisplayInfo, CapabilityError> {
        let current_mode = self
            .connector
            .current_mode
            .or_else(|| self.connector.modes.first().copied())
            .ok_or(CapabilityError::Unsupported(
                "display has no reported modes",
            ))?;
        Ok(DisplayInfo {
            provider: "drm-sysfs".into(),
            display_name: self.connector.connector_name.clone(),
            instance_id: self.connector.connector_name.clone(),
            built_in: self.connector.connector_name.starts_with("eDP")
                || self.connector.connector_name.starts_with("LVDS"),
            primary: self.connector.enabled,
            current_mode,
            modes: self.connector.modes.clone(),
            hdr_capable: false,
            touch_capable: false,
        })
    }

    fn present(&mut self, _request: &DisplayUpdateRequest) -> Result<(), CapabilityError> {
        Err(CapabilityError::Unsupported(
            "drm sysfs backend does not implement presentation control",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("{name}-{unique}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn parse_mode_line_supports_refresh() {
        let mode = parse_mode_line("1920x1080@60").unwrap();
        assert_eq!(mode.width, 1920);
        assert_eq!(mode.height, 1080);
        assert_eq!(mode.refresh_millihz, 60_000);
    }

    #[test]
    fn discover_connectors_from_sysfs_layout() {
        let root = temp_root("drm-display");
        let conn = root.join("card0-eDP-1");
        fs::create_dir_all(&conn).unwrap();
        fs::write(conn.join("status"), "connected\n").unwrap();
        fs::write(conn.join("enabled"), "enabled\n").unwrap();
        fs::write(conn.join("mode"), "1920x1080\n").unwrap();
        fs::write(conn.join("modes"), "1920x1080\n1280x720\n").unwrap();
        let connectors = discover_drm_connectors_in(&root).unwrap();
        assert_eq!(connectors.len(), 1);
        assert!(connectors[0].connected);
        assert_eq!(connectors[0].modes.len(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}
