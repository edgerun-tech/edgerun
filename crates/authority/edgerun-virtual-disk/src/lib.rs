#![no_std]

extern crate alloc;
#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
extern crate std;

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
pub mod image;
#[cfg(any(target_os = "none", target_arch = "wasm32"))]
pub mod image {
    use alloc::string::{String, ToString};
    use core::{fmt, write};

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct VirtualDiskSpec {
        pub path: String,
        pub size_bytes: u64,
        pub format: VirtualDiskFormat,
        pub sparse: bool,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct VirtualDiskInfo {
        pub path: String,
        pub size_bytes: u64,
        pub format: VirtualDiskFormat,
        pub created: bool,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum VirtualDiskFormat {
        Raw,
        Qcow2,
        Vhd,
        Vhdx,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum VirtualDiskError {
        Unsupported,
        InvalidArgument(&'static str),
        AlreadyExists(String),
        CommandMissing {
            command: &'static str,
        },
        CommandFailed {
            command: &'static str,
            status: i32,
            stdout: String,
            stderr: String,
        },
    }

    impl fmt::Display for VirtualDiskFormat {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Raw => f.write_str("raw"),
                Self::Qcow2 => f.write_str("qcow2"),
                Self::Vhd => f.write_str("vpc"),
                Self::Vhdx => f.write_str("vhdx"),
            }
        }
    }

    impl fmt::Display for VirtualDiskError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Unsupported => f.write_str("virtual disk image operations require host I/O"),
                Self::InvalidArgument(msg) => write!(f, "invalid argument: {msg}"),
                Self::AlreadyExists(path) => write!(f, "virtual disk already exists: {path}"),
                Self::CommandMissing { command } => {
                    write!(f, "required command missing: {command}")
                }
                Self::CommandFailed {
                    command,
                    status,
                    stdout,
                    stderr,
                } => write!(
                    f,
                    "command `{command}` failed with status {status}: stdout={stdout:?}, stderr={stderr:?}"
                ),
            }
        }
    }

    impl core::error::Error for VirtualDiskError {}

    pub type Result<T> = core::result::Result<T, VirtualDiskError>;

    pub fn create(_spec: &VirtualDiskSpec) -> Result<VirtualDiskInfo> {
        Err(VirtualDiskError::Unsupported)
    }

    pub fn resize(_path: &str, _format: VirtualDiskFormat, _size_bytes: u64) -> Result<()> {
        Err(VirtualDiskError::Unsupported)
    }

    pub fn clone(
        _src: &str,
        _dst: &str,
        _dst_format: VirtualDiskFormat,
    ) -> Result<VirtualDiskInfo> {
        Err(VirtualDiskError::Unsupported)
    }

    pub fn info(path: &str) -> Result<VirtualDiskInfo> {
        Ok(VirtualDiskInfo {
            path: path.to_string(),
            size_bytes: 0,
            format: detect_format(path)?,
            created: false,
        })
    }

    pub fn remove(_path: &str) -> Result<()> {
        Err(VirtualDiskError::Unsupported)
    }

    pub fn detect_format(path: &str) -> Result<VirtualDiskFormat> {
        let format = if path.ends_with(".qcow2") {
            VirtualDiskFormat::Qcow2
        } else if path.ends_with(".vhd") {
            VirtualDiskFormat::Vhd
        } else if path.ends_with(".vhdx") {
            VirtualDiskFormat::Vhdx
        } else {
            VirtualDiskFormat::Raw
        };
        Ok(format)
    }
}
pub mod nbd;
pub mod remote;

