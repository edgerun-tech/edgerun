//! Benchmark runner for edgerun-agent.
//!
//! Runs the agent against a suite of test cases and scores results.
//!   cargo run --bin benchmark -- --tabby-url http://localhost:5001 --model devstral-small-2:24b
//!   cargo run --bin benchmark -- --list

use std::path::PathBuf;
use std::time::Instant;
use std::io::Write;

use clap::Parser;

use edgerun_agent::client::{ChatRequest, TabbyClient};

const MAX_TOOL_ITERATIONS: usize = 6;

#[derive(serde::Deserialize, Debug, Clone)]
struct TestCase {
    id: String,
    task: String,
    fixture: String,
    #[serde(default)]
    r#type: String,
    expected_patterns: Vec<String>,
    #[serde(default)]
    inverted_patterns: Vec<String>,
    #[serde(default)]
    required: bool,
    #[serde(default = "default_timeout")]
    timeout_secs: u64,
}

fn default_timeout() -> u64 { 60 }

#[derive(Debug, serde::Serialize)]
struct BenchmarkResult {
    id: String,
    task: String,
    task_type: String,
    passed: bool,
    score: f32,
    patterns_matched: usize,
    patterns_total: usize,
    inverted_violations: usize,
    tool_calls: usize,
    elapsed_ms: u64,
    reply_preview: String,
    error: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct BenchmarkReport {
    total: usize,
    passed: usize,
    failed: usize,
    skipped: usize,
    overall_score: f32,
    avg_tool_calls: f32,
    total_elapsed_ms: u64,
    results: Vec<BenchmarkResult>,
}

fn system_prompt() -> String {
    "You are a coding assistant with shell access. Run commands with $ prefix or in ```bash blocks. One command per line, no chaining. Be brief. Cite specific file names and line numbers.".to_string()
}

fn extract_commands(response: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut in_block = false;
    let mut lang = String::new();

    for line in response.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            if in_block {
                in_block = false;
                lang.clear();
            } else {
                in_block = true;
                lang = trimmed.trim_start_matches('`').trim().to_lowercase();
            }
            continue;
        }

        if in_block && (lang.is_empty() || lang == "sh" || lang == "bash" || lang == "text") {
            let cmd = trimmed.trim();
            if !cmd.is_empty() && !cmd.starts_with('#') {
                commands.push(cmd.to_string());
            }
            continue;
        }

        if !in_block {
            if let Some(pos) = trimmed.find("$ ") {
                let after = &trimmed[pos + 2..].trim();
                if !after.is_empty() && !after.starts_with('#') {
                    commands.push(after.to_string());
                }
            }
        }
    }

    commands
}

fn run_command_sync(command: &str, project_root: &str) -> (String, String) {
    use std::process::{Command, Stdio};

    let meta: &[char] = &[';', '&', '|', '$', '`', '(', ')', '{', '}', '<', '>', '!', '\n'];
    if command.contains(meta) {
        return (String::new(), "Shell metacharacters not allowed".to_string());
    }

    let base = command.split_whitespace().next().unwrap_or("");
    let base = if base.contains('/') {
        base.rsplit('/').next().unwrap_or(base)
    } else {
        base
    };

    let allowed = [
        "cat", "head", "tail", "ls", "find", "grep", "rg", "wc",
        "cargo", "rustc", "rustfmt", "clippy-driver",
        "git", "diff", "echo", "mkdir", "cp", "mv",
        "curl", "jq", "sort", "uniq", "awk", "sed",
        "tree", "which", "env", "pwd", "test",
    ];

    if !allowed.contains(&base) {
        return (String::new(), format!("Command '{}' not allowed.", base));
    }

    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(project_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            (stdout, stderr)
        }
        Err(e) => (String::new(), format!("Exec error: {}", e)),
    }
}

