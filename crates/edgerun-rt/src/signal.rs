//! Unix signal handling via `signalfd`.
//!
//! Provides async-aware signal handling without race conditions.
//! Signals are masked from their default disposition and delivered
//! through a file descriptor that can be awaited.
//!
//! # Example
//! ```ignore
//! use edgerun_rt::signal::unix::{signal, SignalKind};
//!
//! let mut sigterm = signal(SignalKind::terminate()).unwrap();
//! sigterm.recv().await;
//! ```

use std::future::Future;
use std::io::{self};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::io_traits::AsyncRead;

// ===========================================================================
// SignalKind
// ===========================================================================

/// A Unix signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalKind(libc::c_int);

impl SignalKind {
    /// SIGHUP — terminal line hangup
    pub const fn hangup() -> Self {
        Self(libc::SIGHUP)
    }
    /// SIGINT — interrupt from keyboard
    pub const fn interrupt() -> Self {
        Self(libc::SIGINT)
    }
    /// SIGQUIT — quit from keyboard
    pub const fn quit() -> Self {
        Self(libc::SIGQUIT)
    }
    /// SIGILL — illegal instruction
    pub const fn illegal_instruction() -> Self {
        Self(libc::SIGILL)
    }
    /// SIGTRAP — trace/breakpoint trap
    pub const fn trap() -> Self {
        Self(libc::SIGTRAP)
    }
    /// SIGABRT — abort signal
    pub const fn abort() -> Self {
        Self(libc::SIGABRT)
    }
    /// SIGBUS — bus error
    pub const fn bus() -> Self {
        Self(libc::SIGBUS)
    }
    /// SIGFPE — floating-point exception
    pub const fn floating_point_exception() -> Self {
        Self(libc::SIGFPE)
    }
    /// SIGKILL — kill (cannot be caught or ignored)
    pub const fn kill() -> Self {
        Self(libc::SIGKILL)
    }
    /// SIGUSR1 — user-defined signal 1
    pub const fn user_defined1() -> Self {
        Self(libc::SIGUSR1)
    }
    /// SIGSEGV — segmentation violation
    pub const fn segmentation_violation() -> Self {
        Self(libc::SIGSEGV)
    }
    /// SIGUSR2 — user-defined signal 2
    pub const fn user_defined2() -> Self {
        Self(libc::SIGUSR2)
    }
    /// SIGPIPE — write on a pipe with no reader
    pub const fn pipe() -> Self {
        Self(libc::SIGPIPE)
    }
    /// SIGALRM — timer signal from alarm
    pub const fn alarm() -> Self {
        Self(libc::SIGALRM)
    }
    /// SIGTERM — termination signal
    pub const fn terminate() -> Self {
        Self(libc::SIGTERM)
    }
    /// SIGCHLD — child process status change
    pub const fn child() -> Self {
        Self(libc::SIGCHLD)
    }
    /// SIGCONT — continue if stopped
    pub const fn r#continue() -> Self {
        Self(libc::SIGCONT)
    }
    /// SIGSTOP — stop (cannot be caught or ignored)
    pub const fn stop() -> Self {
        Self(libc::SIGSTOP)
    }
    /// SIGTSTP — stop from keyboard
    pub const fn tty_stop() -> Self {
        Self(libc::SIGTSTP)
    }
    /// SIGTTIN — background read from tty
    pub const fn tty_in() -> Self {
        Self(libc::SIGTTIN)
    }
    /// SIGTTOU — background write to tty
    pub const fn tty_out() -> Self {
        Self(libc::SIGTTOU)
    }
    /// SIGURG — urgent condition on socket
    pub const fn urgent() -> Self {
        Self(libc::SIGURG)
    }
    /// SIGXCPU — CPU time limit exceeded
    pub const fn cpu_limit_exceeded() -> Self {
        Self(libc::SIGXCPU)
    }
    /// SIGXFSZ — file size limit exceeded
    pub const fn file_size_limit_exceeded() -> Self {
        Self(libc::SIGXFSZ)
    }
    /// SIGVTALRM — virtual timer expired
    pub const fn virtual_timer_expired() -> Self {
        Self(libc::SIGVTALRM)
    }
    /// SIGPROF — profiling timer expired
    pub const fn profile() -> Self {
        Self(libc::SIGPROF)
    }
    /// SIGWINCH — window resize
    pub const fn winch() -> Self {
        Self(libc::SIGWINCH)
    }
    /// SIGIO — I/O is possible
    pub const fn io() -> Self {
        Self(libc::SIGIO)
    }
    /// SIGPWR — power failure
    pub const fn power() -> Self {
        Self(libc::SIGPWR)
    }
    /// SIGSYS — bad system call
    pub const fn bad_system_call() -> Self {
        Self(libc::SIGSYS)
    }

    /// Raw signal number
    pub const fn as_raw(self) -> libc::c_int {
        self.0
    }
}

// ===========================================================================
// Signal
// ===========================================================================

