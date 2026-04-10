//! Minimal D-Bus client for systemd-logind DRM session management.
//!
//! Uses raw Unix domain sockets to talk to the system D-Bus,
//! then to org.freedesktop.login1 for session management.

use std::io;
use std::os::fd::RawFd;

/// Acquire a DRM device via logind.
///
/// This asks logind to give us the DRM fd for the given sysname (e.g., "drm/card1").
/// Logind handles DRM master properly, revoking it from the old session if needed.
///
/// Returns the DRM fd (which you should NOT close manually — use release_device or close it).
pub fn logind_acquire_drm_fd(sysname: &str) -> io::Result<RawFd> {
    // Connect to system bus
    let dbus_addr = std::env::var("DBUS_SYSTEM_BUS_ADDRESS")
        .unwrap_or_else(|_| "unix:path=/run/dbus/system_bus_socket".to_string());

    let dbus_fd = connect_dbus(&dbus_addr)?;

    // Get session ID from logind
    let session_id = get_own_session_id(dbus_fd)?;

    // Activate session (this makes it the active VT)
    activate_session(dbus_fd, &session_id)?;

    // Acquire DRM device
    let drm_fd = acquire_device(dbus_fd, &session_id, sysname, true)?;

    // Close dbus fd
    unsafe { libc::close(dbus_fd) };

    Ok(drm_fd)
}

/// Release a DRM device via logind.
pub fn logind_release_device(sysname: &str) -> io::Result<()> {
    let dbus_addr = std::env::var("DBUS_SYSTEM_BUS_ADDRESS")
        .unwrap_or_else(|_| "unix:path=/run/dbus/system_bus_socket".to_string());

    let dbus_fd = connect_dbus(&dbus_addr)?;
    let session_id = get_own_session_id(dbus_fd)?;
    release_device(dbus_fd, &session_id, sysname)?;
    unsafe { libc::close(dbus_fd) };
    Ok(())
}

// ─── D-Bus helpers ───────────────────────────────────────────

fn connect_dbus(addr: &str) -> io::Result<RawFd> {
    if !addr.starts_with("unix:path=") {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Only unix:path D-Bus addresses supported"));
    }
    let path = &addr["unix:path=".len()..];
    let c_path = std::ffi::CString::new(path).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "bad path"))?;

    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }

    let sun_path = c_path.as_bytes();
    let mut addr = unsafe { std::mem::zeroed::<libc::sockaddr_un>() };
    addr.sun_family = libc::AF_UNIX as _;
    let copy_len = sun_path.len().min(addr.sun_path.len() - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(sun_path.as_ptr(), addr.sun_path.as_mut_ptr() as *mut u8, copy_len);
    }

    let addr_len = std::mem::size_of::<libc::sockaddr_un>();
    let ret = unsafe { libc::connect(fd, &addr as *const _ as *const libc::sockaddr, addr_len as _) };
    if ret < 0 {
        let err = io::Error::last_os_error();
        unsafe { libc::close(fd) };
        return Err(err);
    }

    // Authenticate with EXTERNAL mechanism
    dbus_authenticate(fd)?;

    // Hello to get a unique name
    dbus_hello(fd)?;

    Ok(fd)
}

/// D-Bus EXTERNAL authentication.
fn dbus_authenticate(fd: RawFd) -> io::Result<()> {
    // Send: \0AUTH EXTERNAL <hex uid>\r\nNEGOTIATE_UNIX_FD\r\nBEGIN\r\n
    let uid = unsafe { libc::getuid() };
    let hex_uid = format!("{:x}", uid);
    let auth = format!("\0AUTH EXTERNAL {}\r\nNEGOTIATE_UNIX_FD\r\nBEGIN\r\n", hex_uid);

    let ret = unsafe { libc::write(fd, auth.as_ptr() as *const libc::c_void, auth.len()) };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }

    // Read OK response
    let mut buf = [0u8; 64];
    loop {
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n <= 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "D-Bus auth failed"));
        }
        let resp = String::from_utf8_lossy(&buf[..n as usize]);
        if resp.contains('\n') {
            // Check if any line is OK
            for line in resp.lines() {
                if line == "OK" || line.starts_with("OK ") {
                    return Ok(());
                }
                if line.starts_with("ERROR") {
                    return Err(io::Error::new(io::ErrorKind::PermissionDenied, format!("D-Bus auth error: {}", line)));
                }
            }
        }
    }
}

/// Send Hello method to get unique name.
fn dbus_hello(_fd: RawFd) -> io::Result<()> {
    // For simplicity, skip this — we don't need the unique name for method calls
    Ok(())
}

