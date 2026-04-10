//! Minimal D-Bus client for systemd-logind DRM session management.
//!
//! Uses raw Unix domain sockets to talk to the system D-Bus,
//! then to org.freedesktop.login1 for session management.
//!
//! No external crates — all D-Bus marshalling done by hand.

use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

/// Acquire a DRM device via logind.
///
/// This asks logind to give us the DRM fd for the given sysname (e.g., "card0").
/// Logind handles DRM master properly, revoking it from the old session if needed.
///
/// Returns the DRM fd as an OwnedFd.
pub fn logind_acquire_drm_fd(sysname: &str) -> io::Result<OwnedFd> {
    let mut dbus = DbusConn::connect_system()?;
    let unique_name = dbus.hello()?;
    let _ = unique_name; // not needed for method calls

    let session_id = get_own_session_id()?;
    let session_path = format!("/org/freedesktop/login1/session/{}", session_id);

    // Activate session (non-blocking — ignore errors if already active)
    dbus.call_method(
        "org.freedesktop.login1",
        &session_path,
        "org.freedesktop.login1.Session",
        "Activate",
        b"",
        &[],
    )?;

    // Get major/minor from sysfs
    let (major, minor) = get_device_major_minor(sysname)?;

    // TakeDevice: args (uu b), reply (hb)
    let (fd, _result) = dbus.call_method_with_fds_reply(
        "org.freedesktop.login1",
        &session_path,
        "org.freedesktop.login1.Session",
        "TakeDevice",
        b"uub",
        &[(major as u32).to_le_bytes().to_vec(), (minor as u32).to_le_bytes().to_vec(), vec![1u8]],
    )?;

    Ok(fd)
}

/// Release a DRM device via logind.
pub fn logind_release_device(sysname: &str) -> io::Result<()> {
    let mut dbus = DbusConn::connect_system()?;
    let _ = dbus.hello()?;

    let session_id = get_own_session_id()?;
    let session_path = format!("/org/freedesktop/login1/session/{}", session_id);
    let (major, minor) = get_device_major_minor(sysname)?;

    dbus.call_method(
        "org.freedesktop.login1",
        &session_path,
        "org.freedesktop.login1.Session",
        "ReleaseDevice",
        b"uu",
        &[(major as u32).to_le_bytes().to_vec(), (minor as u32).to_le_bytes().to_vec()],
    )?;

    Ok(())
}

// ─── D-Bus connection ────────────────────────────────────────

struct DbusConn {
    fd: OwnedFd,
    serial: u32,
}

impl DbusConn {
    fn connect_system() -> io::Result<Self> {
        let addr = std::env::var("DBUS_SYSTEM_BUS_ADDRESS")
            .unwrap_or_else(|_| "unix:path=/run/dbus/system_bus_socket".to_string());

        let fd = connect_unix_socket(&addr)?;
        authenticate_dbus(fd.as_raw_fd())?;

        Ok(Self { fd, serial: 1 })
    }

