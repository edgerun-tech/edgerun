use std::collections::HashSet;
use std::path::PathBuf;

use crate::tab::{CwdHistoryEntry, Tab, tab_current_dir};
use term_ui::suggest::guess_command_paths;

pub fn recent_dirs_for_tab(
    active: usize,
    tabs: &[Tab],
    history: &[CwdHistoryEntry],
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    if let Some(tab) = tabs.get(active)
        && let Some(cwd) = tab_current_dir(tab)
    {
        seen.insert(cwd.clone());
        dirs.push(cwd);
    }

    for entry in history.iter().rev().filter(|h| h.tab == active) {
        if seen.insert(entry.path.clone()) {
            dirs.push(entry.path.clone());
        }
    }

    dirs
}

pub fn fix_command_paths(
    entry: &str,
    active: usize,
    tabs: &[Tab],
    history: &[CwdHistoryEntry],
) -> String {
    let current_dir_buf = tabs.get(active).and_then(tab_current_dir);
    let current_dir = current_dir_buf.as_deref();
    let recent_dirs = recent_dirs_for_tab(active, tabs, history);
    guess_command_paths(entry, current_dir, &recent_dirs)
}