/// A stream of Unix signals of a specific kind.
///
/// Created via [`signal()`] or [`Signal::new()`].
/// Signals are delivered via `signalfd(2)`, which integrates cleanly
/// with the async reactor.
pub struct Signal {
    /// File descriptor from signalfd. Owned — we close it in Drop.
    fd: std::sync::Arc<std::sync::atomic::AtomicI32>,
    read_waker: crate::sync::Mutex<Option<std::task::Waker>>,
    refs: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl Signal {
    /// Create a new signal stream for the given signal kind.
    ///
    /// This masks the signal from its default handler and installs
    /// a signalfd that delivers `signalfd_siginfo` structs.
    pub fn new(kind: SignalKind) -> io::Result<Self> {
        Self::new_with_kind(kind)
    }

    /// Create from a raw signal number.
    pub fn from_raw(signal: libc::c_int) -> io::Result<Self> {
        Self::new_with_kind(SignalKind(signal))
    }

    fn new_with_kind(kind: SignalKind) -> io::Result<Self> {
        // Block the signal so it doesn't get delivered by the default handler.
        // Use pthread_sigmask for correctness in multi-threaded programs.
        let mut set = std::mem::MaybeUninit::<libc::sigset_t>::uninit();
        let sfd_fd = unsafe {
            libc::sigemptyset(set.as_mut_ptr());
            libc::sigaddset(set.as_mut_ptr(), kind.0);
            let set = set.assume_init();
            let res = libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut());
            if res != 0 {
                return Err(io::Error::from_raw_os_error(res));
            }

            // Create signalfd.
            let fd = libc::signalfd(-1, &set, libc::SFD_CLOEXEC | libc::SFD_NONBLOCK);
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            fd
        };

        Ok(Self {
            fd: std::sync::Arc::new(std::sync::atomic::AtomicI32::new(sfd_fd)),
            read_waker: crate::sync::Mutex::new(None),
            refs: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(1)),
        })
    }

    /// Receive the next signal.
    ///
    /// Returns the signal number when a signal is delivered.
    /// Returns `None` if the signal fd is closed.
    pub fn recv(&mut self) -> Recv<'_> {
        Recv { signal: self }
    }

    /// Returns a `SignalStream` that can be used in async contexts.
    pub fn as_signal_stream(&mut self) -> SignalStream<'_> {
        SignalStream { signal: self }
    }
}

impl Clone for Signal {
    fn clone(&self) -> Self {
        self.refs.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            fd: self.fd.clone(),
            read_waker: crate::sync::Mutex::new(None),
            refs: self.refs.clone(),
        }
    }
}

impl Drop for Signal {
    fn drop(&mut self) {
        if self.refs.fetch_sub(1, std::sync::atomic::Ordering::Relaxed) == 1 {
            let fd = self.fd.load(std::sync::atomic::Ordering::Relaxed);
            if fd >= 0 {
                unsafe { libc::close(fd) };
            }
        }
    }
}

impl AsyncRead for Signal {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = unsafe { self.get_unchecked_mut() };
        let fd = this.fd.load(std::sync::atomic::Ordering::Relaxed);

        // signalfd_siginfo is 128 bytes.
        if buf.len() < std::mem::size_of::<libc::signalfd_siginfo>() {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "buffer too small for signalfd_siginfo",
            )));
        }

        unsafe {
            let n = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::WouldBlock {
                    this.read_waker.lock().replace(cx.waker().clone());
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            } else if n == 0 {
                Poll::Ready(Ok(0))
            } else {
                Poll::Ready(Ok(n as usize))
            }
        }
    }
}

// ===========================================================================
// Recv future
// ===========================================================================

/// Future that resolves when a signal is received.
pub struct Recv<'a> {
    signal: &'a mut Signal,
}

impl Future for Recv<'_> {
    type Output = io::Result<Option<libc::c_int>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let mut info = std::mem::MaybeUninit::<libc::signalfd_siginfo>::uninit();
        let buf = unsafe {
            std::slice::from_raw_parts_mut(
                info.as_mut_ptr() as *mut u8,
                std::mem::size_of::<libc::signalfd_siginfo>(),
            )
        };

        let fd = this.signal.fd.load(std::sync::atomic::Ordering::Relaxed);
        unsafe {
            let n = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::WouldBlock {
                    this.signal.read_waker.lock().replace(cx.waker().clone());
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            } else if n == 0 {
                Poll::Ready(Ok(None))
            } else {
                let info = info.assume_init();
                Poll::Ready(Ok(Some(info.ssi_signo as libc::c_int)))
            }
        }
    }
}

// ===========================================================================
// SignalStream
// ===========================================================================

/// A wrapper that provides stream-like access to signals.
pub struct SignalStream<'a> {
    signal: &'a mut Signal,
}

impl<'a> SignalStream<'a> {
    /// Receive the next signal.
    pub async fn next(&mut self) -> io::Result<Option<libc::c_int>> {
        self.signal.recv().await
    }
}

// ===========================================================================
// signal() convenience function
// ===========================================================================

/// Create a new `Signal` for the given signal kind.
///
/// This is a convenience wrapper around [`Signal::new()`].
pub fn signal(kind: SignalKind) -> io::Result<Signal> {
    Signal::new(kind)
}
