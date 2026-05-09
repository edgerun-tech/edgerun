//! Terminal and Unix fd-passing helpers shared by run/exec setup.

use crate::libc;
use std::io;
use std::os::raw::{c_char, c_int};

unsafe extern "C" {
    fn posix_openpt(flags: c_int) -> c_int;
    fn grantpt(fd: c_int) -> c_int;
    fn unlockpt(fd: c_int) -> c_int;
    fn ptsname(fd: c_int) -> *const c_char;
}

/// Allocate a pseudo-terminal and connect the slave to stdin/stdout/stderr.
///
/// Returns the master fd so the caller can relay terminal I/O or pass it to a
/// supervisor process. The caller owns the returned fd.
pub(crate) fn setup_pty_stdio() -> io::Result<i32> {
    let master_fd = unsafe { posix_openpt(libc::O_RDWR | libc::O_NOCTTY) };
    if master_fd < 0 {
        return Err(io::Error::last_os_error());
    }

    if unsafe { grantpt(master_fd) } != 0 {
        let err = io::Error::last_os_error();
        unsafe { libc::close(master_fd) };
        return Err(err);
    }

    if unsafe { unlockpt(master_fd) } != 0 {
        let err = io::Error::last_os_error();
        unsafe { libc::close(master_fd) };
        return Err(err);
    }

    let slave_path = unsafe { ptsname(master_fd) };
    if slave_path.is_null() {
        let err = io::Error::last_os_error();
        unsafe { libc::close(master_fd) };
        return Err(err);
    }

    let slave_fd = unsafe { libc::open(slave_path, libc::O_RDWR) };
    if slave_fd < 0 {
        let err = io::Error::last_os_error();
        unsafe { libc::close(master_fd) };
        return Err(err);
    }

    unsafe { libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) };

    if unsafe { libc::dup2(slave_fd, libc::STDIN_FILENO) } < 0
        || unsafe { libc::dup2(slave_fd, libc::STDOUT_FILENO) } < 0
        || unsafe { libc::dup2(slave_fd, libc::STDERR_FILENO) } < 0
    {
        let err = io::Error::last_os_error();
        if slave_fd > 2 {
            unsafe { libc::close(slave_fd) };
        }
        unsafe { libc::close(master_fd) };
        return Err(err);
    }

    if slave_fd > 2 {
        unsafe { libc::close(slave_fd) };
    }

    Ok(master_fd)
}

pub(crate) fn send_fd(sock_fd: i32, fd: i32) -> io::Result<()> {
    let mut msg: libc::msghdr = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    let cmsg_space = unsafe { libc::CMSG_SPACE(std::mem::size_of::<i32>() as u32) as usize };
    let mut cmsg_buf = vec![0u8; cmsg_space];
    let fd_to_send = fd;
    let iov = libc::iovec {
        iov_base: &fd_to_send as *const _ as *mut libc::c_void,
        iov_len: std::mem::size_of::<i32>(),
    };

    msg.msg_iov = &iov as *const _ as *mut libc::iovec;
    msg.msg_iovlen = 1;
    msg.msg_control = cmsg_buf.as_mut_ptr() as *mut libc::c_void;
    msg.msg_controllen = cmsg_space as _;

    let cmsg = unsafe { libc::CMSG_FIRSTHDR(&msg) };
    if cmsg.is_null() {
        return Err(io::Error::other(
            "failed to allocate terminal fd control message",
        ));
    }
    unsafe {
        (*cmsg).cmsg_level = libc::SOL_SOCKET;
        (*cmsg).cmsg_type = libc::SCM_RIGHTS;
        (*cmsg).cmsg_len = libc::CMSG_LEN(std::mem::size_of::<i32>() as u32) as _;
        std::ptr::copy_nonoverlapping(&fd as *const i32, libc::CMSG_DATA(cmsg) as *mut i32, 1);
    }

    let ret = unsafe { libc::sendmsg(sock_fd, &msg, 0) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub(crate) fn recv_fd(sock_fd: i32) -> io::Result<i32> {
    let mut msg: libc::msghdr = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    let cmsg_space = unsafe { libc::CMSG_SPACE(std::mem::size_of::<i32>() as u32) as usize };
    let mut cmsg_buf = vec![0u8; cmsg_space];
    let mut fd_buf = 0i32;
    let iov = libc::iovec {
        iov_base: &mut fd_buf as *mut _ as *mut libc::c_void,
        iov_len: std::mem::size_of::<i32>(),
    };

    msg.msg_iov = &iov as *const _ as *mut libc::iovec;
    msg.msg_iovlen = 1;
    msg.msg_control = cmsg_buf.as_mut_ptr() as *mut libc::c_void;
    msg.msg_controllen = cmsg_space as _;

    let ret = unsafe { libc::recvmsg(sock_fd, &mut msg, 0) };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }

    let cmsg = unsafe { libc::CMSG_FIRSTHDR(&msg) };
    if cmsg.is_null() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "no fd control message received",
        ));
    }

    let fd = unsafe { std::ptr::read_unaligned(libc::CMSG_DATA(cmsg) as *const i32) };
    Ok(fd)
}

