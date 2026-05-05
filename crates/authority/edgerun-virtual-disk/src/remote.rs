use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::option::Option::{self, None, Some};
use core::result::Result::{self, Err, Ok};
use core::{debug_assert_eq, fmt, write};
use edgerun_encoding::byteorder::{read_u16_le, read_u32_le, read_u64_le};
#[cfg(any(target_os = "none", target_arch = "wasm32"))]
use edgerun_encoding::io::{self, Read, Write};
#[cfg(any(target_os = "none", target_arch = "wasm32"))]
use edgerun_rt::Mutex;
#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
use std::fs::{self, File, OpenOptions};
#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
use std::io::{self, Read, Seek, SeekFrom, Write};
#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
use std::os::unix::net::{UnixListener, UnixStream};
#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
use std::path::Path;
#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
use std::sync::Mutex;

pub const BLOCK_PROTOCOL_VERSION: u16 = 1;
const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockDeviceInfo {
    pub block_size: u32,
    pub block_count: u64,
    pub readonly: bool,
    pub supports_flush: bool,
    pub supports_discard: bool,
    pub supports_write_zeroes: bool,
    pub model: String,
    pub serial: String,
}

impl BlockDeviceInfo {
    pub fn total_size_bytes(&self) -> u64 {
        self.block_count * u64::from(self.block_size)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockError {
    OutOfRange,
    ReadOnly,
    Misaligned,
    Unsupported,
    BackendFailure(String),
    ProtocolError(String),
    NotReady,
    Timeout,
}

impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfRange => f.write_str("block request out of range"),
            Self::ReadOnly => f.write_str("block device is read-only"),
            Self::Misaligned => f.write_str("block request is misaligned"),
            Self::Unsupported => f.write_str("operation unsupported"),
            Self::BackendFailure(message) => write!(f, "backend failure: {message}"),
            Self::ProtocolError(message) => write!(f, "protocol error: {message}"),
            Self::NotReady => f.write_str("backend not ready"),
            Self::Timeout => f.write_str("operation timed out"),
        }
    }
}

impl core::error::Error for BlockError {}

impl From<io::Error> for BlockError {
    fn from(value: io::Error) -> Self {
        Self::BackendFailure(value.to_string())
    }
}

impl<B: BlockBackend> BlockBackend for Arc<B> {
    fn info(&self) -> BlockDeviceInfo {
        self.as_ref().info()
    }

    fn read_blocks(&self, lba: u64, blocks: u32, out: &mut [u8]) -> Result<(), BlockError> {
        self.as_ref().read_blocks(lba, blocks, out)
    }

    fn write_blocks(&self, lba: u64, blocks: u32, data: &[u8]) -> Result<(), BlockError> {
        self.as_ref().write_blocks(lba, blocks, data)
    }

    fn flush(&self) -> Result<(), BlockError> {
        self.as_ref().flush()
    }

    fn discard_blocks(&self, lba: u64, blocks: u32) -> Result<(), BlockError> {
        self.as_ref().discard_blocks(lba, blocks)
    }

    fn write_zeroes(&self, lba: u64, blocks: u32) -> Result<(), BlockError> {
        self.as_ref().write_zeroes(lba, blocks)
    }
}

pub trait BlockBackend: Send + Sync + 'static {
    fn info(&self) -> BlockDeviceInfo;
    fn read_blocks(&self, lba: u64, blocks: u32, out: &mut [u8]) -> Result<(), BlockError>;
    fn write_blocks(&self, lba: u64, blocks: u32, data: &[u8]) -> Result<(), BlockError>;
    fn flush(&self) -> Result<(), BlockError>;

    fn discard_blocks(&self, _lba: u64, _blocks: u32) -> Result<(), BlockError> {
        Err(BlockError::Unsupported)
    }

    fn write_zeroes(&self, _lba: u64, _blocks: u32) -> Result<(), BlockError> {
        Err(BlockError::Unsupported)
    }
}

pub struct MemoryBlockBackend {
    info: BlockDeviceInfo,
    data: Mutex<Vec<u8>>,
}

impl fmt::Debug for MemoryBlockBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryBlockBackend")
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

impl MemoryBlockBackend {
    pub fn new(info: BlockDeviceInfo) -> Result<Self, BlockError> {
        validate_device_info(&info)?;
        let len = total_size_len(&info)?;
        Ok(Self {
            info,
            data: Mutex::new(vec![0; len]),
        })
    }

    pub fn from_bytes(info: BlockDeviceInfo, data: Vec<u8>) -> Result<Self, BlockError> {
        validate_device_info(&info)?;
        if data.len() != total_size_len(&info)? {
            return Err(BlockError::ProtocolError(
                "memory backend bytes do not match declared size".into(),
            ));
        }
        Ok(Self {
            info,
            data: Mutex::new(data),
        })
    }
}

impl BlockBackend for MemoryBlockBackend {
    fn info(&self) -> BlockDeviceInfo {
        self.info.clone()
    }

    fn read_blocks(&self, lba: u64, blocks: u32, out: &mut [u8]) -> Result<(), BlockError> {
        validate_transfer(&self.info, lba, blocks, out.len())?;
        let range = byte_range(&self.info, lba, blocks)?;
        #[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
        let data = self
            .data
            .lock()
            .map_err(|_| BlockError::BackendFailure("memory backend lock poisoned".into()))?;
        #[cfg(any(target_os = "none", target_arch = "wasm32"))]
        let data = self.data.lock();
        out.copy_from_slice(&data[range]);
        Ok(())
    }

    fn write_blocks(&self, lba: u64, blocks: u32, input: &[u8]) -> Result<(), BlockError> {
        if self.info.readonly {
            return Err(BlockError::ReadOnly);
        }
        validate_transfer(&self.info, lba, blocks, input.len())?;
        let range = byte_range(&self.info, lba, blocks)?;
        #[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
        let mut data = self
            .data
            .lock()
            .map_err(|_| BlockError::BackendFailure("memory backend lock poisoned".into()))?;
        #[cfg(any(target_os = "none", target_arch = "wasm32"))]
        let mut data = self.data.lock();
        data[range].copy_from_slice(input);
        Ok(())
    }