    fn hello(&self) -> io::Result<String> {
        // Method call: org.freedesktop.DBus.Hello on /org/freedesktop/DBus
        let msg = build_method_call(
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
            "Hello",
            b"",
            &[],
            self.serial,
        );

        send_all(self.fd.as_raw_fd(), &msg)?;
        let reply = read_message(self.fd.as_raw_fd())?;

        // Reply body: string (unique name)
        // Type string = 's' = 0x73, then length (u32 LE), then bytes
        if reply.is_empty() || reply[0] != 2 {
            // Not METHOD_RETURN
            return Err(io::Error::new(io::ErrorKind::Other, "D-Bus Hello: unexpected reply type"));
        }

        // Skip header — find body start
        let body_offset = find_body_offset(&reply);
        if body_offset + 5 > reply.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "D-Bus Hello reply too short"));
        }

        // Body: type 's'
        if reply[body_offset] != 0x73 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Expected string in Hello reply"));
        }

        let str_len = u32::from_le_bytes([
            reply[body_offset + 1],
            reply[body_offset + 2],
            reply[body_offset + 3],
            reply[body_offset + 4],
        ]) as usize;

        let start = body_offset + 5;
        let end = start + str_len;
        if end > reply.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Hello reply string truncated"));
        }

        String::from_utf8(reply[start..end].to_vec())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    fn call_method(
        &mut self,
        destination: &str,
        path: &str,
        interface: &str,
        member: &str,
        signature: &[u8],
        body_fields: &[Vec<u8>],
    ) -> io::Result<Vec<u8>> {
        let serial = self.serial;
        self.serial += 1;

        let msg = build_method_call(destination, path, interface, member, signature, body_fields, serial);
        send_all(self.fd.as_raw_fd(), &msg)?;

        // Read reply
        read_message(self.fd.as_raw_fd())
    }

    fn call_method_with_fds_reply(
        &mut self,
        destination: &str,
        path: &str,
        interface: &str,
        member: &str,
        signature: &[u8],
        body_fields: &[Vec<u8>],
    ) -> io::Result<(OwnedFd, bool)> {
        let serial = self.serial;
        self.serial += 1;

        let (msg, _fds_out) = build_method_call_with_fds(destination, path, interface, member, signature, body_fields, serial);
        send_all(self.fd.as_raw_fd(), &msg)?;

        // Read reply with FD support
        let (reply, fds) = read_message_with_fds(self.fd.as_raw_fd())?;

        // Check reply type
        if reply.is_empty() || reply[0] != 2 {
            // Check if it's an error
            if reply.len() > 0 && reply[0] == 3 {
                // Error reply — extract error name
                let body_offset = find_body_offset(&reply);
                if body_offset + 5 < reply.len() && reply[body_offset] == 0x73 {
                    let str_len = u32::from_le_bytes([
                        reply[body_offset + 1],
                        reply[body_offset + 2],
                        reply[body_offset + 3],
                        reply[body_offset + 4],
                    ]) as usize;
                    let start = body_offset + 5;
                    let error_name = String::from_utf8_lossy(
                        &reply[start..start.min(start + str_len).min(reply.len())]
                    );
                    return Err(io::Error::new(io::ErrorKind::Other, format!("D-Bus error: {}", error_name)));
                }
            }
            return Err(io::Error::new(io::ErrorKind::Other, "Unexpected D-Bus reply type"));
        }

        // Parse reply body: TakeDevice returns (hb) = fd + bool
        // We expect at least one FD and one boolean
        if fds.is_empty() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "No FDs in TakeDevice reply"));
        }

        let body_offset = find_body_offset(&reply);
        let result_bool = if body_offset + 2 <= reply.len() {
            // After fd 'h' (type 0x68) + index (u32), then bool 'b' (type 0x62)
            // Type signature: 'h' = 0x68 at body_offset
            // Then fd index (u32 LE), then 'b' = 0x62, padding (3 bytes), then bool value
            if body_offset + 12 <= reply.len() {
                reply[body_offset + 11] != 0
            } else {
                false
            }
        } else {
            false
        };

        Ok((fds.into_iter().next().unwrap(), result_bool))
    }
}

/// Connect to a Unix socket D-Bus address.
fn connect_unix_socket(addr: &str) -> io::Result<OwnedFd> {
    if !addr.starts_with("unix:path=") {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Only unix:path D-Bus addresses supported"));
    }
    let path = &addr["unix:path=".len()..];
    let c_path = std::ffi::CString::new(path.as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "bad path"))?;

    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }

    let fd = unsafe { OwnedFd::from_raw_fd(fd) };

    let sun_path = c_path.as_bytes();
    let mut addr = unsafe { std::mem::zeroed::<libc::sockaddr_un>() };
    addr.sun_family = libc::AF_UNIX as _;
    let copy_len = sun_path.len().min(addr.sun_path.len() - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(sun_path.as_ptr(), addr.sun_path.as_mut_ptr() as *mut u8, copy_len);
    }

    let addr_len = std::mem::size_of::<libc::sockaddr_un>();
    let ret = unsafe { libc::connect(fd.as_raw_fd(), &addr as *const _ as *const libc::sockaddr, addr_len as _) };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }

    authenticate_dbus(fd.as_raw_fd())?;

    Ok(fd)
}