pub(crate) fn wait_for_exit_code(child_pid: i32) -> io::Result<i32> {
    let mut status = 0i32;
    let pid = unsafe { libc::waitpid(child_pid, &mut status as *mut i32, 0) };
    if pid < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(exit_code_from_status(status))
}

pub(crate) fn relay_pty_until_exit(
    pty_master_fd: i32,
    child_pid: i32,
    interactive: bool,
) -> io::Result<i32> {
    let stdin_fd = libc::STDIN_FILENO;
    let has_terminal = unsafe { libc::isatty(stdin_fd) == 1 };
    let _restore = if interactive && has_terminal {
        TerminalRestore::enter_raw_mode(stdin_fd)?
    } else {
        TerminalRestore::empty()
    };

    let mut buf_in = [0u8; 4096];
    let mut buf_out = [0u8; 4096];
    let mut stdin_open = interactive;
    loop {
        let mut fds = [
            libc::pollfd {
                fd: stdin_fd,
                events: if stdin_open { libc::POLLIN } else { 0 },
                revents: 0,
            },
            libc::pollfd {
                fd: pty_master_fd,
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        let ret = unsafe { libc::poll(fds.as_mut_ptr(), 2, 100) };
        if ret < 0 {
            if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(io::Error::last_os_error());
        }

        if stdin_open && fds[0].revents & libc::POLLIN != 0 {
            let n = unsafe {
                libc::read(
                    stdin_fd,
                    buf_in.as_mut_ptr() as *mut libc::c_void,
                    buf_in.len(),
                )
            };
            if n > 0 {
                write_all_fd(pty_master_fd, &buf_in[..n as usize]);
            } else {
                stdin_open = false;
                write_all_fd(pty_master_fd, &[4]);
            }
        }

        if fds[1].revents & libc::POLLIN != 0 {
            let n = unsafe {
                libc::read(
                    pty_master_fd,
                    buf_out.as_mut_ptr() as *mut libc::c_void,
                    buf_out.len(),
                )
            };
            if n > 0 {
                write_all_fd(libc::STDOUT_FILENO, &buf_out[..n as usize]);
            }
        }

        let mut status = 0i32;
        let wait = unsafe { libc::waitpid(child_pid, &mut status as *mut i32, libc::WNOHANG) };
        if wait > 0 {
            drain_fd_to_stdout(pty_master_fd, &mut buf_out);
            return Ok(exit_code_from_status(status));
        }
        if wait < 0 {
            return Err(io::Error::last_os_error());
        }
    }
}

fn exit_code_from_status(status: i32) -> i32 {
    if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else if libc::WIFSIGNALED(status) {
        128 + libc::WTERMSIG(status)
    } else {
        128
    }
}

fn drain_fd_to_stdout(fd: i32, buffer: &mut [u8]) {
    loop {
        let n = unsafe { libc::read(fd, buffer.as_mut_ptr() as *mut libc::c_void, buffer.len()) };
        if n <= 0 {
            break;
        }
        write_all_fd(libc::STDOUT_FILENO, &buffer[..n as usize]);
    }
}

fn write_all_fd(fd: i32, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        let n = unsafe { libc::write(fd, bytes.as_ptr() as *const libc::c_void, bytes.len()) };
        if n <= 0 {
            break;
        }
        bytes = &bytes[n as usize..];
    }
}

struct TerminalRestore(Option<libc::termios>);

impl TerminalRestore {
    fn empty() -> Self {
        Self(None)
    }

    fn enter_raw_mode(stdin_fd: i32) -> io::Result<Self> {
        let mut original: libc::termios = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        if unsafe { libc::tcgetattr(stdin_fd, &mut original) } != 0 {
            return Err(io::Error::last_os_error());
        }

        let mut raw = original;
        raw.c_lflag &= !(libc::ECHO | libc::ICANON | libc::ISIG | libc::IEXTEN);
        raw.c_iflag &= !(libc::BRKINT | libc::ICRNL | libc::INPCK | libc::ISTRIP | libc::IXON);
        raw.c_cflag &= !(libc::CSIZE | libc::PARENB);
        raw.c_cflag |= libc::CS8;
        raw.c_cc[libc::VMIN] = 1;
        raw.c_cc[libc::VTIME] = 0;
        if unsafe { libc::tcsetattr(stdin_fd, libc::TCSAFLUSH, &raw) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(Some(original)))
    }
}

impl Drop for TerminalRestore {
    fn drop(&mut self) {
        if let Some(termios) = self.0 {
            let _ = unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &termios) };
        }
    }
}
