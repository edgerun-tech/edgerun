// SPDX-License-Identifier: Apache-2.0
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use arboard::Clipboard;
use log::warn;
use term_core::terminal::{Terminal, selection_text, write_bytes};

use crate::platform::run_command_with_timeout;

pub(crate) fn paste_clipboard(writer: Arc<Mutex<Box<dyn Write + Send>>>, bracketed: bool) {
    thread::spawn(move || paste_clipboard_sync(&writer, bracketed));
}

pub(crate) fn compose_bracketed_paste(data: &[u8], bracketed: bool) -> Vec<u8> {
    if bracketed {
        let mut payload = Vec::with_capacity(data.len() + 8);
        payload.extend_from_slice(b"\x1b[200~");
        payload.extend_from_slice(data);
        payload.extend_from_slice(b"\x1b[201~");
        payload
    } else {
        data.to_vec()
    }
}

fn paste_clipboard_sync(writer: &Arc<Mutex<Box<dyn Write + Send>>>, bracketed: bool) {
    let send_text = |data: &[u8]| {
        let payload = compose_bracketed_paste(data, bracketed);
        write_bytes(writer, &payload);
    };

    let wl_paths = ["/usr/bin/wl-paste", "/bin/wl-paste", "wl-paste"];
    let xclip_paths = ["/usr/bin/xclip", "/bin/xclip", "xclip"];
    let xsel_paths = ["/usr/bin/xsel", "/bin/xsel", "xsel"];
    let pbpaste_paths = ["/usr/bin/pbpaste", "/bin/pbpaste", "pbpaste"];
    let powershell_paths = [
        "/usr/bin/powershell",
        "/bin/powershell",
        "powershell",
        "powershell.exe",
    ];

    // Try Wayland/X11 tools first so cliphist/portals stay in sync.
    let mut candidates: Vec<(&str, &[&str])> = wl_paths
        .iter()
        .map(|p| {
            (
                *p,
                &[
                    "--no-newline",
                    "--type",
                    "text/plain",
                    "--selection",
                    "clipboard",
                ][..],
            )
        })
        .chain(wl_paths.iter().map(|p| {
            (
                *p,
                &[
                    "--no-newline",
                    "--type",
                    "text/plain",
                    "--selection",
                    "primary",
                ][..],
            )
        }))
        .collect();
    candidates.extend(
        xclip_paths
            .iter()
            .map(|p| (*p, &["-o", "-selection", "clipboard"][..])),
    );
    candidates.extend(
        xclip_paths
            .iter()
            .map(|p| (*p, &["-o", "-selection", "primary"][..])),
    );
    candidates.extend(xsel_paths.iter().map(|p| (*p, &["-o", "-b"][..])));
    candidates.extend(xsel_paths.iter().map(|p| (*p, &["-o", "-p"][..])));
    candidates.extend(pbpaste_paths.iter().map(|p| (*p, &[][..])));
    candidates.extend(
        powershell_paths
            .iter()
            .map(|p| (*p, &["-NoProfile", "-Command", "Get-Clipboard"][..])),
    );

    for (bin, args) in candidates {
        let exe = Path::new(bin);
        if !exe.exists() && bin.contains('/') {
            continue;
        }
        if let Some(out) = run_command_with_timeout(bin, args, None, Duration::from_millis(500)) {
            if out.is_empty() {
                continue;
            }
            send_text(&out);
            return;
        }
    }

    // Fallback to arboard if shell tools fail or are unavailable.
    if let Ok(mut clipboard) = Clipboard::new()
        && let Ok(text) = clipboard.get_text()
        && !text.is_empty()
    {
        send_text(text.as_bytes());
        return;
    }

    warn!("paste failed: no clipboard provider returned data");
}

pub(crate) fn copy_selection_to_clipboard(term: &Terminal, a: (usize, usize), b: (usize, usize)) {
    let text = selection_text(term, a, b);
    if text.is_empty() {
        return;
    }
    thread::spawn(move || copy_text_to_clipboard_sync(text));
}

fn copy_text_to_clipboard_sync(text: String) {
    let wl_paths = ["/usr/bin/wl-copy", "/bin/wl-copy", "wl-copy"];
    let xclip_paths = ["/usr/bin/xclip", "/bin/xclip", "xclip"];
    let xsel_paths = ["/usr/bin/xsel", "/bin/xsel", "xsel"];
    let pbcopy_paths = ["/usr/bin/pbcopy", "/bin/pbcopy", "pbcopy"];

    // Prefer wl-copy/xclip so cliphist sees updates even if arboard succeeds.
    let mut attempts: Vec<(&str, &[&str])> = wl_paths
        .iter()
        .map(|p| (*p, &["--trim-newline"][..]))
        .chain(
            wl_paths
                .iter()
                .map(|p| (*p, &["--primary", "--trim-newline"][..])),
        )
        .collect();

    attempts.extend(
        xclip_paths
            .iter()
            .map(|p| (*p, &["-selection", "clipboard"][..])),
    );
    attempts.extend(
        xclip_paths
            .iter()
            .map(|p| (*p, &["-selection", "primary"][..])),
    );
    attempts.extend(xsel_paths.iter().map(|p| (*p, &["-b"][..])));
    attempts.extend(xsel_paths.iter().map(|p| (*p, &["-p"][..])));
    attempts.extend(pbcopy_paths.iter().map(|p| (*p, &[][..])));

    let mut copied = false;
    for (bin, args) in &attempts {
        let exe = Path::new(bin);
        if !exe.exists() && bin.contains('/') {
            continue;
        }
        if run_command_with_timeout(bin, args, Some(text.as_bytes()), Duration::from_millis(500))
            .is_some()
        {
            copied = true;
        }
    }

    // Also push to arboard for completeness.
    if let Ok(mut clipboard) = Clipboard::new() {
        let _ = clipboard.set_text(text.clone());
    }

    if !copied {
        warn!("clipboard write failed: no provider accepted data");
    }
}
