// SPDX-License-Identifier: Apache-2.0
use std::time::Instant;

use log::error;
use term_core::render::layout::LayoutMetrics;
use term_core::terminal::{Terminal, write_bytes};
use term_ui::input;
use term_ui::widgets::{Cheatsheet, HistoryMenu, LogFocus, LogViewer, SettingsPanel};
use winit::event::{ElementState, KeyEvent};
use winit::event_loop::EventLoopProxy;
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::clipboard::{copy_selection_to_clipboard, paste_clipboard};
use crate::state::AutocompleteEngine;
use crate::suggest::fix_command_paths;
use crate::tab::{
    CwdHistoryEntry, Tab, TabKind, preferred_cwd, refresh_tab_titles, selection_bounds,
    spawn_shell_tab,
};
use crate::{
    AppEvent, CELL_BLINK_INTERVAL, CURSOR_BLINK_INTERVAL, CopyNotice, HelpToggle,
    add_ssh_host_to_config, maybe_set_copy_notice, smart_history_columns,
};

fn line_input_prefix(term: &Terminal) -> (String, usize) {
    let prefix = term.current_line_prefix();
    let total_cols = prefix.chars().count();

    const PROMPT_MARKERS: &[&str] = &["$ ", "# ", "% ", "> ", "❯ ", "➜ ", "» ", "› ", "λ "];

    let mut prompt_cols = 0;
    for marker in PROMPT_MARKERS {
        if let Some(idx) = prefix.rfind(marker) {
            let end = idx + marker.len();
            prompt_cols = prefix[..end].chars().count();
            break;
        }
    }

    if prompt_cols == 0
        && let Some((idx, ch)) = prefix
            .char_indices()
            .rev()
            .find(|&(_, c)| matches!(c, '$' | '#' | '%' | '>' | '❯' | '➜' | '»' | '›' | 'λ'))
    {
        let mut end_idx = idx + ch.len_utf8();
        while let Some(c) = prefix[end_idx..].chars().next() {
            if c != ' ' {
                break;
            }
            end_idx += c.len_utf8();
        }
        prompt_cols = prefix[..end_idx].chars().count();
    }

    if prompt_cols == 0 {
        let input_len = prefix.trim_start().chars().count();
        prompt_cols = total_cols.saturating_sub(input_len);
    }

    let input: String = prefix.chars().skip(prompt_cols).collect();
    (input.trim_start().to_string(), prompt_cols)
}

fn smart_suggestion_shortcut(event: &KeyEvent, modifiers: ModifiersState) -> bool {
    let ctrl_space = modifiers.control_key()
        && !modifiers.alt_key()
        && !modifiers.super_key()
        && (matches!(&event.logical_key, Key::Named(NamedKey::Space))
            || matches!(&event.logical_key, Key::Character(text) if text == " "));

    let bare_backquote = !modifiers.control_key()
        && !modifiers.alt_key()
        && !modifiers.super_key()
        && matches!(
            &event.logical_key,
            Key::Character(text) if text == "`" || text == "ˇ"
        );

    ctrl_space || bare_backquote
}