    fn flush(&self) -> Result<(), BlockError> {
        Ok(())
    }

    fn discard_blocks(&self, lba: u64, blocks: u32) -> Result<(), BlockError> {
        if self.info.readonly {
            return Err(BlockError::ReadOnly);
        }
        let len = checked_len_bytes(&self.info, blocks)?;
        let range = byte_range(&self.info, lba, blocks)?;
        #[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
        let mut data = self
            .data
            .lock()
            .map_err(|_| BlockError::BackendFailure("memory backend lock poisoned".into()))?;
        #[cfg(any(target_os = "none", target_arch = "wasm32"))]
        let mut data = self.data.lock();
        data[range].fill(0);
        debug_assert_eq!(len, checked_len_bytes(&self.info, blocks).unwrap_or(0));
        Ok(())
    }

    fn write_zeroes(&self, lba: u64, blocks: u32) -> Result<(), BlockError> {
        self.discard_blocks(lba, blocks)
    }
}

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
#[derive(Debug)]
pub struct FileBlockBackend {
    info: BlockDeviceInfo,
    file: Mutex<File>,
}

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
impl FileBlockBackend {
    pub fn open(
        path: impl AsRef<Path>,
        block_size: u32,
        readonly: bool,
    ) -> Result<Self, BlockError> {
        let path = path.as_ref();
        if block_size == 0 {
            return Err(BlockError::ProtocolError("block size must be > 0".into()));
        }
        let metadata = fs::metadata(path).map_err(BlockError::from)?;
        let len = metadata.len();
        let block_size_u64 = u64::from(block_size);
        if len % block_size_u64 != 0 {
            return Err(BlockError::Misaligned);
        }
        let file = OpenOptions::new()
            .read(true)
            .write(!readonly)
            .open(path)
            .map_err(BlockError::from)?;
        let info = BlockDeviceInfo {
            block_size,
            block_count: len / block_size_u64,
            readonly,
            supports_flush: true,
            supports_discard: false,
            supports_write_zeroes: true,
            model: "edgerun-file-backend".into(),
            serial: path.display().to_string(),
        };
        validate_device_info(&info)?;
        Ok(Self {
            info,
            file: Mutex::new(file),
        })
    }
}

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
impl BlockBackend for FileBlockBackend {
    fn info(&self) -> BlockDeviceInfo {
        self.info.clone()
    }

    fn read_blocks(&self, lba: u64, blocks: u32, out: &mut [u8]) -> Result<(), BlockError> {
        validate_transfer(&self.info, lba, blocks, out.len())?;
        let offset = byte_offset(&self.info, lba)?;
        let mut file = self
            .file
            .lock()
            .map_err(|_| BlockError::BackendFailure("file backend lock poisoned".into()))?;
        file.seek(SeekFrom::Start(offset))
            .map_err(BlockError::from)?;
        file.read_exact(out).map_err(BlockError::from)?;
        Ok(())
    }

    fn write_blocks(&self, lba: u64, blocks: u32, data: &[u8]) -> Result<(), BlockError> {
        if self.info.readonly {
            return Err(BlockError::ReadOnly);
        }
        validate_transfer(&self.info, lba, blocks, data.len())?;
        let offset = byte_offset(&self.info, lba)?;
        let mut file = self
            .file
            .lock()
            .map_err(|_| BlockError::BackendFailure("file backend lock poisoned".into()))?;
        file.seek(SeekFrom::Start(offset))
            .map_err(BlockError::from)?;
        file.write_all(data).map_err(BlockError::from)?;
        Ok(())
    }

    fn flush(&self) -> Result<(), BlockError> {
        let file = self
            .file
            .lock()
            .map_err(|_| BlockError::BackendFailure("file backend lock poisoned".into()))?;
        file.sync_all().map_err(BlockError::from)
    }

    fn write_zeroes(&self, lba: u64, blocks: u32) -> Result<(), BlockError> {
        if self.info.readonly {
            return Err(BlockError::ReadOnly);
        }
        let len = checked_len_bytes(&self.info, blocks)?;
        let offset = byte_offset(&self.info, lba)?;
        let mut file = self
            .file
            .lock()
            .map_err(|_| BlockError::BackendFailure("file backend lock poisoned".into()))?;
        file.seek(SeekFrom::Start(offset))
            .map_err(BlockError::from)?;
        const ZERO_CHUNK_LEN: usize = 1024 * 1024;
        let zero_chunk = [0_u8; ZERO_CHUNK_LEN];
        let mut remaining = len;
        while remaining > 0 {
            let chunk_len = remaining.min(ZERO_CHUNK_LEN);
            file.write_all(&zero_chunk[..chunk_len])
                .map_err(BlockError::from)?;
            remaining -= chunk_len;
        }
        Ok(())
    }
}

