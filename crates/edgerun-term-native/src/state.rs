use std::collections::{HashMap, HashSet, hash_map::Entry};
use std::env;
use std::fs;
use std::path::PathBuf;

use arboard::Clipboard;
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use term_ui::debug::DebugRenderMode;
use term_ui::widgets::settings::SettingsPanel;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SettingsState {
    pub(crate) scrollback_enabled: bool,
    pub(crate) show_fps: bool,
    pub(crate) show_copy_notice: bool,
    pub(crate) render_mode: String,
    pub(crate) log_level: String,
}

impl SettingsState {
    pub(crate) fn from_panel(settings: &SettingsPanel) -> Self {
        Self {
            scrollback_enabled: settings.scrollback_enabled,
            show_fps: settings.show_fps,
            show_copy_notice: settings.show_copy_notice,
            render_mode: settings.render_mode.to_string(),
            log_level: settings.log_level.clone(),
        }
    }
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            scrollback_enabled: true,
            show_fps: false,
            show_copy_notice: false,
            render_mode: "auto".to_string(),
            log_level: "debug".to_string(),
        }
    }
}

fn settings_state_path() -> Option<PathBuf> {
    if let Some(dir) = env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(dir).join("term").join("settings.json"));
    }
    if let Some(dir) = config_dir() {
        return Some(dir.join("term").join("settings.json"));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config").join("term").join("settings.json"))
}

pub(crate) fn load_settings_state() -> Option<SettingsState> {
    let path = settings_state_path()?;
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice::<SettingsState>(&bytes).ok()
}

pub(crate) fn persist_settings_state(state: SettingsState) {
    if let Some(path) = settings_state_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(&state) {
            let _ = fs::write(&path, bytes);
        }
    }
}

pub(crate) fn parse_render_mode(value: &str) -> Option<DebugRenderMode> {
    match value.to_ascii_lowercase().as_str() {
        "auto" => Some(DebugRenderMode::Auto),
        "cpu" => Some(DebugRenderMode::CpuOnly),
        "gpu" => Some(DebugRenderMode::GpuOnly),
        _ => None,
    }
}

pub(crate) fn next_render_mode(current: DebugRenderMode) -> DebugRenderMode {
    match current {
        DebugRenderMode::Auto => DebugRenderMode::GpuOnly,
        DebugRenderMode::GpuOnly => DebugRenderMode::CpuOnly,
        DebugRenderMode::CpuOnly => DebugRenderMode::Auto,
    }
}

pub(crate) fn parse_log_level(value: &str) -> Option<log::LevelFilter> {
    match value.to_ascii_lowercase().as_str() {
        "error" => Some(log::LevelFilter::Error),
        "warn" | "warning" => Some(log::LevelFilter::Warn),
        "info" => Some(log::LevelFilter::Info),
        "debug" => Some(log::LevelFilter::Debug),
        "trace" => Some(log::LevelFilter::Trace),
        _ => None,
    }
}

pub(crate) fn next_log_level(current: &str) -> &'static str {
    match current.to_ascii_lowercase().as_str() {
        "error" => "warn",
        "warn" | "warning" => "info",
        "info" => "debug",
        "debug" => "trace",
        "trace" => "error",
        _ => "debug",
    }
}

#[derive(Serialize, Deserialize, Default)]
struct LearnedStore {
    counts: HashMap<String, u32>,
    path: Option<PathBuf>,
}

impl LearnedStore {
    fn load() -> Self {
        let path = autocomplete_store_path();
        if let Some(p) = path.clone()
            && let Ok(bytes) = fs::read(&p)
            && let Ok(counts) = serde_json::from_slice::<HashMap<String, u32>>(&bytes)
        {
            return Self { counts, path };
        }
        Self {
            counts: HashMap::new(),
            path,
        }
    }

    fn record(&mut self, entry: &str) {
        let clean = entry.trim();
        if clean.is_empty() {
            return;
        }
        let count = self.counts.entry(clean.to_string()).or_insert(0);
        *count = count.saturating_add(1);
        self.persist();
    }

