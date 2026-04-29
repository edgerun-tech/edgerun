use edgerun_crypto::aes_gcm::aead::generic_array::GenericArray;
use edgerun_crypto::Aes256GcmCipher;
use edgerun_rt::crc32;
use std::env;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const PREFIX_55AA: u32 = 0x0000_55aa;
const SUFFIX_55AA: u32 = 0x0000_aa55;
const PREFIX_6699: u32 = 0x0000_6699;
const SUFFIX_6699: u32 = 0x0000_9966;

const SESS_KEY_NEG_START: u32 = 0x03;
const SESS_KEY_NEG_RESP: u32 = 0x04;
const SESS_KEY_NEG_FINISH: u32 = 0x05;
const DP_QUERY_NEW: u32 = 0x10;

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

    let step2_plaintext = decrypt_6699_payload(&step2.raw_header, &step2.payload, local_key)?;
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

    let session_key = derive_v35_session_key(local_key, &local_nonce, remote_nonce)?;
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

    let payload = decrypt_6699_payload(&response.raw_header, &response.payload, &session_key)?;
    decode_json_payload(&payload)
}

fn derive_v35_session_key(
    local_key: &[u8],
    local_nonce: &[u8; 16],
    remote_nonce: &[u8],
) -> io::Result<[u8; 16]> {
    let mut xored = [0u8; 16];
    for i in 0..16 {
        xored[i] = local_nonce[i] ^ remote_nonce[i];
    }

    let cipher = Aes256GcmCipher::new(local_key).map_err(|_| invalid_data("invalid local key"))?;
    let nonce = GenericArray::from_slice(&local_nonce[..12]);
    let tag = cipher
        .encrypt_in_place_detached(nonce, &[], &mut xored)
        .map_err(|_| invalid_data("session key derivation failed"))?;
    let mut session_key = [0u8; 16];
    session_key.copy_from_slice(&xored);
    let _ = tag;
    Ok(session_key)
}

struct TuyaWireMessage {
    prefix: u32,
    cmd: u32,
    payload: Vec<u8>,
    raw_header: Vec<u8>,
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
    let seq = u32::from_be_bytes(header_rest[0..4].try_into().unwrap());
    let cmd = u32::from_be_bytes(header_rest[4..8].try_into().unwrap());
    let len = u32::from_be_bytes(header_rest[8..12].try_into().unwrap()) as usize;
    if len < 8 || len > 4096 {
        return Err(invalid_data("invalid 55aa length"));
    }
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body)?;
    let suffix = u32::from_be_bytes(body[len - 4..len].try_into().unwrap());
    if suffix != SUFFIX_55AA {
        return Err(invalid_data("invalid 55aa suffix"));
    }
    let mut raw_header = Vec::with_capacity(16);
    raw_header.extend_from_slice(&prefix);
    raw_header.extend_from_slice(&header_rest);
    let _ = seq;
    Ok(TuyaWireMessage {
        prefix: PREFIX_55AA,
        cmd,
        payload: body[..len - 8].to_vec(),
        raw_header,
    })
}

fn read_6699(stream: &mut TcpStream, prefix: [u8; 4]) -> io::Result<TuyaWireMessage> {
    let mut header_rest = [0u8; 14];
    stream.read_exact(&mut header_rest)?;
    let cmd = u32::from_be_bytes(header_rest[6..10].try_into().unwrap());
    let len = u32::from_be_bytes(header_rest[10..14].try_into().unwrap()) as usize;
    if len < 28 || len > 4096 {
        return Err(invalid_data("invalid 6699 length"));
    }
    let mut body = vec![0u8; len + 4];
    stream.read_exact(&mut body)?;
    let suffix = u32::from_be_bytes(body[len..len + 4].try_into().unwrap());
    if suffix != SUFFIX_6699 {
        return Err(invalid_data("invalid 6699 suffix"));
    }
    let mut raw_header = Vec::with_capacity(18);
    raw_header.extend_from_slice(&prefix);
    raw_header.extend_from_slice(&header_rest);
    Ok(TuyaWireMessage {
        prefix: PREFIX_6699,
        cmd,
        payload: body[..len].to_vec(),
        raw_header,
    })
}