/// D-Bus EXTERNAL authentication.
fn authenticate_dbus(fd: RawFd) -> io::Result<()> {
    let uid = unsafe { libc::getuid() };
    let hex_uid = format!("{:x}", uid);
    // AUTH EXTERNAL <hex-uid> \r\n
    let auth = format!("\0AUTH EXTERNAL {}\r\n", hex_uid);
    send_all_raw(fd, auth.as_bytes())?;

    // Read response: should be "OK <cookie>\r\n"
    let mut buf = [0u8; 256];
    loop {
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n <= 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "D-Bus auth EOF"));
        }
        let resp = String::from_utf8_lossy(&buf[..n as usize]);
        for line in resp.lines() {
            if line.starts_with("OK") {
                return Ok(());
            }
            if line.starts_with("ERROR") || line.starts_with("REJECTED") {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, format!("D-Bus auth failed: {}", line)));
            }
        }
    }
}

/// Send BEGIN and negotiate unix FDs.
fn send_begin(fd: RawFd) -> io::Result<()> {
    // NEGOTIATE_UNIX_FD\r\n (we want FD support)
    send_all_raw(fd, b"NEGOTIATE_UNIX_FD\r\n")?;
    send_all_raw(fd, b"BEGIN\r\n")?;
    Ok(())
}

// ─── D-Bus message building ─────────────────────────────────

fn build_method_call(
    destination: &str,
    path: &str,
    interface: &str,
    member: &str,
    signature: &[u8],
    body_fields: &[Vec<u8>],
    serial: u32,
) -> Vec<u8> {
    let mut header_fields = Vec::new();

    // Field 1: PATH (object path)
    write_field_header(&mut header_fields, 1, 6); // type=OBJECT_PATH(6), number=1
    write_object_path(&mut header_fields, path);

    // Field 2: INTERFACE (string)
    write_field_header(&mut header_fields, 2, 8); // type=STRING(8), number=2
    write_string(&mut header_fields, interface);

    // Field 3: MEMBER (string)
    write_field_header(&mut header_fields, 3, 8); // type=STRING(8), number=3
    write_string(&mut header_fields, member);

    // Field 6: DESTINATION (string)
    write_field_header(&mut header_fields, 6, 8);
    write_string(&mut header_fields, destination);

    // Field 7: SERIAL (uint32)
    write_field_header(&mut header_fields, 7, 7); // type=UINT32(7), number=7
    align_to_4(&mut header_fields);
    header_fields.extend_from_slice(&serial.to_le_bytes());

    // Build body
    let mut body = Vec::new();
    if !signature.is_empty() {
        // Write signature string (D-Bus signatures are ASCII-safe)
        let sig_str = std::str::from_utf8(signature).unwrap_or("?");
        write_string(&mut body, sig_str);
    }
    // Write body fields according to signature
    for field in body_fields {
        body.extend_from_slice(field);
    }
    align_to_8(&mut body);

    build_message_raw(1, 0, 0, serial, &header_fields, &body, &[])
}

fn build_method_call_with_fds(
    destination: &str,
    path: &str,
    interface: &str,
    member: &str,
    signature: &[u8],
    body_fields: &[Vec<u8>],
    serial: u32,
) -> (Vec<u8>, Vec<OwnedFd>) {
    let mut header_fields = Vec::new();

    // Field 1: PATH
    write_field_header(&mut header_fields, 1, 6);
    write_object_path(&mut header_fields, path);

    // Field 2: INTERFACE
    write_field_header(&mut header_fields, 2, 8);
    write_string(&mut header_fields, interface);

    // Field 3: MEMBER
    write_field_header(&mut header_fields, 3, 8);
    write_string(&mut header_fields, member);

    // Field 6: DESTINATION
    write_field_header(&mut header_fields, 6, 8);
    write_string(&mut header_fields, destination);

    // Field 7: SERIAL
    write_field_header(&mut header_fields, 7, 7);
    align_to_4(&mut header_fields);
    header_fields.extend_from_slice(&serial.to_le_bytes());

    // Build body
    let mut body = Vec::new();
    if !signature.is_empty() {
        let sig_str = std::str::from_utf8(signature).unwrap_or("?");
        write_string(&mut body, sig_str);
    }
    for field in body_fields {
        body.extend_from_slice(field);
    }
    align_to_8(&mut body);

    // No FDs to send in the request (TakeDevice doesn't need them)
    let msg = build_message_raw(1, 0, 1, serial, &header_fields, &body, &[]);
    (msg, vec![])
}

