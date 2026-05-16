use edgerun_error::Result;
use std::path::PathBuf;

#[derive(Debug)]
struct Args {
    out_dir: PathBuf,
    experimental: bool,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut out_dir = None;
        let mut experimental = false;
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-o" | "--out" => out_dir = args.next().map(PathBuf::from),
                "--experimental" => experimental = true,
                _ if arg.starts_with("--out=") => {
                    out_dir = Some(PathBuf::from(&arg["--out=".len()..]));
                }
                _ => {}
            }
        }
        let out_dir = out_dir.ok_or_else(|| edgerun_error::anyhow!("missing -o/--out"))?;
        Ok(Self {
            out_dir,
            experimental,
        })
    }
}

fn main() -> Result<()> {
    let args = Args::parse()?;
    codex_app_server_protocol::generate_json_with_experimental(&args.out_dir, args.experimental)
}
