use edgerun_error::Context;
use edgerun_error::Result;
use std::path::PathBuf;

#[derive(Debug)]
struct Args {
    schema_root: Option<PathBuf>,
    prettier: Option<PathBuf>,
    experimental: bool,
}

impl Args {
    fn parse() -> Self {
        let mut schema_root = None;
        let mut prettier = None;
        let mut experimental = false;
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--schema-root" => schema_root = args.next().map(PathBuf::from),
                "-p" | "--prettier" => prettier = args.next().map(PathBuf::from),
                "--experimental" => experimental = true,
                _ if arg.starts_with("--schema-root=") => {
                    schema_root = Some(PathBuf::from(&arg["--schema-root=".len()..]));
                }
                _ if arg.starts_with("--prettier=") => {
                    prettier = Some(PathBuf::from(&arg["--prettier=".len()..]));
                }
                _ => {}
            }
        }
        Self {
            schema_root,
            prettier,
            experimental,
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let schema_root = args
        .schema_root
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schema"));

    codex_app_server_protocol::write_schema_fixtures_with_options(
        &schema_root,
        args.prettier.as_deref(),
        codex_app_server_protocol::SchemaFixtureOptions {
            experimental_api: args.experimental,
        },
    )
    .with_context(|| {
        format!(
            "failed to regenerate schema fixtures under {}",
            schema_root.display()
        )
    })
}