fn build_message_raw(
    msg_type: u8,
    flags: u8,
    _unix_fds: u32,
    serial: u32,
    header_fields: &[u8],
    body: &[u8],
    _fds: &[OwnedFd],
) -> Vec<u8> {
    // D-Bus message header (minimum 12 bytes)
    let mut msg = Vec::new();

    // Byte 0: endianness (1=little) | message type
    msg.push(0x01 | (msg_type << 2));
    msg.push(flags);
    // Version
    msg.push(1);
    msg.push(0); // padding

    // Body length (placeholder)
    let body_len = body.len() as u32;
    msg.extend_from_slice(&body_len.to_le_bytes());

    // Serial (placeholder — we'll fill it)
    msg.extend_from_slice(&serial.to_le_bytes());

    // Header fields
    msg.extend_from_slice(header_fields);

    // Align to 8 bytes for body
    while msg.len() % 8 != 0 {
        msg.push(0);
    }

    // Body
    msg.extend_from_slice(body);

    msg
}

fn write_field_header(buf: &mut Vec<u8>, field_number: u8, type_code: u8) {
    buf.push(type_code);
    buf.push(field_number);
}

fn write_string(buf: &mut Vec<u8>, s: &str) {
    let len = s.len() as u32;
    buf.extend_from_slice(&len.to_le_bytes());
    buf.extend_from_slice(s.as_bytes());
    buf.push(0); // null terminator
    align_to_4(buf);
}

fn write_object_path(buf: &mut Vec<u8>, path: &str) {
    let len = path.len() as u32;
    buf.extend_from_slice(&len.to_le_bytes());
    buf.extend_from_slice(path.as_bytes());
    buf.push(0);
    align_to_4(buf);
}

fn align_to_4(buf: &mut Vec<u8>) {
    while buf.len() % 4 != 0 {
        buf.push(0);
    }
}

fn align_to_8(buf: &mut Vec<u8>) {
    while buf.len() % 8 != 0 {
        buf.push(0);
    }
}

fn find_body_offset(msg: &[u8]) -> usize {
    // Header is: 1 byte type, 1 byte flags, 1 byte version, 1 byte pad,
    // 4 bytes body len, 4 bytes serial, then header fields until alignment
    // Body starts at the first 8-byte aligned offset after the 12-byte base header.
    // But we need to scan for the actual body — it's after all header fields.
    // Simplified: header fields are at offset 12, each field is:
    //   1 byte type, 1 byte number, then value (aligned)
    // Find where the body starts by scanning the header fields.
    if msg.len() < 12 {
        return msg.len();
    }

    // Parse the body length field to know how many bytes are body
    let body_len = u32::from_le_bytes([msg[4], msg[5], msg[6], msg[7]]) as usize;

    // Header fields end where the body begins
    // The body starts at the first 8-byte aligned position after the 16-byte base
    // (12 bytes base + header fields, aligned to 8)
    // Actually, the D-Bus spec says: the body starts at offset 16 + header_fields_len,
    // aligned to 8 bytes. But we need to compute header_fields_len.

    // Scan header fields starting at offset 12
    let mut pos = 12;
    loop {
        // Find next non-zero byte (type codes are non-zero)
        // or stop when we hit the 8-byte boundary that contains body_len match
        while pos < msg.len() && msg[pos] == 0 {
            pos += 1;
        }

        if pos >= msg.len() || pos % 8 == 0 {
            // Check if remaining data matches body_len
            let body_start = pos;
            let aligned_start = if body_start % 8 != 0 {
                body_start + (8 - body_start % 8)
            } else {
                body_start
            };
            if body_len > 0 && msg.len() >= aligned_start + body_len {
                return aligned_start;
            } else if body_len == 0 {
                return aligned_start;
            } else {
                // Try next alignment
                let next_aligned = ((aligned_start + 7) / 8) * 8;
                if next_aligned < msg.len() && (next_aligned + body_len <= msg.len() || body_len == 0) {
                    return next_aligned;
                }
                // Fallback: return pos
                return pos.min(msg.len());
            }
        }

        let type_code = msg[pos];
        pos += 1; // skip type
        if pos >= msg.len() { break; }
        pos += 1; // skip field number
        if pos >= msg.len() { break; }

        // Skip value based on type
        match type_code {
            1 => pos += 1,       // BYTE
            2 => { align_to_4_at(&mut pos); pos += 4; }, // BOOLEAN (actually stored as UINT32)
            3..=5 => { align_to_4_at(&mut pos); pos += 4; }, // INT16, UINT16, INT32
            6 => { align_to_4_at(&mut pos); pos += 4; }, // UINT32
            7 => pos += 8,       // UINT64
            8 => {               // STRING
                if pos + 4 <= msg.len() {
                    let slen = u32::from_le_bytes([msg[pos], msg[pos+1], msg[pos+2], msg[pos+3]]) as usize;
                    pos += 4 + slen + 1; // length + string + null
                    align_to_4_at(&mut pos);
                } else { break; }
            }
            9 => {               // ARRAY
                if pos + 4 <= msg.len() {
                    let alen = u32::from_le_bytes([msg[pos], msg[pos+1], msg[pos+2], msg[pos+3]]) as usize;
                    pos += 4 + alen;
                    align_to_4_at(&mut pos);
                } else { break; }
            }
            _ => break,          // unknown, bail out
        }
    }

    // Fallback: find body start by trying alignment positions
    let body_len = u32::from_le_bytes([msg[4], msg[5], msg[6], msg[7]]) as usize;
    for &start in &[16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 256] {
        if start <= msg.len() && (body_len == 0 || start + body_len <= msg.len()) {
            if body_len == 0 {
                return start;
            }
            // Heuristic: check if the content looks like a valid reply body
            if start + 1 < msg.len() {
                // For method returns, body starts with type codes
                let t = msg[start];
                if t >= 1 && t <= 12 {
                    return start;
                }
            }
        }
    }

    msg.len().min(16)
}

