use edgerun_tuya::{
    decode_json_payload, decrypt_6699_payload, derive_v35_session_key, pack_6699_with_iv,
    parse_55aa_body_len, parse_55aa_wire_message, parse_6699_body_len, parse_6699_wire_message,
    strip_retcode, TuyaProtocolError, TuyaWireMessage, DP_QUERY_NEW, PREFIX_55AA, PREFIX_6699,
    SESS_KEY_NEG_FINISH, SESS_KEY_NEG_RESP, SESS_KEY_NEG_START,
};
use std::env;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() -> io::Result<()> {
    let config = Config::from_args()?;
    let dps_json = query_v35_status(&config.ip, config.key.as_bytes(), config.verbose)?;
    println!("{dps_json}");
    Ok(())
}

struct Config {
    ip: String,
    key: String,
    verbose: bool,
}

impl Config {
    fn from_args() -> io::Result<Self> {
        let mut id = env::var("TUYA_DEVICE_ID").unwrap_or_default();
        let mut ip = env::var("TUYA_DEVICE_IP").unwrap_or_else(|_| "192.168.1.43".to_string());
        let mut key = env::var("TUYA_LOCAL_KEY").unwrap_or_default();
        let mut verbose = false;

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--id" => id = args.next().unwrap_or_default(),
                "--ip" => ip = args.next().unwrap_or_default(),
                "--key" => key = args.next().unwrap_or_default(),
                "--verbose" | "-v" => verbose = true,
                "--help" | "-h" => {
                    eprintln!(
                        "Usage: tuya-power-meter --id <device-id> --ip <addr> --key <local-key> [--verbose]"
                    );
                    std::process::exit(0);
                }
                other => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("unknown argument `{other}`"),
                    ));
                }
            }
        }

        if id.is_empty() || ip.is_empty() || key.len() != 16 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "TUYA_DEVICE_ID/--id, TUYA_DEVICE_IP/--ip, and 16-byte TUYA_LOCAL_KEY/--key are required",
            ));
        }
        Ok(Self { ip, key, verbose })
    }
}

fn query_v35_status(ip: &str, local_key: &[u8], verbose: bool) -> io::Result<String> {
    let mut stream = TcpStream::connect((ip, 6668))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    stream.set_nodelay(true)?;

    let local_nonce = *b"0123456789abcdef";
    let start = pack_6699(1, SESS_KEY_NEG_START, &local_nonce, local_key)?;
    trace(verbose, "connected; sending v3.5 session start");
    stream.write_all(&start)?;
    let step2 = read_tuya_message(&mut stream)?;
    trace(
        verbose,
        &format!(
            "received session step 2: prefix={:08x} cmd={}",
            step2.prefix, step2.cmd
        ),
    );
    if step2.cmd != SESS_KEY_NEG_RESP {
        return Err(invalid_data("unexpected session negotiation response"));
    }

    let step2_plaintext = decrypt_6699_payload(&step2.raw_header, &step2.payload, local_key)
        .map_err(protocol_error)?;
    let step2_payload = strip_retcode(&step2_plaintext);
    if step2_payload.len() < 48 {
        return Err(invalid_data("short session negotiation payload"));
    }

    let remote_nonce = &step2_payload[..16];
    let expected_local_hmac = edgerun_crypto::hmac_sha256(local_key, &local_nonce);
    if expected_local_hmac.as_slice() != &step2_payload[16..48] {
        return Err(invalid_data("session negotiation HMAC check failed"));
    }

    let remote_hmac = edgerun_crypto::hmac_sha256(local_key, remote_nonce);
    let finish = pack_6699(2, SESS_KEY_NEG_FINISH, &remote_hmac, local_key)?;
    trace(verbose, "sending session finish");
    stream.write_all(&finish)?;

    let session_key =
        derive_v35_session_key(local_key, &local_nonce, remote_nonce).map_err(protocol_error)?;
    let query = pack_6699(3, DP_QUERY_NEW, b"{}", &session_key)?;
    trace(verbose, "sending DP_QUERY_NEW");
    stream.write_all(&query)?;

    let response = read_tuya_message(&mut stream)?;
    trace(
        verbose,
        &format!(
            "received query response: prefix={:08x} cmd={}",
            response.prefix, response.cmd
        ),
    );
    if response.prefix != PREFIX_6699 {
        return Err(invalid_data("expected v3.5 6699 response"));
    }

    let payload = decrypt_6699_payload(&response.raw_header, &response.payload, &session_key)
        .map_err(protocol_error)?;
    decode_json_payload(&payload).map_err(protocol_error)
}

fn read_tuya_message(stream: &mut TcpStream) -> io::Result<TuyaWireMessage> {
    let mut prefix_bytes = [0u8; 4];
    stream.read_exact(&mut prefix_bytes)?;
    let prefix = u32::from_be_bytes(prefix_bytes);
    match prefix {
        PREFIX_55AA => read_55aa(stream, prefix_bytes),
        PREFIX_6699 => read_6699(stream, prefix_bytes),
        _ => Err(invalid_data("unknown Tuya frame prefix")),
    }
}

fn read_55aa(stream: &mut TcpStream, prefix: [u8; 4]) -> io::Result<TuyaWireMessage> {
    let mut header_rest = [0u8; 12];
    stream.read_exact(&mut header_rest)?;
    let len = parse_55aa_body_len(&header_rest).map_err(protocol_error)?;
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body)?;
    parse_55aa_wire_message(prefix, &header_rest, &body).map_err(protocol_error)
}

fn read_6699(stream: &mut TcpStream, prefix: [u8; 4]) -> io::Result<TuyaWireMessage> {
    let mut header_rest = [0u8; 14];
    stream.read_exact(&mut header_rest)?;
    let len = parse_6699_body_len(&header_rest).map_err(protocol_error)?;
    let mut body = vec![0u8; len + 4];
    stream.read_exact(&mut body)?;
    parse_6699_wire_message(prefix, &header_rest, &body).map_err(protocol_error)
}

fn pack_6699(seq: u32, cmd: u32, payload: &[u8], key: &[u8]) -> io::Result<Vec<u8>> {
    let iv = tuya_gcm_iv();
    pack_6699_with_iv(seq, cmd, payload, key, &iv).map_err(protocol_error)
}

fn tuya_gcm_iv() -> [u8; 12] {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut iv = [0u8; 12];
    iv.copy_from_slice(&nanos.to_be_bytes()[4..]);
    iv
}

fn invalid_data(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn protocol_error(error: TuyaProtocolError) -> io::Error {
    invalid_data(match error {
        TuyaProtocolError::InvalidKey => "invalid Tuya key",
        TuyaProtocolError::InvalidFrame => "invalid Tuya frame",
        TuyaProtocolError::InvalidPayload => "invalid Tuya payload",
        TuyaProtocolError::AuthenticationFailed => "Tuya payload authentication failed",
    })
}

fn trace(verbose: bool, message: &str) {
    if verbose {
        eprintln!("{message}");
    }
}