pub type RequestId = u64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockRequest {
    Handshake {
        protocol_version: u16,
    },
    GetInfo,
    Read {
        request_id: RequestId,
        lba: u64,
        blocks: u32,
    },
    Write {
        request_id: RequestId,
        lba: u64,
        blocks: u32,
        data: Vec<u8>,
    },
    Flush {
        request_id: RequestId,
    },
    Discard {
        request_id: RequestId,
        lba: u64,
        blocks: u32,
    },
    WriteZeroes {
        request_id: RequestId,
        lba: u64,
        blocks: u32,
    },
    Ping,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockResponse {
    HandshakeAck {
        protocol_version: u16,
    },
    Info(BlockDeviceInfo),
    ReadResult {
        request_id: RequestId,
        data: Vec<u8>,
    },
    WriteAck {
        request_id: RequestId,
    },
    FlushAck {
        request_id: RequestId,
    },
    DiscardAck {
        request_id: RequestId,
    },
    WriteZeroesAck {
        request_id: RequestId,
    },
    Pong,
    Error {
        request_id: Option<RequestId>,
        error: BlockError,
    },
}

pub fn handle_request<B: BlockBackend>(backend: &B, request: BlockRequest) -> BlockResponse {
    match request {
        BlockRequest::Handshake { protocol_version } => {
            if protocol_version == BLOCK_PROTOCOL_VERSION {
                BlockResponse::HandshakeAck {
                    protocol_version: BLOCK_PROTOCOL_VERSION,
                }
            } else {
                BlockResponse::Error {
                    request_id: None,
                    error: BlockError::ProtocolError(format!(
                        "unsupported protocol version {protocol_version}"
                    )),
                }
            }
        }
        BlockRequest::GetInfo => BlockResponse::Info(backend.info()),
        BlockRequest::Read {
            request_id,
            lba,
            blocks,
        } => {
            let info = backend.info();
            match checked_len_bytes(&info, blocks) {
                Ok(len) => {
                    let mut data = vec![0_u8; len];
                    match backend.read_blocks(lba, blocks, &mut data) {
                        Ok(()) => BlockResponse::ReadResult { request_id, data },
                        Err(error) => BlockResponse::Error {
                            request_id: Some(request_id),
                            error,
                        },
                    }
                }
                Err(error) => BlockResponse::Error {
                    request_id: Some(request_id),
                    error,
                },
            }
        }
        BlockRequest::Write {
            request_id,
            lba,
            blocks,
            data,
        } => match backend.write_blocks(lba, blocks, &data) {
            Ok(()) => BlockResponse::WriteAck { request_id },
            Err(error) => BlockResponse::Error {
                request_id: Some(request_id),
                error,
            },
        },
        BlockRequest::Flush { request_id } => match backend.flush() {
            Ok(()) => BlockResponse::FlushAck { request_id },
            Err(error) => BlockResponse::Error {
                request_id: Some(request_id),
                error,
            },
        },
        BlockRequest::Discard {
            request_id,
            lba,
            blocks,
        } => match backend.discard_blocks(lba, blocks) {
            Ok(()) => BlockResponse::DiscardAck { request_id },
            Err(error) => BlockResponse::Error {
                request_id: Some(request_id),
                error,
            },
        },
        BlockRequest::WriteZeroes {
            request_id,
            lba,
            blocks,
        } => match backend.write_zeroes(lba, blocks) {
            Ok(()) => BlockResponse::WriteZeroesAck { request_id },
            Err(error) => BlockResponse::Error {
                request_id: Some(request_id),
                error,
            },
        },
        BlockRequest::Ping => BlockResponse::Pong,
    }
}

pub struct BlockClient<T> {
    stream: T,
    next_request_id: RequestId,
}

impl<T: Read + Write> BlockClient<T> {
    pub fn new(stream: T) -> Self {
        Self {
            stream,
            next_request_id: 1,
        }
    }

    pub fn handshake(&mut self) -> Result<(), BlockError> {
        match self.call(BlockRequest::Handshake {
            protocol_version: BLOCK_PROTOCOL_VERSION,
        })? {
            BlockResponse::HandshakeAck { protocol_version }
                if protocol_version == BLOCK_PROTOCOL_VERSION =>
            {
                Ok(())
            }
            response => Err(BlockError::ProtocolError(format!(
                "unexpected handshake response: {response:?}"
            ))),
        }
    }

    pub fn info(&mut self) -> Result<BlockDeviceInfo, BlockError> {
        match self.call(BlockRequest::GetInfo)? {
            BlockResponse::Info(info) => Ok(info),
            response => Err(BlockError::ProtocolError(format!(
                "unexpected info response: {response:?}"
            ))),
        }
    }

    pub fn ping(&mut self) -> Result<(), BlockError> {
        match self.call(BlockRequest::Ping)? {
            BlockResponse::Pong => Ok(()),
            response => Err(BlockError::ProtocolError(format!(
                "unexpected ping response: {response:?}"
            ))),
        }
    }

    pub fn read_blocks(&mut self, lba: u64, blocks: u32) -> Result<Vec<u8>, BlockError> {
        let request_id = self.allocate_request_id();
        match self.call(BlockRequest::Read {
            request_id,
            lba,
            blocks,
        })? {
            BlockResponse::ReadResult {
                request_id: response_id,
                data,
            } if response_id == request_id => Ok(data),
            BlockResponse::Error {
                request_id: Some(response_id),
                error,
            } if response_id == request_id => Err(error),
            response => Err(BlockError::ProtocolError(format!(
                "unexpected read response: {response:?}"
            ))),
        }
    }

    pub fn write_blocks(&mut self, lba: u64, blocks: u32, data: Vec<u8>) -> Result<(), BlockError> {
        let request_id = self.allocate_request_id();
        match self.call(BlockRequest::Write {
            request_id,
            lba,
            blocks,
            data,
        })? {
            BlockResponse::WriteAck {
                request_id: response_id,
            } if response_id == request_id => Ok(()),
            BlockResponse::Error {
                request_id: Some(response_id),
                error,
            } if response_id == request_id => Err(error),
            response => Err(BlockError::ProtocolError(format!(
                "unexpected write response: {response:?}"
            ))),
        }
    }

    pub fn flush(&mut self) -> Result<(), BlockError> {
        let request_id = self.allocate_request_id();
        match self.call(BlockRequest::Flush { request_id })? {
            BlockResponse::FlushAck {
                request_id: response_id,
            } if response_id == request_id => Ok(()),
            BlockResponse::Error {
                request_id: Some(response_id),
                error,
            } if response_id == request_id => Err(error),
            response => Err(BlockError::ProtocolError(format!(
                "unexpected flush response: {response:?}"
            ))),
        }
    }

    pub fn discard_blocks(&mut self, lba: u64, blocks: u32) -> Result<(), BlockError> {
        let request_id = self.allocate_request_id();
        match self.call(BlockRequest::Discard {
            request_id,
            lba,
            blocks,
        })? {
            BlockResponse::DiscardAck {
                request_id: response_id,
            } if response_id == request_id => Ok(()),
            BlockResponse::Error {
                request_id: Some(response_id),
                error,
            } if response_id == request_id => Err(error),
            response => Err(BlockError::ProtocolError(format!(
                "unexpected discard response: {response:?}"
            ))),
        }
    }

    pub fn write_zeroes(&mut self, lba: u64, blocks: u32) -> Result<(), BlockError> {
        let request_id = self.allocate_request_id();
        match self.call(BlockRequest::WriteZeroes {
            request_id,
            lba,
            blocks,
        })? {
            BlockResponse::WriteZeroesAck {
                request_id: response_id,
            } if response_id == request_id => Ok(()),
            BlockResponse::Error {
                request_id: Some(response_id),
                error,
            } if response_id == request_id => Err(error),
            response => Err(BlockError::ProtocolError(format!(
                "unexpected write_zeroes response: {response:?}"
            ))),
        }
    }

    pub fn into_inner(self) -> T {
        self.stream
    }

    fn call(&mut self, request: BlockRequest) -> Result<BlockResponse, BlockError> {
        send_request(&mut self.stream, &request)?;
        receive_response(&mut self.stream)
    }

    fn allocate_request_id(&mut self) -> RequestId {
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);
        request_id
    }
}

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
impl BlockClient<UnixStream> {
    pub fn connect_unix(path: impl AsRef<Path>) -> Result<Self, BlockError> {
        let stream = UnixStream::connect(path).map_err(BlockError::from)?;
        Ok(Self::new(stream))
    }
}

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
impl BlockClient<TcpStream> {
    pub fn connect_tcp(addr: impl ToSocketAddrs) -> Result<Self, BlockError> {
        let stream = TcpStream::connect(addr).map_err(BlockError::from)?;
        Ok(Self::new(stream))
    }
}

