use crate::remote::{checked_len_bytes, BlockBackend, BlockDeviceInfo, BlockError};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::default::Default;
use core::option::Option::{self, None, Some};
use core::result::Result::{self, Err, Ok};
use core::write;
#[cfg(target_os = "none")]
use edgerun_encoding::io::{Read, Write};
#[cfg(not(target_os = "none"))]
use std::fs::File;
#[cfg(not(target_os = "none"))]
use std::io::{Read, Write};
#[cfg(not(target_os = "none"))]
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
#[cfg(not(target_os = "none"))]
use std::os::fd::AsRawFd;
#[cfg(not(target_os = "none"))]
use std::path::Path;
#[cfg(not(target_os = "none"))]
use std::thread;

const NBD_MAGIC: u64 = 0x4e42444d41474943;
const NBD_OPTS_MAGIC: u64 = 0x49484156454f5054;
const NBD_REQUEST_MAGIC: u32 = 0x2560_9513;
const NBD_REPLY_MAGIC: u32 = 0x6744_6698;
const NBD_FLAG_FIXED_NEWSTYLE: u16 = 1;
const NBD_FLAG_HAS_FLAGS: u16 = 1;
const NBD_FLAG_SEND_FLUSH: u16 = 1 << 2;
const NBD_FLAG_SEND_TRIM: u16 = 1 << 5;
const NBD_FLAG_SEND_WRITE_ZEROES: u16 = 1 << 6;
const NBD_FLAG_READ_ONLY: u16 = 1 << 1;

const NBD_SET_SOCK: u64 = 0xab00;
const NBD_SET_BLKSIZE: u64 = 0xab01;
const NBD_DO_IT: u64 = 0xab03;
const NBD_CLEAR_SOCK: u64 = 0xab04;
const NBD_CLEAR_QUE: u64 = 0xab05;
const NBD_SET_SIZE_BLOCKS: u64 = 0xab07;
const NBD_DISCONNECT: u64 = 0xab08;
const NBD_SET_FLAGS: u64 = 0xab0a;

const NBD_OPT_EXPORT_NAME: u32 = 1;
const NBD_OPT_ABORT: u32 = 2;
const NBD_OPT_LIST: u32 = 3;
const NBD_REP_ACK: u32 = 1;
const NBD_REP_SERVER: u32 = 2;
const NBD_REP_MAGIC: u64 = 0x0003_e889_0455_65a9;

const NBD_CMD_READ: u16 = 0;
const NBD_CMD_WRITE: u16 = 1;
const NBD_CMD_DISC: u16 = 2;
const NBD_CMD_FLUSH: u16 = 3;
const NBD_CMD_TRIM: u16 = 4;
const NBD_CMD_WRITE_ZEROES: u16 = 6;

#[derive(Clone, Debug)]
pub struct NbdExport {
    pub name: String,
    pub description: String,
}

impl Default for NbdExport {
    fn default() -> Self {
        Self {
            name: "edgerun".into(),
            description: "edgerun virtual disk export".into(),
        }
    }
}

#[derive(Clone)]
pub struct NbdExportEntry {
    pub export: NbdExport,
    pub backend: Arc<dyn BlockBackend>,
}

