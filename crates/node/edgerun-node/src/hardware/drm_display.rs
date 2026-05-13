use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use edgerun_devices::display::{
    DisplayDevice, DisplayInfo, DisplayMode, DisplayUpdateRequest, default_display_descriptor,
};
use edgerun_linux_sysfs::prelude::v1::*;
use edgerun_linux_sysfs::{parse_display_mode_line, read_trimmed};
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

fn parse_mode_line(line: &str) -> Option<DisplayMode> {
    parse_display_mode_line(line).map(|mode| DisplayMode {
        width: mode.width,
        height: mode.height,
        refresh_millihz: mode.refresh_millihz,
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
        let enabled = read_trimmed(&path.join("enabled")).as_deref() == Some("enabled");
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
    use alloc::vec;
    use edgerun_linux_sysfs::temp_root;

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

    #[test]
    fn parse_mode_line_without_refresh() {
        let mode = parse_mode_line("1920x1080").unwrap();
        assert_eq!(mode.width, 1920);
        assert_eq!(mode.height, 1080);
        assert_eq!(mode.refresh_millihz, 60_000); // default
    }

    #[test]
    fn parse_mode_line_with_interlaced() {
        let mode = parse_mode_line("1920x1080i@60").unwrap();
        assert_eq!(mode.width, 1920);
        assert_eq!(mode.height, 1080);
        assert_eq!(mode.refresh_millihz, 60_000);
    }

    #[test]
    fn parse_mode_line_with_progressive() {
        let mode = parse_mode_line("1920x1080p@120").unwrap();
        assert_eq!(mode.width, 1920);
        assert_eq!(mode.height, 1080);
        assert_eq!(mode.refresh_millihz, 120_000);
    }

    #[test]
    fn parse_mode_line_rejects_invalid() {
        assert!(parse_mode_line("").is_none());
        assert!(parse_mode_line("invalid").is_none());
        assert!(parse_mode_line("1920").is_none());
        assert!(parse_mode_line("axb@60").is_none());
    }

    #[test]
    fn parse_modes_collects_valid_lines() {
        let input = "1920x1080@60\n1280x720@60\ninvalid\n800x600@75\n";
        let modes = parse_modes(input);
        assert_eq!(modes.len(), 3);
        assert_eq!(modes[0].width, 1920);
        assert_eq!(modes[1].width, 1280);
        assert_eq!(modes[2].width, 800);
    }

    #[test]
    fn parse_modes_empty_input() {
        let modes = parse_modes("");
        assert!(modes.is_empty());
    }

    #[test]
    fn discover_connectors_filters_non_connector_entries() {
        let root = temp_root("drm-filter");
        // Should be skipped: starts with "version"
        let version = root.join("version");
        fs::create_dir_all(&version).unwrap();
        fs::write(version.join("status"), "connected\n").unwrap();
        // Should be skipped: no dash in name
        let nodash = root.join("card0");
        fs::create_dir_all(&nodash).unwrap();
        fs::write(nodash.join("status"), "connected\n").unwrap();
        // Should be included
        let conn = root.join("card0-HDMI-A-1");
        fs::create_dir_all(&conn).unwrap();
        fs::write(conn.join("status"), "connected\n").unwrap();
        fs::write(conn.join("enabled"), "enabled\n").unwrap();

        let connectors = discover_drm_connectors_in(&root).unwrap();
        assert_eq!(connectors.len(), 1);
        assert_eq!(connectors[0].connector_name, "card0-HDMI-A-1");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn discover_connectors_nonexistent_root() {
        let connectors =
            discover_drm_connectors_in(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(connectors.is_ok());
        assert!(connectors.unwrap().is_empty());
    }

    #[test]
    fn discover_connectors_handles_missing_files() {
        let root = temp_root("drm-missing");
        let conn = root.join("card0-DP-1");
        fs::create_dir_all(&conn).unwrap();
        // No status, enabled, mode, modes files — should use defaults
        let connectors = discover_drm_connectors_in(&root).unwrap();
        assert_eq!(connectors.len(), 1);
        assert!(!connectors[0].connected);
        assert!(!connectors[0].enabled);
        assert!(connectors[0].modes.is_empty());
        assert!(connectors[0].current_mode.is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn drm_connector_info_clone_debug() {
        let info = DrmConnectorInfo {
            sysfs_path: PathBuf::from("/sys/class/drm/card0-eDP-1"),
            connector_name: "card0-eDP-1".into(),
            enabled: true,
            connected: true,
            current_mode: Some(DisplayMode {
                width: 1920,
                height: 1080,
                refresh_millihz: 60_000,
            }),
            modes: vec![DisplayMode {
                width: 1280,
                height: 720,
                refresh_millihz: 60_000,
            }],
        };
        let cloned = info.clone();
        assert_eq!(info.connector_name, cloned.connector_name);
        assert!(cloned.connected);
        assert_eq!(cloned.modes.len(), 1);
        let debug_str = format!("{info:?}");
        assert!(debug_str.contains("DrmConnectorInfo"));
    }

    #[test]
    fn drm_display_backend_descriptor() {
        let backend = DrmDisplayBackend {
            sysfs_root: PathBuf::from("/sys/class/drm"),
            connector: DrmConnectorInfo {
                sysfs_path: PathBuf::from("/sys/class/drm/card0-eDP-1"),
                connector_name: "card0-eDP-1".into(),
                enabled: true,
                connected: true,
                current_mode: None,
                modes: vec![],
            },
        };
        let desc = backend.descriptor();
        assert_eq!(desc.provider_name, "drm-sysfs");
        assert_eq!(desc.provider_instance_id, "card0-eDP-1");
    }

    #[test]
    fn drm_display_backend_display_info_with_current_mode() {
        let backend = DrmDisplayBackend {
            sysfs_root: PathBuf::from("/sys/class/drm"),
            connector: DrmConnectorInfo {
                sysfs_path: PathBuf::from("/sys/class/drm/card0-eDP-1"),
                connector_name: "eDP-1".into(),
                enabled: true,
                connected: true,
                current_mode: Some(DisplayMode {
                    width: 2560,
                    height: 1440,
                    refresh_millihz: 144_000,
                }),
                modes: vec![DisplayMode {
                    width: 1920,
                    height: 1080,
                    refresh_millihz: 60_000,
                }],
            },
        };
        let info = backend.display_info().unwrap();
        assert_eq!(info.provider, "drm-sysfs");
        assert_eq!(info.display_name, "eDP-1");
        assert_eq!(info.instance_id, "eDP-1");
        assert!(info.built_in); // eDP-1 starts with "eDP"
        assert!(info.primary);
        assert_eq!(info.current_mode.width, 2560);
        assert_eq!(info.modes.len(), 1);
        assert!(!info.hdr_capable);
        assert!(!info.touch_capable);
    }

    #[test]
    fn drm_display_backend_display_info_falls_back_to_first_mode() {
        let backend = DrmDisplayBackend {
            sysfs_root: PathBuf::from("/sys/class/drm"),
            connector: DrmConnectorInfo {
                sysfs_path: PathBuf::from("/sys/class/drm/card0-HDMI-A-1"),
                connector_name: "card0-HDMI-A-1".into(),
                enabled: false,
                connected: true,
                current_mode: None,
                modes: vec![DisplayMode {
                    width: 3840,
                    height: 2160,
                    refresh_millihz: 30_000,
                }],
            },
        };
        let info = backend.display_info().unwrap();
        assert_eq!(info.current_mode.width, 3840);
        assert!(!info.built_in); // HDMI does not start with eDP or LVDS
        assert!(!info.primary);
    }

    #[test]
    fn drm_display_backend_display_info_errors_without_modes() {
        let backend = DrmDisplayBackend {
            sysfs_root: PathBuf::from("/sys/class/drm"),
            connector: DrmConnectorInfo {
                sysfs_path: PathBuf::from("/sys/class/drm/card0-VGA-1"),
                connector_name: "card0-VGA-1".into(),
                enabled: false,
                connected: false,
                current_mode: None,
                modes: vec![],
            },
        };
        let err = backend.display_info().unwrap_err();
        assert!(matches!(err, CapabilityError::Unsupported(_)));
    }

    #[test]
    fn drm_display_backend_present_returns_unsupported() {
        let mut backend = DrmDisplayBackend {
            sysfs_root: PathBuf::from("/sys/class/drm"),
            connector: DrmConnectorInfo {
                sysfs_path: PathBuf::from("/sys/class/drm/card0-eDP-1"),
                connector_name: "card0-eDP-1".into(),
                enabled: true,
                connected: true,
                current_mode: Some(DisplayMode {
                    width: 1920,
                    height: 1080,
                    refresh_millihz: 60_000,
                }),
                modes: vec![],
            },
        };
        let request = DisplayUpdateRequest {
            content_kind: edgerun_devices::display::DisplayContentKind::Image,
            width: Some(1920),
            height: Some(1080),
            refresh_millihz: Some(60_000),
        };
        let err = backend.present(&request).unwrap_err();
        assert!(matches!(err, CapabilityError::Unsupported(_)));
    }

    #[test]
    fn lvds_connector_is_built_in() {
        let backend = DrmDisplayBackend {
            sysfs_root: PathBuf::from("/sys/class/drm"),
            connector: DrmConnectorInfo {
                sysfs_path: PathBuf::from("/sys/class/drm/card0-LVDS-1"),
                connector_name: "LVDS-1".into(), // starts with "LVDS"
                enabled: true,
                connected: true,
                current_mode: Some(DisplayMode {
                    width: 1366,
                    height: 768,
                    refresh_millihz: 60_000,
                }),
                modes: vec![],
            },
        };
        let info = backend.display_info().unwrap();
        assert!(info.built_in);
    }

    #[test]
    fn connectors_sorted_by_name() {
        let root = temp_root("drm-sort");
        for name in &["card0-HDMI-A-1", "card0-eDP-1", "card0-DP-1"] {
            let conn = root.join(name);
            fs::create_dir_all(&conn).unwrap();
            fs::write(conn.join("status"), "connected\n").unwrap();
            fs::write(conn.join("enabled"), "enabled\n").unwrap();
        }
        let connectors = discover_drm_connectors_in(&root).unwrap();
        assert_eq!(connectors.len(), 3);
        assert!(connectors[0].connector_name < connectors[1].connector_name);
        assert!(connectors[1].connector_name < connectors[2].connector_name);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn display_mode_clone_debug() {
        let mode = DisplayMode {
            width: 1920,
            height: 1080,
            refresh_millihz: 60_000,
        };
        let cloned = mode.clone();
        assert_eq!(mode.width, cloned.width);
        let debug_str = format!("{mode:?}");
        assert!(debug_str.contains("DisplayMode"));
    }

    #[test]
    fn display_update_request_clone_debug() {
        let request = DisplayUpdateRequest {
            content_kind: edgerun_devices::display::DisplayContentKind::Video,
            width: Some(2560),
            height: Some(1440),
            refresh_millihz: Some(144_000),
        };
        let cloned = request.clone();
        assert_eq!(request.width, cloned.width);
        assert_eq!(request.height, cloned.height);
    }

    #[test]
    fn display_info_clone_debug() {
        let info = DisplayInfo {
            provider: "drm-sysfs".into(),
            display_name: "card0-eDP-1".into(),
            instance_id: "card0-eDP-1".into(),
            built_in: true,
            primary: true,
            current_mode: DisplayMode {
                width: 1920,
                height: 1080,
                refresh_millihz: 60_000,
            },
            modes: vec![],
            hdr_capable: false,
            touch_capable: false,
        };
        let cloned = info.clone();
        assert_eq!(info.display_name, cloned.display_name);
        assert!(cloned.built_in);
    }
}