#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub(super) fn handle_key(
    event: KeyEvent,
    modifiers: ModifiersState,
    tabs: &mut Vec<Tab>,
    active_tab: &mut usize,
    layout: &LayoutMetrics,
    history_menu: &mut HistoryMenu,
    autocomplete: &mut AutocompleteEngine,
    proxy: &EventLoopProxy<AppEvent>,
    help: &mut HelpToggle,
    settings: &SettingsPanel,
    copy_notice: &mut Option<CopyNotice>,
    cwd_history: &[CwdHistoryEntry],
) {
    if event.state != ElementState::Pressed {
        return;
    }

    if matches!(event.logical_key, Key::Named(NamedKey::Escape))
        && let Some(tab) = tabs.get_mut(*active_tab)
        && (tab.selection_anchor.is_some() || tab.selection_edge.is_some())
    {
        tab.selection_anchor = None;
        tab.selection_edge = None;
        tab.selecting = false;
        tab.pending_selection = None;
        tab.mouse_down_pos = None;
        return;
    }

    if modifiers.control_key() && matches!(event.logical_key, Key::Named(NamedKey::Space)) {
        if let Some(tab) = tabs.get(*active_tab) {
            let (prefix, _) = line_input_prefix(&tab.terminal);
            let columns = smart_history_columns(
                history_menu,
                &prefix,
                *active_tab,
                tabs,
                cwd_history,
                autocomplete,
            );
            history_menu.open(columns);
        }
        return;
    }

    if matches!(event.logical_key, Key::Named(NamedKey::F1)) {
        help.toggle(Instant::now());
        return;
    }

    if modifiers.super_key()
        && let Key::Character(text) = &event.logical_key
    {
        let lower = text.to_lowercase();
        if lower == "t" {
            let cwd = preferred_cwd(tabs, cwd_history, *active_tab);
            match spawn_shell_tab(
                layout.cols as u16,
                layout.rows as u16,
                layout.usable_width,
                layout.usable_height,
                proxy.clone(),
                cwd,
                settings.scrollback_enabled,
                Instant::now(),
                CELL_BLINK_INTERVAL,
                CURSOR_BLINK_INTERVAL,
            ) {
                Ok(tab) => {
                    tabs.push(tab);
                    refresh_tab_titles(tabs);
                    *active_tab = tabs.len().saturating_sub(1);
                }
                Err(err) => error!("failed to create tab: {err}"),
            }
            return;
        }
    }

    if modifiers.alt_key()
        && let Key::Character(text) = &event.logical_key
    {
        let lower = text.to_lowercase();
        if lower == "t" {
            match spawn_shell_tab(
                layout.cols as u16,
                layout.rows as u16,
                layout.usable_width,
                layout.usable_height,
                proxy.clone(),
                None,
                settings.scrollback_enabled,
                Instant::now(),
                CELL_BLINK_INTERVAL,
                CURSOR_BLINK_INTERVAL,
            ) {
                Ok(tab) => {
                    tabs.push(tab);
                    refresh_tab_titles(tabs);
                    *active_tab = tabs.len().saturating_sub(1);
                }
                Err(err) => error!("failed to create tab: {err}"),
            }
            return;
        }
        if lower == "q" {
            if tabs.len() > 1 {
                tabs.remove(*active_tab);
                if *active_tab >= tabs.len() {
                    *active_tab = tabs.len().saturating_sub(1);
                }
                refresh_tab_titles(tabs);
            }
            return;
        }
        if let Some(digit) = lower.chars().next()
            && ('1'..='9').contains(&digit)
        {
            let idx = (digit as u8 - b'1') as usize;
            if idx < tabs.len() {
                *active_tab = idx;
            }
            return;
        }
    }

    match &event.logical_key {
        Key::Named(NamedKey::Tab) if modifiers.control_key() => {
            if tabs.is_empty() {
                return;
            }
            if modifiers.shift_key() {
                *active_tab = active_tab.saturating_sub(1);
                if *active_tab >= tabs.len() {
                    *active_tab = tabs.len().saturating_sub(1);
                }
            } else {
                *active_tab = (*active_tab + 1) % tabs.len();
            }
            return;
        }
        Key::Character(text) => {
            if modifiers.control_key() && modifiers.shift_key() {
                let lower = text.to_lowercase();
                if lower == "t" {
                    match spawn_shell_tab(
                        layout.cols as u16,
                        layout.rows as u16,
                        layout.usable_width,
                        layout.usable_height,
                        proxy.clone(),
                        None,
                        settings.scrollback_enabled,
                        Instant::now(),
                        CELL_BLINK_INTERVAL,
                        CURSOR_BLINK_INTERVAL,
                    ) {
                        Ok(tab) => {
                            tabs.push(tab);
                            refresh_tab_titles(tabs);
                            *active_tab = tabs.len().saturating_sub(1);
                        }
                        Err(err) => error!("failed to create tab: {err}"),
                    }
                    return;
                } else if lower == "w" {
                    if tabs.len() > 1 {
                        tabs.remove(*active_tab);
                        if *active_tab >= tabs.len() {
                            *active_tab = tabs.len().saturating_sub(1);
                        }
                        refresh_tab_titles(tabs);
                    }
                    return;
                }
            }
        }
        _ => {}
    }

    if tabs.is_empty() {
        return;
    }

    if tabs.get(*active_tab).is_none() {
        return;
    }

    if smart_suggestion_shortcut(&event, modifiers) {
        if let Some(tab) = tabs.get(*active_tab) {
            let (prefix, _) = line_input_prefix(&tab.terminal);
            let columns = smart_history_columns(
                history_menu,
                &prefix,
                *active_tab,
                tabs,
                cwd_history,
                autocomplete,
            );
            history_menu.open(columns);
        }
        return;
    }

    let is_macos = cfg!(target_os = "macos");
    let copy_request = match &event.logical_key {
        Key::Named(NamedKey::Copy) => true,
        Key::Named(NamedKey::Insert) if modifiers.control_key() => true,
        Key::Character(text)
            if text.eq_ignore_ascii_case("c")
                && !modifiers.alt_key()
                && ((modifiers.control_key() && modifiers.shift_key())
                    || (is_macos && modifiers.super_key())) =>
        {
            true
        }
        _ => false,
    };

    if copy_request {
        if let Some(tab) = tabs.get_mut(*active_tab)
            && let Some((a, b)) = selection_bounds(tab)
        {
            copy_selection_to_clipboard(&tab.terminal, a, b);
            maybe_set_copy_notice(settings, copy_notice, "Selection copied", Instant::now());
        }
        return;
    }

    let paste_request = match &event.logical_key {
        Key::Named(NamedKey::Paste) => true,
        Key::Named(NamedKey::Insert) if modifiers.shift_key() => true,
        Key::Character(text)
            if text.eq_ignore_ascii_case("v")
                && !modifiers.alt_key()
                && ((modifiers.control_key() && modifiers.shift_key())
                    || (is_macos && modifiers.super_key())) =>
        {
            true
        }
        _ => false,
    };

    if paste_request {
        if let Some(tab) = tabs.get(*active_tab) {
            let TabKind::Shell(shell) = &tab.kind;
            paste_clipboard(shell.writer.clone(), tab.terminal.bracketed_paste);
        }
        return;
    }

    if let Some(tab) = tabs.get_mut(*active_tab) {
        let TabKind::Shell(shell) = &mut tab.kind;
        input::send_key(
            event,
            modifiers,
            &shell.writer,
            shell.app_cursor_keys,
            tab.terminal.kitty_keyboard,
        );
    }
}

