use anyhow::Result;
use std::env;
use std::path::PathBuf;

/// Generate the JSON Schema for `config.toml` and write it to `config.schema.json`.
struct Args {
    out: Option<PathBuf>,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut out = None;
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-o" | "--out" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("missing value for {arg}"))?;
                    out = Some(PathBuf::from(value));
                }
                "-h" | "--help" => {
                    println!("Usage: codex-write-config-schema [-o|--out PATH]");
                    std::process::exit(0);
                }
                _ if arg.starts_with("--out=") => {
                    out = Some(PathBuf::from(&arg["--out=".len()..]));
                }
                _ => return Err(anyhow::anyhow!("unknown argument: {arg}")),
            }
        }
        Ok(Self { out })
    }
}

fn main() -> Result<()> {
    let args = Args::parse()?;
    let out_path = args
        .out
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.schema.json"));
    codex_config::schema::write_config_schema(&out_path)?;
    Ok(())
}
