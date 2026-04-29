use std::path::PathBuf;
use std::process;

use edgerun_blog::{check_static_site, generate_static_site, start_blog, BlogConfig};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage(args.first().map(String::as_str).unwrap_or("edgerun-blog"));
        return;
    }

    let command = match parse_args(&args) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("{error}");
            print_usage(args.first().map(String::as_str).unwrap_or("edgerun-blog"));
            process::exit(2);
        }
    };

    match command {
        BlogCommand::Serve(config) => serve(config),
        BlogCommand::Generate {
            config,
            output,
            check,
        } => match if check {
            check_static_site(&config, &output)
        } else {
            generate_static_site(&config, &output)
        } {
            Ok(site) => {
                if check {
                    println!(
                        "checked {} generated files for {} posts in {}",
                        site.files.len(),
                        site.posts,
                        output.display()
                    );
                } else {
                    println!(
                        "generated {} files for {} posts in {}",
                        site.files.len(),
                        site.posts,
                        output.display()
                    );
                }
            }
            Err(error) => {
                eprintln!("edgerun-blog generate: {error}");
                process::exit(1);
            }
        },
    }
}

fn serve(config: BlogConfig) {
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

enum BlogCommand {
    Serve(BlogConfig),
    Generate {
        config: BlogConfig,
        output: PathBuf,
        check: bool,
    },
}

fn parse_args(args: &[String]) -> Result<BlogCommand, String> {
    let mut command = "serve";
    let mut first = 1;
    if let Some(arg) = args.get(1) {
        if arg == "serve" || arg == "generate" {
            command = arg;
            first = 2;
        }
    }

    let mut root = None;
    let mut static_root = None;
    let mut output = None;
    let mut check = false;
    let mut bind = "127.0.0.1:8088".to_string();
    let mut title = "Edgerun Blog".to_string();
    let mut description = "Notes from the Edgerun project.".to_string();
    let mut base_url = String::new();

    let mut i = first;
    while i < args.len() {
        match args[i].as_str() {
            "--check" => {
                check = true;
            }
            "--root" if i + 1 < args.len() => {
                root = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            "--out" if i + 1 < args.len() => {
                output = Some(PathBuf::from(&args[i + 1]));
                i += 1;
            }
            "--static-root" if i + 1 < args.len() => {
                static_root = Some(PathBuf::from(&args[i + 1]));
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

    let config = BlogConfig {
        root: root.ok_or_else(|| "missing --root /path/to/git/checkout".to_string())?,
        static_root,
        bind_addr: bind,
        title,
        description,
        base_url,
    };

    match command {
        "serve" => Ok(BlogCommand::Serve(config)),
        "generate" => Ok(BlogCommand::Generate {
            config,
            output: output.ok_or_else(|| "missing --out /path/to/static/output".to_string())?,
            check,
        }),
        _ => unreachable!(),
    }
}

fn print_usage(program: &str) {
    println!(
        "usage:\n  {program} serve --root /srv/blog --bind 127.0.0.1:8088 [--static-root /srv/blog/.generated] \\
         [--title 'Edgerun Blog'] [--description TEXT] [--base-url https://blog.edgerun.tech]\n  {program} generate --root /srv/blog --out /srv/blog/.generated [--check] \\
         [--title 'Edgerun Blog'] [--description TEXT] [--base-url https://blog.edgerun.tech]\n\n\
         The command name is optional; omitted commands default to serve."
    );
}
