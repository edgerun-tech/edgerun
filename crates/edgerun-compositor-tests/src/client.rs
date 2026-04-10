//! Wayland test client — encodes requests and decodes responses.
//!
//! Provides a minimal Wayland client that can:
//! - Connect to a compositor socket
//! - Encode typed requests (sync, bind, create_surface, etc.)
//! - Decode events from the compositor
//! - Track object IDs and message queues

use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;

use edgerun_compositor::wire::{
    ArgType, ArgCursor, Message, parse_message,
    encode, encode_string, encode_array,
};
use edgerun_compositor::protocol::wl_core;
use edgerun_compositor::protocol::wl_compositor;
use edgerun_compositor::protocol::wl_shm;
use edgerun_compositor::protocol::wl_seat;
use edgerun_compositor::protocol::xdg_shell;

/// Next object ID to allocate on the client side.
/// wl_display = 1, so we start at 2.
const FIRST_AVAILABLE_ID: u32 = 2;

/// A decoded event from the compositor.
#[derive(Debug)]
pub struct Event {
    pub sender_id: u32,
    pub opcode: u16,
    /// Raw args (use [`EventCursor`] to decode).
    pub args: Vec<u8>,
    pub fds: Vec<i32>,
}

impl Event {
    pub fn cursor(&self) -> EventCursor {
        EventCursor(ArgCursor::from_args(&self.args, &self.fds))
    }
}

/// Cursor for decoding event arguments.
#[derive(Debug)]
pub struct EventCursor<'a>(ArgCursor<'a>);

impl<'a> EventCursor<'a> {
    pub fn uint(&mut self) -> io::Result<u32> {
        self.0.uint().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{:?}", e)))
    }
    pub fn int(&mut self) -> io::Result<i32> {
        self.0.int().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{:?}", e)))
    }
    pub fn object(&mut self) -> io::Result<u32> {
        self.0.object().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{:?}", e)))
    }
    pub fn string(&mut self) -> io::Result<Option<String>> {
        self.0.string().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{:?}", e)))
            .map(|s| s.map(|ws| ws.0))
    }
    pub fn array(&mut self) -> io::Result<Vec<u8>> {
        self.0.array().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{:?}", e)))
            .map(|arr| arr.0)
    }
}

/// A global advertised by the compositor.
#[derive(Debug, Clone)]
pub struct Global {
    pub name: u32,
    pub interface: String,
    pub version: u32,
}

/// A minimal Wayland test client.
///
/// Manages object ID allocation, message encoding, and event dispatch.
pub struct WlClient {
    stream: UnixStream,
    next_id: u32,
    /// Pending events received from the compositor.
    pub pending_events: Vec<Event>,
    /// Registry: globals discovered via wl_registry.
    pub globals: Vec<Global>,
}

impl WlClient {
    pub fn new(stream: UnixStream) -> Self {
        Self {
            stream,
            next_id: FIRST_AVAILABLE_ID,
            pending_events: Vec::new(),
            globals: Vec::new(),
        }
    }

    // ─── Object ID allocation ────────────────────────────────

    /// Allocate a new client-side object ID.
    pub fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    // ─── I/O ─────────────────────────────────────────────────

    /// Send a message to the compositor.
    pub fn send(&mut self, msg: &Message) -> io::Result<()> {
        let bytes = encode(msg);
        self.stream.write_all(&bytes)
    }

