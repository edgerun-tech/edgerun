#[cfg(any(target_os = "none", target_arch = "wasm32"))]
use crate::io::{Read, Write};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::option::Option::{self, None, Some};
use core::result::Result::{self, Err, Ok};
use edgerun_protocols::block::{checked_len_bytes, BlockBackend, BlockDeviceInfo, BlockError};
use edgerun_protocols::nbd::{
    decode_client_flags, decode_export_info, decode_option_header, decode_option_reply_header,
    decode_option_request, decode_reply_header, decode_request_header, decode_server_handshake,
    encode_client_flags, encode_export_info, encode_option_reply, encode_option_request,
    encode_request_header, encode_server_handshake, encode_simple_reply, map_nbd_error,
    request_to_block_command, NbdBlockCommand, NbdOptionRequest, NBD_CLIENT_FLAGS_LEN,
    NBD_CMD_DISC, NBD_CMD_READ, NBD_CMD_WRITE, NBD_EXPORT_INFO_LEN, NBD_FLAG_FIXED_NEWSTYLE,
    NBD_FLAG_READ_ONLY, NBD_OPTION_HEADER_LEN, NBD_OPTION_REPLY_HEADER_LEN, NBD_OPT_ABORT,
    NBD_OPT_EXPORT_NAME, NBD_OPT_LIST, NBD_REPLY_HEADER_LEN, NBD_REP_ACK, NBD_REP_SERVER,
    NBD_REQUEST_HEADER_LEN,
};
pub use edgerun_protocols::nbd::{NbdExport, NbdNegotiatedExport};
#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
use std::fs::File;
#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
use std::io::{Read, Write};
#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
use std::os::fd::AsRawFd;
#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
use std::path::Path;
#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
use std::thread;

fn block_io_error(error: impl ToString) -> BlockError {
    BlockError::BackendFailure(error.to_string())
}

const NBD_SET_SOCK: u64 = 0xab00;
const NBD_SET_BLKSIZE: u64 = 0xab01;
const NBD_DO_IT: u64 = 0xab03;
const NBD_CLEAR_SOCK: u64 = 0xab04;
const NBD_CLEAR_QUE: u64 = 0xab05;
const NBD_SET_SIZE_BLOCKS: u64 = 0xab07;
const NBD_DISCONNECT: u64 = 0xab08;
const NBD_SET_FLAGS: u64 = 0xab0a;

#[derive(Clone)]
pub struct NbdExportEntry {
    pub export: NbdExport,
    pub backend: Arc<dyn BlockBackend + Send + Sync>,
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
    pub export_name: String,
    pub block_size: Option<u32>,
    pub read_only: bool,
}