fn run_task(client: TabbyClient, project_root: String, task: String) -> (String, usize, u64) {
    let rt = edgerun_rt::Builder::new_multi_thread()
        .worker_threads(2)
        .build()
        .expect("Failed to create runtime");

    rt.block_on(async move {
        let mut tool_call_count = 0;
        let mut current = task.to_string();
        let start = Instant::now();

        for _ in 0..MAX_TOOL_ITERATIONS {
            let current_clone = current.clone();
            let response = client.chat(ChatRequest {
                message: current_clone,
                context: None,
                system_prompt: Some(system_prompt()),
                max_tokens: Some(2048),
                temperature: Some(0.3),
            }).await;

            let reply = match response {
                Ok(r) => r.reply,
                Err(e) => return (format!("API error: {}", e), tool_call_count, start.elapsed().as_millis() as u64),
            };

            let cmds = extract_commands(&reply);
            if cmds.is_empty() {
                return (reply, tool_call_count, start.elapsed().as_millis() as u64);
            }

            let mut results = Vec::new();
            for cmd in &cmds {
                tool_call_count += 1;
                let (out, err) = run_command_sync(cmd, &project_root);
                let output = if err.is_empty() {
                    format!("$ {}\n{}", cmd, out)
                } else {
                    format!("$ {}\n{}\n[stderr]: {}", cmd, out, err)
                };
                results.push(output);
            }

            let tool_output = results.join("\n\n");
            let truncated = if tool_output.len() > 8000 {
                format!("{}\n[...truncated...]", &tool_output[..8000])
            } else {
                tool_output
            };

            if tool_call_count >= MAX_TOOL_ITERATIONS - 1 {
                current = format!("Results:\n{}\n\nFinal answer only — no more commands.", truncated);
            } else {
                current = format!("Results:\n{}\n\nContinue ($ cmd) or give final answer.", truncated);
            }
        }

        (format!("Tool limit reached ({} commands)", tool_call_count), tool_call_count, start.elapsed().as_millis() as u64)
    })
}

fn score_response(reply: &str, patterns: &[String], inverted: &[String]) -> (usize, usize, usize) {
    let reply_lower = reply.to_lowercase();
    let mut matched = 0;
    let total = patterns.len();

    for pat in patterns {
        let pat_lower = pat.to_lowercase();

        if pat.starts_with('[') && pat.contains('-') && pat.ends_with(']') {
            let inner = &pat[1..pat.len()-1];
            if let Some((lo_str, hi_str)) = inner.split_once('-') {
                let lo: i32 = lo_str.parse().unwrap_or(0);
                let hi: i32 = hi_str.parse().unwrap_or(i32::MAX);
                for word in reply.split_whitespace() {
                    if let Ok(n) = word.parse::<i32>() {
                        if n >= lo && n <= hi {
                            matched += 1;
                            break;
                        }
                    }
                }
                continue;
            }
        }

        if reply_lower.contains(&pat_lower) {
            matched += 1;
        }
    }

    let violations = inverted.iter()
        .filter(|pat| reply.to_lowercase().contains(&pat.to_lowercase()))
        .count();

    (matched, total, violations)
}

fn score_to_pct(matched: usize, total: usize) -> f32 {
    if total == 0 { return 1.0; }
    matched as f32 / total as f32
}

#[derive(Parser)]
#[command(name = "benchmark", about = "Benchmark edgerun-agent")]
struct Cli {
    #[arg(long, default_value = "http://10.10.10.1:5001")]
    tabby_url: String,
    #[arg(long, default_value = "devstral-small-2:24b")]
    model: String,
    #[arg(long)]
    cases_file: Option<String>,
    #[arg(long)]
    fixtures_dir: Option<String>,
    #[arg(long)]
    id: Option<Vec<String>>,
    #[arg(long)]
    list: bool,
    #[arg(long)]
    json: bool,
}

