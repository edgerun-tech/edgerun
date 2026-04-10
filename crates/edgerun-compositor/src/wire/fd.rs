//! Send and receive file descriptors over Unix domain sockets using SCM_RIGHTS.

use std::io;
use std::os::fd::RawFd;

const MAX_FDS: usize = 28;

/// Receive data and file descriptors from a Unix socket.
///
/// Returns (bytes_read, fds_received).
pub fn recv_with_fds(
    fd: RawFd,
    buf: &mut [u8],
) -> io::Result<(usize, Vec<RawFd>)> {
    use libc::{iovec, msghdr, recvmsg, CMSG_DATA, CMSG_FIRSTHDR, CMSG_LEN, CMSG_NXTHDR, SCM_RIGHTS};

    let mut iov = iovec {
        iov_base: buf.as_mut_ptr() as *mut libc::c_void,
        iov_len: buf.len(),
    };

    // Control message buffer for SCM_RIGHTS
    // cmsg_space!([RawFd; MAX_FDS]) = size of cmsghdr + MAX_FDS * size_of(RawFd)
    let cmsg_space = std::mem::size_of::<libc::cmsghdr>() + MAX_FDS * std::mem::size_of::<RawFd>();
    // Round up to alignment
    let cmsg_space = (cmsg_space + std::mem::align_of::<libc::cmsghdr>() - 1)
        & !(std::mem::align_of::<libc::cmsghdr>() - 1);
    let mut control = vec![0u8; cmsg_space];

    let mut msg: msghdr = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr() as *mut libc::c_void;
    msg.msg_controllen = control.len() as _;

    let ret = unsafe { recvmsg(fd, &mut msg, libc::MSG_CMSG_CLOEXEC) };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }

    let bytes_read = ret as usize;

    // Extract fds from control messages
    let mut fds = Vec::new();
    let mut cmsg = unsafe { CMSG_FIRSTHDR(&msg) };
    while !cmsg.is_null() {
        let cmsg_ref = unsafe { &*cmsg };
        if cmsg_ref.cmsg_level == libc::SOL_SOCKET && cmsg_ref.cmsg_type == SCM_RIGHTS {
            let data = unsafe { CMSG_DATA(cmsg) };
            let fd_count = (cmsg_ref.cmsg_len as usize - unsafe { CMSG_LEN(0) as usize })
                / std::mem::size_of::<RawFd>();
            for i in 0..fd_count {
                let fd_ptr = unsafe { data.add(i * std::mem::size_of::<RawFd>()) as *const RawFd };
                fds.push(unsafe { *fd_ptr });
            }
        }
        cmsg = unsafe { CMSG_NXTHDR(&msg, cmsg) };
    }

    Ok((bytes_read, fds))
}

/// Send data and file descriptors over a Unix socket.
///
/// Returns bytes written.
pub fn send_with_fds(fd: RawFd, data: &[u8], fds: &[RawFd]) -> io::Result<usize> {
    use libc::{iovec, msghdr, sendmsg, CMSG_DATA, CMSG_FIRSTHDR, CMSG_LEN};

    let mut iov = iovec {
        iov_base: data.as_ptr() as *const libc::c_void as *mut libc::c_void,
        iov_len: data.len(),
    };

    let mut msg: msghdr = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;

    if !fds.is_empty() {
        let cmsg_data_len = std::mem::size_of::<libc::cmsghdr>() + fds.len() * std::mem::size_of::<RawFd>();
        let cmsg_space = (cmsg_data_len + std::mem::align_of::<libc::cmsghdr>() - 1)
            & !(std::mem::align_of::<libc::cmsghdr>() - 1);
        let mut control = vec![0u8; cmsg_space];

        // Set up cmsghdr
        let cmsg_ptr = control.as_mut_ptr() as *mut libc::cmsghdr;
        let cmsg_len = unsafe { CMSG_LEN((fds.len() * std::mem::size_of::<RawFd>()) as libc::c_uint) };
        eprintln!("[edgerun-compositor] send_with_fds: cmsg_len={}, cmsg_space={}, fds={:?} data_len={}",
            cmsg_len, cmsg_space, fds, data.len());
        unsafe {
            (*cmsg_ptr).cmsg_len = cmsg_len as _;
            (*cmsg_ptr).cmsg_level = libc::SOL_SOCKET;
            (*cmsg_ptr).cmsg_type = libc::SCM_RIGHTS;

            // Copy fds into the data area
            let data_ptr = CMSG_DATA(cmsg_ptr) as *mut RawFd;
            std::ptr::copy_nonoverlapping(fds.as_ptr(), data_ptr, fds.len());
        }

        msg.msg_control = control.as_mut_ptr() as *mut libc::c_void;
        msg.msg_controllen = control.len() as _;
    }

    let ret = unsafe { sendmsg(fd, &msg, libc::MSG_NOSIGNAL) };
    if ret < 0 {
        let err = io::Error::last_os_error();
        eprintln!("[edgerun-compositor] sendmsg failed: errno={} ({})", err.raw_os_error().unwrap_or(-1), err);
        return Err(err);
    }

    Ok(ret as usize)
}
