mod common;

use common::{parse_u32, parse_u64};
use edgerun_crypto::sha1::{Digest, Sha1};
use edgerun_encoding::base64::standard_encode;
use edgerun_virtual_disk::{
    decode_request_frame, encode_response_frame, handle_request, BlockBackend, BlockDeviceInfo,
    BlockError, FileBlockBackend, MemoryBlockBackend, TcpBlockServer, UnixBlockServer,
};
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

const MAX_WS_MESSAGE_LEN: usize = 16 * 1024 * 1024 + 4;
const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

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
            if args.len() != 6 && args.len() != 7 {
                print_usage(&args[0]);
                return Ok(());
            }
            let file_path = PathBuf::from(&args[4]);
            let block_size = parse_u32(&args[5], "block_size")?;
            let readonly = args.get(6).is_some_and(|arg| arg == "--read-only");
            if args.len() == 7 && !readonly {
                print_usage(&args[0]);
                return Ok(());
            }
            let backend = Arc::new(FileBlockBackend::open(&file_path, block_size, readonly)?);
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

fn serve_backend<B: BlockBackend + Send + Sync + 'static>(
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
        "ws" => serve_websocket(endpoint, backend, geometry),
        _ => {
            print_usage("block-server");
            Ok(())
        }
    }
}

fn serve_websocket<B: BlockBackend + Send + Sync + 'static>(
    endpoint: &str,
    backend: Arc<B>,
    geometry: Option<(u64, u32)>,
) -> Result<(), BlockError> {
    let listener = TcpListener::bind(endpoint).map_err(BlockError::from)?;
    let addr = listener.local_addr().map_err(BlockError::from)?;
    if let Some((block_count, block_size)) = geometry {
        println!(
            "serving websocket backend on ws://{} ({} blocks x {} bytes)",
            addr, block_count, block_size
        );
    } else {
        println!("serving websocket backend on ws://{addr}");
    }

    for stream in listener.incoming() {
        let stream = stream.map_err(BlockError::from)?;
        let backend = Arc::clone(&backend);
        thread::spawn(move || {
            if let Err(error) = serve_websocket_connection(stream, backend) {
                eprintln!("websocket block connection error: {error}");
            }
        });
    }
    Ok(())
}

fn serve_websocket_connection<B: BlockBackend>(
    mut stream: TcpStream,
    backend: Arc<B>,
) -> Result<(), BlockError> {
    websocket_handshake(&mut stream)?;
    loop {
        match read_ws_message(&mut stream)? {
            WsMessage::Binary(frame) => {
                let request = decode_request_frame(&frame)?;
                let response = handle_request(backend.as_ref(), request);
                let response_frame = encode_response_frame(&response)?;
                write_ws_binary(&mut stream, &response_frame)?;
            }
            WsMessage::Ping(payload) => write_ws_control(&mut stream, 0xA, &payload)?,
            WsMessage::Pong => {}
            WsMessage::Close => {
                let _ = write_ws_control(&mut stream, 0x8, &[]);
                return Ok(());
            }
        }
    }
}

fn websocket_handshake(stream: &mut TcpStream) -> Result<(), BlockError> {
    let request = read_http_upgrade_request(stream)?;
    let key = header_value(&request, "sec-websocket-key")
        .ok_or_else(|| BlockError::ProtocolError("missing Sec-WebSocket-Key".into()))?;
    let upgrade = header_value(&request, "upgrade").unwrap_or_default();
    if !upgrade.eq_ignore_ascii_case("websocket") {
        return Err(BlockError::ProtocolError(
            "request is not a websocket upgrade".into(),
        ));
    }

    let accept = websocket_accept(key.trim());
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept}\r\n\
         Sec-WebSocket-Protocol: edgerun-block-v1\r\n\
         \r\n"
    );
    stream
        .write_all(response.as_bytes())
        .map_err(BlockError::from)
}

fn read_http_upgrade_request(stream: &mut TcpStream) -> Result<String, BlockError> {
    let mut request = Vec::new();
    let mut byte = [0_u8; 1];
    while request.len() < 8192 {
        let n = stream.read(&mut byte).map_err(BlockError::from)?;
        if n == 0 {
            return Err(BlockError::ProtocolError(
                "unexpected EOF during websocket handshake".into(),
            ));
        }
        request.push(byte[0]);
        if request.ends_with(b"\r\n\r\n") {
            return String::from_utf8(request)
                .map_err(|err| BlockError::ProtocolError(err.to_string()));
        }
    }
    Err(BlockError::ProtocolError(
        "websocket handshake exceeds maximum header size".into(),
    ))
}

