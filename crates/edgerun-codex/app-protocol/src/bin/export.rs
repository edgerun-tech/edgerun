use edgerun_error::Result;
use std::path::PathBuf;

#[derive(Debug)]
struct Args {
    out_dir: PathBuf,
    prettier: Option<PathBuf>,
    experimental: bool,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut out_dir = None;
        let mut prettier = None;
        let mut experimental = false;
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-o" | "--out" => out_dir = args.next().map(PathBuf::from),
                "-p" | "--prettier" => prettier = args.next().map(PathBuf::from),
                "--experimental" => experimental = true,
                _ if arg.starts_with("--out=") => {
                    out_dir = Some(PathBuf::from(&arg["--out=".len()..]));
                }
                _ if arg.starts_with("--prettier=") => {
                    prettier = Some(PathBuf::from(&arg["--prettier=".len()..]));
                }
                _ => {}
            }
        }
        let out_dir = out_dir.ok_or_else(|| edgerun_error::anyhow!("missing -o/--out"))?;
        Ok(Self {
            out_dir,
            prettier,
            experimental,
        })
    }
}

fn main() -> Result<()> {
    let args = Args::parse()?;
    codex_app_server_protocol::generate_ts_with_options(
        &args.out_dir,
        args.prettier.as_deref(),
        codex_app_server_protocol::GenerateTsOptions {
            experimental_api: args.experimental,
            ..codex_app_server_protocol::GenerateTsOptions::default()
        },
    )?;
    codex_app_server_protocol::generate_json_with_experimental(&args.out_dir, args.experimental)
}