    fn persist(&self) {
        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(&self.counts) {
                let _ = fs::write(path, json);
            }
        }
    }
}

pub(crate) struct AutocompleteEngine {
    learned: LearnedStore,
}

impl AutocompleteEngine {
    pub(crate) fn load() -> Self {
        Self {
            learned: LearnedStore::load(),
        }
    }

    fn prefix_score(text: &str, prefix: &str) -> i32 {
        if prefix.is_empty() {
            return 0;
        }
        if text.starts_with(prefix) {
            5_000
        } else if text.contains(prefix) {
            2_000
        } else {
            0
        }
    }

    pub(crate) fn suggest(&mut self, prefix: &str, limit: usize) -> Vec<String> {
        let prefix = prefix.trim();
        let mut scores: HashMap<String, i32> = HashMap::new();
        let mut add = |text: String, base: i32, freq: u32| {
            let clean = text.trim();
            if clean.len() < 2 {
                return;
            }
            if !prefix.is_empty() && !clean.starts_with(prefix) && !clean.contains(prefix) {
                return;
            }
            let score = base + Self::prefix_score(clean, prefix) + (freq as i32 * 25);
            match scores.entry(clean.to_string()) {
                Entry::Vacant(v) => {
                    v.insert(score);
                }
                Entry::Occupied(mut o) => {
                    if score > *o.get() {
                        o.insert(score);
                    }
                }
            }
        };

        for (idx, cmd) in load_shell_history(1200).into_iter().enumerate() {
            let base = 2_000 - idx as i32;
            add(cmd, base, 1);
        }

        for (text, freq) in self.learned.counts.clone() {
            add(text, 4_000, freq);
        }

        for (idx, clip) in read_clipboard_suggestions(64).into_iter().enumerate() {
            let base = 800 - idx as i32;
            add(clip, base, 1);
        }

        for (idx, prog) in crate::scan_path_programs(prefix, 256)
            .into_iter()
            .enumerate()
        {
            let base = 1200 - idx as i32;
            add(prog, base, 1);
        }

        let mut entries: Vec<(String, i32)> = scores.into_iter().collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        entries.truncate(limit);
        entries.into_iter().map(|(t, _)| t).collect()
    }

    pub(crate) fn record_accept(&mut self, entry: &str) {
        self.learned.record(entry);
    }
}

fn load_shell_history(limit: usize) -> Vec<String> {
    let mut entries: Vec<String> = Vec::new();
    if let Some(home) = env::var_os("HOME") {
        let home = std::path::PathBuf::from(home);
        let files = [".zsh_history", ".bash_history"];
        'outer: for file in files {
            let path = home.join(file);
            if let Ok(contents) = fs::read_to_string(&path) {
                for line in contents.lines().rev() {
                    let trimmed = if line.starts_with(':') {
                        line.split_once(';')
                            .map(|x| x.1)
                            .unwrap_or("")
                            .trim()
                            .to_string()
                    } else {
                        line.trim().to_string()
                    };
                    if trimmed.is_empty() {
                        continue;
                    }
                    entries.push(trimmed);
                    if entries.len() >= limit * 2 {
                        break 'outer;
                    }
                }
            }
        }
    }

    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for cmd in entries {
        if seen.insert(cmd.clone()) {
            deduped.push(cmd);
            if deduped.len() >= limit {
                break;
            }
        }
    }
    deduped
}

fn read_clipboard_suggestions(limit: usize) -> Vec<String> {
    if let Ok(mut clipboard) = Clipboard::new()
        && let Ok(text) = clipboard.get_text()
    {
        let mut lines = Vec::new();
        for line in text.lines().rev() {
            let trimmed = line.trim();
            if trimmed.len() < 2 {
                continue;
            }
            lines.push(trimmed.to_string());
            if lines.len() >= limit {
                break;
            }
        }
        return lines;
    }
    Vec::new()
}

fn autocomplete_store_path() -> Option<PathBuf> {
    if let Some(dir) = config_dir() {
        return Some(dir.join("term").join("autocomplete.json"));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config").join("term").join("autocomplete.json"))
}
