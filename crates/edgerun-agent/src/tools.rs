//! Shell tool with hints, smart formatting, and command caching.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::process;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_OUTPUT_CHARS: usize = 2000;
const CACHE_TTL_SECS: u64 = 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub timed_out: bool,
}

pub struct ToolExecutor {
    allowed_commands: HashSet<String>,
    project_root: String,
    timeout: Duration,
    cache: Mutex<HashMap<u64, CacheEntry>>,
    hints: CommandHints,
}

type HashSet<T> = std::collections::HashSet<T>;

#[derive(Clone)]
struct CommandHints;

impl CommandHints {
    fn hint(&self, cmd: &str, stderr: &str) -> Option<String> {
        let stderr_lower = stderr.to_lowercase();
        let cmd_lower = cmd.to_lowercase();

        if cmd_lower.contains("cargo") && stderr_lower.contains("could not find") {
            if let Some(pkg) = stderr_lower.split("could not find package `").nth(1) {
                let pkg = pkg.split('`').next().unwrap_or("?");
                return Some(format!("Try: cargo add {}", pkg));
            }
            if stderr_lower.contains("package") && stderr_lower.contains("not found") {
                return Some("Run: cargo update".to_string());
            }
        }

        if (cmd_lower.starts_with("grep") || cmd_lower.starts_with("rg"))
            && (stderr_lower.contains("no such file") || stderr_lower.contains("missing operand"))
        {
            return Some("Usage: grep 'pattern' path. Pattern must be quoted.".to_string());
        }

        if stderr_lower.contains("not a git repository") {
            return Some("Run: git init && git remote add origin <url>".to_string());
        }

        if cmd_lower.starts_with("cargo") && stderr_lower.contains("error:") {
            let lines: Vec<&str> = stderr.lines().filter(|l| l.trim().starts_with("-->")).collect();
            if !lines.is_empty() {
                return Some(format!("Error at: {}", lines[0]));
            }
        }

        if cmd_lower.contains("find") && stderr_lower.contains("unknown option") {
            return Some("Syntax: find . -name '*.rs' -type f".to_string());
        }

        None
    }
}

struct CacheEntry {
    result: ToolResult,
    expires: Instant,
}

impl Clone for ToolExecutor {
    fn clone(&self) -> Self {
        Self {
            allowed_commands: self.allowed_commands.clone(),
            project_root: self.project_root.clone(),
            timeout: self.timeout,
            cache: Mutex::new(HashMap::new()),
            hints: CommandHints,
        }
    }
}

