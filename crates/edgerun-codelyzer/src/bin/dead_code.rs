use std::env;
use std::path::PathBuf;

use edgerun_codelyzer::dead_code::{
    AnalyzeOptions, DeleteKind, DeleteOptions, analyze_dead_code, apply_deletions,
};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut opts = AnalyzeOptions {
        workspace_root: PathBuf::from("."),
        package: None,
        path: None,
        configs: Vec::new(),
        include_public: false,
        include_tests_as_roots: true,
        top: 80,
    };
    let mut delete = DeleteOptions::default();

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--workspace" => {
                opts.workspace_root = PathBuf::from(next_arg(&mut args, "--workspace")?)
            }
            "--package" => opts.package = Some(next_arg(&mut args, "--package")?),
            "--path" => opts.path = Some(PathBuf::from(next_arg(&mut args, "--path")?)),
            "--config" => opts.configs.push(next_arg(&mut args, "--config")?),
            "--include-public" => opts.include_public = true,
            "--no-test-roots" => opts.include_tests_as_roots = false,
            "--top" => {
                opts.top = next_arg(&mut args, "--top")?
                    .parse()
                    .map_err(|_| "--top must be a number".to_string())?;
            }
            "--apply" => delete.apply = true,
            "--delete-inactive-cfg" => delete.kinds.push(DeleteKind::InactiveCfgItem),
            "--delete-dead-fn" => delete.kinds.push(DeleteKind::DeadFunction),
            "--delete-crates" => delete.kinds.push(DeleteKind::UnusedCrate),
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    let report = analyze_dead_code(&opts)?;
    print_report(&report);

    if delete.apply {
        let summary = apply_deletions(&report, &delete)?;
        println!(
            "\napplied: removed_items={} removed_crates={} bytes_removed={}",
            summary.removed_items, summary.removed_crates, summary.bytes_removed
        );
    } else if !delete.kinds.is_empty() {
        println!("\npass --apply to perform the requested deletion selectors");
    }

    Ok(())
}

fn next_arg(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn print_report(report: &edgerun_codelyzer::dead_code::DeadCodeReport) {
    println!("configs:");
    for config in &report.configs {
        println!("  {}", config.name);
    }

    println!("\nsummary:");
    println!("  files={}", report.file_count);
    println!("  functions={}", report.function_count);
    println!("  edges={}", report.edge_count);
    println!("  inactive_cfg_items={}", report.inactive_cfg_items.len());
    println!("  dead_functions={}", report.dead_functions.len());
    println!("  unused_crates={}", report.unused_crates.len());

    if !report.inactive_cfg_items.is_empty() {
        println!("\ninactive cfg items:");
        for item in &report.inactive_cfg_items {
            println!(
                "  {:>5} lines  {}:{}  {}  cfg={}",
                item.lines, item.file, item.line, item.name, item.cfg
            );
        }
    }

    if !report.dead_functions.is_empty() {
        println!("\ndead functions:");
        for item in &report.dead_functions {
            println!(
                "  {:>5} lines  out={:<3} active={:<2} {}:{}  {}",
                item.lines,
                item.outgoing_edges,
                item.active_config_count,
                item.file,
                item.line,
                item.name
            );
        }
    }

    if !report.unused_crates.is_empty() {
        println!("\nunused workspace crates:");
        for item in &report.unused_crates {
            println!("  {}  {}", item.name, item.manifest_path.display());
        }
    }
}

fn print_help() {
    println!(
        "edgerun-dead-code [--workspace PATH] [--package NAME | --path PATH]\n\
         \n\
         Reports config-aware dead code. Default configs are default-features,\n\
         all-features, no-default-features, test, and each declared feature.\n\
         \n\
         Options:\n\
           --config SPEC            Add a config: default, all, none, test, feature:NAME,\n\
                                    target:x86_64-unknown-linux-gnu, wasm\n\
           --include-public         Allow public zero-caller functions as dead candidates\n\
           --no-test-roots          Do not treat #[test] functions as roots\n\
           --top N                  Limit report rows per section\n\
           --delete-inactive-cfg    Select items inactive in every config for deletion\n\
           --delete-dead-fn         Select zero-reachable functions for deletion\n\
           --delete-crates          Select unused workspace crates for directory deletion\n\
           --apply                  Apply selected deletions"
    );
}
