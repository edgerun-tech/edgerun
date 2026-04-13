//! Async child process management.
//!
//! Wraps `std::process::Command` with async `output()`, `status()`,
//! and `Child::wait()` — all run on the blocking thread pool.

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::process::{Command, ExitStatus, Stdio};
use std::task::{Context, Poll};

// ===========================================================================
// output
// ===========================================================================

/// Execute a command and collect its stdout/stderr.
///
/// The command is built by the provided closure, which runs on the
/// blocking thread to avoid issues with `Command` not being `Send`.
pub async fn output<F>(f: F) -> io::Result<std::process::Output>
where
    F: FnOnce() -> Command + Send + 'static,
{
    crate::runtime::spawn_blocking(move || {
        let mut cmd = f();
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.output()
    })
    .await
    .unwrap_or_else(|_| Err(io::Error::other("blocking task panicked")))
}

/// Execute a command and return its exit status.
pub async fn status<F>(f: F) -> io::Result<ExitStatus>
where
    F: FnOnce() -> Command + Send + 'static,
{
    crate::runtime::spawn_blocking(move || {
        f().status()
    })
    .await
    .unwrap_or_else(|_| Err(io::Error::other("blocking task panicked")))
}

/// Execute a command and return its stdout as a `String`.
pub async fn output_string<F>(f: F) -> io::Result<String>
where
    F: FnOnce() -> Command + Send + 'static,
{
    let out = output(f).await?;
    String::from_utf8(out.stdout)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

// ===========================================================================
// Child — a running child process
// ===========================================================================

/// An asynchronously managed child process.
pub struct Child {
    child: std::process::Child,
}

impl Child {
    /// Spawn a command asynchronously.
    ///
    /// The command is built by the provided closure.
    pub fn spawn<F>(f: F) -> io::Result<Self>
    where
        F: FnOnce() -> Command + Send + 'static,
    {
        let child = f().spawn()?;
        Ok(Self { child })
    }

    /// Wait for the child process to exit, returning its status.
    pub fn wait(self) -> ChildWait {
        ChildWait { inner: Some(self.child) }
    }

    /// Get the process ID.
    pub fn id(&self) -> u32 {
        self.child.id()
    }
}

impl Unpin for Child {}

pub struct ChildWait {
    inner: Option<std::process::Child>,
}

impl Future for ChildWait {
    type Output = io::Result<ExitStatus>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let child = this.inner.as_mut().unwrap();
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut child = this.inner.take().unwrap();
                let _ = child.wait(); // reap
                Poll::Ready(Ok(status))
            }
            Ok(None) => {
                let rt = crate::runtime::current_rt();
                let deadline =
                    crate::reactor::Instant::now() + std::time::Duration::from_millis(10);
                rt.reactor.register_timer(deadline, cx.waker().clone());
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

/// Kill a child process.
pub fn kill(child: &mut std::process::Child) -> io::Result<()> {
    child.kill()
}
