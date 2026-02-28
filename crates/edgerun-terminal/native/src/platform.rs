// SPDX-License-Identifier: Apache-2.0
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::CommandBuilder;
use term_core::render::GlyphCache;
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoopProxy;
use winit::window::Window;

use crate::AppEvent;

pub(crate) fn resolve_window_size(window: &Window) -> PhysicalSize<u32> {
    window.inner_size()
}

pub(crate) fn default_shell() -> CommandBuilder {
    #[cfg(windows)]
    {
        CommandBuilder::new("cmd.exe")
    }
    #[cfg(not(windows))]
    {
        let shell = env::var("TERM_FORCE_SHELL")
            .or_else(|_| env::var("SHELL"))
            .unwrap_or_else(|_| "/bin/bash".to_string());
        let shell_name = Path::new(&shell)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(shell.as_str());
        let mut cmd = CommandBuilder::new(&shell);
        let clean_shell = env::var("TERM_CLEAN_SHELL").is_ok();
        if clean_shell {
            // Keep the shell clean for debugging: skip user rc/profile when requested.
            match shell_name {
                "bash" | "sh" => {
                    cmd.arg("--noprofile");
                    if let Some(rcfile) = ensure_term_rcfile() {
                        cmd.arg("--rcfile");
                        cmd.arg(rcfile);
                    } else {
                        cmd.arg("--norc");
                        cmd.env("BASH_ENV", "/dev/null");
                    }
                }
                "zsh" => {
                    cmd.arg("-f");
                }
                _ => {}
            }
        }
        if clean_shell && env::var("TERM_KEEP_PROMPT").is_err() {
            // Self-contained prompt (time, cwd, git, last status) scoped to this app only.
            let prompt_command = "TERM_LAST=$?; TERM_PROMPT_GIT=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || true)";
            let ps1 = r#"\[\e[38;5;117m\]\t\[\e[0m\] \
\[\e[38;5;148m\]\w\[\e[0m\]\
\[\e[38;5;215m\]${TERM_PROMPT_GIT:+ [${TERM_PROMPT_GIT}]}\[\e[0m\] \
\[\e[38;5;246m\]✦ $TERM_LAST\[\e[0m\]\n\[$(if [ $TERM_LAST -eq 0 ]; then printf '\e[38;5;41m'; else printf '\e[38;5;196m'; fi)\]\$\[\e[0m\] "#;
            cmd.env("PROMPT_COMMAND", prompt_command);
            cmd.env("PS1", ps1);
        }
        cmd
    }
}

fn ensure_term_rcfile() -> Option<PathBuf> {
    let base = dirs::cache_dir()?.join("term");
    let _ = fs::create_dir_all(&base);
    let rc = base.join("rc.sh");
    let contents = r#"
# term minimal rc (does not touch user shell configs)
if command -v eza >/dev/null 2>&1; then
  alias ls='eza --icons --group-directories-first -lh'
elif command -v exa >/dev/null 2>&1; then
  alias ls='exa --icons --group-directories-first -lh'
else
alias ls='ls --color=auto -lh --group-directories-first --classify'
fi
# keep grep colored for convenience
alias grep='grep --color=auto'
# readline tweaks to make completion feel closer to “just works”
bind 'set colored-completion-prefix on'
bind 'set show-all-if-ambiguous on'
bind 'set completion-ignore-case on'
bind 'set menu-complete-display-prefix on'
bind 'set completion-query-items 0'
# no other user env is modified
"#;
    if fs::write(&rc, contents).is_ok() {
        Some(rc)
    } else {
        None
    }
}

pub(crate) fn run_command_with_timeout(
    bin: &str,
    args: &[&str],
    stdin_data: Option<&[u8]>,
    timeout: Duration,
) -> Option<Vec<u8>> {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(if stdin_data.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    if let Some(input) = stdin_data
        && let Some(stdin) = child.stdin.as_mut()
    {
        let _ = stdin.write_all(input);
        // Close stdin so clipboard helpers don't block waiting for EOF.
        drop(child.stdin.take());
    }

    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().ok()? {
            if status.success() {
                let mut buf = Vec::new();
                if let Some(mut stdout) = child.stdout.take() {
                    let _ = stdout.read_to_end(&mut buf);
                }
                return Some(buf);
            }
            return None;
        }
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn kitty_font_family() -> String {
    if let Some(config) = run_command_with_timeout(
        "kitty",
        &["@", "get-config"],
        None,
        Duration::from_millis(500),
    ) && let Some(family) = parse_kitty_font_family(&String::from_utf8_lossy(&config))
    {
        return family;
    }

    if let Some(path) = kitty_config_path()
        && let Ok(contents) = fs::read_to_string(path)
        && let Some(family) = parse_kitty_font_family(&contents)
    {
        return family;
    }

    "monospace".to_string()
}

fn kitty_config_path() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|base| base.join("kitty").join("kitty.conf"))
}

fn parse_kitty_font_family(contents: &str) -> Option<String> {
    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if !line.starts_with("font_family") {
            continue;
        }
        let rest = line.trim_start_matches("font_family").trim();
        if rest.is_empty() {
            continue;
        }
        return Some(rest.trim_matches('"').trim_matches('\'').to_string());
    }
    None
}

pub(crate) fn load_kitty_primary_font() -> Option<Arc<Vec<u8>>> {
    let family = kitty_font_family();
    let output = run_command_with_timeout(
        "fc-match",
        &["-f", "%{file}", &family],
        None,
        Duration::from_millis(500),
    )?;
    let path = String::from_utf8_lossy(&output).trim().to_string();
    if path.is_empty() {
        return None;
    }
    fs::read(path).ok().map(Arc::new)
}

pub(crate) fn spawn_font_loader(proxy: EventLoopProxy<AppEvent>) {
    thread::spawn(move || {
        let fonts = GlyphCache::load_fallback_fonts();
        if !fonts.is_empty() {
            let _ = proxy.send_event(AppEvent::Fonts(fonts));
        }
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::run_command_with_timeout;

    #[test]
    fn run_command_with_timeout_captures_stdout() {
        // printf exists on POSIX; if absent the test will simply fail.
        let out = run_command_with_timeout("printf", &["abc"], None, Duration::from_millis(500))
            .expect("stdout captured");
        assert_eq!(out, b"abc");
    }

    #[test]
    fn run_command_with_timeout_allows_empty_stdout() {
        #[cfg(windows)]
        let cmd = ("cmd.exe", &["/C", "exit", "0"][..]);
        #[cfg(not(windows))]
        let cmd = ("true", &[][..]);

        let out = run_command_with_timeout(cmd.0, cmd.1, None, Duration::from_millis(500))
            .expect("command success still returns buffer");
        assert!(out.is_empty());
    }
}
