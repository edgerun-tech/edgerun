// SPDX-License-Identifier: Apache-2.0
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const FONT_SEARCH_DIRS: &[&str] = &[
    r"/usr/share/fonts",
    r"/usr/local/share/fonts",
    r"/Library/Fonts",
    r"/System/Library/Fonts",
    r"/usr/share/fonts/TTF",
    r"/usr/share/fonts/truetype",
    r"/usr/share/fonts/opentype",
    r"C:\\Windows\\Fonts",
];

pub const FONT_FALLBACK_PATTERNS: &[&str] = &[
    "symbolsnerdfont",
    "symbols nfm",
    "nerdfont",
    "symbola",
    "notosanscjk",
    "noto sans cjk",
    "noto sans mono cjk",
    "sourcehan",
    "source han",
    "wqy",
    "wenquanyi",
    "unifont",
    "notoemoji",
    "color-emoji",
    "emoji",
    "twemoji",
    "emojione",
    "segoeuiemoji",
    "seguiemj",
    "seguisym",
];

// Common color emoji locations we want to prioritize.
pub const COLOR_EMOJI_PATHS: &[&str] = &[
    r"/usr/share/fonts/noto/NotoColorEmoji.ttf",
    r"/usr/share/fonts/noto-emoji/NotoColorEmoji.ttf",
    r"/usr/share/fonts/noto-color-emoji/NotoColorEmoji.ttf",
    r"/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
    r"/System/Library/Fonts/Apple Color Emoji.ttc",
    r"/Library/Fonts/Apple Color Emoji.ttc",
    r"C:\\Windows\\Fonts\\seguiemj.ttf",
];

pub const FALLBACK_FONT_PATHS: &[&str] = &[
    r"/usr/share/fonts/noto-emoji/NotoEmoji[wght].ttf",
    r"/usr/share/fonts/noto-emoji/NotoEmoji-Regular.ttf",
    r"/usr/share/fonts/noto-color-emoji/NotoColorEmoji.ttf",
    r"/usr/share/fonts/noto/NotoColorEmoji.ttf",
    r"/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
    r"/usr/share/fonts/noto-color-emoji-compat-test/NotoColorEmojiCompatTest-Regular.ttf",
    r"/usr/share/fonts/truetype/noto/NotoSansSymbols2-Regular.ttf",
    r"/usr/share/fonts/noto/NotoSansSymbols2-Regular.ttf",
    r"/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    r"/usr/share/fonts/opentype/noto/NotoSansCJKsc-Regular.otf",
    r"/usr/share/fonts/opentype/noto/NotoSansCJKjp-Regular.otf",
    r"/usr/share/fonts/opentype/noto/NotoSansCJKkr-Regular.otf",
    r"/usr/share/fonts/opentype/noto/NotoSansCJKtc-Regular.otf",
    r"/usr/share/fonts/opentype/noto/NotoSansMonoCJKsc-Regular.otf",
    r"/usr/share/fonts/opentype/noto/NotoSansMonoCJKjp-Regular.otf",
    r"/usr/share/fonts/opentype/noto/NotoSansMonoCJKkr-Regular.otf",
    r"/usr/share/fonts/opentype/noto/NotoSansMonoCJKtc-Regular.otf",
    r"/usr/share/fonts/truetype/arphic/uming.ttc",
    r"/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    r"/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    r"/usr/share/fonts/truetype/unifont/unifont.ttf",
    r"/usr/share/fonts/TTF/Symbola.ttf",
    r"/usr/share/fonts/truetype/nerd-fonts/SymbolsNerdFont-Regular.ttf",
    r"/usr/share/fonts/truetype/nerd-fonts/SymbolsNerdFontMono-Regular.ttf",
    r"/usr/share/fonts/truetype/nerd-fonts/SymbolsNerdFontPropo-Regular.ttf",
    r"/usr/share/fonts/truetype/nerd-fonts/SymbolsNFM-Regular.ttf",
    r"/usr/share/fonts/TTF/SymbolsNerdFont-Regular.ttf",
    r"/usr/share/fonts/TTF/SymbolsNerdFontMono-Regular.ttf",
    r"/usr/share/fonts/TTF/SymbolsNerdFontPropo-Regular.ttf",
    r"/usr/share/fonts/TTF/SymbolsNFM-Regular.ttf",
    r"/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    r"/usr/share/fonts/TTF/DejaVuSans.ttf",
    r"/System/Library/Fonts/Apple Color Emoji.ttc",
    r"/System/Library/Fonts/Apple Symbols.ttf",
    r"/Library/Fonts/Apple Color Emoji.ttc",
    r"C:\\Windows\\Fonts\\seguiemj.ttf",
    r"C:\\Windows\\Fonts\\seguisym.ttf",
];

// Match the user's kitty font config.
const KITTYS_FONT_NAMES: &[&str] = &[
    "JetBrainsMono Nerd Font",
    "JetBrainsMono Nerd Font Mono",
    "JetBrainsMonoNL Nerd Font",
    "JetBrains Mono Nerd Font",
];

