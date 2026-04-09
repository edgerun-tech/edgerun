//! D-Bus session bus client — connects to the system bus, authenticates,
//! and registers `org.freedesktop.secrets` so desktop apps find us.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

// ===========================================================================
// Helpers
// ===========================================================================

/// Get the real UID of the current process by reading `/proc/self/status`.
/// No libc dependency needed.
fn get_uid() -> Option<u32> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("Uid:\t") {
            return rest.split_whitespace().next()?.parse().ok();
        }
    }
    None
}

// ===========================================================================
// Bus connection
// ===========================================================================

/// An active session bus connection with our service name registered.
pub struct BusConnection {
    stream: UnixStream,
    unique_name: String,
    service_name: String,
}

impl BusConnection {
    /// Connect to the session bus and register our well-known name.
    /// Returns `None` if the bus is not available or the name is taken.
    pub fn connect(service_name: &str) -> Option<Self> {
        let addr = find_session_bus_address()?;
        let mut stream = connect_to_bus(&addr)?;

        // Authenticate
        let unique = authenticate_external(&mut stream)?;

        // Register our name
        let rc = request_name(&mut stream, service_name)?;
        if rc != 1 && rc != 4 {
            return None; // name taken
        }

        eprintln!("edgerun-secret-service: registered '{}' on D-Bus session bus", service_name);
        Some(Self {
            stream,
            unique_name: unique,
            service_name: service_name.to_string(),
        })
    }

    /// Accept one D-Bus message routed to us by the bus daemon.
    /// Returns the raw message bytes and the sender's unique name.
    pub fn accept_one(&mut self) -> io::Result<Option<(Vec<u8>, String)>> {
        let msg = match read_one_message(&mut self.stream)? {
            Some(m) => m,
            None => return Ok(None),
        };
        let sender = extract_sender_from_raw(&msg).unwrap_or_else(|| self.unique_name.clone());
        Ok(Some((msg, sender)))
    }

    /// Send a reply back through the bus.
    pub fn send(&mut self, data: &[u8]) -> io::Result<()> {
        self.stream.write_all(data)?;
        self.stream.flush()
    }

    pub fn unique_name(&self) -> &str {
        &self.unique_name
    }

    pub fn service_name(&self) -> &str {
        &self.service_name
    }
}

// ===========================================================================
// Finding the bus
// ===========================================================================

fn find_session_bus_address() -> Option<PathBuf> {
    // 1. DBUS_SESSION_BUS_ADDRESS env var
    if let Ok(addr) = std::env::var("DBUS_SESSION_BUS_ADDRESS") {
        if let Some(path) = addr.strip_prefix("unix:path=") {
            let p = path.split(',').next().unwrap();
            return Some(PathBuf::from(p));
        }
        if addr.starts_with("unix:abstract=") {
            // Abstract namespace sockets not easily available from Rust std
            // Fall through to filesystem paths
        }
    }

    // 2. XDG_RUNTIME_DIR/bus (systemd)
    if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
        let bus = PathBuf::from(xdg).join("bus");
        if bus.exists() {
            return Some(bus);
        }
    }

    // 3. Fallbacks
    for p in &["/run/user/1000/bus", "/run/dbus/system_bus_socket"] {
        let path = PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

fn connect_to_bus(addr: &PathBuf) -> Option<UnixStream> {
    // Abstract sockets start with \0
    let path_str = addr.to_string_lossy();
    if path_str.starts_with('\0') {
        // Abstract socket — use raw address
        #[cfg(target_os = "linux")]
        {
            use std::os::linux::net::SocketAddrExt;
            let name = &path_str[1..];
            let addr = std::os::unix::net::SocketAddr::from_abstract_name(name.as_bytes()).ok()?;
            return UnixStream::connect_addr(&addr).ok();
        }
        #[cfg(not(target_os = "linux"))]
        return None;
    }

    UnixStream::connect(addr).ok()
}

// ===========================================================================
// Authentication (EXTERNAL mechanism)
// ===========================================================================

fn authenticate_external(stream: &mut UnixStream) -> Option<String> {
    // Get our real UID from /proc/self/status (no libc needed)
    let uid = get_uid()?;
    let uid_hex = format!("{:02X}", uid);

    // AUTH EXTERNAL <uid>\r\n
    write_bus(stream, &format!("AUTH EXTERNAL {}\r\n", uid_hex))?;

    // Read response
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;

    if !line.starts_with("OK ") {
        return None;
    }

    // BEGIN\r\n — from now on it's raw D-Bus protocol
    write_bus(stream, "BEGIN\r\n")?;

    // HELLO — get our unique name
    call_hello(stream)
}

fn write_bus(stream: &mut UnixStream, data: &str) -> Option<()> {
    stream.write_all(data.as_bytes()).ok()?;
    stream.flush().ok()?;
    Some(())
}

fn call_hello(stream: &mut UnixStream) -> Option<String> {
    let msg = build_hello();
    stream.write_all(&msg).ok()?;
    stream.flush().ok()?;

    // Read reply
    let reply = read_one_message(stream).ok()??;
    parse_hello_reply(&reply)
}

// ===========================================================================
// Name registration
// ===========================================================================

fn request_name(stream: &mut UnixStream, name: &str) -> Option<u32> {
    let serial = next_serial();
    let msg = build_request_name(serial, name);
    stream.write_all(&msg).ok()?;
    stream.flush().ok()?;

    let reply = read_one_message(stream).ok()??;
    parse_u32_reply(&reply)
}

// ===========================================================================
// Message I/O
// ===========================================================================

/// Read one complete D-Bus message from the stream.
pub fn read_one_message(stream: &mut UnixStream) -> io::Result<Option<Vec<u8>>> {
    // Read 16-byte fixed header
    let mut fixed = [0u8; 16];
    match stream.read_exact(&mut fixed) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(None),
        Err(e) if e.kind() == io::ErrorKind::Interrupted => return Ok(None),
        Err(e) => return Err(e),
    }

    if fixed[0] != b'l' {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "not little-endian D-Bus"));
    }

    let body_len = u32::from_le_bytes([fixed[4], fixed[5], fixed[6], fixed[7]]) as usize;
    let hf_len = u32::from_le_bytes([fixed[12], fixed[13], fixed[14], fixed[15]]) as usize;

    let total_hf = 16 + hf_len;
    let aligned_hf = (total_hf + 7) & !7;
    let pad = aligned_hf - total_hf;
    let total_len = aligned_hf + body_len;

    // Read the rest
    let mut msg = vec![0u8; total_len];
    msg[..16].copy_from_slice(&fixed);
    stream.read_exact(&mut msg[16..])?;

    Ok(Some(msg))
}

