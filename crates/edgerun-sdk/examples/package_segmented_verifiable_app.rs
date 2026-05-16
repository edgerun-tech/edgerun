use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use edgerun_sdk::browser_authoring::{BrowserAppArtifact, BrowserAppSpec, build_publishable_app};

fn main() {
    let out_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("frontend/public/apps/segmented-verifiable-app"));
    if let Err(err) = package(&out_dir) {
        eprintln!("package segmented-verifiable-app failed: {err}");
        std::process::exit(1);
    }
}

fn package(out_dir: &Path) -> Result<(), String> {
    let index = fs::read(out_dir.join("index.html")).map_err(|err| err.to_string())?;
    let browser = fs::read(out_dir.join("edgerun-browser.js")).map_err(|err| err.to_string())?;
    let manifest = fs::read(out_dir.join("manifest.json")).map_err(|err| err.to_string())?;
    let package = build_publishable_app(BrowserAppSpec {
        slug: "segmented-verifiable-app",
        name: "Segmented Verifiable Execution",
        version: "0.1.0",
        summary: "Browser-hosted demo for five-node segmented deterministic execution, user-node anchoring, node identity, and signed hash-linked event-log verification.",
        developer_seed: [0x11; 32],
        code_sha256: None,
        routes: Vec::new(),
        storage_namespaces: vec![b"segmented-verifiable-app/state"],
        provided_capabilities: Vec::new(),
        required_capabilities: Vec::new(),
        artifacts: vec![
            BrowserAppArtifact {
                path: "index.html",
                bytes: &index,
            },
            BrowserAppArtifact {
                path: "edgerun-browser.js",
                bytes: &browser,
            },
            BrowserAppArtifact {
                path: "manifest.json",
                bytes: &manifest,
            },
        ],
    })?;
    fs::write(out_dir.join("app.edapp"), package.app_manifest_bytes)
        .map_err(|err| err.to_string())?;
    fs::write(out_dir.join("app.eapp"), package.app_graph_bytes).map_err(|err| err.to_string())?;
    fs::write(out_dir.join("developer.esig"), package.developer_signature)
        .map_err(|err| err.to_string())?;

    println!("app_id: {}", hex(&package.app_id));
    println!("release_id: {}", hex(&package.release_id));
    println!(
        "developer_public_key: {}",
        hex(&package.developer_public_key)
    );
    println!("manifest_sha256: {}", hex(&package.app_manifest_sha256));
    println!("app_graph_sha256: {}", hex(&package.app_graph_sha256));
    println!("artifact_graph: {}", out_dir.join("app.eapp").display());
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}