    /// Flush the write buffer.
    pub fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }

    /// Read pending events from the compositor. Returns the number of events read.
    pub fn dispatch(&mut self) -> io::Result<usize> {
        self.stream.set_nonblocking(true).ok();
        let mut buf = [0u8; 4096];
        let mut total = 0;

        loop {
            match self.stream.read(&mut buf) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    let mut consumed = 0;
                    while consumed < n {
                        match parse_message(&buf[consumed..n], vec![]) {
                            Ok(Some((msg, used))) => {
                                self.pending_events.push(Event {
                                    sender_id: msg.sender_id,
                                    opcode: msg.opcode,
                                    args: msg.args,
                                    fds: msg.fds,
                                });
                                consumed += used;
                                total += 1;
                            }
                            Ok(None) => break, // need more data
                            Err(_) => break,
                        }
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }

        self.stream.set_nonblocking(false).ok();
        Ok(total)
    }

    /// Wait for at least one event, dispatching until something arrives.
    pub fn wait_for_event(&mut self) -> io::Result<()> {
        while self.pending_events.is_empty() {
            let n = self.dispatch()?;
            if n == 0 {
                // Block until data arrives
                self.stream.set_nonblocking(false).ok();
                let mut buf = [0u8; 4096];
                let n = self.stream.read(&mut buf)?;
                self.stream.set_nonblocking(true).ok();
                let mut consumed = 0;
                while consumed < n {
                    match parse_message(&buf[consumed..n], vec![]) {
                        Ok(Some((msg, used))) => {
                            self.pending_events.push(Event {
                                sender_id: msg.sender_id,
                                opcode: msg.opcode,
                                args: msg.args,
                                fds: msg.fds,
                            });
                            consumed += used;
                        }
                        Ok(None) => break,
                        Err(_) => break,
                    }
                }
            }
        }
        Ok(())
    }

    /// Pop the first pending event.
    pub fn pop_event(&mut self) -> Option<Event> {
        if self.pending_events.is_empty() {
            return None;
        }
        Some(self.pending_events.remove(0))
    }

    /// Find the first event matching a predicate.
    pub fn find_event<F>(&mut self, pred: F) -> Option<Event>
    where
        F: Fn(&Event) -> bool,
    {
        if let Some(pos) = self.pending_events.iter().position(&pred) {
            Some(self.pending_events.remove(pos))
        } else {
            None
        }
    }

    /// Drain all pending events.
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.pending_events)
    }

    // ─── Protocol helpers ────────────────────────────────────

    /// wl_display.sync — create a callback and send a sync request.
    /// Returns the callback object ID.
    pub fn display_sync(&mut self) -> io::Result<u32> {
        let cb_id = self.alloc_id();
        let msg = Message {
            sender_id: 1, // wl_display
            opcode: wl_core::display_request::SYNC,
            size: 12, // 8 + 4
            args: cb_id.to_le_bytes().to_vec(),
            fds: Vec::new(),
        };
        self.send(&msg)?;
        self.flush()?;
        Ok(cb_id)
    }

    /// Create a wl_registry object. Returns the registry ID.
    pub fn display_get_registry(&mut self) -> io::Result<u32> {
        let reg_id = self.alloc_id();
        let msg = Message {
            sender_id: 1, // wl_display
            opcode: wl_core::display_request::GET_REGISTRY,
            size: 12,
            args: reg_id.to_le_bytes().to_vec(),
            fds: Vec::new(),
        };
        self.send(&msg)?;
        self.flush()?;
        Ok(reg_id)
    }

    /// Read all globals from the compositor.
    pub fn read_globals(&mut self) -> io::Result<Vec<Global>> {
        let reg_id = self.display_get_registry()?;
        self.wait_for_event()?;

        let mut globals = Vec::new();
        loop {
            let evt = match self.pop_event() {
                Some(e) => e,
                None => break,
            };
            if evt.sender_id == reg_id && evt.opcode == wl_core::registry_event::GLOBAL {
                let mut c = evt.cursor();
                let name = c.uint()?;
                let iface = c.string()?.unwrap_or_default();
                let version = c.uint()?;
                globals.push(Global { name, interface: iface, version });
            }
        }
        self.globals = globals.clone();
        Ok(globals)
    }

    /// Bind a global by interface, sending the request to the given registry ID.
    /// Returns the newly created object ID.
    pub fn bind_with_registry(&mut self, registry_id: u32, global_name: u32,
                              interface: &str, desired_version: u32) -> io::Result<u32> {
        let bound_id = self.alloc_id();
        let ver = desired_version;

        let mut args = Vec::new();
        args.extend_from_slice(&global_name.to_le_bytes());
        encode_string(&mut args, interface);
        args.extend_from_slice(&ver.to_le_bytes());
        args.extend_from_slice(&bound_id.to_le_bytes());

        let msg = Message {
            sender_id: registry_id,
            opcode: wl_core::registry_request::BIND,
            size: (8 + args.len()) as u16,
            args,
            fds: Vec::new(),
        };
        self.send(&msg)?;
        self.flush()?;
        Ok(bound_id)
    }
}

