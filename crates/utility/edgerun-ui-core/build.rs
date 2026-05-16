use std::env;
use std::fs;
use std::path::PathBuf;

const REQUIRED_TABLER_SVG_ICONS: &[&str] = &[
    "activity",
    "alert-triangle",
    "apps",
    "arrow-up",
    "bell",
    "check",
    "chevron-right",
    "code",
    "cpu",
    "database",
    "eye",
    "file",
    "key",
    "lock",
    "menu-2",
    "message-circle",
    "message-plus",
    "network",
    "route",
    "search",
    "server",
    "settings",
    "shield-check",
    "sparkles",
    "terminal-2",
    "trash",
    "user",
    "wallet",
    "x",
];

const REQUIRED_LUCIDE_SVG_ICONS: &[&str] = &[
    "activity",
    "app-window",
    "bell",
    "message-circle",
    "check",
    "chevron-right",
    "code",
    "cpu",
    "database",
    "eye",
    "file",
    "key",
    "lock",
    "menu",
    "message-circle-plus",
    "network",
    "route",
    "search",
    "arrow-up",
    "server",
    "settings",
    "shield-check",
    "sparkles",
    "square-terminal",
    "trash-2",
    "user",
    "wallet",
    "triangle-alert",
    "x",
];

fn main() {
    println!("cargo:rerun-if-changed=src/tabler_svg_atlas_generated.rs");
    println!("cargo:rerun-if-changed=src/lucide_svg_atlas_generated.rs");

    if env::var_os("CARGO_FEATURE_TABLER_SVG_ATLAS").is_some() {
        assert_generated_icons(
            "tabler-svg-atlas",
            "src/tabler_svg_atlas_generated.rs",
            REQUIRED_TABLER_SVG_ICONS,
        );
    }

    if env::var_os("CARGO_FEATURE_LUCIDE_SVG_ATLAS").is_some() {
        assert_generated_icons(
            "lucide-svg-atlas",
            "src/lucide_svg_atlas_generated.rs",
            REQUIRED_LUCIDE_SVG_ICONS,
        );
    }
}

fn assert_generated_icons(feature: &str, generated: &str, required: &[&str]) {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let generated_path = manifest_dir.join(generated);
    let generated = fs::read_to_string(&generated_path).unwrap_or_else(|err| {
        panic!(
            "{feature} is enabled but {} could not be read: {err}",
            generated_path.display()
        )
    });

    for name in required {
        let needle = format!("name: \"{name}\"");
        assert!(
            generated.contains(&needle),
            "{feature} is enabled but generated atlas is missing required icon `{name}`"
        );
    }
}
