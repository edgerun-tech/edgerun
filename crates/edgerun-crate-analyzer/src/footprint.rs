use std::path::Path;
use crate::crate_model::FootprintInfo;

pub fn collect_footprint(crate_dir: &Path, _scenario: &str) -> FootprintInfo {
    let mut info = FootprintInfo {
        binary_size: None,
        stripped_size: None,
        compressed_size: None,
        idle_rss: None,
        peak_rss: None,
        threads: None,
        open_fds: None,
        target_triple: Some(std::env::consts::ARCH.to_string()),
        measured: false,
    };

    check_binary_size(crate_dir, &mut info);

    info
}

fn check_binary_size(crate_dir: &Path, info: &mut FootprintInfo) {
    let target_dir = crate_dir.ancestors()
        .find(|p| p.join("Cargo.toml").exists())
        .and_then(|p| p.parent())
        .map(|p| p.join("target"));

    if let Some(ref target) = target_dir {
        let release_dir = target.join("release");
        if release_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&release_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_file() && path.extension().is_none() {
                        if let Ok(metadata) = std::fs::metadata(&path) {
                            info.binary_size = Some(metadata.len() as usize);
                            info.measured = true;
                        }
                    }
                }
            }
        }
    }
}