pub use image::{
    Result as VirtualDiskResult, VirtualDiskError, VirtualDiskFormat, VirtualDiskInfo,
    VirtualDiskSpec, clone, create, detect_format, info, remove, resize,
};
pub use nbd::{
    LinuxNbdAttachSpec, NbdExport, NbdExportEntry, NbdNegotiatedExport, attach_nbd, detach_nbd,
    negotiate_nbd_export, serve_nbd_connection, serve_nbd_connection_multi,
};
pub use remote::{
    BlockClient, BlockServer, FileBlockBackend, MemoryBlockBackend, receive_request,
    receive_response, send_request, send_response,
};

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use alloc::format;
    use alloc::string::ToString;
    use alloc::sync::Arc;
    use alloc::vec;
    use edgerun_protocols::block::{
        BLOCK_PROTOCOL_VERSION, BlockBackend, BlockDeviceInfo, BlockError, BlockRequest,
        BlockResponse, RequestId, handle_request, validate_range,
    };

    // Tests for re-exported types from the image module

    #[test]
    fn virtual_disk_format_variants() {
        let _raw = VirtualDiskFormat::Raw;
        let _qcow2 = VirtualDiskFormat::Qcow2;
        let _vhd = VirtualDiskFormat::Vhd;
        let _vhdx = VirtualDiskFormat::Vhdx;
    }

    #[test]
    fn virtual_disk_format_display() {
        assert_eq!(format!("{}", VirtualDiskFormat::Raw), "raw");
        assert_eq!(format!("{}", VirtualDiskFormat::Qcow2), "qcow2");
    }

    #[test]
    fn virtual_disk_spec_clone_debug() {
        let spec = VirtualDiskSpec {
            path: "/tmp/test.img".into(),
            format: VirtualDiskFormat::Raw,
            size_bytes: 1024 * 1024 * 100,
            sparse: true,
        };
        let cloned = spec.clone();
        assert_eq!(spec.path, cloned.path);
        assert_eq!(spec.format, cloned.format);
        assert_eq!(spec.size_bytes, cloned.size_bytes);
        assert!(cloned.sparse);
        let debug_str = format!("{spec:?}");
        assert!(debug_str.contains("VirtualDiskSpec"));
    }

    #[test]
    fn virtual_disk_info_clone_debug() {
        let info = VirtualDiskInfo {
            path: "/tmp/test.img".into(),
            format: VirtualDiskFormat::Raw,
            size_bytes: 1024 * 1024 * 100,
            created: true,
        };
        let cloned = info.clone();
        assert_eq!(info.path, cloned.path);
        assert_eq!(info.size_bytes, cloned.size_bytes);
        assert!(cloned.created);
    }

    #[test]
    fn virtual_disk_error_display() {
        #[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
        {
            let err = VirtualDiskError::Io(std::io::Error::from_raw_os_error(2));
            let msg = format!("{err}");
            assert!(!msg.is_empty());
        }

        let err = VirtualDiskError::InvalidArgument("bad path");
        let msg = format!("{err}");
        assert!(msg.contains("bad path"));
    }

    #[test]
    fn virtual_disk_error_is_std_error() {
        let err: Box<dyn core::error::Error> = Box::new(VirtualDiskError::InvalidArgument("test"));
        assert!(!err.to_string().is_empty());
    }

    // Tests for re-exported types from the nbd module

    #[test]
    fn linux_nbd_attach_spec_clone_debug() {
        let spec = LinuxNbdAttachSpec {
            device: "/dev/nbd0".into(),
            export_name: "test".into(),
            block_size: Some(512),
            read_only: false,
        };
        let cloned = spec.clone();
        assert_eq!(spec.device, cloned.device);
        assert_eq!(spec.export_name, cloned.export_name);
        let debug_str = format!("{spec:?}");
        assert!(debug_str.contains("LinuxNbdAttachSpec"));
    }

    #[test]
    fn nbd_negotiated_export_clone_debug() {
        let export = NbdNegotiatedExport {
            size_bytes: 1024 * 1024,
            transmission_flags: 0,
        };
        let cloned = export.clone();
        assert_eq!(export.size_bytes, cloned.size_bytes);
        let debug_str = format!("{export:?}");
        assert!(debug_str.contains("NbdNegotiatedExport"));
    }

    #[test]
    fn nbd_export_clone_debug() {
        let export = NbdExport {
            name: "test-export".into(),
            description: "A test export".into(),
        };
        let cloned = export.clone();
        assert_eq!(export.name, cloned.name);
        let debug_str = format!("{export:?}");
        assert!(debug_str.contains("NbdExport"));
    }

    #[test]
    fn nbd_export_default() {
        let export = NbdExport::default();
        assert_eq!(export.name, "edgerun");
        assert!(!export.description.is_empty());
    }

    #[test]
    fn nbd_export_entry_clone_debug() {
        let info = BlockDeviceInfo {
            block_size: 512,
            block_count: 2048,
            readonly: false,
            supports_flush: true,
            supports_discard: false,
            supports_write_zeroes: false,
            model: "test".into(),
            serial: "001".into(),
        };
        let backend = Arc::new(MemoryBlockBackend::new(info.clone()).unwrap());
        let entry = NbdExportEntry {
            export: NbdExport {
                name: "entry-0".into(),
                description: "Test".into(),
            },
            backend,
        };
        let cloned = entry.clone();
        assert_eq!(entry.export.name, cloned.export.name);
    }

    // Tests for re-exported types from the remote module

    #[test]
    fn block_protocol_version_is_one() {
        assert_eq!(BLOCK_PROTOCOL_VERSION, 1);
    }

    #[test]
    fn request_id_is_u64_type() {
        let id: RequestId = 42;
        assert_eq!(id, 42u64);
    }

    #[test]
    fn block_request_variants() {
        let _handshake = BlockRequest::Handshake {
            protocol_version: 1,
        };
        let _get_info = BlockRequest::GetInfo;
        let _read = BlockRequest::Read {
            request_id: 1,
            lba: 0,
            blocks: 8,
        };
        let _write = BlockRequest::Write {
            request_id: 2,
            lba: 0,
            blocks: 8,
            data: vec![0u8; 4096],
        };
        let _flush = BlockRequest::Flush { request_id: 3 };
        let _discard = BlockRequest::Discard {
            request_id: 4,
            lba: 0,
            blocks: 8,
        };
        let _write_zeroes = BlockRequest::WriteZeroes {
            request_id: 5,
            lba: 0,
            blocks: 8,
        };
        let _ping = BlockRequest::Ping;
    }

    #[test]
    fn block_request_clone_debug() {
        let request = BlockRequest::Read {
            request_id: 1,
            lba: 0,
            blocks: 8,
        };
        let cloned = request.clone();
        assert_eq!(request, cloned);
        let debug_str = format!("{request:?}");
        assert!(debug_str.contains("Read"));
    }

    #[test]
    fn block_response_variants() {
        let _handshake = BlockResponse::HandshakeAck {
            protocol_version: 1,
        };
        let _pong = BlockResponse::Pong;
        let _write_ack = BlockResponse::WriteAck { request_id: 1 };
        let _flush_ack = BlockResponse::FlushAck { request_id: 1 };
    }

    #[test]
    fn block_error_display() {
        assert!(format!("{}", BlockError::OutOfRange).contains("out of range"));
        assert!(format!("{}", BlockError::ReadOnly).contains("read-only"));
        assert!(format!("{}", BlockError::Misaligned).contains("misaligned"));
        assert!(format!("{}", BlockError::Unsupported).contains("unsupported"));
        assert!(format!("{}", BlockError::NotReady).contains("not ready"));
        assert!(format!("{}", BlockError::Timeout).contains("timed out"));
        assert!(format!("{}", BlockError::BackendFailure("test".into())).contains("test"));
        assert!(format!("{}", BlockError::ProtocolError("bad".into())).contains("bad"));
    }

    #[test]
    fn block_error_is_std_error() {
        let err: Box<dyn core::error::Error> = Box::new(BlockError::OutOfRange);
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn block_device_info_clone_debug() {
        let info = BlockDeviceInfo {
            block_size: 4096,
            block_count: 256,
            readonly: false,
            supports_flush: true,
            supports_discard: true,
            supports_write_zeroes: false,
            model: "TestDisk".into(),
            serial: "SN-001".into(),
        };
        let cloned = info.clone();
        assert_eq!(info.block_size, cloned.block_size);
        assert_eq!(info.block_count, cloned.block_count);
        assert_eq!(info.total_size_bytes(), 4096 * 256);
    }

    #[test]
    fn memory_block_backend_new() {
        let info = BlockDeviceInfo {
            block_size: 512,
            block_count: 2048,
            readonly: false,
            supports_flush: true,
            supports_discard: false,
            supports_write_zeroes: false,
            model: "mem".into(),
            serial: "0".into(),
        };
        let backend = MemoryBlockBackend::new(info).unwrap();
        assert_eq!(backend.info().block_count, 2048);
    }

    #[test]
    fn memory_block_backend_read_write_roundtrip() {
        let info = BlockDeviceInfo {
            block_size: 512,
            block_count: 8,
            readonly: false,
            supports_flush: true,
            supports_discard: false,
            supports_write_zeroes: false,
            model: "mem".into(),
            serial: "0".into(),
        };
        let mut backend = MemoryBlockBackend::new(info).unwrap();
        // Write some data (1 block = 512 bytes)
        let data = vec![0xAB; 512];
        backend.write_blocks(0, 1, &data).unwrap();
        // Read it back
        let mut out = vec![0u8; 512];
        backend.read_blocks(0, 1, &mut out).unwrap();
        assert_eq!(out, data);
    }

    #[test]
    fn memory_block_backend_flush() {
        let info = BlockDeviceInfo {
            block_size: 512,
            block_count: 8,
            readonly: false,
            supports_flush: true,
            supports_discard: false,
            supports_write_zeroes: false,
            model: "mem".into(),
            serial: "0".into(),
        };
        let backend = MemoryBlockBackend::new(info).unwrap();
        assert!(backend.flush().is_ok());
    }

    #[test]
    fn handle_request_handshake() {
        let info = BlockDeviceInfo {
            block_size: 512,
            block_count: 8,
            readonly: false,
            supports_flush: true,
            supports_discard: false,
            supports_write_zeroes: false,
            model: "mem".into(),
            serial: "0".into(),
        };
        let backend = MemoryBlockBackend::new(info).unwrap();
        let resp = handle_request(
            &backend,
            BlockRequest::Handshake {
                protocol_version: 1,
            },
        );
        assert!(matches!(
            resp,
            BlockResponse::HandshakeAck {
                protocol_version: 1
            }
        ));
    }

    #[test]
    fn handle_request_handshake_bad_version() {
        let info = BlockDeviceInfo {
            block_size: 512,
            block_count: 8,
            readonly: false,
            supports_flush: true,
            supports_discard: false,
            supports_write_zeroes: false,
            model: "mem".into(),
            serial: "0".into(),
        };
        let backend = MemoryBlockBackend::new(info).unwrap();
        let resp = handle_request(
            &backend,
            BlockRequest::Handshake {
                protocol_version: 99,
            },
        );
        assert!(matches!(resp, BlockResponse::Error { .. }));
    }

    #[test]
    fn handle_request_get_info() {
        let info = BlockDeviceInfo {
            block_size: 4096,
            block_count: 16,
            readonly: false,
            supports_flush: true,
            supports_discard: false,
            supports_write_zeroes: false,
            model: "mem".into(),
            serial: "0".into(),
        };
        let backend = MemoryBlockBackend::new(info.clone()).unwrap();
        let resp = handle_request(&backend, BlockRequest::GetInfo);
        match resp {
            BlockResponse::Info(got) => {
                assert_eq!(got.block_size, info.block_size);
                assert_eq!(got.block_count, info.block_count);
            }
            _ => panic!("expected Info response"),
        }
    }

    #[test]
    fn handle_request_ping() {
        let info = BlockDeviceInfo {
            block_size: 512,
            block_count: 8,
            readonly: false,
            supports_flush: true,
            supports_discard: false,
            supports_write_zeroes: false,
            model: "mem".into(),
            serial: "0".into(),
        };
        let backend = MemoryBlockBackend::new(info).unwrap();
        let resp = handle_request(&backend, BlockRequest::Ping);
        assert!(matches!(resp, BlockResponse::Pong));
    }

    #[test]
    fn validate_range_checks() {
        let info = BlockDeviceInfo {
            block_size: 512,
            block_count: 10,
            readonly: false,
            supports_flush: true,
            supports_discard: false,
            supports_write_zeroes: false,
            model: "mem".into(),
            serial: "0".into(),
        };
        // Valid range
        assert!(validate_range(&info, 0, 10).is_ok());
        assert!(validate_range(&info, 5, 5).is_ok());
        // Out of bounds
        assert!(validate_range(&info, 5, 6).is_err());
        assert!(validate_range(&info, 10, 1).is_err());
    }
}