fn pack_55aa(seq: u32, cmd: u32, payload: &[u8]) -> Vec<u8> {
    let len = payload.len() as u32 + 8;
    let mut frame = Vec::with_capacity(16 + payload.len() + 8);
    frame.extend_from_slice(&PREFIX_55AA.to_be_bytes());
    frame.extend_from_slice(&seq.to_be_bytes());
    frame.extend_from_slice(&cmd.to_be_bytes());
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(payload);
    let checksum = crc32(&frame);
    frame.extend_from_slice(&checksum.to_be_bytes());
    frame.extend_from_slice(&SUFFIX_55AA.to_be_bytes());
    frame
}

fn pack_6699(seq: u32, cmd: u32, payload: &[u8], key: &[u8]) -> io::Result<Vec<u8>> {
    let mut encrypted = payload.to_vec();
    let iv = tuya_gcm_iv();
    let len = (iv.len() + encrypted.len() + 16) as u32;

    let mut frame = Vec::with_capacity(18 + len as usize + 4);
    frame.extend_from_slice(&PREFIX_6699.to_be_bytes());
    frame.extend_from_slice(&0u16.to_be_bytes());
    frame.extend_from_slice(&seq.to_be_bytes());
    frame.extend_from_slice(&cmd.to_be_bytes());
    frame.extend_from_slice(&len.to_be_bytes());

    let cipher = Aes256GcmCipher::new(key).map_err(|_| invalid_data("invalid AES-GCM key"))?;
    let tag = cipher
        .encrypt_in_place_detached(GenericArray::from_slice(&iv), &frame[4..], &mut encrypted)
        .map_err(|_| invalid_data("v3.5 payload encryption failed"))?;

    frame.extend_from_slice(&iv);
    frame.extend_from_slice(&encrypted);
    frame.extend_from_slice(tag.as_slice());
    frame.extend_from_slice(&SUFFIX_6699.to_be_bytes());
    Ok(frame)
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

fn strip_retcode(payload: &[u8]) -> &[u8] {
    if payload.len() >= 4 && payload[..4] == [0, 0, 0, 0] {
        &payload[4..]
    } else {
        payload
    }
}

fn decrypt_6699_payload(header: &[u8], payload: &[u8], session_key: &[u8]) -> io::Result<Vec<u8>> {
    if payload.len() < 28 {
        return Err(invalid_data("short 6699 payload"));
    }
    let iv = &payload[..12];
    let tag = &payload[payload.len() - 16..];
    let mut ciphertext = payload[12..payload.len() - 16].to_vec();
    let cipher =
        Aes256GcmCipher::new(session_key).map_err(|_| invalid_data("invalid session key"))?;
    cipher
        .decrypt_in_place_detached(
            GenericArray::from_slice(iv),
            &header[4..],
            &mut ciphertext,
            GenericArray::from_slice(tag),
        )
        .map_err(|_| invalid_data("v3.5 payload authentication failed"))?;
    Ok(ciphertext)
}

fn decode_json_payload(payload: &[u8]) -> io::Result<String> {
    let mut start = 0usize;
    if payload.len() >= 5 && payload[4] == b'{' {
        start = 4;
    }
    let payload = &payload[start..];
    let json_start = payload
        .iter()
        .position(|b| *b == b'{')
        .ok_or_else(|| invalid_data("decrypted payload did not contain JSON"))?;
    let json_end = payload
        .iter()
        .rposition(|b| *b == b'}')
        .ok_or_else(|| invalid_data("decrypted payload did not contain JSON"))?;
    String::from_utf8(payload[json_start..=json_end].to_vec())
        .map_err(|_| invalid_data("decrypted payload was not UTF-8"))
}

fn invalid_data(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn trace(verbose: bool, message: &str) {
    if verbose {
        eprintln!("{message}");
    }
}
