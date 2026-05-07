use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::option::Option::{self, None, Some};
use core::result::Result::{self, Err, Ok};
use core::{debug_assert_eq, fmt, write};
#[cfg(any(target_os = "none", target_arch = "wasm32"))]
use edgerun_encoding::io::{self, Read, Write};
#[cfg(any(target_os = "none", target_arch = "wasm32"))]
use edgerun_rt::Mutex;
#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
use std::io::{self, Read, Write};
#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
use std::sync::Mutex;

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
pub use host::{FileBlockBackend, TcpBlockServer, UnixBlockServer};

pub use edgerun_protocols::block::{
    byte_offset, byte_range, checked_len_bytes, decode_request_frame, decode_request_payload,
    decode_response_frame, decode_response_payload, encode_request_frame, encode_request_payload,
    encode_response_frame, encode_response_payload, frame_payload, handle_request, total_size_len,
    validate_device_info, validate_range, validate_transfer, BlockBackend, BlockDeviceInfo,
    BlockError, BlockRequest, BlockResponse, RequestId, BLOCK_PROTOCOL_VERSION,
};

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

pub mod protocol {
    pub use super::{
        checked_len_bytes, handle_request, validate_range, BlockBackend, BlockDeviceInfo,
        BlockError, BlockRequest, BlockResponse, MemoryBlockBackend, RequestId,
        BLOCK_PROTOCOL_VERSION,
    };
}

pub mod transport {
    pub use super::{
        receive_request, receive_response, send_request, send_response, BlockClient, BlockServer,
    };
}

pub mod wire {
    pub use super::{
        decode_request_frame, decode_response_frame, encode_request_frame, encode_response_frame,
    };
}

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
pub mod host {
    use super::*;
    use std::fs::{self, File, OpenOptions};
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::net::{TcpListener, TcpStream, ToSocketAddrs};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::Path;

    #[derive(Debug)]
    pub struct FileBlockBackend {
        info: BlockDeviceInfo,
        file: Mutex<File>,
    }

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
            let metadata = fs::metadata(path).map_err(block_io_error)?;
            let len = metadata.len();
            let block_size_u64 = u64::from(block_size);
            if len % block_size_u64 != 0 {
                return Err(BlockError::Misaligned);
            }
            let file = OpenOptions::new()
                .read(true)
                .write(!readonly)
                .open(path)
                .map_err(block_io_error)?;
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
            file.seek(SeekFrom::Start(offset)).map_err(block_io_error)?;
            file.read_exact(out).map_err(block_io_error)?;
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
            file.seek(SeekFrom::Start(offset)).map_err(block_io_error)?;
            file.write_all(data).map_err(block_io_error)?;
            Ok(())
        }

        fn flush(&self) -> Result<(), BlockError> {
            let file = self
                .file
                .lock()
                .map_err(|_| BlockError::BackendFailure("file backend lock poisoned".into()))?;
            file.sync_all().map_err(block_io_error)
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
            file.seek(SeekFrom::Start(offset)).map_err(block_io_error)?;
            const ZERO_CHUNK_LEN: usize = 1024 * 1024;
            let zero_chunk = [0_u8; ZERO_CHUNK_LEN];
            let mut remaining = len;
            while remaining > 0 {
                let chunk_len = remaining.min(ZERO_CHUNK_LEN);
                file.write_all(&zero_chunk[..chunk_len])
                    .map_err(block_io_error)?;
                remaining -= chunk_len;
            }
            Ok(())
        }
    }

    impl BlockClient<UnixStream> {
        pub fn connect_unix(path: impl AsRef<Path>) -> Result<Self, BlockError> {
            let stream = UnixStream::connect(path).map_err(block_io_error)?;
            Ok(Self::new(stream))
        }
    }

    impl BlockClient<TcpStream> {
        pub fn connect_tcp(addr: impl ToSocketAddrs) -> Result<Self, BlockError> {
            let stream = TcpStream::connect(addr).map_err(block_io_error)?;
            Ok(Self::new(stream))
        }
    }

    pub struct UnixBlockServer<B> {
        listener: UnixListener,
        backend: Arc<B>,
    }

    impl<B: BlockBackend + Send + Sync + 'static> UnixBlockServer<B> {
        pub fn bind(path: impl AsRef<Path>, backend: B) -> Result<Self, BlockError> {
            Self::bind_shared(path, Arc::new(backend))
        }

        pub fn bind_shared(path: impl AsRef<Path>, backend: Arc<B>) -> Result<Self, BlockError> {
            let path = path.as_ref();
            if path.exists() {
                fs::remove_file(path).map_err(block_io_error)?;
            }
            let listener = UnixListener::bind(path).map_err(block_io_error)?;
            Ok(Self { listener, backend })
        }

        pub fn local_addr(&self) -> Result<std::os::unix::net::SocketAddr, BlockError> {
            self.listener.local_addr().map_err(block_io_error)
        }

        pub fn accept_once(&self) -> Result<(), BlockError> {
            let (stream, _) = self.listener.accept().map_err(block_io_error)?;
            let mut server = BlockServer::new(stream, Arc::clone(&self.backend));
            server.serve_until_eof()
        }

        pub fn serve_forever(&self) -> Result<(), BlockError> {
            loop {
                self.accept_once()?;
            }
        }
    }

    impl<B> Drop for UnixBlockServer<B> {
        fn drop(&mut self) {
            if let Ok(addr) = self.listener.local_addr() {
                if let Some(path) = addr.as_pathname() {
                    let _ = fs::remove_file(path);
                }
            }
        }
    }

    pub struct TcpBlockServer<B> {
        listener: TcpListener,
        backend: Arc<B>,
    }

    impl<B: BlockBackend + Send + Sync + 'static> TcpBlockServer<B> {
        pub fn bind(addr: impl ToSocketAddrs, backend: B) -> Result<Self, BlockError> {
            Self::bind_shared(addr, Arc::new(backend))
        }

        pub fn bind_shared(addr: impl ToSocketAddrs, backend: Arc<B>) -> Result<Self, BlockError> {
            let listener = TcpListener::bind(addr).map_err(block_io_error)?;
            Ok(Self { listener, backend })
        }

        pub fn local_addr(&self) -> Result<std::net::SocketAddr, BlockError> {
            self.listener.local_addr().map_err(block_io_error)
        }

        pub fn accept_once(&self) -> Result<(), BlockError> {
            let (stream, _) = self.listener.accept().map_err(block_io_error)?;
            let mut server = BlockServer::new(stream, Arc::clone(&self.backend));
            server.serve_until_eof()
        }

        pub fn serve_forever(&self) -> Result<(), BlockError> {
            loop {
                self.accept_once()?;
            }
        }
    }
}