#[allow(clippy::ptr_arg)]
pub(super) fn handle_history_key(
    event: KeyEvent,
    history_menu: &mut HistoryMenu,
    tabs: &mut Vec<Tab>,
    active_tab: usize,
    cwd_history: &[CwdHistoryEntry],
    autocomplete: &mut AutocompleteEngine,
) -> Option<String> {
    if event.state != ElementState::Pressed {
        return None;
    }
    match &event.logical_key {
        Key::Named(NamedKey::Escape) => history_menu.close(),
        Key::Named(NamedKey::ArrowUp) => history_menu.move_dir(0, -1),
        Key::Named(NamedKey::ArrowDown) => history_menu.move_dir(0, 1),
        Key::Named(NamedKey::ArrowLeft) => history_menu.move_dir(-1, 0),
        Key::Named(NamedKey::ArrowRight) => history_menu.move_dir(1, 0),
        Key::Named(NamedKey::Tab) => history_menu.move_dir(1, 0),
        Key::Named(NamedKey::PageUp) => history_menu.move_selection(-5),
        Key::Named(NamedKey::PageDown) => history_menu.move_selection(5),
        Key::Named(NamedKey::Enter) => {
            return apply_history_selection(history_menu, tabs, active_tab, true, cwd_history);
        }
        Key::Named(NamedKey::F8) => {
            if let Some((col, row)) = history_menu.selected_cell()
                && let Some(col_data) = history_menu.columns.get(col)
                && let Some(entry) = col_data.entries.get(row).cloned()
            {
                let command_text = entry.command.clone();
                if col_data.title == "Bookmarks" {
                    history_menu
                        .bookmarks
                        .retain(|b| b.command != entry.command);
                } else if !history_menu
                    .bookmarks
                    .iter()
                    .any(|b| b.command == entry.command)
                {
                    history_menu.bookmarks.push(entry);
                }
                if let Some(tab) = tabs.get(active_tab) {
                    let (prefix, _) = line_input_prefix(&tab.terminal);
                    let cols = smart_history_columns(
                        history_menu,
                        &prefix,
                        active_tab,
                        tabs,
                        cwd_history,
                        autocomplete,
                    );
                    history_menu.open(cols);
                }
                if command_text.starts_with("ssh ") || command_text.starts_with("sudo ssh") {
                    let _ = add_ssh_host_to_config(&command_text);
                }
            }
        }
        _ => {}
    }

    None
}

pub(super) fn apply_history_selection(
    history_menu: &mut HistoryMenu,
    tabs: &mut [Tab],
    active_tab: usize,
    run: bool,
    cwd_history: &[CwdHistoryEntry],
) -> Option<String> {
    let entry = history_menu.selected_entry().cloned()?;
    let text = fix_command_paths(&entry.command, active_tab, tabs, cwd_history);
    if let Some(tab) = tabs.get_mut(active_tab) {
        let TabKind::Shell(shell) = &tab.kind;
        write_bytes(&shell.writer, text.as_bytes());
        if run {
            write_bytes(&shell.writer, b"\n");
        }
        tab.pending_cursor_to_line_end = true;
    }
    history_menu.close();
    Some(entry.command)
}

