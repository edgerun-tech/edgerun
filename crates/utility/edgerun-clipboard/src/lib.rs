//! Edgerun clipboard adapter.

use std::fmt;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub struct Clipboard;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl Clipboard {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn get_text(&mut self) -> Result<String> {
        for (bin, args) in paste_commands() {
            if let Some(output) = run_command(bin, args, None, Duration::from_millis(500)) {
                if output.is_empty() {
                    continue;
                }
                return String::from_utf8(output).map_err(|err| Error(err.to_string()));
            }
        }
        Err(Error("no clipboard provider returned data".into()))
    }

    pub fn set_text<S: Into<String>>(&mut self, text: S) -> Result<()> {
        let text = text.into();
        let mut accepted = false;
        for (bin, args) in copy_commands() {
            if run_command(bin, args, Some(text.as_bytes()), Duration::from_millis(500)).is_some() {
                accepted = true;
            }
        }
        accepted
            .then_some(())
            .ok_or_else(|| Error("no clipboard provider accepted data".into()))
    }
}

fn paste_commands() -> Vec<(&'static str, &'static [&'static str])> {
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

    let mut commands: Vec<(&str, &[&str])> = wl_paths
        .iter()
        .map(|path| {
            (
                *path,
                &[
                    "--no-newline",
                    "--type",
                    "text/plain",
                    "--selection",
                    "clipboard",
                ][..],
            )
        })
        .chain(wl_paths.iter().map(|path| {
            (
                *path,
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
    commands.extend(
        xclip_paths
            .iter()
            .map(|path| (*path, &["-o", "-selection", "clipboard"][..])),
    );
    commands.extend(
        xclip_paths
            .iter()
            .map(|path| (*path, &["-o", "-selection", "primary"][..])),
    );
    commands.extend(xsel_paths.iter().map(|path| (*path, &["-o", "-b"][..])));
    commands.extend(xsel_paths.iter().map(|path| (*path, &["-o", "-p"][..])));
    commands.extend(pbpaste_paths.iter().map(|path| (*path, &[][..])));
    commands.extend(
        powershell_paths
            .iter()
            .map(|path| (*path, &["-NoProfile", "-Command", "Get-Clipboard"][..])),
    );
    commands
}

fn copy_commands() -> Vec<(&'static str, &'static [&'static str])> {
    let wl_paths = ["/usr/bin/wl-copy", "/bin/wl-copy", "wl-copy"];
    let xclip_paths = ["/usr/bin/xclip", "/bin/xclip", "xclip"];
    let xsel_paths = ["/usr/bin/xsel", "/bin/xsel", "xsel"];
    let pbcopy_paths = ["/usr/bin/pbcopy", "/bin/pbcopy", "pbcopy"];

    let mut commands: Vec<(&str, &[&str])> = wl_paths
        .iter()
        .map(|path| (*path, &["--trim-newline"][..]))
        .chain(
            wl_paths
                .iter()
                .map(|path| (*path, &["--primary", "--trim-newline"][..])),
        )
        .collect();
    commands.extend(
        xclip_paths
            .iter()
            .map(|path| (*path, &["-selection", "clipboard"][..])),
    );
    commands.extend(
        xclip_paths
            .iter()
            .map(|path| (*path, &["-selection", "primary"][..])),
    );
    commands.extend(xsel_paths.iter().map(|path| (*path, &["-b"][..])));
    commands.extend(xsel_paths.iter().map(|path| (*path, &["-p"][..])));
    commands.extend(pbcopy_paths.iter().map(|path| (*path, &[][..])));
    commands
}

fn run_command(
    bin: &str,
    args: &[&str],
    stdin_data: Option<&[u8]>,
    timeout: Duration,
) -> Option<Vec<u8>> {
    if bin.contains('/') && !Path::new(bin).exists() {
        return None;
    }

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

    if let Some(data) = stdin_data {
        if let Some(mut stdin) = child.stdin.take() {
            let bytes = data.to_vec();
            thread::spawn(move || {
                let _ = stdin.write_all(&bytes);
            });
        }
    }

    let start = Instant::now();
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            if !status.success() {
                return None;
            }
            let mut out = Vec::new();
            if let Some(mut stdout) = child.stdout.take() {
                let _ = stdout.read_to_end(&mut out);
            }
            return Some(out);
        }
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        thread::sleep(Duration::from_millis(10));
    }
}