/// Extended client with protocol-specific helpers.
impl WlClient {
    /// Complete the full registry binding flow: get registry, read globals,
    /// and return (registry_id, globals).
    pub fn full_registry(&mut self) -> io::Result<(u32, Vec<Global>)> {
        let reg_id = self.display_get_registry()?;

        // Give compositor time to process get_registry and send global events
        std::thread::sleep(std::time::Duration::from_millis(200));

        // First, drain all pending events into globals
        let mut globals = Vec::new();
        while let Some(evt) = self.pending_events.first() {
            if evt.sender_id == reg_id && evt.opcode == wl_core::registry_event::GLOBAL {
                let evt = self.pending_events.remove(0);
                let mut c = evt.cursor();
                let name = c.uint()?;
                let iface = c.string()?.unwrap_or_default();
                let version = c.uint()?;
                globals.push(Global { name, interface: iface, version });
            } else {
                break;
            }
        }

        // Now read more from socket
        for _iteration in 0..20 {
            // Try to read more from socket
            self.stream.set_nonblocking(true).ok();
            let n = self.dispatch()?;
            self.stream.set_nonblocking(false).ok();

            // Process newly received events
            while let Some(evt) = self.pending_events.first() {
                if evt.sender_id == reg_id && evt.opcode == wl_core::registry_event::GLOBAL {
                    let evt = self.pending_events.remove(0);
                    let mut c = evt.cursor();
                    let name = c.uint()?;
                    let iface = c.string()?.unwrap_or_default();
                    let version = c.uint()?;
                    globals.push(Global { name, interface: iface, version });
                } else {
                    break;
                }
            }

            if n == 0 {
                std::thread::sleep(std::time::Duration::from_millis(20));
                // Try one more read
                self.stream.set_nonblocking(true).ok();
                let n2 = self.dispatch()?;
                self.stream.set_nonblocking(false).ok();
                if n2 == 0 {
                    break;
                }
            }
        }

        // Process any remaining pending events
        while let Some(evt) = self.pending_events.first() {
            if evt.sender_id == reg_id && evt.opcode == wl_core::registry_event::GLOBAL {
                let evt = self.pending_events.remove(0);
                let mut c = evt.cursor();
                let name = c.uint()?;
                let iface = c.string()?.unwrap_or_default();
                let version = c.uint()?;
                globals.push(Global { name, interface: iface, version });
            } else {
                break;
            }
        }

        self.globals.clone_from(&globals);
        eprintln!("[client] Registry: found {} globals: {:?}", globals.len(),
            globals.iter().map(|g| &g.interface).collect::<Vec<_>>());
        Ok((reg_id, globals))
    }