pub(super) fn handle_cheatsheet_key(event: KeyEvent, cheatsheet: &mut Cheatsheet) {
    if event.state != ElementState::Pressed {
        return;
    }
    match &event.logical_key {
        Key::Named(NamedKey::Escape) | Key::Named(NamedKey::F2) => cheatsheet.close(),
        _ => {}
    }
}

pub(super) fn handle_log_viewer_key(
    event: &KeyEvent,
    log_viewer: &mut LogViewer,
    log_refresh_needed: &mut bool,
) -> bool {
    if event.state != ElementState::Pressed {
        return false;
    }

    match &event.logical_key {
        Key::Named(NamedKey::Escape) | Key::Named(NamedKey::F5) => {
            log_viewer.close();
            return true;
        }
        _ => {}
    }

    if log_viewer.editing {
        match &event.logical_key {
            Key::Named(NamedKey::Enter) => {
                log_viewer.query = log_viewer.input.trim().to_string();
                log_viewer.editing = false;
                *log_refresh_needed = true;
            }
            Key::Named(NamedKey::Backspace) => {
                log_viewer.input.pop();
            }
            Key::Character(text) => {
                log_viewer.input.push_str(text);
            }
            _ => {}
        }
        return true;
    }

    match &event.logical_key {
        Key::Character(text) if text == "/" => {
            log_viewer.editing = true;
            log_viewer.input = log_viewer.query.clone();
        }
        Key::Character(text) if text.eq_ignore_ascii_case("f") => {
            log_viewer.follow = !log_viewer.follow;
            *log_refresh_needed = true;
        }
        Key::Character(text) if text.eq_ignore_ascii_case("s") => {
            log_viewer.sudo = !log_viewer.sudo;
            *log_refresh_needed = true;
        }
        Key::Character(text) if text.eq_ignore_ascii_case("r") => {
            *log_refresh_needed = true;
        }
        Key::Named(NamedKey::Tab) => {
            log_viewer.focus = match log_viewer.focus {
                LogFocus::Sources => LogFocus::Logs,
                LogFocus::Logs => LogFocus::Sources,
            };
        }
        Key::Named(NamedKey::ArrowUp) => {
            if log_viewer.focus == LogFocus::Sources {
                if log_viewer.selected > 0 {
                    log_viewer.selected -= 1;
                    *log_refresh_needed = true;
                }
            } else {
                log_viewer.follow = false;
                log_viewer.scroll = log_viewer.scroll.saturating_sub(1);
            }
        }
        Key::Named(NamedKey::ArrowDown) => {
            if log_viewer.focus == LogFocus::Sources {
                if log_viewer.selected + 1 < log_viewer.sources.len() {
                    log_viewer.selected += 1;
                    *log_refresh_needed = true;
                }
            } else {
                log_viewer.follow = false;
                log_viewer.scroll = log_viewer.scroll.saturating_add(1);
            }
        }
        Key::Named(NamedKey::PageUp) => {
            log_viewer.follow = false;
            log_viewer.scroll = log_viewer.scroll.saturating_sub(10);
        }
        Key::Named(NamedKey::PageDown) => {
            log_viewer.follow = false;
            log_viewer.scroll = log_viewer.scroll.saturating_add(10);
        }
        Key::Named(NamedKey::Home) => {
            log_viewer.follow = false;
            log_viewer.scroll = 0;
        }
        Key::Named(NamedKey::End) => {
            log_viewer.follow = true;
        }
        _ => {}
    }
    true
}

pub(super) fn pos_to_cell(
    x: f64,
    y: f64,
    layout: &LayoutMetrics,
    cell_w: u32,
    cell_h: u32,
    active_idx: usize,
    tabs: &[Tab],
) -> Option<(usize, usize)> {
    if active_idx >= tabs.len() {
        return None;
    }
    if x < layout.content_x as f64
        || y < layout.content_y as f64
        || x >= (layout.content_x + layout.usable_width) as f64
        || y >= (layout.content_y + layout.usable_height) as f64
    {
        return None;
    }
    let rel_x = x - layout.content_x as f64;
    let rel_y = y - layout.content_y as f64;
    let col = (rel_x / cell_w.max(1) as f64).floor() as usize;
    let row = (rel_y / cell_h.max(1) as f64).floor() as usize;
    if row < tabs[active_idx].terminal.rows && col < tabs[active_idx].terminal.cols {
        Some((col, row))
    } else {
        None
    }
}