pub struct BlockServer<T, B> {
    stream: T,
    backend: B,
}

impl<T: Read + Write, B: BlockBackend> BlockServer<T, B> {
    pub fn new(stream: T, backend: B) -> Self {
        Self { stream, backend }
    }

    pub fn serve_once(&mut self) -> Result<(), BlockError> {
        let request = receive_request(&mut self.stream)?;
        let response = handle_request(&self.backend, request);
        send_response(&mut self.stream, &response)
    }

    pub fn serve_until_eof(&mut self) -> Result<(), BlockError> {
        loop {
            match receive_request(&mut self.stream) {
                Ok(request) => {
                    let response = handle_request(&self.backend, request);
                    send_response(&mut self.stream, &response)?;
                }
                Err(BlockError::ProtocolError(message)) if message == "unexpected EOF" => {
                    return Ok(());
                }
                Err(error) => return Err(error),
            }
        }
    }
}

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
pub struct UnixBlockServer<B> {
    listener: UnixListener,
    backend: Arc<B>,
}

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
impl<B: BlockBackend> UnixBlockServer<B> {
    pub fn bind(path: impl AsRef<Path>, backend: B) -> Result<Self, BlockError> {
        Self::bind_shared(path, Arc::new(backend))
    }

    pub fn bind_shared(path: impl AsRef<Path>, backend: Arc<B>) -> Result<Self, BlockError> {
        let path = path.as_ref();
        if path.exists() {
            fs::remove_file(path).map_err(BlockError::from)?;
        }
        let listener = UnixListener::bind(path).map_err(BlockError::from)?;
        Ok(Self { listener, backend })
    }

    pub fn local_addr(&self) -> Result<std::os::unix::net::SocketAddr, BlockError> {
        self.listener.local_addr().map_err(BlockError::from)
    }

    pub fn accept_once(&self) -> Result<(), BlockError> {
        let (stream, _) = self.listener.accept().map_err(BlockError::from)?;
        let mut server = BlockServer::new(stream, Arc::clone(&self.backend));
        server.serve_until_eof()
    }

    pub fn serve_forever(&self) -> Result<(), BlockError> {
        loop {
            self.accept_once()?;
        }
    }
}

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
impl<B> Drop for UnixBlockServer<B> {
    fn drop(&mut self) {
        if let Ok(addr) = self.listener.local_addr() {
            if let Some(path) = addr.as_pathname() {
                let _ = fs::remove_file(path);
            }
        }
    }
}

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
pub struct TcpBlockServer<B> {
    listener: TcpListener,
    backend: Arc<B>,
}

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
impl<B: BlockBackend> TcpBlockServer<B> {
    pub fn bind(addr: impl ToSocketAddrs, backend: B) -> Result<Self, BlockError> {
        Self::bind_shared(addr, Arc::new(backend))
    }

    pub fn bind_shared(addr: impl ToSocketAddrs, backend: Arc<B>) -> Result<Self, BlockError> {
        let listener = TcpListener::bind(addr).map_err(BlockError::from)?;
        Ok(Self { listener, backend })
    }

    pub fn local_addr(&self) -> Result<std::net::SocketAddr, BlockError> {
        self.listener.local_addr().map_err(BlockError::from)
    }

    pub fn accept_once(&self) -> Result<(), BlockError> {
        let (stream, _) = self.listener.accept().map_err(BlockError::from)?;
        let mut server = BlockServer::new(stream, Arc::clone(&self.backend));
        server.serve_until_eof()
    }

    pub fn serve_forever(&self) -> Result<(), BlockError> {
        loop {
            self.accept_once()?;
        }
    }
}

#[cfg(any(target_os = "none", target_arch = "wasm32"))]
#[derive(Debug)]
pub struct FileBlockBackend;

#[cfg(any(target_os = "none", target_arch = "wasm32"))]
impl FileBlockBackend {
    pub fn open<P>(_path: P, _block_size: u32, _readonly: bool) -> Result<Self, BlockError> {
        Err(BlockError::Unsupported)
    }
}

