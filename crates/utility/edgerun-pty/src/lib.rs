//! Edgerun PTY abstraction.

use std::ffi::{CStr, OsStr, OsString};
use std::fmt;
use std::fs::File;
use std::io::{Read, Result as IoResult, Write};
use std::process::{Command, Stdio};
use std::sync::Mutex;

#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(unix)]
pub mod unix {
    pub type RawFd = std::os::fd::RawFd;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

impl Default for PtySize {
    fn default() -> Self {
        Self {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitStatus {
    code: u32,
    signal: Option<String>,
}

impl ExitStatus {
    pub fn with_exit_code(code: u32) -> Self {
        Self { code, signal: None }
    }

    pub fn with_signal(signal: &str) -> Self {
        Self {
            code: 1,
            signal: Some(signal.to_string()),
        }
    }

    pub fn success(&self) -> bool {
        self.signal.is_none() && self.code == 0
    }

    pub fn exit_code(&self) -> u32 {
        self.code
    }

    pub fn signal(&self) -> Option<&str> {
        self.signal.as_deref()
    }
}

impl fmt::Display for ExitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.success() {
            f.write_str("Success")
        } else if let Some(signal) = &self.signal {
            write!(f, "Terminated by {signal}")
        } else {
            write!(f, "Exited with code {}", self.code)
        }
    }
}

impl From<std::process::ExitStatus> for ExitStatus {
    fn from(status: std::process::ExitStatus) -> Self {
        #[cfg(unix)]
        if let Some(signal) = status.signal() {
            return Self {
                code: status.code().map(|code| code as u32).unwrap_or(1),
                signal: Some(format!("Signal {signal}")),
            };
        }

        Self {
            code: status
                .code()
                .map(|code| code as u32)
                .unwrap_or_else(|| if status.success() { 0 } else { 1 }),
            signal: None,
        }
    }
}

pub struct CommandBuilder {
    args: Vec<OsString>,
    envs: Vec<(OsString, OsString)>,
    cwd: Option<OsString>,
}

impl CommandBuilder {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        Self {
            args: vec![program.as_ref().to_owned()],
            envs: Vec::new(),
            cwd: None,
        }
    }

    pub fn from_argv(args: Vec<OsString>) -> Self {
        Self {
            args,
            envs: Vec::new(),
            cwd: None,
        }
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.args.push(arg.as_ref().to_owned());
        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.arg(arg);
        }
        self
    }

    pub fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.envs
            .push((key.as_ref().to_owned(), value.as_ref().to_owned()));
        self
    }

    pub fn cwd<D: AsRef<OsStr>>(&mut self, dir: D) -> &mut Self {
        self.cwd = Some(dir.as_ref().to_owned());
        self
    }
}

pub trait MasterPty: Send {
    fn resize(&self, size: PtySize) -> Result<()>;
    fn get_size(&self) -> Result<PtySize>;
    fn try_clone_reader(&self) -> Result<Box<dyn Read + Send>>;
    fn take_writer(&self) -> Result<Box<dyn Write + Send>>;

    #[cfg(unix)]
    fn process_group_leader(&self) -> Option<PidT> {
        None
    }

    #[cfg(unix)]
    fn as_raw_fd(&self) -> Option<unix::RawFd> {
        None
    }

    #[cfg(unix)]
    fn tty_name(&self) -> Option<std::path::PathBuf> {
        None
    }
}

#[cfg(unix)]
pub type PidT = i32;

pub trait ChildKiller: fmt::Debug + Send {
    fn kill(&mut self) -> IoResult<()>;
    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync>;
}

pub trait Child: fmt::Debug + ChildKiller + Send {
    fn try_wait(&mut self) -> IoResult<Option<ExitStatus>>;
    fn wait(&mut self) -> IoResult<ExitStatus>;
    fn process_id(&self) -> Option<u32>;
}

pub trait SlavePty: Send {
    fn spawn_command(&self, cmd: CommandBuilder) -> Result<Box<dyn Child + Send + Sync>>;
}