pub fn serve_nbd_connection<T: Read + Write, B: BlockBackend>(
    stream: &mut T,
    backend: &B,
    export: &NbdExport,
) -> Result<(), BlockError> {
    stream
        .write_all(&encode_server_handshake())
        .map_err(block_io_error)?;
    stream.flush().map_err(block_io_error)?;

    let _client_flags = decode_client_flags(&read_exact_vec(stream, NBD_CLIENT_FLAGS_LEN)?)?;
    let option = decode_option_header(&read_exact_vec(stream, NBD_OPTION_HEADER_LEN)?)?;
    let data = read_exact_vec(stream, option.length as usize)?;
    match decode_option_request(option.option, data) {
        NbdOptionRequest::ExportName(data) => {
            let name = String::from_utf8(data)
                .map_err(|err| BlockError::ProtocolError(err.to_string()))?;
            if name != export.name {
                return Err(BlockError::ProtocolError(format!(
                    "unknown export `{name}`"
                )));
            }
        }
        NbdOptionRequest::Abort => return Ok(()),
        _ => return Err(BlockError::Unsupported),
    }

    let info = backend.info();
    stream
        .write_all(&encode_export_info(&info))
        .map_err(block_io_error)?;
    stream.flush().map_err(block_io_error)?;

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
        .write_all(&encode_server_handshake())
        .map_err(block_io_error)?;
    stream.flush().map_err(block_io_error)?;

    let _client_flags = decode_client_flags(&read_exact_vec(stream, NBD_CLIENT_FLAGS_LEN)?)?;

    let selected = loop {
        let option = decode_option_header(&read_exact_vec(stream, NBD_OPTION_HEADER_LEN)?)?;
        let data = read_exact_vec(stream, option.length as usize)?;

        match decode_option_request(option.option, data) {
            NbdOptionRequest::ExportName(data) => {
                let name = String::from_utf8(data)
                    .map_err(|err| BlockError::ProtocolError(err.to_string()))?;
                let selected = exports
                    .iter()
                    .find(|entry| entry.export.name == name)
                    .ok_or_else(|| BlockError::ProtocolError(format!("unknown export `{name}`")))?;
                break selected;
            }
            NbdOptionRequest::List => {
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
            NbdOptionRequest::Abort => return Ok(()),
            _ => return Err(BlockError::Unsupported),
        }
    };

    let info = selected.backend.info();
    stream
        .write_all(&encode_export_info(&info))
        .map_err(block_io_error)?;
    stream.flush().map_err(block_io_error)?;

    serve_selected_export(stream, selected.backend.as_ref(), &info)
}

fn serve_selected_export<T: Read + Write>(
    stream: &mut T,
    backend: &dyn BlockBackend,
    info: &BlockDeviceInfo,
) -> Result<(), BlockError> {
    loop {
        let request = decode_request_header(&read_exact_vec(stream, NBD_REQUEST_HEADER_LEN)?)?;
        let command = match request_to_block_command(info, &request) {
            Ok(command) => command,
            Err(error) => {
                write_nbd_reply(stream, request.handle, map_nbd_error(&error), None)?;
                continue;
            }
        };
        let handle = command.handle();

        let result = match command {
            NbdBlockCommand::Read {
                handle: _,
                lba,
                blocks,
            } => {
                let mut data = vec![0_u8; checked_len_bytes(info, blocks)?];
                backend
                    .read_blocks(lba, blocks, &mut data)
                    .map(|_| Some(data))
            }
            NbdBlockCommand::Write {
                handle: _,
                lba,
                blocks,
                length,
            } => {
                let data = read_exact_vec(stream, length as usize)?;
                backend.write_blocks(lba, blocks, &data).map(|_| None)
            }
            NbdBlockCommand::Flush { .. } => backend.flush().map(|_| None),
            NbdBlockCommand::Trim {
                handle: _,
                lba,
                blocks,
            } => backend.discard_blocks(lba, blocks).map(|_| None),
            NbdBlockCommand::WriteZeroes {
                handle: _,
                lba,
                blocks,
            } => backend.write_zeroes(lba, blocks).map(|_| None),
            NbdBlockCommand::Disconnect { .. } => return Ok(()),
        };

        match result {
            Ok(Some(data)) => write_nbd_reply(stream, handle, 0, Some(&data))?,
            Ok(None) => write_nbd_reply(stream, handle, 0, None)?,
            Err(error) => write_nbd_reply(stream, handle, map_nbd_error(&error), None)?,
        }
    }
}

#[cfg(any(target_os = "none", target_arch = "wasm32"))]
pub fn detach_nbd<P>(_device: P) -> Result<(), BlockError> {
    Err(BlockError::Unsupported)
}

#[cfg(any(target_os = "none", target_arch = "wasm32"))]
pub fn attach_nbd<T: Read + Write>(
    _spec: &LinuxNbdAttachSpec,
    _stream: T,
) -> Result<(), BlockError> {
    Err(BlockError::Unsupported)
}

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
pub fn attach_nbd<T>(spec: &LinuxNbdAttachSpec, mut stream: T) -> Result<(), BlockError>
where
    T: Read + Write + AsRawFd,
{
    let negotiated = negotiate_nbd_export(&mut stream, &spec.export_name)?;
    let block_size = spec.block_size.unwrap_or(512);
    if negotiated.size_bytes % u64::from(block_size) != 0 {
        return Err(BlockError::Misaligned);
    }

    let device = File::options()
        .read(true)
        .write(true)
        .open(&spec.device)
        .map_err(block_io_error)?;
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

    let do_it_fd = device.try_clone().map_err(block_io_error)?;
    let cleanup_fd = device.try_clone().map_err(block_io_error)?;
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

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
pub fn detach_nbd(device: impl AsRef<Path>) -> Result<(), BlockError> {
    let device = File::options()
        .read(true)
        .write(true)
        .open(device.as_ref())
        .map_err(block_io_error)?;
    let fd = device.as_raw_fd();
    ioctl_noarg(fd, NBD_DISCONNECT)?;
    let _ = ioctl_noarg(fd, NBD_CLEAR_QUE);
    let _ = ioctl_noarg(fd, NBD_CLEAR_SOCK);
    Ok(())
}

pub fn negotiate_nbd_export<T: Read + Write>(
    stream: &mut T,
    export_name: &str,
) -> Result<NbdNegotiatedExport, BlockError> {
    let _handshake = decode_server_handshake(&read_exact_vec(stream, 18)?)?;
    stream
        .write_all(&encode_client_flags(0))
        .map_err(block_io_error)?;
    let name = export_name.as_bytes();
    stream
        .write_all(&encode_option_request(NBD_OPT_EXPORT_NAME, name))
        .map_err(block_io_error)?;
    stream.flush().map_err(block_io_error)?;

    decode_export_info(&read_exact_vec(stream, NBD_EXPORT_INFO_LEN)?)
}

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
unsafe extern "C" {
    fn ioctl(fd: i32, request: u64, ...) -> i32;
}

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
fn ioctl_noarg(fd: i32, request: u64) -> Result<(), BlockError> {
    let rc = unsafe { ioctl(fd, request) };
    if rc < 0 {
        Err(block_io_error(std::io::Error::last_os_error()))
    } else {
        Ok(())
    }
}

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
fn ioctl_with_value(fd: i32, request: u64, value: u64) -> Result<(), BlockError> {
    let rc = unsafe { ioctl(fd, request, value) };
    if rc < 0 {
        Err(block_io_error(std::io::Error::last_os_error()))
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
        .write_all(&encode_simple_reply(handle, error, payload))
        .map_err(block_io_error)?;
    stream.flush().map_err(block_io_error)
}

fn write_option_reply<T: Write>(
    stream: &mut T,
    option: u32,
    reply_type: u32,
    payload: &[u8],
) -> Result<(), BlockError> {
    stream
        .write_all(&encode_option_reply(option, reply_type, payload))
        .map_err(block_io_error)?;
    stream.flush().map_err(block_io_error)
}

fn read_exact_vec<T: Read>(stream: &mut T, len: usize) -> Result<Vec<u8>, BlockError> {
    let mut bytes = vec![0_u8; len];
    stream.read_exact(&mut bytes).map_err(block_io_error)?;
    Ok(bytes)
}

#[cfg(all(test, not(any(target_os = "none", target_arch = "wasm32"))))]
mod tests {
    use super::*;
    use crate::remote::MemoryBlockBackend;
    use edgerun_protocols::block::BlockDeviceInfo;
    use std::net::{SocketAddr, TcpListener, TcpStream};
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
        let (addr, worker) = spawn_nbd_once(Arc::clone(&backend), NbdExport::default()).unwrap();

        let mut stream = TcpStream::connect(addr).unwrap();
        perform_nbd_handshake(&mut stream, "edgerun").unwrap();
        issue_nbd_write(&mut stream, 0, &[0x33; 512]).unwrap();
        let bytes = issue_nbd_read(&mut stream, 0, 512).unwrap();
        assert_eq!(bytes, vec![0x33; 512]);
        issue_nbd_disconnect(&mut stream).unwrap();
        worker.join().unwrap().unwrap();

        let mut verify = vec![0_u8; 512];
        backend.read_blocks(0, 1, &mut verify).unwrap();
        assert_eq!(verify, vec![0x33; 512]);
    }

    #[test]
    fn multi_export_server_selects_requested_export() {
        let first = Arc::new(MemoryBlockBackend::new(test_info("first")).unwrap());
        let second = Arc::new(MemoryBlockBackend::new(test_info("second")).unwrap());
        second.write_blocks(0, 1, &[0x77; 512]).unwrap();
        let (addr, worker) = spawn_multi_nbd_once(vec![
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
        ])
        .unwrap();
        let mut stream = TcpStream::connect(addr).unwrap();
        perform_nbd_handshake(&mut stream, "second").unwrap();
        let bytes = issue_nbd_read(&mut stream, 0, 512).unwrap();
        assert_eq!(bytes, vec![0x77; 512]);
        issue_nbd_disconnect(&mut stream).unwrap();
        worker.join().unwrap().unwrap();
    }

    #[test]
    fn multi_export_server_lists_exports() {
        let (addr, worker) = spawn_multi_nbd_once(vec![
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
        ])
        .unwrap();
        let mut stream = TcpStream::connect(addr).unwrap();
        let handshake = decode_server_handshake(&read_exact_vec(&mut stream, 18).unwrap()).unwrap();
        assert_eq!(handshake.handshake_flags, NBD_FLAG_FIXED_NEWSTYLE);
        stream.write_all(&encode_client_flags(0)).unwrap();
        stream
            .write_all(&encode_option_request(NBD_OPT_LIST, &[]))
            .unwrap();
        let reply = decode_option_reply_header(
            &read_exact_vec(&mut stream, NBD_OPTION_REPLY_HEADER_LEN).unwrap(),
        )
        .unwrap();
        assert_eq!(reply.option, NBD_OPT_LIST);
        assert_eq!(reply.reply_type, NBD_REP_SERVER);
        let first = read_exact_vec(&mut stream, reply.length as usize).unwrap();
        assert_eq!(String::from_utf8(first).unwrap(), "alpha");
        let reply = decode_option_reply_header(
            &read_exact_vec(&mut stream, NBD_OPTION_REPLY_HEADER_LEN).unwrap(),
        )
        .unwrap();
        assert_eq!(reply.option, NBD_OPT_LIST);
        assert_eq!(reply.reply_type, NBD_REP_SERVER);
        let second = read_exact_vec(&mut stream, reply.length as usize).unwrap();
        assert_eq!(String::from_utf8(second).unwrap(), "beta");
        let reply = decode_option_reply_header(
            &read_exact_vec(&mut stream, NBD_OPTION_REPLY_HEADER_LEN).unwrap(),
        )
        .unwrap();
        assert_eq!(reply.option, NBD_OPT_LIST);
        assert_eq!(reply.reply_type, NBD_REP_ACK);
        assert_eq!(reply.length, 0);
        stream
            .write_all(&encode_option_request(NBD_OPT_ABORT, &[]))
            .unwrap();
        worker.join().unwrap().unwrap();
    }

    fn spawn_nbd_once<B: BlockBackend + Send + Sync + 'static>(
        backend: Arc<B>,
        export: NbdExport,
    ) -> Result<(SocketAddr, thread::JoinHandle<Result<(), BlockError>>), BlockError> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(block_io_error)?;
        let addr = listener.local_addr().map_err(block_io_error)?;
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().map_err(block_io_error)?;
            serve_nbd_connection(&mut stream, backend.as_ref(), &export)
        });
        Ok((addr, worker))
    }

    fn spawn_multi_nbd_once(
        exports: Vec<NbdExportEntry>,
    ) -> Result<(SocketAddr, thread::JoinHandle<Result<(), BlockError>>), BlockError> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(block_io_error)?;
        let addr = listener.local_addr().map_err(block_io_error)?;
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().map_err(block_io_error)?;
            serve_nbd_connection_multi(&mut stream, &exports)
        });
        Ok((addr, worker))
    }

    fn perform_nbd_handshake(stream: &mut TcpStream, export_name: &str) -> Result<(), BlockError> {
        let handshake = decode_server_handshake(&read_exact_vec(stream, 18)?)?;
        assert_eq!(handshake.handshake_flags, NBD_FLAG_FIXED_NEWSTYLE);
        stream
            .write_all(&encode_client_flags(0))
            .map_err(block_io_error)?;
        let name = export_name.as_bytes();
        stream
            .write_all(&encode_option_request(NBD_OPT_EXPORT_NAME, name))
            .map_err(block_io_error)?;
        let _export = decode_export_info(&read_exact_vec(stream, NBD_EXPORT_INFO_LEN)?)?;
        Ok(())
    }

    fn issue_nbd_write(stream: &mut TcpStream, offset: u64, data: &[u8]) -> Result<(), BlockError> {
        stream
            .write_all(&encode_request_header(
                NBD_CMD_WRITE,
                1,
                offset,
                data.len() as u32,
            ))
            .map_err(block_io_error)?;
        stream.write_all(data).map_err(block_io_error)?;
        let reply = decode_reply_header(&read_exact_vec(stream, NBD_REPLY_HEADER_LEN)?)?;
        assert_eq!(reply.error, 0);
        assert_eq!(reply.handle, 1);
        Ok(())
    }

    fn issue_nbd_read(
        stream: &mut TcpStream,
        offset: u64,
        length: u32,
    ) -> Result<Vec<u8>, BlockError> {
        stream
            .write_all(&encode_request_header(NBD_CMD_READ, 2, offset, length))
            .map_err(block_io_error)?;
        let reply = decode_reply_header(&read_exact_vec(stream, NBD_REPLY_HEADER_LEN)?)?;
        assert_eq!(reply.error, 0);
        assert_eq!(reply.handle, 2);
        read_exact_vec(stream, length as usize)
    }

    fn issue_nbd_disconnect(stream: &mut TcpStream) -> Result<(), BlockError> {
        stream
            .write_all(&encode_request_header(NBD_CMD_DISC, 3, 0, 0))
            .map_err(block_io_error)
    }
}
