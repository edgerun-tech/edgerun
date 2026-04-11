//! Virtual Terminal management for standalone compositor operation.
//!
//! No external crates — raw ioctl calls to the Linux VT subsystem.

use std::io;
use std::os::fd::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};

/// VT ioctl commands
const VT_OPENQRY: libc::c_ulong = 0x5600;
const VT_ACTIVATE: libc::c_ulong = 0x5606;
const VT_WAITACTIVE: libc::c_ulong = 0x5607;
const VT_SETMODE: libc::c_ulong = 0x5602;
const VT_GETMODE: libc::c_ulong = 0x5603;
const VT_RELDISP: libc::c_ulong = 0x5605;
const KDSETMODE: libc::c_ulong = 0x4B3A;
const KDGETMODE: libc::c_ulong = 0x4B3B;

const VT_AUTO: libc::c_char = 0;
const VT_PROCESS: libc::c_char = 1;

const KD_TEXT: libc::c_int = 0;
const KD_GRAPHICS: libc::c_int = 1;

/// VT release signal — sent when the VT is being released to another session.
#[derive(Debug, Clone, Copy)]
pub enum VtEvent {
    /// The kernel wants us to release the display (e.g., user pressed Ctrl+Alt+Fn).
    Release,
    /// The display has been re-acquired after a release.
    Acquire,
}

/// A callback invoked on VT events.
pub type VtCallback = Box<dyn FnMut(VtEvent) + Send + 'static>;

/// VT state — manages the virtual terminal for the compositor.
pub struct VtManager {
    vt_fd: RawFd,
    vt_num: i32,
    /// Whether we've set VT_PROCESS mode.
    vt_process_mode: bool,
    /// Flag set by signal handler on VT release.
    vt_release: &'static AtomicBool,
    /// Flag set by signal handler on VT acquire.
    vt_acquire: &'static AtomicBool,
}

// Static atomics for signal handlers
static VT_RELEASE_FLAG: AtomicBool = AtomicBool::new(false);
static VT_ACQUIRE_FLAG: AtomicBool = AtomicBool::new(false);

extern "C" fn vt_release_signal(_sig: libc::c_int) {
    VT_RELEASE_FLAG.store(true, Ordering::SeqCst);
}

extern "C" fn vt_acquire_signal(_sig: libc::c_int) {
    VT_ACQUIRE_FLAG.store(true, Ordering::SeqCst);
}

