use edgerun_virtual_disk::{
    BlockDeviceInfo, BlockError, FileBlockBackend, MemoryBlockBackend, MultiExportTcpNbdServer,
    NbdExport, NbdExportEntry, TcpNbdServer,
};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

fn main() {
    if let Err(error) = run() {
        eprintln!("nbd-server error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), BlockError> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 {
        print_usage(&args[0]);
        return Ok(());
    }

    let endpoint = &args[1];
    if args[2] == "multi-mem" {
        let exports = parse_multi_mem_exports(&args[3..])?;
        let export_count = exports.len();
        let server = MultiExportTcpNbdServer::bind(endpoint, exports)?;
        println!(
            "serving {} NBD exports on {}",
            export_count,
            server.local_addr()?
        );
        return server.serve_forever();
    }

    let export_name = &args[2];
    let mode = &args[3];

    match mode.as_str() {
        "mem" => {
            if args.len() != 6 {
                print_usage(&args[0]);
                return Ok(());
            }
            let block_size = parse_u32(&args[4], "block_size")?;
            let block_count = parse_u64(&args[5], "block_count")?;
            let backend = Arc::new(MemoryBlockBackend::new(BlockDeviceInfo {
                block_size,
                block_count,
                readonly: false,
                supports_flush: true,
                supports_discard: true,
                supports_write_zeroes: true,
                model: "edgerun-nbd-mem-demo".into(),
                serial: format!("nbd-mem:{}x{}", block_count, block_size),
            })?);
            let server = TcpNbdServer::bind_shared(
                endpoint,
                backend,
                NbdExport {
                    name: export_name.clone(),
                    description: "edgerun nbd memory export".into(),
                },
            )?;
            println!(
                "serving NBD export `{}` on {}",
                export_name,
                server.local_addr()?
            );
            server.serve_forever()
        }
        "file" => {
            if args.len() != 6 {
                print_usage(&args[0]);
                return Ok(());
            }
            let file_path = PathBuf::from(&args[4]);
            let block_size = parse_u32(&args[5], "block_size")?;
            let backend = Arc::new(FileBlockBackend::open(&file_path, block_size, false)?);
            let server = TcpNbdServer::bind_shared(
                endpoint,
                backend,
                NbdExport {
                    name: export_name.clone(),
                    description: format!("edgerun nbd file export {}", file_path.display()),
                },
            )?;
            println!(
                "serving NBD export `{}` on {}",
                export_name,
                server.local_addr()?
            );
            server.serve_forever()
        }
        _ => {
            print_usage(&args[0]);
            Ok(())
        }
    }
}

fn parse_multi_mem_exports(args: &[String]) -> Result<Vec<NbdExportEntry>, BlockError> {
    if args.is_empty() || !args.len().is_multiple_of(3) {
        return Err(BlockError::ProtocolError(
            "multi-mem expects repeating <export-name> <block-size> <block-count> triplets".into(),
        ));
    }
    let mut exports = Vec::new();
    for chunk in args.chunks_exact(3) {
        let export_name = &chunk[0];
        let block_size = parse_u32(&chunk[1], "block_size")?;
        let block_count = parse_u64(&chunk[2], "block_count")?;
        let backend = Arc::new(MemoryBlockBackend::new(BlockDeviceInfo {
            block_size,
            block_count,
            readonly: false,
            supports_flush: true,
            supports_discard: true,
            supports_write_zeroes: true,
            model: format!("edgerun-nbd-mem-{export_name}"),
            serial: format!("nbd-mem:{}x{}", block_count, block_size),
        })?);
        exports.push(NbdExportEntry {
            export: NbdExport {
                name: export_name.clone(),
                description: format!("edgerun nbd memory export {export_name}"),
            },
            backend,
        });
    }
    Ok(exports)
}

fn parse_u32(value: &str, field: &str) -> Result<u32, BlockError> {
    value
        .parse::<u32>()
        .map_err(|err| BlockError::ProtocolError(format!("invalid {field}: {err}")))
}

fn parse_u64(value: &str, field: &str) -> Result<u64, BlockError> {
    value
        .parse::<u64>()
        .map_err(|err| BlockError::ProtocolError(format!("invalid {field}: {err}")))
}

fn print_usage(program: &str) {
    eprintln!("usage:");
    eprintln!("  {program} <host:port> <export-name> mem <block-size> <block-count>");
    eprintln!("  {program} <host:port> <export-name> file <disk-path> <block-size>");
    eprintln!("  {program} <host:port> multi-mem <name1> <block-size1> <block-count1> [<name2> <block-size2> <block-count2> ...]");
}