fn load_font_from_path(path: &Path) -> Option<Arc<Vec<u8>>> {
    fs::read(path).ok().map(Arc::new)
}

fn search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = FONT_SEARCH_DIRS
        .iter()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect();

    if let Ok(home) = env::var("HOME") {
        for suffix in [".local/share/fonts", ".fonts"] {
            let path = PathBuf::from(format!("{home}/{suffix}"));
            if path.exists() {
                dirs.push(path);
            }
        }
    }

    dirs
}

fn load_fonts_by_name(names: &[String], loaded: &mut HashSet<String>) -> Vec<Arc<Vec<u8>>> {
    if names.is_empty() {
        return Vec::new();
    }

    let mut targets: HashSet<String> = names.iter().cloned().collect();
    let mut found: HashMap<String, Arc<Vec<u8>>> = HashMap::new();

    for dir in search_dirs() {
        walk_font_dir(&dir, &mut targets, &mut found);
        if targets.is_empty() {
            break;
        }
    }

    let mut ordered = Vec::new();
    for name in names {
        if let Some(font) = found.remove(name) {
            loaded.insert(name.clone());
            ordered.push(font);
        }
    }
    ordered
}

fn load_fonts_by_pattern(
    patterns: &[&str],
    limit: usize,
    _loaded: &mut HashSet<String>,
) -> Vec<Arc<Vec<u8>>> {
    let mut fonts = Vec::new();
    let mut found = HashMap::new();
    let mut targets: HashSet<String> = patterns.iter().map(|p| p.to_ascii_lowercase()).collect();

    for dir in search_dirs() {
        walk_font_dir(&dir, &mut targets, &mut found);
        if fonts.len() >= limit {
            break;
        }
    }

    for (_k, font) in found.into_iter().take(limit.saturating_sub(fonts.len())) {
        fonts.push(font);
    }
    fonts
}

fn walk_font_dir(
    dir: &Path,
    targets: &mut HashSet<String>,
    found: &mut HashMap<String, Arc<Vec<u8>>>,
) {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        let entries = match fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let key = file_name.to_ascii_lowercase();
            if !targets.contains(&key) {
                continue;
            }

            if let Ok(bytes) = fs::read(&path) {
                found.insert(key.clone(), Arc::new(bytes));
            }
        }
    }
}

fn load_dir_fonts(dir: &Path, loaded: &mut HashSet<String>) -> Vec<Arc<Vec<u8>>> {
    let mut fonts = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        let entries = match fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let key = name.to_ascii_lowercase();
            if loaded.contains(&key) {
                continue;
            }
            if let Some(font) = load_font_from_path(&p) {
                loaded.insert(key);
                fonts.push(font);
            }
        }
    }
    fonts
}

pub fn load_fallback_fonts() -> Vec<Arc<Vec<u8>>> {
    let mut fonts = Vec::new();
    let mut loaded = HashSet::new();
    let mut seen_names = HashSet::new();
    let mut missing_names = Vec::new();

    // Prioritize explicit color emoji paths so emoji pick the color face first.
    for path in COLOR_EMOJI_PATHS {
        let file_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_ascii_lowercase());
        if let Some(name) = &file_name {
            if !seen_names.insert(name.clone()) {
                continue;
            }
        }
        if let Some(font) = load_font_from_path(Path::new(path)) {
            if let Some(name) = &file_name {
                loaded.insert(name.clone());
            }
            fonts.push(font);
        } else if let Some(name) = file_name {
            if !missing_names.contains(&name) {
                missing_names.push(name);
            }
        }
    }

    for path in FALLBACK_FONT_PATHS {
        let file_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_ascii_lowercase());

        if let Some(name) = &file_name {
            if !seen_names.insert(name.clone()) {
                continue;
            }
        }

        if let Some(font) = load_font_from_path(Path::new(path)) {
            if let Some(name) = &file_name {
                loaded.insert(name.clone());
            }
            fonts.push(font);
        } else if let Some(name) = file_name {
            if !missing_names.contains(&name) {
                missing_names.push(name);
            }
        }
    }

    for font in load_fonts_by_name(&missing_names, &mut loaded) {
        fonts.push(font);
    }
    for font in load_fonts_by_pattern(FONT_FALLBACK_PATTERNS, 8, &mut loaded) {
        fonts.push(font);
    }

    // Also load any user-downloaded fonts in our private emoji directory.
    if let Ok(home) = env::var("HOME") {
        let dir = PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("fonts")
            .join("term-emoji");
        if dir.exists() {
            for font in load_dir_fonts(&dir, &mut loaded) {
                fonts.push(font);
            }
        }
    }

    fonts
}

/// Try to load the primary font that kitty is configured to use.
pub fn load_kitty_primary_font() -> Option<Arc<Vec<u8>>> {
    let mut loaded = HashSet::new();
    let names: Vec<String> = KITTYS_FONT_NAMES.iter().map(|s| s.to_string()).collect();
    load_fonts_by_name(&names, &mut loaded).into_iter().next()
}