fn header_value<'a>(request: &'a str, name: &str) -> Option<&'a str> {
    request.lines().skip(1).find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim()
            .eq_ignore_ascii_case(name)
            .then_some(value.trim())
    })
}

fn websocket_accept(key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(WS_GUID.as_bytes());
    let digest = hasher.finalize();
    standard_encode(&digest)
}

enum WsMessage {
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong,
    Close,
}

fn read_ws_message(stream: &mut TcpStream) -> Result<WsMessage, BlockError> {
    let mut header = [0_u8; 2];
    stream.read_exact(&mut header).map_err(BlockError::from)?;
    let fin = header[0] & 0x80 != 0;
    let opcode = header[0] & 0x0f;
    if !fin {
        return Err(BlockError::Unsupported);
    }

    let masked = header[1] & 0x80 != 0;
    if !masked {
        return Err(BlockError::ProtocolError(
            "client websocket frames must be masked".into(),
        ));
    }

    let mut len = u64::from(header[1] & 0x7f);
    if len == 126 {
        let mut extended = [0_u8; 2];
        stream.read_exact(&mut extended).map_err(BlockError::from)?;
        len = u64::from(u16::from_be_bytes(extended));
    } else if len == 127 {
        let mut extended = [0_u8; 8];
        stream.read_exact(&mut extended).map_err(BlockError::from)?;
        len = u64::from_be_bytes(extended);
    }

    let len = usize::try_from(len)
        .map_err(|_| BlockError::ProtocolError("websocket message too large".into()))?;
    if len > MAX_WS_MESSAGE_LEN {
        return Err(BlockError::ProtocolError(
            "websocket message exceeds maximum size".into(),
        ));
    }

    let mut mask = [0_u8; 4];
    stream.read_exact(&mut mask).map_err(BlockError::from)?;
    let mut payload = vec![0_u8; len];
    stream.read_exact(&mut payload).map_err(BlockError::from)?;
    for (index, byte) in payload.iter_mut().enumerate() {
        *byte ^= mask[index % 4];
    }

    match opcode {
        0x2 => Ok(WsMessage::Binary(payload)),
        0x8 => Ok(WsMessage::Close),
        0x9 => Ok(WsMessage::Ping(payload)),
        0xA => Ok(WsMessage::Pong),
        _ => Err(BlockError::Unsupported),
    }
}

fn write_ws_binary(stream: &mut TcpStream, payload: &[u8]) -> Result<(), BlockError> {
    write_ws_frame(stream, 0x2, payload)
}

fn write_ws_control(stream: &mut TcpStream, opcode: u8, payload: &[u8]) -> Result<(), BlockError> {
    if payload.len() > 125 {
        return Err(BlockError::ProtocolError(
            "websocket control payload too large".into(),
        ));
    }
    write_ws_frame(stream, opcode, payload)
}

fn write_ws_frame(stream: &mut TcpStream, opcode: u8, payload: &[u8]) -> Result<(), BlockError> {
    let mut header = Vec::with_capacity(10);
    header.push(0x80 | (opcode & 0x0f));
    if payload.len() < 126 {
        header.push(payload.len() as u8);
    } else if payload.len() <= u16::MAX as usize {
        header.push(126);
        header.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        header.push(127);
        header.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }
    stream.write_all(&header).map_err(BlockError::from)?;
    stream.write_all(payload).map_err(BlockError::from)?;
    stream.flush().map_err(BlockError::from)
}

fn print_usage(program: &str) {
    eprintln!("usage:");
    eprintln!("  {program} unix <socket-path> mem <block-size> <block-count>");
    eprintln!("  {program} unix <socket-path> file <disk-path> <block-size> [--read-only]");
    eprintln!("  {program} tcp  <host:port>  mem <block-size> <block-count>");
    eprintln!("  {program} tcp  <host:port>  file <disk-path> <block-size> [--read-only]");
    eprintln!("  {program} ws   <host:port>  mem <block-size> <block-count>");
    eprintln!("  {program} ws   <host:port>  file <disk-path> <block-size> [--read-only]");
}