pub struct PtyPair {
    pub slave: Box<dyn SlavePty + Send>,
    pub master: Box<dyn MasterPty + Send>,
}

pub trait PtySystem: Send {
    fn openpty(&self, size: PtySize) -> Result<PtyPair>;
}

#[derive(Default)]
pub struct NativePtySystem;

impl PtySystem for NativePtySystem {
    fn openpty(&self, size: PtySize) -> Result<PtyPair> {
        open_native_pty(size)
    }
}

pub fn native_pty_system() -> Box<dyn PtySystem + Send> {
    Box::new(NativePtySystem)
}

#[cfg(unix)]
struct NativeMaster {
    file: File,
    size: Mutex<PtySize>,
}

#[cfg(unix)]
impl MasterPty for NativeMaster {
    fn resize(&self, size: PtySize) -> Result<()> {
        set_winsize(self.file.as_raw_fd(), size)?;
        *self
            .size
            .lock()
            .map_err(|_| msg_error("pty size lock poisoned"))? = size;
        Ok(())
    }

    fn get_size(&self) -> Result<PtySize> {
        get_winsize(self.file.as_raw_fd()).or_else(|_| {
            self.size
                .lock()
                .map(|size| *size)
                .map_err(|_| msg_error("pty size lock poisoned"))
        })
    }

    fn try_clone_reader(&self) -> Result<Box<dyn Read + Send>> {
        Ok(Box::new(self.file.try_clone()?))
    }

    fn take_writer(&self) -> Result<Box<dyn Write + Send>> {
        Ok(Box::new(self.file.try_clone()?))
    }

    fn as_raw_fd(&self) -> Option<unix::RawFd> {
        Some(self.file.as_raw_fd())
    }
}

#[cfg(unix)]
struct NativeSlave {
    file: File,
}

#[cfg(unix)]
impl SlavePty for NativeSlave {
    fn spawn_command(&self, cmd: CommandBuilder) -> Result<Box<dyn Child + Send + Sync>> {
        let Some(program) = cmd.args.first() else {
            return Err(msg_error("empty command"));
        };
        let stdin = self.file.try_clone()?;
        let stdout = self.file.try_clone()?;
        let stderr = self.file.try_clone()?;
        let mut command = Command::new(program);
        command.args(cmd.args.iter().skip(1));
        for (key, value) in cmd.envs {
            command.env(key, value);
        }
        if let Some(cwd) = cmd.cwd {
            command.current_dir(cwd);
        }
        command
            .stdin(Stdio::from(stdin))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        let child = command.spawn()?;
        Ok(Box::new(NativeChild { child }))
    }
}

#[derive(Debug)]
struct NativeChild {
    child: std::process::Child,
}

impl ChildKiller for NativeChild {
    fn kill(&mut self) -> IoResult<()> {
        self.child.kill()
    }

    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        Box::new(ProcessKiller {
            pid: self.child.id(),
        })
    }
}

impl Child for NativeChild {
    fn try_wait(&mut self) -> IoResult<Option<ExitStatus>> {
        self.child
            .try_wait()
            .map(|status| status.map(ExitStatus::from))
    }

    fn wait(&mut self) -> IoResult<ExitStatus> {
        self.child.wait().map(ExitStatus::from)
    }

    fn process_id(&self) -> Option<u32> {
        Some(self.child.id())
    }
}

#[derive(Debug)]
struct ProcessKiller {
    pid: u32,
}

impl ChildKiller for ProcessKiller {
    fn kill(&mut self) -> IoResult<()> {
        kill_pid(self.pid)
    }

    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        Box::new(Self { pid: self.pid })
    }
}