#[cfg(any(target_os = "none", target_arch = "wasm32"))]
pub struct UnixBlockServer<B> {
    _backend: core::marker::PhantomData<B>,
}

#[cfg(any(target_os = "none", target_arch = "wasm32"))]
pub struct TcpBlockServer<B> {
    _backend: core::marker::PhantomData<B>,
}

pub fn send_request<W: Write>(writer: &mut W, request: &BlockRequest) -> Result<(), BlockError> {
    let payload = encode_request(request)?;
    write_frame(writer, &payload)
}

pub fn receive_request<R: Read>(reader: &mut R) -> Result<BlockRequest, BlockError> {
    let payload = read_frame(reader)?;
    decode_request(&payload)
}

pub fn send_response<W: Write>(writer: &mut W, response: &BlockResponse) -> Result<(), BlockError> {
    let payload = encode_response(response)?;
    write_frame(writer, &payload)
}

pub fn receive_response<R: Read>(reader: &mut R) -> Result<BlockResponse, BlockError> {
    let payload = read_frame(reader)?;
    decode_response(&payload)
}

pub fn encode_request_frame(request: &BlockRequest) -> Result<Vec<u8>, BlockError> {
    frame_payload(&encode_request(request)?)
}

pub fn decode_request_frame(frame: &[u8]) -> Result<BlockRequest, BlockError> {
    decode_request(&decode_frame(frame)?)
}

pub fn encode_response_frame(response: &BlockResponse) -> Result<Vec<u8>, BlockError> {
    frame_payload(&encode_response(response)?)
}

pub fn decode_response_frame(frame: &[u8]) -> Result<BlockResponse, BlockError> {
    decode_response(&decode_frame(frame)?)
}

pub fn checked_len_bytes(info: &BlockDeviceInfo, blocks: u32) -> Result<usize, BlockError> {
    (blocks as usize)
        .checked_mul(info.block_size as usize)
        .ok_or_else(|| BlockError::ProtocolError("byte length overflow".into()))
}

pub fn validate_range(info: &BlockDeviceInfo, lba: u64, blocks: u32) -> Result<(), BlockError> {
    if blocks == 0 {
        return Err(BlockError::ProtocolError("block count must be > 0".into()));
    }
    let end = lba
        .checked_add(u64::from(blocks))
        .ok_or(BlockError::OutOfRange)?;
    if end > info.block_count {
        return Err(BlockError::OutOfRange);
    }
    Ok(())
}

fn validate_transfer(
    info: &BlockDeviceInfo,
    lba: u64,
    blocks: u32,
    actual_len: usize,
) -> Result<(), BlockError> {
    validate_range(info, lba, blocks)?;
    let expected_len = checked_len_bytes(info, blocks)?;
    if actual_len != expected_len {
        return Err(BlockError::Misaligned);
    }
    Ok(())
}

fn validate_device_info(info: &BlockDeviceInfo) -> Result<(), BlockError> {
    if info.block_size == 0 {
        return Err(BlockError::ProtocolError("block size must be > 0".into()));
    }
    if info.block_count == 0 {
        return Err(BlockError::ProtocolError("block count must be > 0".into()));
    }
    let _ = total_size_len(info)?;
    Ok(())
}

fn total_size_len(info: &BlockDeviceInfo) -> Result<usize, BlockError> {
    let bytes = info
        .block_count
        .checked_mul(u64::from(info.block_size))
        .ok_or_else(|| BlockError::ProtocolError("device size overflow".into()))?;
    usize::try_from(bytes).map_err(|_| BlockError::ProtocolError("device size too large".into()))
}

fn byte_offset(info: &BlockDeviceInfo, lba: u64) -> Result<u64, BlockError> {
    lba.checked_mul(u64::from(info.block_size))
        .ok_or_else(|| BlockError::ProtocolError("byte offset overflow".into()))
}

fn byte_range(
    info: &BlockDeviceInfo,
    lba: u64,
    blocks: u32,
) -> Result<core::ops::Range<usize>, BlockError> {
    validate_range(info, lba, blocks)?;
    let start = byte_offset(info, lba)?;
    let len = checked_len_bytes(info, blocks)?;
    let start = usize::try_from(start)
        .map_err(|_| BlockError::ProtocolError("byte offset too large".into()))?;
    let end = start
        .checked_add(len)
        .ok_or_else(|| BlockError::ProtocolError("byte range overflow".into()))?;
    Ok(start..end)
}

fn write_frame<W: Write>(writer: &mut W, payload: &[u8]) -> Result<(), BlockError> {
    let frame = frame_payload(payload)?;
    writer.write_all(&frame).map_err(BlockError::from)?;
    writer.flush().map_err(BlockError::from)
}

fn frame_payload(payload: &[u8]) -> Result<Vec<u8>, BlockError> {
    let len = u32::try_from(payload.len())
        .map_err(|_| BlockError::ProtocolError("frame too large to encode".into()))?;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(payload);
    Ok(frame)
}

fn read_frame<R: Read>(reader: &mut R) -> Result<Vec<u8>, BlockError> {
    let mut len_bytes = [0_u8; 4];
    match reader.read_exact(&mut len_bytes) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => {
            return Err(BlockError::ProtocolError("unexpected EOF".into()));
        }
        Err(err) => return Err(BlockError::from(err)),
    }
    let len = read_u32_le(&len_bytes, 0) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(BlockError::ProtocolError(
            "frame exceeds maximum size".into(),
        ));
    }
    let mut payload = vec![0_u8; len];
    reader.read_exact(&mut payload).map_err(|err| {
        if err.kind() == io::ErrorKind::UnexpectedEof {
            BlockError::ProtocolError("unexpected EOF".into())
        } else {
            BlockError::from(err)
        }
    })?;
    Ok(payload)
}

