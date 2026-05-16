use alloc::format;
use alloc::string::{String, ToString};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualDiskSpec {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub format: VirtualDiskFormat,
    pub sparse: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualDiskInfo {
    pub path: PathBuf,
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

#[derive(Debug)]
pub enum VirtualDiskError {
    Io(io::Error),
    InvalidArgument(&'static str),
    AlreadyExists(PathBuf),
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
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::InvalidArgument(msg) => write!(f, "invalid argument: {msg}"),
            Self::AlreadyExists(path) => {
                write!(f, "virtual disk already exists: {}", path.display())
            }
            Self::CommandMissing { command } => write!(f, "required command missing: {command}"),
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

impl std::error::Error for VirtualDiskError {}

impl From<io::Error> for VirtualDiskError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl VirtualDiskFormat {
    fn requires_external_tool(self) -> bool {
        matches!(self, Self::Qcow2 | Self::Vhd | Self::Vhdx)
    }

    fn qemu_image_keyword(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Qcow2 => "qcow2",
            Self::Vhd => "vpc",
            Self::Vhdx => "vhdx",
        }
    }
}

pub fn create(spec: &VirtualDiskSpec) -> Result<VirtualDiskInfo> {
    validate_spec(spec)?;
    ensure_parent_dir(&spec.path)?;

    if spec.path.exists() {
        return Err(VirtualDiskError::AlreadyExists(spec.path.clone()));
    }

    if spec.format == VirtualDiskFormat::Raw {
        create_raw(&spec.path, spec.size_bytes, spec.sparse)?;
    } else {
        create_qemu_image(spec)?;
    }

    Ok(VirtualDiskInfo {
        path: spec.path.clone(),
        size_bytes: spec.size_bytes,
        format: spec.format,
        created: true,
    })
}

pub fn resize(path: &Path, format: VirtualDiskFormat, size_bytes: u64) -> Result<()> {
    validate_path(path)?;
    validate_size(size_bytes)?;
    if format == VirtualDiskFormat::Raw {
        let file = File::options().write(true).open(path)?;
        file.set_len(size_bytes)?;
        return Ok(());
    }

    let size_arg = qemu_size_arg(size_bytes);
    run_qemu_img(
        "qemu-img",
        &[
            OsString::from("resize"),
            path.as_os_str().to_os_string(),
            OsString::from(size_arg),
        ],
    )
}

pub fn clone(src: &Path, dst: &Path, dst_format: VirtualDiskFormat) -> Result<VirtualDiskInfo> {
    if dst.exists() {
        return Err(VirtualDiskError::AlreadyExists(dst.to_path_buf()));
    }
    ensure_parent_dir(dst)?;

    validate_path(src)?;
    let src_format = detect_format(src)?;
    let src_size = fs::metadata(src)?.len();
    if dst_format == VirtualDiskFormat::Raw && src_format == VirtualDiskFormat::Raw {
        fs::copy(src, dst)?;
        return Ok(VirtualDiskInfo {
            path: dst.to_path_buf(),
            size_bytes: src_size,
            format: dst_format,
            created: true,
        });
    }

    run_qemu_img(
        "qemu-img",
        &[
            OsString::from("convert"),
            OsString::from("-p"),
            OsString::from("-f"),
            OsString::from(src_format.qemu_image_keyword()),
            OsString::from("-O"),
            OsString::from(dst_format.qemu_image_keyword()),
            src.as_os_str().to_os_string(),
            dst.as_os_str().to_os_string(),
        ],
    )?;
    let meta = fs::metadata(dst)?;
    let size_bytes = if dst_format == VirtualDiskFormat::Raw {
        meta.len()
    } else {
        src_size
    };
    Ok(VirtualDiskInfo {
        path: dst.to_path_buf(),
        size_bytes,
        format: dst_format,
        created: true,
    })
}

pub fn info(path: &Path) -> Result<VirtualDiskInfo> {
    validate_path(path)?;
    let metadata = fs::metadata(path)?;
    let format = detect_format(path)?;
    Ok(VirtualDiskInfo {
        path: path.to_path_buf(),
        size_bytes: metadata.len(),
        format,
        created: false,
    })
}

pub fn remove(path: &Path) -> Result<()> {
    validate_path(path)?;
    fs::remove_file(path)?;
    Ok(())
}

pub fn detect_format(path: &Path) -> Result<VirtualDiskFormat> {
    let format = match path.extension().and_then(OsStr::to_str) {
        Some(ext) if ext.eq_ignore_ascii_case("raw") => VirtualDiskFormat::Raw,
        Some(ext) if ext.eq_ignore_ascii_case("qcow2") => VirtualDiskFormat::Qcow2,
        Some(ext) if ext.eq_ignore_ascii_case("vhd") => VirtualDiskFormat::Vhd,
        Some(ext) if ext.eq_ignore_ascii_case("vhdx") => VirtualDiskFormat::Vhdx,
        Some(_) | None => VirtualDiskFormat::Raw,
    };
    Ok(format)
}

fn validate_spec(spec: &VirtualDiskSpec) -> Result<()> {
    validate_size(spec.size_bytes)?;
    if spec.path.as_os_str().is_empty() {
        return Err(VirtualDiskError::InvalidArgument("empty path"));
    }
    if spec.format.requires_external_tool() {
        ensure_command_available("qemu-img")?;
    }
    Ok(())
}

fn validate_path(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(VirtualDiskError::InvalidArgument("path does not exist"));
    }
    if !path.is_file() {
        return Err(VirtualDiskError::InvalidArgument(
            "path must point to a file",
        ));
    }
    Ok(())
}

fn validate_size(size: u64) -> Result<()> {
    if size == 0 {
        return Err(VirtualDiskError::InvalidArgument("size must be > 0"));
    }
    Ok(())
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn create_raw(path: &Path, size: u64, sparse: bool) -> Result<()> {
    let mut file = File::options().create_new(true).write(true).open(path)?;
    if sparse {
        file.set_len(size)?;
        return Ok(());
    }

    const ZERO_CHUNK_LEN: usize = 1024 * 1024;
    let zero_chunk = [0_u8; ZERO_CHUNK_LEN];
    let mut remaining = size;
    while remaining > 0 {
        let write_len = remaining.min(ZERO_CHUNK_LEN as u64) as usize;
        file.write_all(&zero_chunk[..write_len])?;
        remaining -= write_len as u64;
    }
    file.flush()?;
    Ok(())
}

fn create_qemu_image(spec: &VirtualDiskSpec) -> Result<()> {
    let size_arg = qemu_size_arg(spec.size_bytes);
    run_qemu_img(
        "qemu-img",
        &[
            OsString::from("create"),
            OsString::from("-f"),
            OsString::from(spec.format.qemu_image_keyword()),
            spec.path.as_os_str().to_os_string(),
            OsString::from(size_arg),
        ],
    )
}

fn run_qemu_img(command: &'static str, args: &[OsString]) -> Result<()> {
    ensure_command_available(command)?;
    let output = Command::new(command).args(args).output()?;
    if !output.status.success() {
        let status = output.status.code().unwrap_or(-1);
        return Err(VirtualDiskError::CommandFailed {
            command,
            status,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(())
}

fn ensure_command_available(command: &'static str) -> Result<()> {
    if Command::new(command).arg("-V").output().is_ok() {
        return Ok(());
    }
    Err(VirtualDiskError::CommandMissing { command })
}

fn qemu_size_arg(size_bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;
    if size_bytes.is_multiple_of(GIB) {
        format!("{}G", size_bytes / GIB)
    } else if size_bytes.is_multiple_of(MIB) {
        format!("{}M", size_bytes / MIB)
    } else if size_bytes.is_multiple_of(KIB) {
        format!("{}K", size_bytes / KIB)
    } else {
        size_bytes.to_string()
    }
}

pub type Result<T> = std::result::Result<T, VirtualDiskError>;