#[cfg(unix)]
fn open_native_pty(size: PtySize) -> Result<PtyPair> {
    let master = unsafe { posix_openpt(O_RDWR | O_NOCTTY | O_CLOEXEC) };
    if master < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let master = OwnedFd::new(master);
    cvt(unsafe { grantpt(master.raw()) })?;
    cvt(unsafe { unlockpt(master.raw()) })?;
    let slave_name = unsafe {
        let ptr = ptsname(master.raw());
        if ptr.is_null() {
            return Err(std::io::Error::last_os_error().into());
        }
        CStr::from_ptr(ptr).to_owned()
    };
    let slave = unsafe { open(slave_name.as_ptr(), O_RDWR | O_NOCTTY | O_CLOEXEC) };
    if slave < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let master_file = unsafe { File::from_raw_fd(master.into_raw()) };
    let slave_file = unsafe { File::from_raw_fd(slave) };
    let native_master = NativeMaster {
        file: master_file,
        size: Mutex::new(size),
    };
    native_master.resize(size)?;
    Ok(PtyPair {
        slave: Box::new(NativeSlave { file: slave_file }),
        master: Box::new(native_master),
    })
}

#[cfg(not(unix))]
fn open_native_pty(_size: PtySize) -> Result<PtyPair> {
    Err(msg_error("native pty backend is only implemented on unix"))
}

#[cfg(unix)]
fn set_winsize(fd: RawFd, size: PtySize) -> IoResult<()> {
    let winsize = Winsize {
        ws_row: size.rows,
        ws_col: size.cols,
        ws_xpixel: size.pixel_width,
        ws_ypixel: size.pixel_height,
    };
    cvt(unsafe { ioctl(fd, TIOCSWINSZ, &winsize) }).map(|_| ())
}

#[cfg(unix)]
fn get_winsize(fd: RawFd) -> IoResult<PtySize> {
    let mut winsize = Winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    cvt(unsafe { ioctl(fd, TIOCGWINSZ, &mut winsize) })?;
    Ok(PtySize {
        rows: winsize.ws_row,
        cols: winsize.ws_col,
        pixel_width: winsize.ws_xpixel,
        pixel_height: winsize.ws_ypixel,
    })
}

#[cfg(unix)]
fn kill_pid(pid: u32) -> IoResult<()> {
    cvt(unsafe { kill(pid as i32, SIGTERM) }).map(|_| ())
}

#[cfg(unix)]
fn cvt(ret: i32) -> IoResult<i32> {
    if ret < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(ret)
    }
}

#[derive(Debug)]
struct BackendError(String);

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for BackendError {}

fn msg_error(message: impl Into<String>) -> Error {
    Box::new(BackendError(message.into()))
}

#[cfg(unix)]
struct OwnedFd(RawFd);

#[cfg(unix)]
impl OwnedFd {
    fn new(fd: RawFd) -> Self {
        Self(fd)
    }

    fn raw(&self) -> RawFd {
        self.0
    }

    fn into_raw(self) -> RawFd {
        let fd = self.0;
        std::mem::forget(self);
        fd
    }
}

#[cfg(unix)]
impl Drop for OwnedFd {
    fn drop(&mut self) {
        unsafe {
            close(self.0);
        }
    }
}

#[cfg(unix)]
#[repr(C)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

#[cfg(unix)]
const O_RDWR: i32 = 0o2;
#[cfg(unix)]
const O_NOCTTY: i32 = 0o400;
#[cfg(unix)]
const O_CLOEXEC: i32 = 0o2000000;
#[cfg(unix)]
const TIOCSWINSZ: usize = 0x5414;
#[cfg(unix)]
const TIOCGWINSZ: usize = 0x5413;
#[cfg(unix)]
const SIGTERM: i32 = 15;

#[cfg(unix)]
unsafe extern "C" {
    fn posix_openpt(flags: i32) -> i32;
    fn grantpt(fd: i32) -> i32;
    fn unlockpt(fd: i32) -> i32;
    fn ptsname(fd: i32) -> *const std::ffi::c_char;
    fn open(path: *const std::ffi::c_char, flags: i32, ...) -> i32;
    fn ioctl(fd: i32, request: usize, ...) -> i32;
    fn close(fd: i32) -> i32;
    fn kill(pid: i32, sig: i32) -> i32;
}