fn main() {
    let cli = Cli::parse();

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cases_file = cli.cases_file
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.join("benchmark/cases.json"));

    let fixtures_dir = cli.fixtures_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.join("benchmark/fixtures"));

    let cases_json = std::fs::read_to_string(&cases_file)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", cases_file.display(), e));
    let cases: Vec<TestCase> = edgerun_json::from_str(&cases_json)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", cases_file.display(), e));

    if cli.list {
        println!("\n  edgerun-agent benchmark — {} test cases\n", cases.len());
        for c in &cases {
            let req = if c.required { "*" } else { " " };
            println!(
                "  [{}{}] {:20} {} ({})",
                req, c.id,
                format!("[{}]", c.r#type),
                c.task.chars().take(55).collect::<String>(),
                if c.expected_patterns.is_empty() { "no patterns" } else { "" }
            );
        }
        println!("\n  * = required to pass\n");
        return;
    }

    let cases: Vec<TestCase> = if let Some(ids) = &cli.id {
        cases.into_iter().filter(|c| ids.contains(&c.id)).collect()
    } else {
        cases
    };

    if cases.is_empty() {
        eprintln!("No matching test cases.");
        return;
    }

    println!("\n  edgerun-agent benchmark");
    println!("  TabbyAPI:  {}", cli.tabby_url);
    println!("  Model:     {}", cli.model);
    println!("  Fixtures:  {}", fixtures_dir.display());
    println!("  Cases:     {} ({})\n", cases.len(), cases_file.display());
    println!("  {}", "─".repeat(55));

    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for case in &cases {
        let fixture_path = fixtures_dir.join(&case.fixture);
        if !fixture_path.exists() {
            eprintln!("  [SKIP] {} — fixture '{}' not found", case.id, case.fixture);
            skipped += 1;
            continue;
        }

        let project_root = fixture_path.to_string_lossy().as_ref().to_string();
        let client = TabbyClient::new(&cli.tabby_url, &cli.model);

        print!("  [ RUN ] {:20} ", format!("[{}]", case.id));
        let _ = std::io::stdout().flush();

        let (reply, tool_calls, elapsed) = run_task(client.clone(), project_root.clone(), case.task.clone());

        let (patterns_matched, patterns_total, inverted_violations) =
            score_response(&reply, &case.expected_patterns, &case.inverted_patterns);

        let mut score = score_to_pct(patterns_matched, patterns_total);

        if !case.inverted_patterns.is_empty() && inverted_violations > 0 {
            score *= 0.5_f32.powi(inverted_violations as i32);
        }

        let passed_check = if case.required {
            score >= 0.5 && patterns_matched >= patterns_total.saturating_sub(1)
        } else {
            patterns_matched > 0
        };

        if reply.starts_with("API error") {
            failed += 1;
            print!("FAIL");
        } else if passed_check {
            passed += 1;
            print!("PASS");
        } else {
            failed += 1;
            print!("FAIL");
        }

        println!(
            "  {:.0}% ({}/{} patterns, {} calls, {}ms)",
            score * 100.0,
            patterns_matched,
            patterns_total,
            tool_calls,
            elapsed
        );

        results.push(BenchmarkResult {
            id: case.id.clone(),
            task: case.task.clone(),
            task_type: case.r#type.clone(),
            passed: passed_check,
            score,
            patterns_matched,
            patterns_total,
            inverted_violations,
            tool_calls,
            elapsed_ms: elapsed,
            reply_preview: reply.chars().take(200).collect(),
            error: if reply.starts_with("API error") { Some(reply.clone()) } else { None },
        });
    }

    let total_tested = passed + failed;
    let overall_score = if total_tested > 0 {
        results.iter().map(|r| r.score).sum::<f32>() / total_tested as f32
    } else { 0.0 };
    let avg_tool_calls = if total_tested > 0 {
        results.iter().map(|r| r.tool_calls as f32).sum::<f32>() / total_tested as f32
    } else { 0.0 };
    let total_elapsed: u64 = results.iter().map(|r| r.elapsed_ms).sum();

    println!("\n  {}", "─".repeat(55));
    println!("  {} total | {} passed | {} failed | {} skipped", cases.len(), passed, failed, skipped);
    println!("  Overall:  {:.0}%  |  Avg calls: {:.1}  |  Total: {}ms",
             overall_score * 100.0, avg_tool_calls, total_elapsed);
    println!();

    if cli.json {
        let report = BenchmarkReport {
            total: cases.len(),
            passed, failed, skipped,
            overall_score,
            avg_tool_calls,
            total_elapsed_ms: total_elapsed,
            results,
        };
        println!("{}", edgerun_json::to_string_pretty(&report).unwrap());
    }

    if failed > 0 || (skipped == cases.len() && cases.len() > 0) {
        std::process::exit(1);
    }
}