impl core::fmt::Debug for NbdExportEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NbdExportEntry")
            .field("export", &self.export)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct LinuxNbdAttachSpec {
    pub device: String,
    pub host: String,
    pub port: u16,
    pub export_name: String,
    pub block_size: Option<u32>,
    pub read_only: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxNbdNegotiatedExport {
    pub size_bytes: u64,
    pub transmission_flags: u16,
}

pub fn serve_nbd_connection<T: Read + Write, B: BlockBackend>(
    stream: &mut T,
    backend: &B,
    export: &NbdExport,
) -> Result<(), BlockError> {
    stream
        .write_all(&NBD_MAGIC.to_be_bytes())
        .map_err(BlockError::from)?;
    stream
        .write_all(&NBD_OPTS_MAGIC.to_be_bytes())
        .map_err(BlockError::from)?;
    stream
        .write_all(&NBD_FLAG_FIXED_NEWSTYLE.to_be_bytes())
        .map_err(BlockError::from)?;
    stream.flush().map_err(BlockError::from)?;

    let _client_flags = read_u32(stream)?;
    let option_magic = read_u64(stream)?;
    if option_magic != NBD_OPTS_MAGIC {
        return Err(BlockError::ProtocolError("invalid NBD option magic".into()));
    }
    let option = read_u32(stream)?;
    let length = read_u32(stream)? as usize;
    let data = read_exact_vec(stream, length)?;
    match option {
        NBD_OPT_EXPORT_NAME => {
            let name = String::from_utf8(data)
                .map_err(|err| BlockError::ProtocolError(err.to_string()))?;
            if name != export.name {
                return Err(BlockError::ProtocolError(format!(
                    "unknown export `{name}`"
                )));
            }
        }
        NBD_OPT_ABORT => return Ok(()),
        _ => return Err(BlockError::Unsupported),
    }

    let info = backend.info();
    let mut flags = NBD_FLAG_HAS_FLAGS;
    if info.supports_flush {
        flags |= NBD_FLAG_SEND_FLUSH;
    }
    if info.supports_discard {
        flags |= NBD_FLAG_SEND_TRIM;
    }
    if info.supports_write_zeroes {
        flags |= NBD_FLAG_SEND_WRITE_ZEROES;
    }
    stream
        .write_all(&info.total_size_bytes().to_be_bytes())
        .map_err(BlockError::from)?;
    stream
        .write_all(&flags.to_be_bytes())
        .map_err(BlockError::from)?;
    stream.write_all(&[0_u8; 124]).map_err(BlockError::from)?;
    stream.flush().map_err(BlockError::from)?;

    serve_selected_export(stream, backend, &info)
}

pub fn serve_nbd_connection_multi<T: Read + Write>(
    stream: &mut T,
    exports: &[NbdExportEntry],
) -> Result<(), BlockError> {
    if exports.is_empty() {
        return Err(BlockError::ProtocolError(
            "at least one NBD export is required".into(),
        ));
    }

    stream
        .write_all(&NBD_MAGIC.to_be_bytes())
        .map_err(BlockError::from)?;
    stream
        .write_all(&NBD_OPTS_MAGIC.to_be_bytes())
        .map_err(BlockError::from)?;
    stream
        .write_all(&NBD_FLAG_FIXED_NEWSTYLE.to_be_bytes())
        .map_err(BlockError::from)?;
    stream.flush().map_err(BlockError::from)?;

    let _client_flags = read_u32(stream)?;

    let selected = loop {
        let option_magic = read_u64(stream)?;
        if option_magic != NBD_OPTS_MAGIC {
            return Err(BlockError::ProtocolError("invalid NBD option magic".into()));
        }
        let option = read_u32(stream)?;
        let length = read_u32(stream)? as usize;
        let data = read_exact_vec(stream, length)?;

        match option {
            NBD_OPT_EXPORT_NAME => {
                let name = String::from_utf8(data)
                    .map_err(|err| BlockError::ProtocolError(err.to_string()))?;
                let selected = exports
                    .iter()
                    .find(|entry| entry.export.name == name)
                    .ok_or_else(|| BlockError::ProtocolError(format!("unknown export `{name}`")))?;
                break selected;
            }
            NBD_OPT_LIST => {
                for entry in exports {
                    write_option_reply(
                        stream,
                        NBD_OPT_LIST,
                        NBD_REP_SERVER,
                        entry.export.name.as_bytes(),
                    )?;
                }
                write_option_reply(stream, NBD_OPT_LIST, NBD_REP_ACK, &[])?;
            }
            NBD_OPT_ABORT => return Ok(()),
            _ => return Err(BlockError::Unsupported),
        }
    };

    let info = selected.backend.info();
    let mut flags = NBD_FLAG_HAS_FLAGS;
    if info.supports_flush {
        flags |= NBD_FLAG_SEND_FLUSH;
    }
    if info.supports_discard {
        flags |= NBD_FLAG_SEND_TRIM;
    }
    if info.supports_write_zeroes {
        flags |= NBD_FLAG_SEND_WRITE_ZEROES;
    }
    stream
        .write_all(&info.total_size_bytes().to_be_bytes())
        .map_err(BlockError::from)?;
    stream
        .write_all(&flags.to_be_bytes())
        .map_err(BlockError::from)?;
    stream.write_all(&[0_u8; 124]).map_err(BlockError::from)?;
    stream.flush().map_err(BlockError::from)?;

    serve_selected_export(stream, selected.backend.as_ref(), &info)
}

fn serve_selected_export<T: Read + Write>(
    stream: &mut T,
    backend: &dyn BlockBackend,
    info: &BlockDeviceInfo,
) -> Result<(), BlockError> {
    loop {
        let request_magic = read_u32(stream)?;
        if request_magic != NBD_REQUEST_MAGIC {
            return Err(BlockError::ProtocolError(
                "invalid NBD request magic".into(),
            ));
        }
        let _flags = read_u16(stream)?;
        let command = read_u16(stream)?;
        let handle = read_u64(stream)?;
        let offset = read_u64(stream)?;
        let length = read_u32(stream)?;

        let block_size = u64::from(info.block_size);
        if offset % block_size != 0 || u64::from(length) % block_size != 0 {
            write_nbd_reply(stream, handle, 22, None)?;
            continue;
        }

        let lba = offset / block_size;
        let blocks = length / info.block_size;
        let result = match command {
            NBD_CMD_READ => {
                let mut data = vec![0_u8; checked_len_bytes(info, blocks)?];
                backend
                    .read_blocks(lba, blocks, &mut data)
                    .map(|_| Some(data))
            }
            NBD_CMD_WRITE => {
                let data = read_exact_vec(stream, length as usize)?;
                backend.write_blocks(lba, blocks, &data).map(|_| None)
            }
            NBD_CMD_FLUSH => backend.flush().map(|_| None),
            NBD_CMD_TRIM => backend.discard_blocks(lba, blocks).map(|_| None),
            NBD_CMD_WRITE_ZEROES => backend.write_zeroes(lba, blocks).map(|_| None),
            NBD_CMD_DISC => return Ok(()),
            _ => Err(BlockError::Unsupported),
        };

        match result {
            Ok(Some(data)) => write_nbd_reply(stream, handle, 0, Some(&data))?,
            Ok(None) => write_nbd_reply(stream, handle, 0, None)?,
            Err(error) => write_nbd_reply(stream, handle, map_nbd_error(&error), None)?,
        }
    }
}

#[cfg(not(target_os = "none"))]
pub struct TcpNbdServer<B> {
    listener: TcpListener,
    backend: Arc<B>,
    export: NbdExport,
}

#[cfg(not(target_os = "none"))]
impl<B: BlockBackend> TcpNbdServer<B> {
    pub fn bind(
        addr: impl ToSocketAddrs,
        backend: B,
        export: NbdExport,
    ) -> Result<Self, BlockError> {
        Self::bind_shared(addr, Arc::new(backend), export)
    }

    pub fn bind_shared(
        addr: impl ToSocketAddrs,
        backend: Arc<B>,
        export: NbdExport,
    ) -> Result<Self, BlockError> {
        let listener = TcpListener::bind(addr).map_err(BlockError::from)?;
        Ok(Self {
            listener,
            backend,
            export,
        })
    }

    pub fn local_addr(&self) -> Result<std::net::SocketAddr, BlockError> {
        self.listener.local_addr().map_err(BlockError::from)
    }

    pub fn accept_once(&self) -> Result<(), BlockError> {
        let (mut stream, _) = self.listener.accept().map_err(BlockError::from)?;
        serve_nbd_connection(&mut stream, self.backend.as_ref(), &self.export)
    }

    pub fn serve_forever(&self) -> Result<(), BlockError> {
        loop {
            self.accept_once()?;
        }
    }
}

#[cfg(not(target_os = "none"))]
pub struct MultiExportTcpNbdServer {
    listener: TcpListener,
    exports: Vec<NbdExportEntry>,
}

#[cfg(not(target_os = "none"))]
impl MultiExportTcpNbdServer {
    pub fn bind(
        addr: impl ToSocketAddrs,
        exports: Vec<NbdExportEntry>,
    ) -> Result<Self, BlockError> {
        let listener = TcpListener::bind(addr).map_err(BlockError::from)?;
        Ok(Self { listener, exports })
    }

    pub fn local_addr(&self) -> Result<std::net::SocketAddr, BlockError> {
        self.listener.local_addr().map_err(BlockError::from)
    }

    pub fn accept_once(&self) -> Result<(), BlockError> {
        let (mut stream, _) = self.listener.accept().map_err(BlockError::from)?;
        serve_nbd_connection_multi(&mut stream, &self.exports)
    }

    pub fn serve_forever(&self) -> Result<(), BlockError> {
        loop {
            self.accept_once()?;
        }
    }
}

#[cfg(target_os = "none")]
pub struct TcpNbdServer<B> {
    _backend: core::marker::PhantomData<B>,
}

#[cfg(target_os = "none")]
pub struct MultiExportTcpNbdServer;

#[cfg(target_os = "none")]
pub fn attach_nbd(_spec: &LinuxNbdAttachSpec) -> Result<(), BlockError> {
    Err(BlockError::Unsupported)
}

#[cfg(target_os = "none")]
pub fn detach_nbd<P>(_device: P) -> Result<(), BlockError> {
    Err(BlockError::Unsupported)
}

#[cfg(target_os = "none")]
pub fn negotiate_nbd_export<T: Read + Write>(
    _stream: &mut T,
    _export_name: &str,
) -> Result<LinuxNbdNegotiatedExport, BlockError> {
    Err(BlockError::Unsupported)
}

#[cfg(not(target_os = "none"))]
pub fn attach_nbd(spec: &LinuxNbdAttachSpec) -> Result<(), BlockError> {
    let mut stream =
        TcpStream::connect((spec.host.as_str(), spec.port)).map_err(BlockError::from)?;
    let negotiated = negotiate_nbd_export(&mut stream, &spec.export_name)?;
    let block_size = spec.block_size.unwrap_or(512);
    if negotiated.size_bytes % u64::from(block_size) != 0 {
        return Err(BlockError::Misaligned);
    }

    let device = File::options()
        .read(true)
        .write(true)
        .open(&spec.device)
        .map_err(BlockError::from)?;
    let nbd_fd = device.as_raw_fd();
    let sock_fd = stream.as_raw_fd();
    let block_count = negotiated.size_bytes / u64::from(block_size);
    let mut flags = negotiated.transmission_flags;
    if spec.read_only {
        flags |= NBD_FLAG_READ_ONLY;
    }

    ioctl_with_value(nbd_fd, NBD_SET_BLKSIZE, u64::from(block_size))?;
    ioctl_with_value(nbd_fd, NBD_SET_SIZE_BLOCKS, block_count)?;
    ioctl_with_value(nbd_fd, NBD_SET_FLAGS, u64::from(flags))?;
    ioctl_with_value(nbd_fd, NBD_SET_SOCK, sock_fd as u64)?;

    let do_it_fd = device.try_clone().map_err(BlockError::from)?;
    let cleanup_fd = device.try_clone().map_err(BlockError::from)?;
    let worker = thread::spawn(move || {
        let result = ioctl_noarg(do_it_fd.as_raw_fd(), NBD_DO_IT);
        let _ = ioctl_noarg(cleanup_fd.as_raw_fd(), NBD_CLEAR_QUE);
        let _ = ioctl_noarg(cleanup_fd.as_raw_fd(), NBD_CLEAR_SOCK);
        result
    });

    match worker.join() {
        Ok(result) => result,
        Err(_) => Err(BlockError::BackendFailure(
            "nbd kernel worker thread panicked".into(),
        )),
    }
}

#[cfg(not(target_os = "none"))]
pub fn detach_nbd(device: impl AsRef<Path>) -> Result<(), BlockError> {
    let device = File::options()
        .read(true)
        .write(true)
        .open(device.as_ref())
        .map_err(BlockError::from)?;
    let fd = device.as_raw_fd();
    ioctl_noarg(fd, NBD_DISCONNECT)?;
    let _ = ioctl_noarg(fd, NBD_CLEAR_QUE);
    let _ = ioctl_noarg(fd, NBD_CLEAR_SOCK);
    Ok(())
}

#[cfg(not(target_os = "none"))]
pub fn negotiate_nbd_export(
    stream: &mut TcpStream,
    export_name: &str,
) -> Result<LinuxNbdNegotiatedExport, BlockError> {
    if read_u64(stream)? != NBD_MAGIC {
        return Err(BlockError::ProtocolError("invalid NBD magic".into()));
    }
    if read_u64(stream)? != NBD_OPTS_MAGIC {
        return Err(BlockError::ProtocolError(
            "invalid NBD options magic".into(),
        ));
    }
    let _handshake_flags = read_u16(stream)?;
    stream
        .write_all(&0_u32.to_be_bytes())
        .map_err(BlockError::from)?;
    let name = export_name.as_bytes();
    stream
        .write_all(&NBD_OPTS_MAGIC.to_be_bytes())
        .map_err(BlockError::from)?;
    stream
        .write_all(&NBD_OPT_EXPORT_NAME.to_be_bytes())
        .map_err(BlockError::from)?;
    stream
        .write_all(&(name.len() as u32).to_be_bytes())
        .map_err(BlockError::from)?;
    stream.write_all(name).map_err(BlockError::from)?;
    stream.flush().map_err(BlockError::from)?;

    let size_bytes = read_u64(stream)?;
    let transmission_flags = read_u16(stream)?;
    let _zeros = read_exact_vec(stream, 124)?;
    Ok(LinuxNbdNegotiatedExport {
        size_bytes,
        transmission_flags,
    })
}

fn map_nbd_error(error: &BlockError) -> u32 {
    match error {
        BlockError::ReadOnly => 30,
        BlockError::OutOfRange => 22,
        BlockError::Misaligned => 22,
        BlockError::Unsupported => 95,
        BlockError::Timeout => 110,
        BlockError::NotReady => 11,
        BlockError::BackendFailure(_) | BlockError::ProtocolError(_) => 5,
    }
}

#[cfg(not(target_os = "none"))]
unsafe extern "C" {
    fn ioctl(fd: i32, request: u64, ...) -> i32;
}

#[cfg(not(target_os = "none"))]
fn ioctl_noarg(fd: i32, request: u64) -> Result<(), BlockError> {
    let rc = unsafe { ioctl(fd, request) };
    if rc < 0 {
        Err(BlockError::from(std::io::Error::last_os_error()))
    } else {
        Ok(())
    }
}

#[cfg(not(target_os = "none"))]
fn ioctl_with_value(fd: i32, request: u64, value: u64) -> Result<(), BlockError> {
    let rc = unsafe { ioctl(fd, request, value) };
    if rc < 0 {
        Err(BlockError::from(std::io::Error::last_os_error()))
    } else {
        Ok(())
    }
}

fn write_nbd_reply<T: Write>(
    stream: &mut T,
    handle: u64,
    error: u32,
    payload: Option<&[u8]>,
) -> Result<(), BlockError> {
    stream
        .write_all(&NBD_REPLY_MAGIC.to_be_bytes())
        .map_err(BlockError::from)?;
    stream
        .write_all(&error.to_be_bytes())
        .map_err(BlockError::from)?;
    stream
        .write_all(&handle.to_be_bytes())
        .map_err(BlockError::from)?;
    if let Some(payload) = payload {
        stream.write_all(payload).map_err(BlockError::from)?;
    }
    stream.flush().map_err(BlockError::from)
}

fn write_option_reply<T: Write>(
    stream: &mut T,
    option: u32,
    reply_type: u32,
    payload: &[u8],
) -> Result<(), BlockError> {
    stream
        .write_all(&NBD_REP_MAGIC.to_be_bytes())
        .map_err(BlockError::from)?;
    stream
        .write_all(&option.to_be_bytes())
        .map_err(BlockError::from)?;
    stream
        .write_all(&reply_type.to_be_bytes())
        .map_err(BlockError::from)?;
    stream
        .write_all(&(payload.len() as u32).to_be_bytes())
        .map_err(BlockError::from)?;
    if !payload.is_empty() {
        stream.write_all(payload).map_err(BlockError::from)?;
    }
    stream.flush().map_err(BlockError::from)
}

fn read_exact_vec<T: Read>(stream: &mut T, len: usize) -> Result<Vec<u8>, BlockError> {
    let mut bytes = vec![0_u8; len];
    stream.read_exact(&mut bytes).map_err(BlockError::from)?;
    Ok(bytes)
}

fn read_u16<T: Read>(stream: &mut T) -> Result<u16, BlockError> {
    let mut bytes = [0_u8; 2];
    stream.read_exact(&mut bytes).map_err(BlockError::from)?;
    Ok(u16::from_be_bytes(bytes))
}

fn read_u32<T: Read>(stream: &mut T) -> Result<u32, BlockError> {
    let mut bytes = [0_u8; 4];
    stream.read_exact(&mut bytes).map_err(BlockError::from)?;
    Ok(u32::from_be_bytes(bytes))
}

fn read_u64<T: Read>(stream: &mut T) -> Result<u64, BlockError> {
    let mut bytes = [0_u8; 8];
    stream.read_exact(&mut bytes).map_err(BlockError::from)?;
    Ok(u64::from_be_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::{BlockDeviceInfo, MemoryBlockBackend};
    use std::net::TcpStream;
    use std::thread;

    fn test_info(name: &str) -> BlockDeviceInfo {
        BlockDeviceInfo {
            block_size: 512,
            block_count: 8,
            readonly: false,
            supports_flush: true,
            supports_discard: true,
            supports_write_zeroes: true,
            model: format!("edgerun-{name}"),
            serial: name.into(),
        }
    }

    #[test]
    fn tcp_nbd_server_handles_basic_read_write() {
        let backend = Arc::new(MemoryBlockBackend::new(test_info("single")).unwrap());
        let server =
            TcpNbdServer::bind_shared("127.0.0.1:0", Arc::clone(&backend), NbdExport::default())
                .unwrap();
        let addr = server.local_addr().unwrap();
        let worker = thread::spawn(move || server.accept_once().unwrap());

        let mut stream = TcpStream::connect(addr).unwrap();
        perform_nbd_handshake(&mut stream, "edgerun").unwrap();
        issue_nbd_write(&mut stream, 0, &[0x33; 512]).unwrap();
        let bytes = issue_nbd_read(&mut stream, 0, 512).unwrap();
        assert_eq!(bytes, vec![0x33; 512]);
        issue_nbd_disconnect(&mut stream).unwrap();
        worker.join().unwrap();

        let mut verify = vec![0_u8; 512];
        backend.read_blocks(0, 1, &mut verify).unwrap();
        assert_eq!(verify, vec![0x33; 512]);
    }

    #[test]
    fn multi_export_server_selects_requested_export() {
        let first = Arc::new(MemoryBlockBackend::new(test_info("first")).unwrap());
        let second = Arc::new(MemoryBlockBackend::new(test_info("second")).unwrap());
        second.write_blocks(0, 1, &[0x77; 512]).unwrap();
        let server = MultiExportTcpNbdServer::bind(
            "127.0.0.1:0",
            vec![
                NbdExportEntry {
                    export: NbdExport {
                        name: "first".into(),
                        description: "first export".into(),
                    },
                    backend: first,
                },
                NbdExportEntry {
                    export: NbdExport {
                        name: "second".into(),
                        description: "second export".into(),
                    },
                    backend: second,
                },
            ],
        )
        .unwrap();
        let addr = server.local_addr().unwrap();
        let worker = thread::spawn(move || server.accept_once().unwrap());
        let mut stream = TcpStream::connect(addr).unwrap();
        perform_nbd_handshake(&mut stream, "second").unwrap();
        let bytes = issue_nbd_read(&mut stream, 0, 512).unwrap();
        assert_eq!(bytes, vec![0x77; 512]);
        issue_nbd_disconnect(&mut stream).unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn multi_export_server_lists_exports() {
        let server = MultiExportTcpNbdServer::bind(
            "127.0.0.1:0",
            vec![
                NbdExportEntry {
                    export: NbdExport {
                        name: "alpha".into(),
                        description: "alpha export".into(),
                    },
                    backend: Arc::new(MemoryBlockBackend::new(test_info("alpha")).unwrap()),
                },
                NbdExportEntry {
                    export: NbdExport {
                        name: "beta".into(),
                        description: "beta export".into(),
                    },
                    backend: Arc::new(MemoryBlockBackend::new(test_info("beta")).unwrap()),
                },
            ],
        )
        .unwrap();
        let addr = server.local_addr().unwrap();
        let worker = thread::spawn(move || server.accept_once().unwrap());
        let mut stream = TcpStream::connect(addr).unwrap();
        assert_eq!(read_u64(&mut stream).unwrap(), NBD_MAGIC);
        assert_eq!(read_u64(&mut stream).unwrap(), NBD_OPTS_MAGIC);
        assert_eq!(read_u16(&mut stream).unwrap(), NBD_FLAG_FIXED_NEWSTYLE);
        stream.write_all(&0_u32.to_be_bytes()).unwrap();
        stream.write_all(&NBD_OPTS_MAGIC.to_be_bytes()).unwrap();
        stream.write_all(&NBD_OPT_LIST.to_be_bytes()).unwrap();
        stream.write_all(&0_u32.to_be_bytes()).unwrap();
        assert_eq!(read_u64(&mut stream).unwrap(), NBD_REP_MAGIC);
        assert_eq!(read_u32(&mut stream).unwrap(), NBD_OPT_LIST);
        assert_eq!(read_u32(&mut stream).unwrap(), NBD_REP_SERVER);
        let len = read_u32(&mut stream).unwrap() as usize;
        let first = read_exact_vec(&mut stream, len).unwrap();
        assert_eq!(String::from_utf8(first).unwrap(), "alpha");
        assert_eq!(read_u64(&mut stream).unwrap(), NBD_REP_MAGIC);
        assert_eq!(read_u32(&mut stream).unwrap(), NBD_OPT_LIST);
        assert_eq!(read_u32(&mut stream).unwrap(), NBD_REP_SERVER);
        let len = read_u32(&mut stream).unwrap() as usize;
        let second = read_exact_vec(&mut stream, len).unwrap();
        assert_eq!(String::from_utf8(second).unwrap(), "beta");
        assert_eq!(read_u64(&mut stream).unwrap(), NBD_REP_MAGIC);
        assert_eq!(read_u32(&mut stream).unwrap(), NBD_OPT_LIST);
        assert_eq!(read_u32(&mut stream).unwrap(), NBD_REP_ACK);
        assert_eq!(read_u32(&mut stream).unwrap(), 0);
        stream.write_all(&NBD_OPTS_MAGIC.to_be_bytes()).unwrap();
        stream.write_all(&NBD_OPT_ABORT.to_be_bytes()).unwrap();
        stream.write_all(&0_u32.to_be_bytes()).unwrap();
        worker.join().unwrap();
    }

    fn perform_nbd_handshake(stream: &mut TcpStream, export_name: &str) -> Result<(), BlockError> {
        assert_eq!(read_u64(stream)?, NBD_MAGIC);
        assert_eq!(read_u64(stream)?, NBD_OPTS_MAGIC);
        assert_eq!(read_u16(stream)?, NBD_FLAG_FIXED_NEWSTYLE);
        stream
            .write_all(&0_u32.to_be_bytes())
            .map_err(BlockError::from)?;
        let name = export_name.as_bytes();
        stream
            .write_all(&NBD_OPTS_MAGIC.to_be_bytes())
            .map_err(BlockError::from)?;
        stream
            .write_all(&NBD_OPT_EXPORT_NAME.to_be_bytes())
            .map_err(BlockError::from)?;
        stream
            .write_all(&(name.len() as u32).to_be_bytes())
            .map_err(BlockError::from)?;
        stream.write_all(name).map_err(BlockError::from)?;
        let _size = read_u64(stream)?;
        let _flags = read_u16(stream)?;
        let _zeros = read_exact_vec(stream, 124)?;
        Ok(())
    }

    fn issue_nbd_write(stream: &mut TcpStream, offset: u64, data: &[u8]) -> Result<(), BlockError> {
        stream
            .write_all(&NBD_REQUEST_MAGIC.to_be_bytes())
            .map_err(BlockError::from)?;
        stream
            .write_all(&0_u16.to_be_bytes())
            .map_err(BlockError::from)?;
        stream
            .write_all(&NBD_CMD_WRITE.to_be_bytes())
            .map_err(BlockError::from)?;
        stream
            .write_all(&1_u64.to_be_bytes())
            .map_err(BlockError::from)?;
        stream
            .write_all(&offset.to_be_bytes())
            .map_err(BlockError::from)?;
        stream
            .write_all(&(data.len() as u32).to_be_bytes())
            .map_err(BlockError::from)?;
        stream.write_all(data).map_err(BlockError::from)?;
        assert_eq!(read_u32(stream)?, NBD_REPLY_MAGIC);
        assert_eq!(read_u32(stream)?, 0);
        assert_eq!(read_u64(stream)?, 1);
        Ok(())
    }

    fn issue_nbd_read(
        stream: &mut TcpStream,
        offset: u64,
        length: u32,
    ) -> Result<Vec<u8>, BlockError> {
        stream
            .write_all(&NBD_REQUEST_MAGIC.to_be_bytes())
            .map_err(BlockError::from)?;
        stream
            .write_all(&0_u16.to_be_bytes())
            .map_err(BlockError::from)?;
        stream
            .write_all(&NBD_CMD_READ.to_be_bytes())
            .map_err(BlockError::from)?;
        stream
            .write_all(&2_u64.to_be_bytes())
            .map_err(BlockError::from)?;
        stream
            .write_all(&offset.to_be_bytes())
            .map_err(BlockError::from)?;
        stream
            .write_all(&length.to_be_bytes())
            .map_err(BlockError::from)?;
        assert_eq!(read_u32(stream)?, NBD_REPLY_MAGIC);
        assert_eq!(read_u32(stream)?, 0);
        assert_eq!(read_u64(stream)?, 2);
        read_exact_vec(stream, length as usize)
    }

    fn issue_nbd_disconnect(stream: &mut TcpStream) -> Result<(), BlockError> {
        stream
            .write_all(&NBD_REQUEST_MAGIC.to_be_bytes())
            .map_err(BlockError::from)?;
        stream
            .write_all(&0_u16.to_be_bytes())
            .map_err(BlockError::from)?;
        stream
            .write_all(&NBD_CMD_DISC.to_be_bytes())
            .map_err(BlockError::from)?;
        stream
            .write_all(&3_u64.to_be_bytes())
            .map_err(BlockError::from)?;
        stream
            .write_all(&0_u64.to_be_bytes())
            .map_err(BlockError::from)?;
        stream
            .write_all(&0_u32.to_be_bytes())
            .map_err(BlockError::from)
    }
}