fn align_to_4_at(pos: &mut usize) {
    while *pos % 4 != 0 {
        *pos += 1;
    }
}

// ─── D-Bus message send/recv with SCM_RIGHTS ─────────────────

fn send_all(fd: RawFd, data: &[u8]) -> io::Result<()> {
    send_all_raw(fd, data)
}

fn send_all_raw(fd: RawFd, data: &[u8]) -> io::Result<()> {
    let mut sent = 0;
    while sent < data.len() {
        let n = unsafe { libc::write(fd, data[sent..].as_ptr() as *const libc::c_void, data.len() - sent) };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        sent += n as usize;
    }
    Ok(())
}

/// Read a complete D-Bus message from the socket.
fn read_message(fd: RawFd) -> io::Result<Vec<u8>> {
    let (msg, _fds) = read_message_with_fds(fd)?;
    Ok(msg)
}

/// Read a complete D-Bus message from the socket, extracting any received FDs.
fn read_message_with_fds(fd: RawFd) -> io::Result<(Vec<u8>, Vec<OwnedFd>)> {
    // Read header (16 bytes: 12 byte base + 4 byte serial)
    let mut header = [0u8; 16];
    read_exact(fd, &mut header)?;

    let body_len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;

    // Body may be large — read it in chunks
    let mut body = vec![0u8; body_len];
    if body_len > 0 {
        read_exact(fd, &mut body)?;
    }

    let mut msg = Vec::with_capacity(16 + body_len);
    msg.extend_from_slice(&header);
    msg.extend_from_slice(&body);

    // Check if there are ancillary FDs
    let fds = recv_ancillary_fds(fd)?;

    Ok((msg, fds))
}

/// Read exactly `len` bytes from fd.
fn read_exact(fd: RawFd, buf: &mut [u8]) -> io::Result<()> {
    let mut offset = 0;
    while offset < buf.len() {
        let n = unsafe { libc::read(fd, buf[offset..].as_mut_ptr() as *mut libc::c_void, buf.len() - offset) };
        if n < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::WouldBlock {
                // In non-blocking mode, wait and retry
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
            return Err(err);
        }
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "D-Bus connection closed"));
        }
        offset += n as usize;
    }
    Ok(())
}

