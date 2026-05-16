#![allow(non_camel_case_types, non_snake_case)]

pub type c_char = i8;
pub type c_int = i32;
pub type c_long = i64;
pub type c_ulong = u64;
pub type c_void = core::ffi::c_void;
pub type mode_t = u32;
pub type pid_t = i32;
pub type sighandler_t = usize;
pub type socklen_t = u32;
pub type size_t = usize;
pub type ssize_t = isize;
pub type tcflag_t = u32;
pub type cc_t = u8;
pub type speed_t = u32;

pub const AF_UNIX: c_int = 1;
pub const SOCK_STREAM: c_int = 1;
pub const SOL_SOCKET: c_int = 1;
pub const SCM_RIGHTS: c_int = 1;

pub const STDIN_FILENO: c_int = 0;
pub const STDOUT_FILENO: c_int = 1;
pub const STDERR_FILENO: c_int = 2;

pub const SIGHUP: c_int = 1;
pub const SIGINT: c_int = 2;
pub const SIGQUIT: c_int = 3;
pub const SIGKILL: c_int = 9;
pub const SIGTERM: c_int = 15;
pub const SIGCHLD: c_int = 17;
pub const SIG_DFL: sighandler_t = 0;
pub const SIG_IGN: sighandler_t = 1;

pub const WNOHANG: c_int = 1;

pub const F_GETFD: c_int = 1;
pub const F_SETFD: c_int = 2;
pub const FD_CLOEXEC: c_int = 1;

pub const O_RDONLY: c_int = 0;
pub const O_RDWR: c_int = 2;
pub const O_NOCTTY: c_int = 0o400;
pub const O_CLOEXEC: c_int = 0o2000000;

pub const POLLIN: i16 = 0x001;

pub const EBUSY: c_int = 16;
pub const EINVAL: c_int = 22;

pub const S_IFIFO: mode_t = 0o010000;
pub const S_IFCHR: mode_t = 0o020000;
pub const S_IFBLK: mode_t = 0o060000;

pub const AT_FDCWD: c_int = -100;
pub const CLONE_NEWUSER: c_int = 0x10000000;

pub const _SC_OPEN_MAX: c_int = 4;

pub const TIOCSCTTY: c_ulong = 0x540e;

pub const BRKINT: tcflag_t = 0o000002;
pub const ICRNL: tcflag_t = 0o000400;
pub const INPCK: tcflag_t = 0o000020;
pub const ISTRIP: tcflag_t = 0o000040;
pub const IXON: tcflag_t = 0o002000;
pub const ECHO: tcflag_t = 0o000010;
pub const ICANON: tcflag_t = 0o000002;
pub const ISIG: tcflag_t = 0o000001;
pub const IEXTEN: tcflag_t = 0o100000;
pub const CSIZE: tcflag_t = 0o000060;
pub const CS8: tcflag_t = 0o000060;
pub const PARENB: tcflag_t = 0o000400;
pub const VTIME: usize = 5;
pub const VMIN: usize = 6;
pub const TCSAFLUSH: c_int = 2;