impl VtManager {
    /// Open a VT and switch to it.
    ///
    /// If `vt_num` is 0, finds an available VT automatically.
    /// Prefer using the current VT (from `ioctl(TIOCGVTN)`) or letting the
    /// system allocate one.
    pub fn open(vt_num: i32) -> io::Result<Self> {
        // Open /dev/tty0 (the current VT) or the specific VT
        let vt_path = if vt_num > 0 {
            format!("/dev/tty{}", vt_num)
        } else {
            "/dev/tty0".to_string()
        };

        let c_path = std::ffi::CString::new(vt_path.as_str()).unwrap();
        let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDWR | libc::O_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        // Determine actual VT number
        let actual_vt = if vt_num > 0 {
            vt_num
        } else {
            // Query for an available VT
            let mut query_vt: libc::c_int = 0;
            let ret = unsafe { libc::ioctl(fd, VT_OPENQRY as _, &mut query_vt) };
            if ret < 0 || query_vt <= 0 {
                unsafe { libc::close(fd) };
                return Err(io::Error::new(io::ErrorKind::Other, "No available VT"));
            }
            query_vt
        };

        eprintln!("[vt] Opening VT{} (path: {})", actual_vt, vt_path);

        // Set KD mode to GRAPHICS (disables text rendering)
        let ret = unsafe { libc::ioctl(fd, KDSETMODE as _, KD_GRAPHICS) };
        if ret < 0 {
            let err = io::Error::last_os_error();
            eprintln!("[vt] KDSETMODE KD_GRAPHICS failed: {}", err);
            unsafe { libc::close(fd) };
            return Err(err);
        }

        // Set VT processing mode
        let mut mode = VtMode {
            mode: VT_PROCESS,
            waitvto: 0,
            relsig: libc::SIGUSR1 as libc::c_ushort,
            acqsig: libc::SIGUSR2 as libc::c_ushort,
            frsig: 0,
        };

        let ret = unsafe { libc::ioctl(fd, VT_SETMODE as _, &mut mode) };
        if ret < 0 {
            let err = io::Error::last_os_error();
            eprintln!("[vt] VT_SETMODE failed: {}", err);
            // Try to restore KD_TEXT before failing
            let _ = unsafe { libc::ioctl(fd, KDSETMODE as _, KD_TEXT) };
            unsafe { libc::close(fd) };
            return Err(err);
        }

        // Install signal handlers for VT release/acquire using the simpler signal() API
        let prev = unsafe { libc::signal(libc::SIGUSR1, vt_release_signal as *const () as libc::sighandler_t) };
        if prev == libc::SIG_ERR {
            eprintln!("[vt] Failed to install SIGUSR1 handler");
        }
        let prev = unsafe { libc::signal(libc::SIGUSR2, vt_acquire_signal as *const () as libc::sighandler_t) };
        if prev == libc::SIG_ERR {
            eprintln!("[vt] Failed to install SIGUSR2 handler");
        }

        // Activate the VT
        if actual_vt > 0 {
            let ret = unsafe { libc::ioctl(fd, VT_ACTIVATE as _, actual_vt) };
            if ret < 0 {
                eprintln!("[vt] VT_ACTIVATE failed: {}", io::Error::last_os_error());
            }

            // Wait for it to become active
            let ret = unsafe { libc::ioctl(fd, VT_WAITACTIVE as _, actual_vt) };
            if ret < 0 {
                eprintln!("[vt] VT_WAITACTIVE failed: {}", io::Error::last_os_error());
            }
        }

        eprintln!("[vt] VT{} active, mode: GRAPHICS", actual_vt);

        Ok(Self {
            vt_fd: fd,
            vt_num: actual_vt,
            vt_process_mode: true,
            vt_release: &VT_RELEASE_FLAG,
            vt_acquire: &VT_ACQUIRE_FLAG,
        })
    }

    /// Check for pending VT events (non-blocking).
    /// Call this every event loop iteration.
    pub fn poll_events(&mut self, mut callback: impl FnMut(VtEvent)) {
        if self.vt_release.swap(false, Ordering::SeqCst) {
            // VT release requested by kernel
            eprintln!("[vt] Release requested — acknowledging");
            callback(VtEvent::Release);
            // Acknowledge the release
            let _ = unsafe { libc::ioctl(self.vt_fd, VT_RELDISP as _, 1) };
        }

        if self.vt_acquire.swap(false, Ordering::SeqCst) {
            // VT re-acquired after release
            eprintln!("[vt] Re-acquired");
            callback(VtEvent::Acquire);
        }
    }

    /// Get the VT number we're running on.
    pub fn vt_num(&self) -> i32 {
        self.vt_num
    }

    /// Get the VT fd (for epoll registration).
    pub fn as_raw_fd(&self) -> RawFd {
        self.vt_fd
    }
}

impl Drop for VtManager {
    fn drop(&mut self) {
        // Restore VT to text mode
        if self.vt_fd >= 0 {
            // Reset VT mode to auto
            if self.vt_process_mode {
                let mut mode = VtMode {
                    mode: VT_AUTO,
                    waitvto: 0,
                    relsig: 0,
                    acqsig: 0,
                    frsig: 0,
                };
                let _ = unsafe { libc::ioctl(self.vt_fd, VT_SETMODE as _, &mut mode) };
            }

            // Restore KD mode to text
            let _ = unsafe { libc::ioctl(self.vt_fd, KDSETMODE as _, KD_TEXT) };

            eprintln!("[vt] VT{} restored to text mode", self.vt_num);

            // Don't close the fd if it's the current tty0 — just restore mode
            // Actually close if we opened a specific VT
            unsafe { libc::close(self.vt_fd) };
        }
    }
}

/// VT mode structure (matches kernel vt_mode).
#[repr(C)]
struct VtMode {
    mode: libc::c_char,
    waitvto: libc::c_char,
    relsig: libc::c_ushort,
    acqsig: libc::c_ushort,
    frsig: libc::c_ushort,
}