    /// Bind and create a wl_shm pool.
    /// Returns (shm_id, pool_id, fd, buffer_id).
    pub fn create_shm_buffer(&mut self, registry_id: u32,
                              width: u32, height: u32) -> io::Result<(u32, u32, i32, u32)> {
        // Find wl_shm global
        let shm_global = self.globals.iter()
            .find(|g| g.interface == wl_shm::WL_SHM)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "wl_shm not advertised"))?;

        let shm_id = self.bind_with_registry(registry_id, shm_global.name,
                                              wl_shm::WL_SHM, shm_global.version)?;

        // Create an anonymous file for SHM
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::CString::new("edgerun-test-shm").unwrap();
        let fd = unsafe { libc::memfd_create(name.as_ptr(), 0) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        let stride = width * 4; // ARGB8888
        let size = stride * height;
        if unsafe { libc::ftruncate(fd, size as libc::off_t) } < 0 {
            unsafe { libc::close(fd) };
            return Err(io::Error::last_os_error());
        }

        let pool_id = self.alloc_id();
        let mut args = Vec::new();
        args.extend_from_slice(&pool_id.to_le_bytes());
        args.extend_from_slice(&[0u8; 4]); // fd placeholder
        args.extend_from_slice(&(size as i32).to_le_bytes());

        let mut msg = Message {
            sender_id: shm_id,
            opcode: wl_shm::shm_request::CREATE_POOL,
            size: (8 + args.len()) as u16,
            args,
            fds: vec![fd],
        };
        self.send(&msg)?;
        self.flush()?;

        // Create buffer from pool
        let buffer_id = self.alloc_id();
        let mut args = Vec::new();
        args.extend_from_slice(&buffer_id.to_le_bytes());
        args.extend_from_slice(&0i32.to_le_bytes()); // offset
        args.extend_from_slice(&(width as i32).to_le_bytes());
        args.extend_from_slice(&(height as i32).to_le_bytes());
        args.extend_from_slice(&(stride as i32).to_le_bytes());
        args.extend_from_slice(&wl_shm::format::XRGB8888.to_le_bytes());

        let msg = Message {
            sender_id: pool_id,
            opcode: wl_shm::shm_pool_request::CREATE_BUFFER,
            size: (8 + args.len()) as u16,
            args,
            fds: Vec::new(),
        };
        self.send(&msg)?;
        self.flush()?;

        Ok((shm_id, pool_id, fd, buffer_id))
    }

    /// Create a wl_surface.
    pub fn create_surface(&mut self, registry_id: u32) -> io::Result<(u32, u32)> {
        let compositor_global = self.globals.iter()
            .find(|g| g.interface == wl_compositor::WL_COMPOSITOR)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "wl_compositor not advertised"))?;

        let compositor_id = self.bind_with_registry(registry_id, compositor_global.name,
                                                     wl_compositor::WL_COMPOSITOR,
                                                     compositor_global.version)?;
        let surface_id = self.alloc_id();
        let msg = Message {
            sender_id: compositor_id,
            opcode: wl_compositor::compositor_request::CREATE_SURFACE,
            size: 12,
            args: surface_id.to_le_bytes().to_vec(),
            fds: Vec::new(),
        };
        self.send(&msg)?;
        self.flush()?;
        Ok((compositor_id, surface_id))
    }

    /// Commit a surface.
    pub fn surface_commit(&mut self, surface_id: u32) -> io::Result<()> {
        let msg = Message {
            sender_id: surface_id,
            opcode: wl_compositor::surface_request::COMMIT,
            size: 8,
            args: Vec::new(),
            fds: Vec::new(),
        };
        self.send(&msg)?;
        self.flush()
    }

    /// Attach a buffer to a surface.
    pub fn surface_attach(&mut self, surface_id: u32, buffer_id: u32) -> io::Result<()> {
        let mut args = Vec::new();
        args.extend_from_slice(&buffer_id.to_le_bytes());
        args.extend_from_slice(&0i32.to_le_bytes());
        args.extend_from_slice(&0i32.to_le_bytes());
        let msg = Message {
            sender_id: surface_id,
            opcode: wl_compositor::surface_request::ATTACH,
            size: (8 + args.len()) as u16,
            args,
            fds: Vec::new(),
        };
        self.send(&msg)?;
        self.flush()
    }

    /// Request a frame callback.
    pub fn surface_frame(&mut self, surface_id: u32) -> io::Result<u32> {
        let cb_id = self.alloc_id();
        let msg = Message {
            sender_id: surface_id,
            opcode: wl_compositor::surface_request::FRAME,
            size: 12,
            args: cb_id.to_le_bytes().to_vec(),
            fds: Vec::new(),
        };
        self.send(&msg)?;
        self.flush()?;
        Ok(cb_id)
    }

    /// Full xdg-shell setup: bind xdg_wm_base, create xdg_surface + xdg_toplevel.
    /// Returns (wm_base_id, xdg_surface_id, toplevel_id).
    pub fn create_xdg_toplevel(&mut self, registry_id: u32, surface_id: u32)
                               -> io::Result<(u32, u32, u32)> {
        let xdg_global = self.globals.iter()
            .find(|g| g.interface == xdg_shell::XDG_WM_BASE)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "xdg_wm_base not advertised"))?;

        let wm_base_id = self.bind_with_registry(registry_id, xdg_global.name,
                                                  xdg_shell::XDG_WM_BASE, xdg_global.version)?;

        let xdg_surface_id = self.alloc_id();
        let mut args = Vec::new();
        args.extend_from_slice(&xdg_surface_id.to_le_bytes());
        args.extend_from_slice(&surface_id.to_le_bytes());
        let msg = Message {
            sender_id: wm_base_id,
            opcode: xdg_shell::xdg_wm_base_request::GET_XDG_SURFACE,
            size: (8 + args.len()) as u16,
            args,
            fds: Vec::new(),
        };
        self.send(&msg)?;

        let toplevel_id = self.alloc_id();
        let msg = Message {
            sender_id: xdg_surface_id,
            opcode: xdg_shell::xdg_surface_request::GET_TOPLEVEL,
            size: 12,
            args: toplevel_id.to_le_bytes().to_vec(),
            fds: Vec::new(),
        };
        self.send(&msg)?;
        self.flush()?;

        Ok((wm_base_id, xdg_surface_id, toplevel_id))
    }

    /// Ack an xdg_surface configure.
    pub fn xdg_surface_ack_configure(&mut self, xdg_surface_id: u32, serial: u32) -> io::Result<()> {
        let msg = Message {
            sender_id: xdg_surface_id,
            opcode: xdg_shell::xdg_surface_request::ACK_CONFIGURE,
            size: 12,
            args: serial.to_le_bytes().to_vec(),
            fds: Vec::new(),
        };
        self.send(&msg)?;
        self.flush()
    }

    /// Get a wl_keyboard from the seat.
    pub fn seat_get_keyboard(&mut self, registry_id: u32) -> io::Result<Option<u32>> {
        let seat_global = match self.globals.iter().find(|g| g.interface == wl_seat::WL_SEAT) {
            Some(g) => g,
            None => return Ok(None),
        };
        let seat_id = self.bind_with_registry(registry_id, seat_global.name,
                                               wl_seat::WL_SEAT, seat_global.version)?;
        let kb_id = self.alloc_id();
        let msg = Message {
            sender_id: seat_id,
            opcode: wl_seat::seat_request::GET_KEYBOARD,
            size: 12,
            args: kb_id.to_le_bytes().to_vec(),
            fds: Vec::new(),
        };
        self.send(&msg)?;
        self.flush()?;
        Some(Ok(kb_id)).transpose()
    }

    /// Get a wl_pointer from the seat.
    pub fn seat_get_pointer(&mut self, registry_id: u32) -> io::Result<Option<u32>> {
        let seat_global = match self.globals.iter().find(|g| g.interface == wl_seat::WL_SEAT) {
            Some(g) => g,
            None => return Ok(None),
        };
        let seat_id = self.bind_with_registry(registry_id, seat_global.name,
                                               wl_seat::WL_SEAT, seat_global.version)?;
        let ptr_id = self.alloc_id();
        let msg = Message {
            sender_id: seat_id,
            opcode: wl_seat::seat_request::GET_POINTER,
            size: 12,
            args: ptr_id.to_le_bytes().to_vec(),
            fds: Vec::new(),
        };
        self.send(&msg)?;
        self.flush()?;
        Some(Ok(ptr_id)).transpose()
    }

    /// Pong to xdg_wm_base.
    pub fn xdg_wm_base_pong(&mut self, wm_base_id: u32, serial: u32) -> io::Result<()> {
        let msg = Message {
            sender_id: wm_base_id,
            opcode: xdg_shell::xdg_wm_base_request::PONG,
            size: 12,
            args: serial.to_le_bytes().to_vec(),
            fds: Vec::new(),
        };
        self.send(&msg)?;
        self.flush()
    }
}