const NCCS: usize = 32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct iovec {
    pub iov_base: *mut c_void,
    pub iov_len: size_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct msghdr {
    pub msg_name: *mut c_void,
    pub msg_namelen: socklen_t,
    pub msg_iov: *mut iovec,
    pub msg_iovlen: size_t,
    pub msg_control: *mut c_void,
    pub msg_controllen: size_t,
    pub msg_flags: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct cmsghdr {
    pub cmsg_len: size_t,
    pub cmsg_level: c_int,
    pub cmsg_type: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct pollfd {
    pub fd: c_int,
    pub events: i16,
    pub revents: i16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct termios {
    pub c_iflag: tcflag_t,
    pub c_oflag: tcflag_t,
    pub c_cflag: tcflag_t,
    pub c_lflag: tcflag_t,
    pub c_line: cc_t,
    pub c_cc: [cc_t; NCCS],
    pub c_ispeed: speed_t,
    pub c_ospeed: speed_t,
}

const fn cmsg_align(len: usize) -> usize {
    let align = core::mem::size_of::<usize>();
    (len + align - 1) & !(align - 1)
}

pub unsafe fn CMSG_SPACE(len: u32) -> u32 {
    (cmsg_align(core::mem::size_of::<cmsghdr>()) + cmsg_align(len as usize)) as u32
}

pub unsafe fn CMSG_LEN(len: u32) -> u32 {
    (cmsg_align(core::mem::size_of::<cmsghdr>()) + len as usize) as u32
}

pub unsafe fn CMSG_FIRSTHDR(msg: *const msghdr) -> *mut cmsghdr {
    if msg.is_null() || unsafe { (*msg).msg_controllen } < core::mem::size_of::<cmsghdr>() {
        core::ptr::null_mut()
    } else {
        unsafe { (*msg).msg_control.cast() }
    }
}

pub unsafe fn CMSG_DATA(cmsg: *const cmsghdr) -> *mut u8 {
    unsafe {
        cmsg.cast::<u8>()
            .add(cmsg_align(core::mem::size_of::<cmsghdr>())) as *mut u8
    }
}

pub fn WIFEXITED(status: c_int) -> bool {
    (status & 0x7f) == 0
}

pub fn WEXITSTATUS(status: c_int) -> c_int {
    (status >> 8) & 0xff
}

pub fn WIFSIGNALED(status: c_int) -> bool {
    ((status & 0x7f) + 1) >= 2
}

pub fn WTERMSIG(status: c_int) -> c_int {
    status & 0x7f
}

unsafe extern "C" {
    pub fn _exit(status: c_int) -> !;
    pub fn chdir(path: *const c_char) -> c_int;
    pub fn chroot(path: *const c_char) -> c_int;
    pub fn clearenv() -> c_int;
    pub fn close(fd: c_int) -> c_int;
    pub fn dup(fd: c_int) -> c_int;
    pub fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    pub fn execv(path: *const c_char, argv: *const *const c_char) -> c_int;
    pub fn execvp(file: *const c_char, argv: *const *const c_char) -> c_int;
    pub fn fchdir(fd: c_int) -> c_int;
    pub fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    pub fn fork() -> pid_t;
    pub fn getcwd(buf: *mut c_char, size: size_t) -> *mut c_char;
    pub fn geteuid() -> u32;
    pub fn getgid() -> u32;
    pub fn getpid() -> pid_t;
    pub fn getuid() -> u32;
    pub fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    pub fn isatty(fd: c_int) -> c_int;
    pub fn kill(pid: pid_t, sig: c_int) -> c_int;
    pub fn mkfifo(path: *const c_char, mode: mode_t) -> c_int;
    pub fn mknod(path: *const c_char, mode: mode_t, dev: u64) -> c_int;
    pub fn mount(
        source: *const c_char,
        target: *const c_char,
        filesystemtype: *const c_char,
        mountflags: c_ulong,
        data: *const c_void,
    ) -> c_int;
    pub fn open(pathname: *const c_char, flags: c_int, ...) -> c_int;
    pub fn pipe(pipefd: *mut c_int) -> c_int;
    pub fn poll(fds: *mut pollfd, nfds: usize, timeout: c_int) -> c_int;
    pub fn read(fd: c_int, buf: *mut c_void, count: size_t) -> ssize_t;
    pub fn readlink(pathname: *const c_char, buf: *mut c_char, bufsiz: size_t) -> ssize_t;
    pub fn recvmsg(sockfd: c_int, msg: *mut msghdr, flags: c_int) -> ssize_t;
    pub fn sendmsg(sockfd: c_int, msg: *const msghdr, flags: c_int) -> ssize_t;
    pub fn setdomainname(name: *const c_char, len: size_t) -> c_int;
    pub fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    pub fn setresgid(rgid: u32, egid: u32, sgid: u32) -> c_int;
    pub fn setresuid(ruid: u32, euid: u32, suid: u32) -> c_int;
    pub fn signal(signum: c_int, handler: sighandler_t) -> sighandler_t;
    pub fn snprintf(s: *mut c_char, n: size_t, format: *const c_char, ...) -> c_int;
    pub fn socketpair(domain: c_int, type_: c_int, protocol: c_int, sv: *mut c_int) -> c_int;
    pub fn syscall(num: c_long, ...) -> c_long;
    pub fn sysconf(name: c_int) -> c_long;
    pub fn tcgetattr(fd: c_int, termios_p: *mut termios) -> c_int;
    pub fn tcsetattr(fd: c_int, optional_actions: c_int, termios_p: *const termios) -> c_int;
    pub fn unshare(flags: c_int) -> c_int;
    pub fn waitpid(pid: pid_t, status: *mut c_int, options: c_int) -> pid_t;
    pub fn write(fd: c_int, buf: *const c_void, count: size_t) -> ssize_t;
}