pub fn send_request<W: Write>(writer: &mut W, request: &BlockRequest) -> Result<(), BlockError> {
    let payload = encode_request_payload(request)?;
    write_frame(writer, &payload)
}

pub fn receive_request<R: Read>(reader: &mut R) -> Result<BlockRequest, BlockError> {
    let payload = read_frame(reader)?;
    decode_request_payload(&payload)
}

pub fn send_response<W: Write>(writer: &mut W, response: &BlockResponse) -> Result<(), BlockError> {
    let payload = encode_response_payload(response)?;
    write_frame(writer, &payload)
}

pub fn receive_response<R: Read>(reader: &mut R) -> Result<BlockResponse, BlockError> {
    let payload = read_frame(reader)?;
    decode_response_payload(&payload)
}

fn write_frame<W: Write>(writer: &mut W, payload: &[u8]) -> Result<(), BlockError> {
    let frame = frame_payload(payload)?;
    writer.write_all(&frame).map_err(block_io_error)?;
    writer.flush().map_err(block_io_error)
}

fn read_frame<R: Read>(reader: &mut R) -> Result<Vec<u8>, BlockError> {
    let mut len_bytes = [0_u8; 4];
    match reader.read_exact(&mut len_bytes) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => {
            return Err(BlockError::ProtocolError("unexpected EOF".into()));
        }
        Err(err) => return Err(block_io_error(err)),
    }
    let len = edgerun_encoding::byteorder::read_u32_le(&len_bytes, 0) as usize;
    if len > edgerun_protocols::block::MAX_FRAME_SIZE {
        return Err(BlockError::ProtocolError(
            "frame exceeds maximum size".into(),
        ));
    }
    let mut payload = vec![0_u8; len];
    reader.read_exact(&mut payload).map_err(|err| {
        if err.kind() == io::ErrorKind::UnexpectedEof {
            BlockError::ProtocolError("unexpected EOF".into())
        } else {
            block_io_error(err)
        }
    })?;
    Ok(payload)
}

fn block_io_error(error: io::Error) -> BlockError {
    BlockError::BackendFailure(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
    use crate::image::{create, VirtualDiskFormat, VirtualDiskSpec};
    #[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
    use std::env;
    #[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
    use std::fs;
    #[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
    use std::os::unix::net::UnixStream;
    #[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
    use std::thread;

    #[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
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
        let encoded = encode_request_payload(&request).unwrap();
        let decoded = decode_request_payload(&encoded).unwrap();
        assert_eq!(decoded, request);
    }

    #[test]
    fn response_roundtrip() {
        let response = BlockResponse::Info(test_info());
        let encoded = encode_response_payload(&response).unwrap();
        let decoded = decode_response_payload(&encoded).unwrap();
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
    #[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
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
    #[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
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
    #[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
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