fn decode_frame(frame: &[u8]) -> Result<Vec<u8>, BlockError> {
    if frame.len() < 4 {
        return Err(BlockError::ProtocolError("truncated frame header".into()));
    }
    let len = read_u32_le(frame, 0) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(BlockError::ProtocolError(
            "frame exceeds maximum size".into(),
        ));
    }
    let end = 4_usize
        .checked_add(len)
        .ok_or_else(|| BlockError::ProtocolError("frame length overflow".into()))?;
    if frame.len() != end {
        return Err(BlockError::ProtocolError("frame length mismatch".into()));
    }
    Ok(frame[4..].to_vec())
}

fn encode_request(request: &BlockRequest) -> Result<Vec<u8>, BlockError> {
    let mut bytes = Vec::new();
    match request {
        BlockRequest::Handshake { protocol_version } => {
            push_u8(&mut bytes, 0);
            push_u16(&mut bytes, *protocol_version);
        }
        BlockRequest::GetInfo => push_u8(&mut bytes, 1),
        BlockRequest::Read {
            request_id,
            lba,
            blocks,
        } => {
            push_u8(&mut bytes, 2);
            push_u64(&mut bytes, *request_id);
            push_u64(&mut bytes, *lba);
            push_u32(&mut bytes, *blocks);
        }
        BlockRequest::Write {
            request_id,
            lba,
            blocks,
            data,
        } => {
            push_u8(&mut bytes, 3);
            push_u64(&mut bytes, *request_id);
            push_u64(&mut bytes, *lba);
            push_u32(&mut bytes, *blocks);
            push_bytes(&mut bytes, data)?;
        }
        BlockRequest::Flush { request_id } => {
            push_u8(&mut bytes, 4);
            push_u64(&mut bytes, *request_id);
        }
        BlockRequest::Discard {
            request_id,
            lba,
            blocks,
        } => {
            push_u8(&mut bytes, 5);
            push_u64(&mut bytes, *request_id);
            push_u64(&mut bytes, *lba);
            push_u32(&mut bytes, *blocks);
        }
        BlockRequest::WriteZeroes {
            request_id,
            lba,
            blocks,
        } => {
            push_u8(&mut bytes, 6);
            push_u64(&mut bytes, *request_id);
            push_u64(&mut bytes, *lba);
            push_u32(&mut bytes, *blocks);
        }
        BlockRequest::Ping => push_u8(&mut bytes, 7),
    }
    Ok(bytes)
}

fn decode_request(payload: &[u8]) -> Result<BlockRequest, BlockError> {
    let mut cursor = Cursor::new(payload);
    let tag = cursor.read_u8()?;
    let request = match tag {
        0 => BlockRequest::Handshake {
            protocol_version: cursor.read_u16()?,
        },
        1 => BlockRequest::GetInfo,
        2 => BlockRequest::Read {
            request_id: cursor.read_u64()?,
            lba: cursor.read_u64()?,
            blocks: cursor.read_u32()?,
        },
        3 => BlockRequest::Write {
            request_id: cursor.read_u64()?,
            lba: cursor.read_u64()?,
            blocks: cursor.read_u32()?,
            data: cursor.read_bytes()?,
        },
        4 => BlockRequest::Flush {
            request_id: cursor.read_u64()?,
        },
        5 => BlockRequest::Discard {
            request_id: cursor.read_u64()?,
            lba: cursor.read_u64()?,
            blocks: cursor.read_u32()?,
        },
        6 => BlockRequest::WriteZeroes {
            request_id: cursor.read_u64()?,
            lba: cursor.read_u64()?,
            blocks: cursor.read_u32()?,
        },
        7 => BlockRequest::Ping,
        _ => {
            return Err(BlockError::ProtocolError(format!(
                "unknown request tag {tag}"
            )));
        }
    };
    cursor.finish()?;
    Ok(request)
}

fn encode_response(response: &BlockResponse) -> Result<Vec<u8>, BlockError> {
    let mut bytes = Vec::new();
    match response {
        BlockResponse::HandshakeAck { protocol_version } => {
            push_u8(&mut bytes, 0);
            push_u16(&mut bytes, *protocol_version);
        }
        BlockResponse::Info(info) => {
            push_u8(&mut bytes, 1);
            encode_device_info(&mut bytes, info)?;
        }
        BlockResponse::ReadResult { request_id, data } => {
            push_u8(&mut bytes, 2);
            push_u64(&mut bytes, *request_id);
            push_bytes(&mut bytes, data)?;
        }
        BlockResponse::WriteAck { request_id } => {
            push_u8(&mut bytes, 3);
            push_u64(&mut bytes, *request_id);
        }
        BlockResponse::FlushAck { request_id } => {
            push_u8(&mut bytes, 4);
            push_u64(&mut bytes, *request_id);
        }
        BlockResponse::DiscardAck { request_id } => {
            push_u8(&mut bytes, 5);
            push_u64(&mut bytes, *request_id);
        }
        BlockResponse::WriteZeroesAck { request_id } => {
            push_u8(&mut bytes, 6);
            push_u64(&mut bytes, *request_id);
        }
        BlockResponse::Pong => push_u8(&mut bytes, 7),
        BlockResponse::Error { request_id, error } => {
            push_u8(&mut bytes, 8);
            match request_id {
                Some(value) => {
                    push_u8(&mut bytes, 1);
                    push_u64(&mut bytes, *value);
                }
                None => push_u8(&mut bytes, 0),
            }
            encode_block_error(&mut bytes, error)?;
        }
    }
    Ok(bytes)
}