/// Extract the sender field (F_SENDER = 7) from a raw D-Bus message header.
fn extract_sender_from_raw(raw: &[u8]) -> Option<String> {
    if raw.len() < 16 { return None; }
    let hf_len = u32::from_le_bytes([raw[12], raw[13], raw[14], raw[15]]) as usize;
    if hf_len == 0 { return None; }

    let end = 16 + hf_len;
    let mut pos = 16;
    while pos + 2 <= end {
        let field_code = raw[pos];
        let field_type = raw[pos + 1];
        pos += 2;

        if field_type == b's' {
            // STRING variant
            if pos + 4 > end { break; }
            let str_len = u32::from_le_bytes([raw[pos], raw[pos+1], raw[pos+2], raw[pos+3]]) as usize;
            pos += 4;
            if pos + str_len > end { break; }
            if field_code == 7 { // F_SENDER
                return String::from_utf8(raw[pos..pos+str_len].to_vec()).ok();
            }
            pos += str_len + 1;
        } else if field_type == b'o' {
            if pos + 4 > end { break; }
            let str_len = u32::from_le_bytes([raw[pos], raw[pos+1], raw[pos+2], raw[pos+3]]) as usize;
            pos += 4 + str_len + 1;
        } else if field_type == b'u' {
            pos += 4;
        } else if field_type == b'g' {
            if pos >= end { break; }
            let sig_len = raw[pos] as usize;
            pos += 1 + sig_len + 1;
        } else {
            break;
        }

        // Align to 8 bytes from start of header
        pos = 16 + (((pos - 16) + 7) & !7);
        if pos >= end { break; }
    }
    None
}

// ===========================================================================
// Message builders
// ===========================================================================

static SERIAL: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