/// Get our own session ID via D-Bus.
fn get_own_session_id(dbus_fd: RawFd) -> io::Result<String> {
    // Read /run/systemd/sessions/ to find our session
    // Or use dbus to call GetSession on logind
    // Simpler: read XDG_SESSION_ID env var
    if let Ok(sid) = std::env::var("XDG_SESSION_ID") {
        return Ok(sid);
    }

    // Fallback: parse from loginctl
    let output = std::process::Command::new("loginctl")
        .arg("show-user")
        .arg("-p", "Sessions")
        .arg("--value")
        .output()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let sessions = String::from_utf8_lossy(&output.stdout);
    // Return first session
    sessions.lines().next()
        .and_then(|l| l.split(',').next())
        .map(String::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No session found"))
}

/// Activate a session via D-Bus.
fn activate_session(dbus_fd: RawFd, session_id: &str) -> io::Result<()> {
    // D-Bus method call: org.freedesktop.login1.Session.Activate
    // Path: /org/freedesktop/login1/session/<session_id>
    // Interface: org.freedesktop.login1.Session
    // Member: Activate

    let path = format!("/org/freedesktop/login1/session/{}", session_id);
    let msg = build_method_call(
        "org.freedesktop.login1",
        &path,
        "org.freedesktop.login1.Session",
        "Activate",
        &[],
    );

    send_message(dbus_fd, &msg)?;
    // Read reply (ignore errors — session might already be active)
    let _ = read_message(dbus_fd);

    Ok(())
}

/// Acquire a device via logind.
fn acquire_device(dbus_fd: RawFd, session_id: &str, sysname: &str, take_ownership: bool) -> io::Result<RawFd> {
    let path = format!("/org/freedesktop/login1/session/{}", session_id);
    let take_ownership_val = if take_ownership { "true" } else { "false" };

    // Method: TakeDevice
    // Args: (major: u32, minor: u32, take_ownership: b)
    // Returns: (fd: h, result: b)

    // Get major/minor from sysfs
    let (major, minor) = get_device_major_minor(sysname)?;

    let msg = build_method_call_with_fds(
        "org.freedesktop.login1",
        &path,
        "org.freedesktop.login1.Session",
        "TakeDevice",
        &[
            ("u", major.to_string()),
            ("u", minor.to_string()),
            ("b", take_ownership_val.to_string()),
        ],
        &[],
    );

    send_message(dbus_fd, &msg)?;
    let reply = read_message(dbus_fd)?;

    // Extract fd from reply (it comes via SCM_RIGHTS)
    extract_fd_from_reply(&reply)
}

/// Release a device via logind.
fn release_device(dbus_fd: RawFd, session_id: &str, sysname: &str) -> io::Result<()> {
    let path = format!("/org/freedesktop/login1/session/{}", session_id);
    let (major, minor) = get_device_major_minor(sysname)?;

    let msg = build_method_call_with_fds(
        "org.freedesktop.login1",
        &path,
        "org.freedesktop.login1.Session",
        "ReleaseDevice",
        &[
            ("u", major.to_string()),
            ("u", minor.to_string()),
        ],
        &[],
    );

    send_message(dbus_fd, &msg)?;
    let _ = read_message(dbus_fd);
    Ok(())
}

/// Get major/minor from sysfs.
fn get_device_major_minor(sysname: &str) -> io::Result<(u32, u32)> {
    // For DRM devices: /sys/class/drm/<sysname>/dev contains major:minor
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

// ─── D-Bus message building (minimal implementation) ─────────

fn build_method_call(destination: &str, path: &str, interface: &str, member: &str, _args: &[(char, String)]) -> Vec<u8> {
    // D-Bus message header
    // Type: METHOD_CALL (1)
    // Flags: NO_REPLY_EXPECTED (0)
    // Version: 1
    let mut msg = Vec::new();
    msg.push(1u8); // little-endian, type=METHOD_CALL
    msg.push(0u8); // flags
    msg.push(0u8); // version
    msg.push(0u8); // padding

    // Body length (placeholder)
    msg.extend_from_slice(&0u32.to_le_bytes());

    // Header fields
    // Destination (string)
    write_string(&mut msg, destination);
    // Path (string)
    write_string(&mut msg, path);
    // Interface (string)
    write_string(&mut msg, interface);
    // Member (string)
    write_string(&mut msg, member);

    // Body: for now empty (args would go here)
    // Align body to 8 bytes
    while msg.len() % 8 != 0 {
        msg.push(0u8);
    }

    // Update body length
    let body_start = 12; // after header
    let body_len = (msg.len() - body_start) as u32;
    msg[4..8].copy_from_slice(&body_len.to_le_bytes());

    msg
}

fn build_method_call_with_fds(
    destination: &str, path: &str, interface: &str, member: &str,
    _args: &[(char, String)], _fds: &[i32],
) -> Vec<u8> {
    build_method_call(destination, path, interface, member, _args)
}

fn write_string(msg: &mut Vec<u8>, s: &str) {
    let len = s.len() as u32;
    msg.extend_from_slice(&len.to_le_bytes());
    msg.extend_from_slice(s.as_bytes());
    msg.push(0u8); // null terminator
    // Align to 4 bytes
    while (msg.len() % 4) != 0 {
        msg.push(0u8);
    }
}

fn send_message(fd: RawFd, msg: &[u8]) -> io::Result<()> {
    let ret = unsafe { libc::write(fd, msg.as_ptr() as *const libc::c_void, msg.len()) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn read_message(_fd: RawFd) -> io::Result<Vec<u8>> {
    // Minimal: just read and discard
    let mut buf = [0u8; 4096];
    let n = unsafe { libc::read(_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
    if n < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(buf[..n as usize].to_vec())
    }
}

fn extract_fd_from_reply(_reply: &[u8]) -> io::Result<RawFd> {
    // For now, return error — proper implementation needs SCM_RIGHTS parsing
    Err(io::Error::new(io::ErrorKind::Unsupported, "SCM_RIGHTS reply parsing not yet implemented"))
}