impl ToolExecutor {
    pub fn new(project_root: &str) -> Self {
        Self {
            allowed_commands: Self::default_commands(),
            project_root: project_root.to_string(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            cache: Mutex::new(HashMap::new()),
            hints: CommandHints,
        }
    }

    pub fn with_allowed_commands(mut self, commands: Vec<String>) -> Self {
        self.allowed_commands = commands.into_iter().collect();
        self
    }

    pub fn timeout_secs(&self) -> u64 {
        self.timeout.as_secs()
    }

    fn default_commands() -> HashSet<String> {
        [
            "cat", "head", "tail", "ls", "find", "grep", "rg", "wc",
            "cargo", "rustc", "rustfmt", "clippy-driver",
            "git", "diff", "echo", "mkdir", "cp", "mv",
            "curl", "jq", "sort", "uniq", "awk", "sed",
            "tree", "which", "env", "pwd", "test",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn validate(&self, command: &str) -> Result<(), String> {
        if command.trim().is_empty() {
            return Err("Empty command".to_string());
        }

        let meta: &[char] = &[';', '&', '|', '$', '`', '(', ')', '{', '}', '<', '>', '!', '\n', '\r'];
        if command.contains(meta) {
            return Err("Shell metacharacters forbidden.".to_string());
        }

        let base = command.split_whitespace().next().unwrap_or("");
        let base = if base.contains('/') {
            base.rsplit('/').next().unwrap_or(base)
        } else {
            base
        };

        if !self.allowed_commands.contains(base) {
            return Err(format!("'{}' not allowed.", base));
        }

        Ok(())
    }

    pub async fn execute(&self, command: &str) -> ToolResult {
        if let Err(e) = self.validate(command) {
            return ToolResult { success: false, output: String::new(), error: Some(e), timed_out: false };
        }

        let (cachable, cache_key) = {
            let cmd_lower = command.to_lowercase();
            let is_cachable = cmd_lower.starts_with("grep")
                || cmd_lower.starts_with("rg")
                || cmd_lower.starts_with("find")
                || cmd_lower.starts_with("ls");

            if is_cachable {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                command.hash(&mut hasher);
                self.project_root.hash(&mut hasher);
                (true, Some(hasher.finish()))
            } else {
                (false, None)
            }
        };

        if cachable {
            if let Some(hash) = cache_key {
                if let Ok(cache) = self.cache.lock() {
                    if let Some(entry) = cache.get(&hash) {
                        if Instant::now() < entry.expires {
                            let mut cached = entry.result.clone();
                            cached.output = format!("[cached] {}", cached.output);
                            return cached;
                        }
                    }
                }
            }
        }

        let cmd = command.to_string();
        let project_root = self.project_root.clone();
        let timeout = self.timeout;

        let handle = edgerun_rt::spawn_blocking(move || run_command(&cmd, &project_root));
        let timeout_result = edgerun_rt::timeout(timeout, handle).await;

        let result = match timeout_result {
            Ok(Ok(r)) => r,
            Ok(Err(_)) => ToolResult { success: false, output: String::new(), error: Some("Panicked".to_string()), timed_out: false },
            Err(_) => ToolResult { success: false, output: String::new(), error: Some(format!("Timeout ({}s)", timeout.as_secs())), timed_out: true },
        };

        if cachable {
            if let Some(hash) = cache_key {
                if let Ok(mut cache) = self.cache.lock() {
                    cache.insert(hash, CacheEntry { result: result.clone(), expires: Instant::now() + Duration::from_secs(CACHE_TTL_SECS) });
                    cache.retain(|_, v| Instant::now() < v.expires);
                }
            }
        }

        result
    }

    pub fn format_output(&self, command: &str, result: &ToolResult) -> String {
        let cmd_lower = command.to_lowercase();

        if !result.success {
            let stderr = result.error.as_deref().unwrap_or("");
            if let Some(hint) = self.hints.hint(command, stderr) {
                return format!("{}\nHint: {}", result.error.as_ref().unwrap(), hint);
            }
        }

        let output = &result.output;

        if cmd_lower.starts_with("cargo") {
            format_cargo_output(output)
        } else if cmd_lower.starts_with("grep") || cmd_lower.starts_with("rg") {
            format_grep_output(output)
        } else if cmd_lower.starts_with("find") {
            format_find_output(output)
        } else if cmd_lower.starts_with("git") {
            format_git_output(output, command)
        } else {
            output.clone()
        }
    }

    pub fn get_allowed_commands(&self) -> Vec<String> {
        let mut cmds: Vec<String> = self.allowed_commands.iter().cloned().collect();
        cmds.sort();
        cmds
    }
}

fn run_command(command: &str, project_root: &str) -> ToolResult {
    let output = process::Command::new("sh")
        .arg("-c").arg(command)
        .current_dir(project_root)
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .stdin(process::Stdio::null())
        .output();

    match output {
        Ok(o) => {
            let stdout = strip_ansi(&String::from_utf8_lossy(&o.stdout));
            let stderr = strip_ansi(&String::from_utf8_lossy(&o.stderr));
            if o.status.success() {
                ToolResult { success: true, output: stdout, error: if stderr.is_empty() { None } else { Some(stderr) }, timed_out: false }
            } else {
                ToolResult { success: false, output: stdout, error: Some(stderr), timed_out: false }
            }
        }
        Err(e) => ToolResult { success: false, output: String::new(), error: Some(format!("Exec error: {}", e)), timed_out: false },
    }
}

fn strip_ansi(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            let mut j = i + 2;
            while j < bytes.len() && j < i + 20 {
                let c = bytes[j];
                if (c >= b'0' && c <= b'9') || c == b';' || c == b':' {
                    j += 1;
                    continue;
                }
                if c == b'm' || c == b'J' || c == b'H' || c == b'A' || c == b'B' || c == b'C' || c == b'D' || c == b'K' || c == b'f' {
                    i = j + 1;
                    break;
                }
                j += 1;
            }
            if j >= bytes.len() || j < i + 2 { i += 1; continue; }
            if i == j { i += 1; }
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&result).to_string()
}

fn format_cargo_output(output: &str) -> String {
    if output.is_empty() { return "(no output)".to_string(); }
    let lines: Vec<&str> = output.lines().collect();

    if lines.iter().any(|l| l.contains("error[E")) {
        let errors: Vec<&str> = lines.iter()
            .filter(|l| l.contains("error[E") || l.trim().starts_with("--> ") || (l.contains("not found") && l.len() < 200))
            .rev().take(50).cloned().collect();
        format!("{}\n{}", errors.into_iter().rev().collect::<Vec<_>>().join("\n"), if lines.len() > 50 { "\n[...more output...]" } else { "" })
    } else {
        let last: Vec<&str> = lines.iter().rev().take(20).cloned().collect();
        let s = last.into_iter().rev().collect::<Vec<_>>().join("\n");
        format!("{}{}", s, if lines.len() > 20 { " [...]" } else { "" })
    }
}

fn format_grep_output(output: &str) -> String {
    if output.is_empty() { return "(no matches)".to_string(); }
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() > 50 {
        let first: Vec<&str> = lines.iter().take(30).cloned().collect();
        format!("{}\n[...{} more...]", first.join("\n"), lines.len() - 30)
    } else { output.to_string() }
}

fn format_find_output(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() > 60 {
        let first: Vec<&str> = lines.iter().take(30).cloned().collect();
        format!("{}\n[...{} more...]", first.join("\n"), lines.len() - 30)
    } else { output.to_string() }
}

fn format_git_output(output: &str, command: &str) -> String {
    if output.len() > 2000 { format!("{}\n[...truncated...]", &output[..2000]) } else { output.to_string() }
}

impl Default for ToolExecutor {
    fn default() -> Self { Self::new(".") }
}
