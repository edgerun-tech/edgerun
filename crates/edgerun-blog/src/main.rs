use std::path::PathBuf;
use std::process;

use edgerun_blog::{start_blog, BlogConfig};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage(args.first().map(String::as_str).unwrap_or("edgerun-blog"));
        return;
    }

    let config = match parse_args(&args) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            print_usage(args.first().map(String::as_str).unwrap_or("edgerun-blog"));
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
        if let Err(error) = start_blog(config).await {
            eprintln!("edgerun-blog: {error}");
            process::exit(1);
        }
    });
    eprintln!("edgerun-blog: stopped {bind_addr}");
}

fn parse_args(args: &[String]) -> Result<BlogConfig, String> {
    let mut root = None;
    let mut bind = "127.0.0.1:8088".to_string();
    let mut title = "Edgerun Blog".to_string();
    let mut description = "Notes from the Edgerun project.".to_string();
    let mut base_url = String::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
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

    Ok(BlogConfig {
        root: root.ok_or_else(|| "missing --root /path/to/git/checkout".to_string())?,
        bind_addr: bind,
        title,
        description,
        base_url,
    })
}

fn print_usage(program: &str) {
    println!(
        "usage: {program} --root /srv/blog --bind 127.0.0.1:8088 \\
         [--title 'Edgerun Blog'] [--description TEXT] [--base-url https://blog.edgerun.tech]"
    );
}
