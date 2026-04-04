use lifegraph_virtual_disk::{BlockClient, BlockError};
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

fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn decode_hex(input: &str) -> Result<Vec<u8>, BlockError> {
    if !input.len().is_multiple_of(2) {
        return Err(BlockError::ProtocolError(
            "hex payload must contain an even number of characters".into(),
        ));
    }
    let mut bytes = Vec::with_capacity(input.len() / 2);
    let mut chars = input.as_bytes().chunks_exact(2);
    for pair in &mut chars {
        let pair_str = std::str::from_utf8(pair)
            .map_err(|err| BlockError::ProtocolError(format!("invalid hex: {err}")))?;
        let byte = u8::from_str_radix(pair_str, 16).map_err(|err| {
            BlockError::ProtocolError(format!("invalid hex byte `{pair_str}`: {err}"))
        })?;
        bytes.push(byte);
    }
    Ok(bytes)
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
