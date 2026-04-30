use std::path::PathBuf;
use std::process;

use edgerun_git::{start_git, GitConfig};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage(args.first().map(String::as_str).unwrap_or("edgerun-git"));
        return;
    }
    let config = match parse_args(&args) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            print_usage(args.first().map(String::as_str).unwrap_or("edgerun-git"));
            process::exit(2);
        }
    };
    let bind_addr = config.bind_addr.clone();
    let rt = edgerun_rt::Runtime::new_multi_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|error| {
            eprintln!("failed to build runtime: {error}");
            process::exit(1);
        });
    rt.block_on(async move {
        if let Err(error) = start_git(config).await {
            eprintln!("edgerun-git: {error}");
            process::exit(1);
        }
    });
    eprintln!("edgerun-git: stopped {bind_addr}");
}

fn parse_args(args: &[String]) -> Result<GitConfig, String> {
    let mut root = None;
    let mut bind = "127.0.0.1:8089".to_string();
    let mut title = "Edgerun Git".to_string();
    let mut description = "Code released from the Edgerun project.".to_string();
    let mut base_url = String::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "serve" => {}
            "--root" if i + 1 < args.len() => {
                root = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            "--bind" if i + 1 < args.len() => {
                bind = args[i + 1].clone();
                i += 1;
            }
            "--title" if i + 1 < args.len() => {
                title = args[i + 1].clone();
                i += 1;
            }
            "--description" if i + 1 < args.len() => {
                description = args[i + 1].clone();
                i += 1;
            }
            "--base-url" if i + 1 < args.len() => {
                base_url = args[i + 1].trim_end_matches('/').to_string();
                i += 1;
            }
            other => return Err(format!("unknown or incomplete argument: {other}")),
        }
        i += 1;
    }
    let root = root.ok_or_else(|| "missing --root /path/to/repos".to_string())?;
    Ok(GitConfig {
        root,
        bind_addr: bind,
        title,
        description,
        base_url,
    })
}

fn print_usage(program: &str) {
    println!(
        "usage: {program} serve --root /srv/git --bind 127.0.0.1:8089 \
         [--title 'Edgerun Git'] [--description TEXT] [--base-url https://git.edgerun.tech]\n\n\
         Repositories are hidden unless they contain .edgerun/git.yaml with visible: true."
    );
}
