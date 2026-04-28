use std::io::IoSliceMut;
use std::os::fd::{AsFd, AsRawFd};
use std::os::unix::prelude::RawFd;
use std::path::Path;

use anyhow::{Context, Result, bail};
use edgerun_json::JsonValue;
use nix::sys::socket::{self, Backlog, UnixAddr};
use nix::unistd;

const DEFAULT_BUFFER_SIZE: usize = 4096;

pub type SeccompAgentResult = Result<(ContainerProcessState, RawFd)>;

#[derive(Debug, Clone)]
pub struct ContainerProcessState {
    state: ProcessState,
    pid: i32,
    metadata: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProcessState {
    id: String,
}

impl ContainerProcessState {
    pub fn state(&self) -> &ProcessState {
        &self.state
    }

    pub fn pid(&self) -> &i32 {
        &self.pid
    }

    pub fn metadata(&self) -> &Option<String> {
        &self.metadata
    }
}

impl ProcessState {
    pub fn id(&self) -> &String {
        &self.id
    }
}

// Receive information from seccomp notify listener. We will receive 2 items, 1
// container process state and 1 seccomp notify fd. This function will only
// receive one connection from the listener and will terminate all socket when
// returning, since we only expect at most 1 connection to the listener based on
// the spec.
pub fn recv_seccomp_listener(seccomp_listener: &Path) -> SeccompAgentResult {
    let addr = socket::UnixAddr::new(seccomp_listener)?;
    let socket = socket::socket(
        socket::AddressFamily::Unix,
        socket::SockType::Stream,
        socket::SockFlag::empty(),
        None,
    )
    .context("failed to create seccomp listener socket")?;

    socket::bind(socket.as_raw_fd(), &addr).context("failed to bind to seccomp listener socket")?;
    // Force the backlog to be 1 so in the case of an error, only one connection
    // from clients will be waiting.
    socket::listen(&socket.as_fd(), Backlog::new(1)?)
        .context("failed to listen on seccomp listener")?;
    let conn = match socket::accept(socket.as_raw_fd()) {
        Ok(conn) => conn,
        Err(e) => {
            bail!("failed to accept connection: {}", e);
        }
    };
    let mut cmsgspace = nix::cmsg_space!([RawFd; 1]);
    let mut buf = vec![0u8; DEFAULT_BUFFER_SIZE];
    let mut iov = [IoSliceMut::new(&mut buf)];
    let msg = match socket::recvmsg::<UnixAddr>(
        conn,
        &mut iov,
        Some(&mut cmsgspace),
        socket::MsgFlags::MSG_CMSG_CLOEXEC,
    ) {
        Ok(msg) => msg,
        Err(e) => {
            let _ = unistd::close(conn);
            bail!("failed to receive message: {}", e);
        }
    };

    // We received the message correctly here, so we can now safely close the socket and connection.
    let _ = unistd::close(conn);
    drop(socket);
    // We are expecting 1 SCM_RIGHTS message with 1 fd.
    let cmsg = msg
        .cmsgs()?
        .next()
        .context("expecting at least 1 SCM_RIGHTS message")?;
    let fd = match cmsg {
        socket::ControlMessageOwned::ScmRights(fds) => {
            if fds.len() != 1 {
                bail!("expecting 1 fds, but received: {:?}", fds);
            }

            fds[0]
        }
        _ => {
            bail!(
                "expecting 1 SCM_RIGHTS message, but received {:?} instead",
                cmsg
            );
        }
    };

    // We have to truncate the message to the correct size before parsing it.
    if msg.bytes >= DEFAULT_BUFFER_SIZE {
        bail!("received more than the DEFAULT_BUFFER_SIZE");
    }
    let msg_bytes = msg.bytes;

    buf.truncate(msg_bytes);

    let container_process_state = parse_container_process_state(&buf)
        .context("failed to parse the received message as container process state")?;

    Ok((container_process_state, fd))
}

fn parse_container_process_state(bytes: &[u8]) -> Result<ContainerProcessState> {
    let value = crate::utils::json::parse_slice(bytes)?;
    let JsonValue::Object(object) = value else {
        bail!("container process state is not an object");
    };
    let state_value = object
        .get("state")
        .ok_or_else(|| anyhow::anyhow!("missing state"))?;
    let JsonValue::Object(state_object) = state_value else {
        bail!("state is not an object");
    };
    let id = state_object
        .get("id")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing state.id"))?
        .to_string();
    let pid = object
        .get("pid")
        .and_then(JsonValue::as_i32)
        .ok_or_else(|| anyhow::anyhow!("missing pid"))?;
    let metadata = object
        .get("metadata")
        .and_then(JsonValue::as_str)
        .map(str::to_string);
    Ok(ContainerProcessState {
        state: ProcessState { id },
        pid,
        metadata,
    })
}