fn decode_response(payload: &[u8]) -> Result<BlockResponse, BlockError> {
    let mut cursor = Cursor::new(payload);
    let tag = cursor.read_u8()?;
    let response = match tag {
        0 => BlockResponse::HandshakeAck {
            protocol_version: cursor.read_u16()?,
        },
        1 => BlockResponse::Info(decode_device_info(&mut cursor)?),
        2 => BlockResponse::ReadResult {
            request_id: cursor.read_u64()?,
            data: cursor.read_bytes()?,
        },
        3 => BlockResponse::WriteAck {
            request_id: cursor.read_u64()?,
        },
        4 => BlockResponse::FlushAck {
            request_id: cursor.read_u64()?,
        },
        5 => BlockResponse::DiscardAck {
            request_id: cursor.read_u64()?,
        },
        6 => BlockResponse::WriteZeroesAck {
            request_id: cursor.read_u64()?,
        },
        7 => BlockResponse::Pong,
        8 => {
            let request_id = match cursor.read_u8()? {
                0 => None,
                1 => Some(cursor.read_u64()?),
                flag => {
                    return Err(BlockError::ProtocolError(format!(
                        "invalid response request-id flag {flag}"
                    )));
                }
            };
            BlockResponse::Error {
                request_id,
                error: decode_block_error(&mut cursor)?,
            }
        }
        _ => {
            return Err(BlockError::ProtocolError(format!(
                "unknown response tag {tag}"
            )));
        }
    };
    cursor.finish()?;
    Ok(response)
}

fn encode_device_info(bytes: &mut Vec<u8>, info: &BlockDeviceInfo) -> Result<(), BlockError> {
    push_u32(bytes, info.block_size);
    push_u64(bytes, info.block_count);
    push_bool(bytes, info.readonly);
    push_bool(bytes, info.supports_flush);
    push_bool(bytes, info.supports_discard);
    push_bool(bytes, info.supports_write_zeroes);
    push_string(bytes, &info.model)?;
    push_string(bytes, &info.serial)
}

fn decode_device_info(cursor: &mut Cursor<'_>) -> Result<BlockDeviceInfo, BlockError> {
    Ok(BlockDeviceInfo {
        block_size: cursor.read_u32()?,
        block_count: cursor.read_u64()?,
        readonly: cursor.read_bool()?,
        supports_flush: cursor.read_bool()?,
        supports_discard: cursor.read_bool()?,
        supports_write_zeroes: cursor.read_bool()?,
        model: cursor.read_string()?,
        serial: cursor.read_string()?,
    })
}

fn encode_block_error(bytes: &mut Vec<u8>, error: &BlockError) -> Result<(), BlockError> {
    match error {
        BlockError::OutOfRange => push_u8(bytes, 0),
        BlockError::ReadOnly => push_u8(bytes, 1),
        BlockError::Misaligned => push_u8(bytes, 2),
        BlockError::Unsupported => push_u8(bytes, 3),
        BlockError::BackendFailure(message) => {
            push_u8(bytes, 4);
            push_string(bytes, message)?;
        }
        BlockError::ProtocolError(message) => {
            push_u8(bytes, 5);
            push_string(bytes, message)?;
        }
        BlockError::NotReady => push_u8(bytes, 6),
        BlockError::Timeout => push_u8(bytes, 7),
    }
    Ok(())
}

fn decode_block_error(cursor: &mut Cursor<'_>) -> Result<BlockError, BlockError> {
    let tag = cursor.read_u8()?;
    match tag {
        0 => Ok(BlockError::OutOfRange),
        1 => Ok(BlockError::ReadOnly),
        2 => Ok(BlockError::Misaligned),
        3 => Ok(BlockError::Unsupported),
        4 => Ok(BlockError::BackendFailure(cursor.read_string()?)),
        5 => Ok(BlockError::ProtocolError(cursor.read_string()?)),
        6 => Ok(BlockError::NotReady),
        7 => Ok(BlockError::Timeout),
        _ => Err(BlockError::ProtocolError(format!(
            "unknown error tag {tag}"
        ))),
    }
}

