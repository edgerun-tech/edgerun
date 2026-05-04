#![allow(clippy::unwrap_used)]

use std::{env, fs, time::Instant};

use edgerun_codelyzer::parser::ParserPool;

fn main() {
    let dir = env::args().nth(1).expect("Usage: profile-files <path>");
    let top_n: usize = env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    let pool_start = Instant::now();
    let mut slow_files: Vec<(String, f64, usize)> = Vec::new();
    let mut failed_files: Vec<(String, String)> = Vec::new();
    let mut total_funcs = 0;
    let mut total_calls = 0;
    let mut total_files = 0;
    let mut mut_pool = ParserPool::new();

    // Walk the tree
    let mut stack = vec![dir.clone()];
    while let Some(dir_path) = stack.pop() {
        let entries = match fs::read_dir(&dir_path) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .map(|n| n.to_str().unwrap_or(""))
                    .unwrap_or("");
                if name.starts_with('.')
                    || name == "target"
                    || name == "node_modules"
                    || name == ".git"
                {
                    continue;
                }
                stack.push(path.display().to_string());
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ["c", "h"].contains(&ext) {
                    let file_path = path.display().to_string();
                    match fs::read_to_string(&file_path) {
                        Ok(source) => {
                            let loc = source.lines().count();
                            let t0 = Instant::now();
                            let result = mut_pool.parse_file(&file_path, &source);
                            let elapsed = t0.elapsed().as_millis() as f64 / 1000.0;

                            if let Some(res) = result {
                                total_funcs += res.functions.len();
                                total_calls += res.calls.len();
                            } else {
                                failed_files
                                    .push((file_path.clone(), "parse returned None".into()));
                            }

                            slow_files.push((file_path.clone(), elapsed, loc));
                            total_files += 1;
                        }
                        Err(e) => {
                            failed_files.push((file_path.clone(), format!("read error: {e}")));
                        }
                    }
                }
            }
        }
    }

    slow_files.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    println!("\n=== Summary ===");
    println!("Files processed:    {total_files}");
    println!("Functions found:    {total_funcs}");
    println!("Calls found:        {total_calls}");
    println!(
        "Parse failures:     {} ({:.1}%)",
        failed_files.len(),
        if total_files > 0 {
            failed_files.len() as f64 / total_files as f64 * 100.0
        } else {
            0.0
        }
    );
    println!(
        "Total pool time:    {:.3}s",
        pool_start.elapsed().as_secs_f64()
    );

    println!("\n=== Top {} slowest files ===", top_n);
    for (i, (path, time, loc)) in slow_files.iter().take(top_n).enumerate() {
        let short = path.rsplitn(3, '/').collect::<Vec<_>>().join("/");
        println!("  {:>2}. {:8.3}s  {:>7} LOC  {short}", i + 1, time, loc);
    }

    if !failed_files.is_empty() {
        println!("\n=== Failed files (first 20) ===");
        for (path, reason) in failed_files.iter().take(20) {
            let short = path.rsplitn(3, '/').collect::<Vec<_>>().join("/");
            println!("  FAIL {short}: {reason}");
        }
    }
}
