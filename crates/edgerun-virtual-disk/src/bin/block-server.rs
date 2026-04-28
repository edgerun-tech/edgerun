mod common;

use common::{parse_u32, parse_u64};
use edgerun_virtual_disk::{
    BlockBackend, BlockDeviceInfo, BlockError, FileBlockBackend, MemoryBlockBackend,
    TcpBlockServer, UnixBlockServer,
};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

fn main() {
    if let Err(error) = run() {
        eprintln!("block-server error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), BlockError> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        print_usage(&args[0]);
        return Ok(());
    }

    let transport = &args[1];
    let endpoint = &args[2];
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
                model: format!("edgerun-{}-mem-demo", transport),
                serial: format!("mem:{}x{}", block_count, block_size),
            })?);
            serve_backend(
                transport,
                endpoint,
                backend,
                Some((block_count, block_size)),
            )
        }
        "file" => {
            if args.len() != 6 {
                print_usage(&args[0]);
                return Ok(());
            }
            let file_path = PathBuf::from(&args[4]);
            let block_size = parse_u32(&args[5], "block_size")?;
            let backend = Arc::new(FileBlockBackend::open(&file_path, block_size, false)?);
            let info = backend.info();
            serve_backend(
                transport,
                endpoint,
                backend,
                Some((info.block_count, info.block_size)),
            )
        }
        _ => {
            print_usage(&args[0]);
            Ok(())
        }
    }
}

fn serve_backend<B: BlockBackend>(
    transport: &str,
    endpoint: &str,
    backend: Arc<B>,
    geometry: Option<(u64, u32)>,
) -> Result<(), BlockError> {
    match transport {
        "unix" => {
            let server = UnixBlockServer::bind_shared(endpoint, Arc::clone(&backend))?;
            if let Some((block_count, block_size)) = geometry {
                println!(
                    "serving unix backend on {} ({} blocks x {} bytes)",
                    endpoint, block_count, block_size
                );
            } else {
                println!("serving unix backend on {endpoint}");
            }
            server.serve_forever()
        }
        "tcp" => {
            let server = TcpBlockServer::bind_shared(endpoint, Arc::clone(&backend))?;
            let addr = server.local_addr()?;
            if let Some((block_count, block_size)) = geometry {
                println!(
                    "serving tcp backend on {} ({} blocks x {} bytes)",
                    addr, block_count, block_size
                );
            } else {
                println!("serving tcp backend on {addr}");
            }
            server.serve_forever()
        }
        _ => {
            print_usage("block-server");
            Ok(())
        }
    }
}

fn print_usage(program: &str) {
    eprintln!("usage:");
    eprintln!("  {program} unix <socket-path> mem <block-size> <block-count>");
    eprintln!("  {program} unix <socket-path> file <disk-path> <block-size>");
    eprintln!("  {program} tcp  <host:port>  mem <block-size> <block-count>");
    eprintln!("  {program} tcp  <host:port>  file <disk-path> <block-size>");
}