fn next_serial() -> u32 {
    SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Build a HELLO method call (no dest, to org.freedesktop.DBus).
fn build_hello() -> Vec<u8> {
    build_method_call(
        next_serial(),
        "org.freedesktop.DBus", // destination
        "/",                    // path
        "org.freedesktop.DBus", // interface
        "Hello",                // member
        "",                     // signature
        &[],                    // body
    )
}

/// Build a RequestName method call.
fn build_request_name(serial: u32, name: &str) -> Vec<u8> {
    let nb = name.as_bytes();
    let flags: u32 = 0x4; // DBUS_NAME_FLAG_ALLOW_REPLACEMENT

    let mut body = Vec::new();
    // arg0: name (STRING)
    body.extend_from_slice(&(nb.len() as u32).to_le_bytes());
    body.extend_from_slice(nb);
    body.push(0);
    // pad to 8-byte boundary (STRING is already aligned after null)
    // arg1: flags (UINT32) — needs 4-byte alignment, string ends on 8-byte boundary
    body.extend_from_slice(&flags.to_le_bytes());

    build_method_call(
        serial,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "RequestName",
        "su",
        &body,
    )
}

/// Build a D-Bus method call message.
fn build_method_call(
    serial: u32,
    dest: &str,
    path: &str,
    iface: &str,
    member: &str,
    signature: &str,
    body_bytes: &[u8],
) -> Vec<u8> {
    let body_len = body_bytes.len();

    // Build header fields
    let mut fields: Vec<u8> = Vec::new();

    // F_PATH (1): OBJECT_PATH
    let pb = path.as_bytes();
    fields.push(1);
    fields.push(b'o');
    fields.extend_from_slice(&(pb.len() as u32).to_le_bytes());
    fields.extend_from_slice(pb);
    fields.push(0);

    // F_IFACE (2): STRING
    let ib = iface.as_bytes();
    fields.push(2);
    fields.push(b's');
    fields.extend_from_slice(&(ib.len() as u32).to_le_bytes());
    fields.extend_from_slice(ib);
    fields.push(0);

    // F_MEMBER (3): STRING
    let mb = member.as_bytes();
    fields.push(3);
    fields.push(b's');
    fields.extend_from_slice(&(mb.len() as u32).to_le_bytes());
    fields.extend_from_slice(mb);
    fields.push(0);

    // F_DEST (6): STRING
    let db = dest.as_bytes();
    fields.push(6);
    fields.push(b's');
    fields.extend_from_slice(&(db.len() as u32).to_le_bytes());
    fields.extend_from_slice(db);
    fields.push(0);

    // F_SIGNATURE (8): SIGNATURE (type 'g')
    let sb = signature.as_bytes();
    fields.push(8);
    fields.push(b'g');
    fields.push(sb.len() as u8);
    fields.extend_from_slice(sb);
    fields.push(0);

    let fields_len = fields.len();
    let total_hf = 16 + fields_len;
    let aligned_hf = (total_hf + 7) & !7;
    let pad_len = aligned_hf - total_hf;

    let mut msg = Vec::with_capacity(aligned_hf + body_len);

    // Fixed header
    msg.push(b'l');
    msg.push(1); // method_call
    msg.push(0);
    msg.push(1);
    msg.extend_from_slice(&(body_len as u32).to_le_bytes());
    msg.extend_from_slice(&serial.to_le_bytes());
    msg.extend_from_slice(&(fields_len as u32).to_le_bytes());

    msg.extend_from_slice(&fields);
    for _ in 0..pad_len { msg.push(0); }
    msg.extend_from_slice(body_bytes);

    msg
}

// ===========================================================================
// Reply parsing
// ===========================================================================

fn parse_hello_reply(msg: &[u8]) -> Option<String> {
    // Method reply with body: STRING (unique name)
    // Fixed header: [0]=type (2=reply), body_len at [4..8]
    if msg.len() < 16 || msg[1] != 2 {
        return None;
    }

    let body_len = u32::from_le_bytes([msg[4], msg[5], msg[6], msg[7]]) as usize;
    if body_len == 0 {
        return None;
    }

    let hf_len = u32::from_le_bytes([msg[12], msg[13], msg[14], msg[15]]) as usize;
    let total_hf = 16 + hf_len;
    let aligned_hf = (total_hf + 7) & !7;
    let body_start = aligned_hf;

    if body_start + 4 > msg.len() {
        return None;
    }

    let str_len = u32::from_le_bytes([
        msg[body_start],
        msg[body_start + 1],
        msg[body_start + 2],
        msg[body_start + 3],
    ]) as usize;

    if body_start + 4 + str_len + 1 > msg.len() {
        return None;
    }

    let name_bytes = &msg[body_start + 4..body_start + 4 + str_len];
    String::from_utf8(name_bytes.to_vec()).ok()
}

fn parse_u32_reply(msg: &[u8]) -> Option<u32> {
    if msg.len() < 16 || msg[1] != 2 {
        return None;
    }

    let body_len = u32::from_le_bytes([msg[4], msg[5], msg[6], msg[7]]) as usize;
    if body_len < 4 {
        return None;
    }

    let hf_len = u32::from_le_bytes([msg[12], msg[13], msg[14], msg[15]]) as usize;
    let total_hf = 16 + hf_len;
    let aligned_hf = (total_hf + 7) & !7;
    let body_start = aligned_hf;

    if body_start + 4 > msg.len() {
        return None;
    }

    // UINT32 needs 4-byte alignment — pad from body_start to 4-byte boundary
    let aligned_body = (body_start + 3) & !3;
    if aligned_body + 4 > msg.len() {
        return None;
    }

    Some(u32::from_le_bytes([
        msg[aligned_body],
        msg[aligned_body + 1],
        msg[aligned_body + 2],
        msg[aligned_body + 3],
    ]))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_hello_message() {
        let msg = build_hello();
        assert!(msg.len() > 16);
        assert_eq!(msg[0], b'l');
        assert_eq!(msg[1], 1); // method_call
    }

    #[test]
    fn build_request_name_message() {
        let msg = build_request_name(42, "org.freedesktop.secrets");
        assert!(msg.len() > 16);
        assert_eq!(msg[0], b'l');
        assert_eq!(msg[1], 1);
    }

    #[test]
    fn serial_increments() {
        let a = next_serial();
        let b = next_serial();
        assert_eq!(b, a + 1);
    }
}
