mod common;

use common::{decode_hex, encode_hex, parse_u32, parse_u64};
use edgerun_virtual_disk::{BlockClient, BlockError};
use std::env;

fn main() {
    if let Err(error) = run() {
        eprintln!("block-client error: {error}");
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
    match transport.as_str() {
        "unix" => {
            let mut client = BlockClient::connect_unix(endpoint)?;
            run_command(&mut client, &args, 3)
        }
        "tcp" => {
            let mut client = BlockClient::connect_tcp(endpoint)?;
            run_command(&mut client, &args, 3)
        }
        _ => {
            print_usage(&args[0]);
            Ok(())
        }
    }
}

fn run_command<T: std::io::Read + std::io::Write>(
    client: &mut BlockClient<T>,
    args: &[String],
    command_index: usize,
) -> Result<(), BlockError> {
    client.handshake()?;
    match args[command_index].as_str() {
        "info" => {
            let info = client.info()?;
            println!("block_size={}", info.block_size);
            println!("block_count={}", info.block_count);
            println!("total_size_bytes={}", info.total_size_bytes());
            println!("readonly={}", info.readonly);
            println!("supports_flush={}", info.supports_flush);
            println!("supports_discard={}", info.supports_discard);
            println!("supports_write_zeroes={}", info.supports_write_zeroes);
            println!("model={}", info.model);
            println!("serial={}", info.serial);
        }
        "ping" => {
            client.ping()?;
            println!("pong");
        }
        "read" => {
            if args.len() != command_index + 3 {
                print_usage(&args[0]);
                return Ok(());
            }
            let lba = parse_u64(&args[command_index + 1], "lba")?;
            let blocks = parse_u32(&args[command_index + 2], "blocks")?;
            let data = client.read_blocks(lba, blocks)?;
            println!("{}", encode_hex(&data));
        }
        "write" => {
            if args.len() != command_index + 4 {
                print_usage(&args[0]);
                return Ok(());
            }
            let lba = parse_u64(&args[command_index + 1], "lba")?;
            let blocks = parse_u32(&args[command_index + 2], "blocks")?;
            let data = decode_hex(&args[command_index + 3])?;
            client.write_blocks(lba, blocks, data)?;
            println!("ok");
        }
        "zero" => {
            if args.len() != command_index + 3 {
                print_usage(&args[0]);
                return Ok(());
            }
            let lba = parse_u64(&args[command_index + 1], "lba")?;
            let blocks = parse_u32(&args[command_index + 2], "blocks")?;
            client.write_zeroes(lba, blocks)?;
            println!("ok");
        }
        "flush" => {
            client.flush()?;
            println!("ok");
        }
        _ => print_usage(&args[0]),
    }

    Ok(())
}

fn print_usage(program: &str) {
    eprintln!("usage:");
    eprintln!("  {program} unix <socket-path> info");
    eprintln!("  {program} unix <socket-path> ping");
    eprintln!("  {program} unix <socket-path> read <lba> <blocks>");
    eprintln!("  {program} unix <socket-path> write <lba> <blocks> <hex-bytes>");
    eprintln!("  {program} unix <socket-path> zero <lba> <blocks>");
    eprintln!("  {program} unix <socket-path> flush");
    eprintln!("  {program} tcp  <host:port>  info");
    eprintln!("  {program} tcp  <host:port>  ping");
    eprintln!("  {program} tcp  <host:port>  read <lba> <blocks>");
    eprintln!("  {program} tcp  <host:port>  write <lba> <blocks> <hex-bytes>");
    eprintln!("  {program} tcp  <host:port>  zero <lba> <blocks>");
    eprintln!("  {program} tcp  <host:port>  flush");
}