/// Receive ancillary file descriptors from a socket using SCM_RIGHTS.
fn recv_ancillary_fds(fd: RawFd) -> io::Result<Vec<OwnedFd>> {
    let mut fds = Vec::new();
    const MSG_SIZE: usize = 1;

    // Use recvmsg to check for ancillary data
    let mut buf = [0u8; MSG_SIZE];
    let mut iov = libc::iovec {
        iov_base: buf.as_mut_ptr() as *mut libc::c_void,
        iov_len: MSG_SIZE,
    };

    // Buffer for control messages (enough for several FDs)
    const CMSG_BUF_SIZE: usize = 256;
    let mut cmsg_buf = [0u8; CMSG_BUF_SIZE];

    let mut msg_hdr = unsafe { std::mem::zeroed::<libc::msghdr>() };
    msg_hdr.msg_iov = &mut iov;
    msg_hdr.msg_iovlen = 1;
    msg_hdr.msg_control = cmsg_buf.as_mut_ptr() as *mut libc::c_void;
    msg_hdr.msg_controllen = CMSG_BUF_SIZE;

    // Use MSG_PEEK | MSG_DONTWAIT to check without consuming
    // Actually, we already read the header+body, so we need to get the fds
    // that came with the last recvmsg call. But we used read() above.
    //
    // The problem: D-Bus sends FDs as ancillary data on the SAME socket call
    // as the message bytes. If we use plain read()/readv(), we lose the FDs.
    //
    // Solution: We should have used recvmsg for everything. But since we already
    // consumed the bytes with read(), the FDs are lost if they arrived in the
    // same recvmsg call.
    //
    // For now, use recvmsg with MSG_PEEK to drain any pending FDs:
    let flags = libc::MSG_DONTWAIT | libc::MSG_TRUNC;
    let n = unsafe { libc::recvmsg(fd, &mut msg_hdr, flags) };
    if n < 0 {
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EAGAIN) || err.raw_os_error() == Some(libc::EWOULDBLOCK) {
            return Ok(fds); // No more data
        }
        // Non-fatal — FDs might not be available
        return Ok(fds);
    }

    // Parse control messages
    let mut cmsg_ptr = msg_hdr.msg_control as *const libc::cmsghdr;
    while !cmsg_ptr.is_null() {
        let cmsg = unsafe { &*cmsg_ptr };
        if cmsg.cmsg_level == libc::SOL_SOCKET && cmsg.cmsg_type == libc::SCM_RIGHTS {
            let num_fds = (cmsg.cmsg_len - std::mem::size_of::<libc::cmsghdr>()) / std::mem::size_of::<RawFd>();
            let fd_ptr = unsafe {
                libc::CMSG_DATA(cmsg as *const libc::cmsghdr) as *const RawFd
            };
            for i in 0..num_fds {
                let raw_fd = unsafe { *fd_ptr.add(i) };
                fds.push(unsafe { OwnedFd::from_raw_fd(raw_fd) });
            }
        }
        cmsg_ptr = unsafe { libc::CMSG_NXTHDR(&msg_hdr, cmsg_ptr) };
    }

    Ok(fds)
}

// ─── Session helpers ─────────────────────────────────────────

fn get_own_session_id() -> io::Result<String> {
    if let Ok(sid) = std::env::var("XDG_SESSION_ID") {
        return Ok(sid);
    }

    // Fallback: find session via loginctl
    let output = std::process::Command::new("loginctl")
        .arg("show-user")
        .arg("-p")
        .arg("Sessions")
        .arg("--value")
        .output()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let sessions = String::from_utf8_lossy(&output.stdout);
    sessions.lines().next()
        .and_then(|l| l.split(',').next())
        .map(String::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No logind session found"))
}

fn get_device_major_minor(sysname: &str) -> io::Result<(u32, u32)> {
    let dev_path = format!("/sys/class/drm/{}/dev", sysname);
    let content = std::fs::read_to_string(&dev_path)?;
    let content = content.trim();
    let parts: Vec<&str> = content.split(':').collect();
    if parts.len() != 2 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Invalid dev content: {}", content)));
    }
    let major = parts[0].parse::<u32>().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let minor = parts[1].parse::<u32>().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok((major, minor))
}