fn push_u8(bytes: &mut Vec<u8>, value: u8) {
    bytes.push(value);
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_bool(bytes: &mut Vec<u8>, value: bool) {
    bytes.push(u8::from(value));
}

fn push_bytes(bytes: &mut Vec<u8>, value: &[u8]) -> Result<(), BlockError> {
    let len = u32::try_from(value.len())
        .map_err(|_| BlockError::ProtocolError("payload field too large".into()))?;
    push_u32(bytes, len);
    bytes.extend_from_slice(value);
    Ok(())
}

fn push_string(bytes: &mut Vec<u8>, value: &str) -> Result<(), BlockError> {
    push_bytes(bytes, value.as_bytes())
}

struct Cursor<'a> {
    payload: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(payload: &'a [u8]) -> Self {
        Self { payload, offset: 0 }
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], BlockError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| BlockError::ProtocolError("cursor overflow".into()))?;
        let slice = self
            .payload
            .get(self.offset..end)
            .ok_or_else(|| BlockError::ProtocolError("truncated payload".into()))?;
        self.offset = end;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, BlockError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, BlockError> {
        Ok(read_u16_le(self.read_exact(2)?, 0))
    }

    fn read_u32(&mut self) -> Result<u32, BlockError> {
        Ok(read_u32_le(self.read_exact(4)?, 0))
    }

    fn read_u64(&mut self) -> Result<u64, BlockError> {
        Ok(read_u64_le(self.read_exact(8)?, 0))
    }

    fn read_bool(&mut self) -> Result<bool, BlockError> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(BlockError::ProtocolError(format!(
                "invalid bool tag {value}"
            ))),
        }
    }

    fn read_bytes(&mut self) -> Result<Vec<u8>, BlockError> {
        let len = self.read_u32()? as usize;
        Ok(self.read_exact(len)?.to_vec())
    }

    fn read_string(&mut self) -> Result<String, BlockError> {
        String::from_utf8(self.read_bytes()?)
            .map_err(|err| BlockError::ProtocolError(err.to_string()))
    }

    fn finish(&self) -> Result<(), BlockError> {
        if self.offset == self.payload.len() {
            Ok(())
        } else {
            Err(BlockError::ProtocolError(
                "trailing bytes in payload".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::{create, VirtualDiskFormat, VirtualDiskSpec};
    use std::env;
    use std::thread;

    fn temp_dir() -> std::path::PathBuf {
        let mut path = env::temp_dir();
        path.push(format!(
            "edgerun-remote-block-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        path
    }

    fn test_info() -> BlockDeviceInfo {
        BlockDeviceInfo {
            block_size: 512,
            block_count: 8,
            readonly: false,
            supports_flush: true,
            supports_discard: true,
            supports_write_zeroes: true,
            model: "edgerun-test".into(),
            serial: "test-serial".into(),
        }
    }

    #[test]
    fn request_roundtrip() {
        let request = BlockRequest::Write {
            request_id: 7,
            lba: 3,
            blocks: 2,
            data: vec![1, 2, 3, 4],
        };
        let encoded = encode_request(&request).unwrap();
        let decoded = decode_request(&encoded).unwrap();
        assert_eq!(decoded, request);
    }

    #[test]
    fn response_roundtrip() {
        let response = BlockResponse::Info(test_info());
        let encoded = encode_response(&response).unwrap();
        let decoded = decode_response(&encoded).unwrap();
        assert_eq!(decoded, response);
    }

    #[test]
    fn framed_request_response_roundtrip() {
        let request = BlockRequest::Read {
            request_id: 8,
            lba: 1,
            blocks: 1,
        };
        let request_frame = encode_request_frame(&request).unwrap();
        assert_eq!(decode_request_frame(&request_frame).unwrap(), request);

        let response = BlockResponse::ReadResult {
            request_id: 8,
            data: vec![0x88; 512],
        };
        let response_frame = encode_response_frame(&response).unwrap();
        assert_eq!(decode_response_frame(&response_frame).unwrap(), response);
    }

    #[test]
    fn memory_backend_read_write_zeroes() {
        let backend = MemoryBlockBackend::new(test_info()).unwrap();
        let bytes = vec![0x5a; 1024];
        backend.write_blocks(1, 2, &bytes).unwrap();
        let mut out = vec![0_u8; 1024];
        backend.read_blocks(1, 2, &mut out).unwrap();
        assert_eq!(out, bytes);
        backend.write_zeroes(1, 2).unwrap();
        backend.read_blocks(1, 2, &mut out).unwrap();
        assert_eq!(out, vec![0_u8; 1024]);
    }

    #[test]
    fn memory_backend_rejects_out_of_range() {
        let backend = MemoryBlockBackend::new(test_info()).unwrap();
        let err = backend
            .read_blocks(7, 2, &mut vec![0_u8; 1024])
            .unwrap_err();
        assert_eq!(err, BlockError::OutOfRange);
    }

    #[test]
    fn handle_request_reads_from_backend() {
        let backend = MemoryBlockBackend::new(test_info()).unwrap();
        backend.write_blocks(0, 1, &[9_u8; 512]).unwrap();
        let response = handle_request(
            &backend,
            BlockRequest::Read {
                request_id: 42,
                lba: 0,
                blocks: 1,
            },
        );
        assert_eq!(
            response,
            BlockResponse::ReadResult {
                request_id: 42,
                data: vec![9_u8; 512],
            }
        );
    }

    #[test]
    fn client_and_server_exchange_requests() {
        let (client_stream, server_stream) = UnixStream::pair().unwrap();
        let backend = MemoryBlockBackend::new(test_info()).unwrap();
        let server = thread::spawn(move || {
            let mut server = BlockServer::new(server_stream, backend);
            server.serve_until_eof().unwrap();
        });

        let mut client = BlockClient::new(client_stream);
        client.handshake().unwrap();
        let info = client.info().unwrap();
        assert_eq!(info, test_info());
        client.write_blocks(2, 1, vec![3_u8; 512]).unwrap();
        assert_eq!(client.read_blocks(2, 1).unwrap(), vec![3_u8; 512]);
        client.ping().unwrap();
        drop(client);
        server.join().unwrap();
    }

    #[test]
    fn file_backend_reads_and_writes_blocks() {
        let root = temp_dir();
        fs::create_dir_all(&root).unwrap();
        let path = root.join("disk.raw");
        create(&VirtualDiskSpec {
            path: path.clone(),
            size_bytes: 4096,
            format: VirtualDiskFormat::Raw,
            sparse: true,
        })
        .unwrap();

        let backend = FileBlockBackend::open(&path, 512, false).unwrap();
        backend.write_blocks(1, 2, &[0x44_u8; 1024]).unwrap();
        backend.flush().unwrap();
        let mut out = vec![0_u8; 1024];
        backend.read_blocks(1, 2, &mut out).unwrap();
        assert_eq!(out, vec![0x44_u8; 1024]);
        backend.write_zeroes(1, 2).unwrap();
        backend.read_blocks(1, 2, &mut out).unwrap();
        assert_eq!(out, vec![0_u8; 1024]);

        fs::remove_file(path).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn unix_listener_server_accepts_client_connection() {
        let root = temp_dir();
        fs::create_dir_all(&root).unwrap();
        let socket_path = root.join("block.sock");
        let backend = Arc::new(MemoryBlockBackend::new(test_info()).unwrap());
        let server_backend = Arc::clone(&backend);
        let server = UnixBlockServer::bind_shared(&socket_path, server_backend).unwrap();

        let worker = thread::spawn(move || {
            server.accept_once().unwrap();
        });

        let mut client = BlockClient::connect_unix(&socket_path).unwrap();
        client.handshake().unwrap();
        client.write_blocks(0, 1, vec![0x2a; 512]).unwrap();
        assert_eq!(client.read_blocks(0, 1).unwrap(), vec![0x2a; 512]);
        drop(client);
        worker.join().unwrap();

        let mut out = vec![0_u8; 512];
        backend.read_blocks(0, 1, &mut out).unwrap();
        assert_eq!(out, vec![0x2a; 512]);
        fs::remove_dir(root).unwrap();
    }
}
