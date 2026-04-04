use lifegraph_virtual_disk::{attach_nbd, detach_nbd, BlockError, LinuxNbdAttachSpec};
use std::env;

fn main() {
    if let Err(error) = run() {
        eprintln!("nbd-attach error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), BlockError> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        print_usage(&args[0]);
        return Ok(());
    }
    match args[1].as_str() {
        "attach" => {
            if args.len() < 6 || args.len() > 8 {
                print_usage(&args[0]);
                return Ok(());
            }
            let host = args[2].clone();
            let port = parse_u16(&args[3], "port")?;
            let export_name = args[4].clone();
            let device = args[5].clone();
            let mut read_only = false;
            let mut block_size = None;
            for extra in &args[6..] {
                if extra == "--read-only" {
                    read_only = true;
                } else if let Some(value) = extra.strip_prefix("--block-size=") {
                    block_size = Some(parse_u32(value, "block_size")?);
                } else {
                    return Err(BlockError::ProtocolError(format!(
                        "unknown option `{extra}`"
                    )));
                }
            }
            attach_nbd(&LinuxNbdAttachSpec {
                device,
                host,
                port,
                export_name,
                block_size,
                read_only,
            })
        }
        "detach" => {
            if args.len() != 3 {
                print_usage(&args[0]);
                return Ok(());
            }
            detach_nbd(&args[2])
        }
        _ => {
            print_usage(&args[0]);
            Ok(())
        }
    }
}

fn parse_u16(value: &str, field: &str) -> Result<u16, BlockError> {
    value
        .parse::<u16>()
        .map_err(|err| BlockError::ProtocolError(format!("invalid {field}: {err}")))
}

fn parse_u32(value: &str, field: &str) -> Result<u32, BlockError> {
    value
        .parse::<u32>()
        .map_err(|err| BlockError::ProtocolError(format!("invalid {field}: {err}")))
}

fn print_usage(program: &str) {
    eprintln!("usage:");
    eprintln!("  {program} attach <host> <port> <export-name> <device> [--read-only] [--block-size=<bytes>]");
    eprintln!("  {program} detach <device>");
}
