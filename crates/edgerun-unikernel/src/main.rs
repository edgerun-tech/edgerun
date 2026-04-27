//! Edgerun unikernel - bare shell with networking

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

extern crate alloc;
extern crate edgerun_dhcp;
extern crate edgerun_oci;
extern crate edgerun_platform;
extern crate edgerun_rt as rt;
extern crate edgerun_tftp;
extern crate edgerun_tpm;
extern crate edgerun_virtio;

use edgerun_dhcp::message::{DHCP_CLIENT_PORT, DHCP_SERVER_PORT};
use edgerun_dhcp::{DhcpMessage, DhcpMessageType};
use edgerun_tftp::message::{TftpMessage, TFTP_PORT};
use rt::ip::{ParsedPacket, ARP_OP_REQUEST, ICMP_ECHO_REQUEST};
use rt::{block_on, crc32, IpAddr, IpStack, Network, RingBuffer, Rng, TcpSocket};

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
#[allow(dead_code)]
mod oci_syscall {
    use super::rt;
    use edgerun_oci::prelude::String;
    use edgerun_oci::rootfs_access::OciRootfs;
    use edgerun_oci::{
        dispatch_x86_64_linux_syscall_frame, prepare_and_load_oci_elf_program_with_load_bias,
        OciElfError, OciElfLoadBias, OciElfUnsafeIdentityMapper, OciPreparedLaunchState,
        OciSyscallAction, OciSyscallError, OciSyscallMemory, OciSyscallSink, OciX86_64SyscallFrame,
    };
    use edgerun_platform::arch::x86_64::{
        self, SyscallFrame, KERNEL_CODE_SELECTOR, USER_COMPAT_CODE_SELECTOR,
    };

    struct DirectMemory;

    impl OciSyscallMemory for DirectMemory {
        fn read_bytes(&self, addr: u64, len: usize, out: &mut [u8]) -> Result<(), OciSyscallError> {
            let ptr = usize::try_from(addr).map_err(|_| OciSyscallError::BadAddress)? as *const u8;
            let Some(out) = out.get_mut(..len) else {
                return Err(OciSyscallError::BadAddress);
            };
            unsafe {
                core::ptr::copy_nonoverlapping(ptr, out.as_mut_ptr(), len);
            }
            Ok(())
        }
    }

    struct LogSink;

    impl OciSyscallSink for LogSink {
        fn write_fd(&mut self, fd: u64, bytes: &[u8]) -> Result<usize, OciSyscallError> {
            if fd == 1 || fd == 2 {
                if let Ok(text) = core::str::from_utf8(bytes) {
                    rt::log::log(1, text.trim_end_matches('\n'));
                } else {
                    rt::log::log(1, "container wrote non-UTF8 bytes");
                }
            }
            Ok(bytes.len())
        }
    }

    pub unsafe fn install(kernel_code_selector: u16, user_code_selector: u16) {
        x86_64::set_syscall_handler(handle_syscall);
        unsafe {
            x86_64::enable_syscall_entry(kernel_code_selector, user_code_selector);
        }
    }

    pub unsafe fn install_flat_gdt() {
        unsafe {
            x86_64::load_flat_gdt();
            install(KERNEL_CODE_SELECTOR, USER_COMPAT_CODE_SELECTOR);
        }
    }

    pub unsafe fn enter_launch_state(launch: &OciPreparedLaunchState) -> ! {
        unsafe {
            install_flat_gdt();
            x86_64::enter_user64(launch.entry_point, launch.stack_pointer);
        }
    }

    pub unsafe fn launch_rootfs<R: OciRootfs>(
        rootfs: &R,
        args: &[String],
        env: &[String],
        cwd: &str,
        scratch: &mut [u8],
        stack_base: u64,
        stack_top: u64,
        stack: &mut [u8],
        load_bias: OciElfLoadBias,
    ) -> Result<core::convert::Infallible, OciElfError> {
        let mut mapper = OciElfUnsafeIdentityMapper::new();
        let Some(launch) = prepare_and_load_oci_elf_program_with_load_bias(
            rootfs,
            args,
            env,
            cwd,
            &mut mapper,
            scratch,
            stack_base,
            stack_top,
            stack,
            load_bias,
        )?
        else {
            return Err(OciElfError::NotFound(
                args.first().cloned().unwrap_or_default(),
            ));
        };

        unsafe {
            enter_launch_state(&launch);
        }
    }

    extern "C" fn handle_syscall(frame: &mut SyscallFrame) {
        let mut oci_frame = OciX86_64SyscallFrame {
            rax: frame.rax,
            rdi: frame.rdi,
            rsi: frame.rsi,
            rdx: frame.rdx,
            r10: frame.r10,
            r8: frame.r8,
            r9: frame.r9,
        };
        let memory = DirectMemory;
        let mut sink = LogSink;
        let mut scratch = [0u8; 256];

        match dispatch_x86_64_linux_syscall_frame(&memory, &mut sink, &mut scratch, &mut oci_frame)
        {
            Ok(OciSyscallAction::Return(_)) => {
                frame.rax = oci_frame.rax;
            }
            Ok(OciSyscallAction::Exit(code)) => {
                let _ = code;
                rt::log::log(1, "container exited");
                loop {
                    unsafe {
                        core::arch::asm!("hlt");
                    }
                }
            }
            Err(OciSyscallError::Unsupported(_)) => {
                frame.rax = (-38i64) as u64;
            }
            Err(OciSyscallError::BadAddress) => {
                frame.rax = (-14i64) as u64;
            }
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
#[allow(dead_code)]
mod oci_image_boot {
    use super::boot_config::BootConfig;
    use super::disk_boot::{self, DiskBootError, OpenedEdgeFs};
    use super::oci_syscall;
    use edgerun_edgefs::EdgeFs;
    use edgerun_oci::prelude::String;
    use edgerun_oci::{
        EdgeFsImagePullReport, ImageRef, OciElfError, OciElfLoadBias, RegistryClient, RegistryError,
    };
    use edgerun_storage::{BlockStorage, PartitionBlockDevice};

    #[derive(Debug)]
    pub enum OciImageBootError {
        InvalidImageRef(String),
        Registry(RegistryError),
        Elf(OciElfError),
    }

    #[derive(Debug)]
    pub enum OciDiskImageBootError {
        Disk(DiskBootError),
        Image(OciImageBootError),
    }

    pub enum PulledConfiguredEdgeFs<S: BlockStorage> {
        Partition {
            fs: EdgeFs<PartitionBlockDevice<S>>,
            report: EdgeFsImagePullReport,
            config: BootConfig,
        },
        WholeDisk {
            fs: EdgeFs<S>,
            report: EdgeFsImagePullReport,
            config: BootConfig,
        },
    }

    impl From<RegistryError> for OciImageBootError {
        fn from(error: RegistryError) -> Self {
            Self::Registry(error)
        }
    }

    impl From<OciElfError> for OciImageBootError {
        fn from(error: OciElfError) -> Self {
            Self::Elf(error)
        }
    }

    impl From<DiskBootError> for OciDiskImageBootError {
        fn from(error: DiskBootError) -> Self {
            Self::Disk(error)
        }
    }

    impl From<OciImageBootError> for OciDiskImageBootError {
        fn from(error: OciImageBootError) -> Self {
            Self::Image(error)
        }
    }

    pub async unsafe fn pull_image_into_edgefs<S: BlockStorage>(
        fs: &mut EdgeFs<S>,
        image_ref: &str,
        rootfs_path: &str,
    ) -> Result<edgerun_oci::EdgeFsImagePullReport, OciImageBootError> {
        let image: ImageRef = image_ref
            .parse()
            .map_err(OciImageBootError::InvalidImageRef)?;
        let mut client = RegistryClient::new();
        client
            .pull_into_edgefs(&image, rootfs_path, fs)
            .await
            .map_err(Into::into)
    }

    pub async unsafe fn pull_configured_image_into_edgefs<S: BlockStorage>(
        fs: &mut EdgeFs<S>,
        config: &BootConfig,
    ) -> Result<edgerun_oci::EdgeFsImagePullReport, OciImageBootError> {
        unsafe { pull_image_into_edgefs(fs, &config.image, &config.rootfs_path).await }
    }

    pub async unsafe fn pull_configured_disk_image<S: BlockStorage>(
        device: S,
        key: [u8; 32],
        fs_id: [u8; 16],
    ) -> Result<PulledConfiguredEdgeFs<S>, OciDiskImageBootError> {
        let mut device = device;
        let config = disk_boot::read_default_fat_boot_config_mut(&mut device)?;
        let opened = disk_boot::open_configured_edgefs(device, &config, key, fs_id)?;

        match opened {
            OpenedEdgeFs::Partition(mut fs) => {
                let report = unsafe { pull_configured_image_into_edgefs(&mut fs, &config).await? };
                Ok(PulledConfiguredEdgeFs::Partition { fs, report, config })
            }
            OpenedEdgeFs::WholeDisk(mut fs) => {
                let report = unsafe { pull_configured_image_into_edgefs(&mut fs, &config).await? };
                Ok(PulledConfiguredEdgeFs::WholeDisk { fs, report, config })
            }
        }
    }

    pub unsafe fn launch_pulled_edgefs<S: BlockStorage>(
        fs: &mut EdgeFs<S>,
        report: &EdgeFsImagePullReport,
        scratch: &mut [u8],
        stack_base: u64,
        stack_top: u64,
        stack: &mut [u8],
        load_bias: OciElfLoadBias,
    ) -> Result<core::convert::Infallible, OciImageBootError> {
        let runtime = &report.plan.runtime;
        unsafe {
            oci_syscall::launch_rootfs(
                fs,
                &runtime.args,
                &runtime.env,
                runtime.cwd.as_str(),
                scratch,
                stack_base,
                stack_top,
                stack,
                load_bias,
            )
            .map_err(Into::into)
        }
    }

    pub async unsafe fn pull_and_launch_edgefs<S: BlockStorage>(
        fs: &mut EdgeFs<S>,
        image_ref: &str,
        rootfs_path: &str,
        scratch: &mut [u8],
        stack_base: u64,
        stack_top: u64,
        stack: &mut [u8],
        load_bias: OciElfLoadBias,
    ) -> Result<core::convert::Infallible, OciImageBootError> {
        let report = unsafe { pull_image_into_edgefs(fs, image_ref, rootfs_path).await? };
        unsafe {
            launch_pulled_edgefs(
                fs, &report, scratch, stack_base, stack_top, stack, load_bias,
            )
        }
    }

    pub async unsafe fn pull_and_launch_configured_edgefs<S: BlockStorage>(
        fs: &mut EdgeFs<S>,
        config: &BootConfig,
        scratch: &mut [u8],
        stack_base: u64,
        stack_top: u64,
        stack: &mut [u8],
        load_bias: OciElfLoadBias,
    ) -> Result<core::convert::Infallible, OciImageBootError> {
        unsafe {
            pull_and_launch_edgefs(
                fs,
                &config.image,
                &config.rootfs_path,
                scratch,
                stack_base,
                stack_top,
                stack,
                load_bias,
            )
            .await
        }
    }

    pub async unsafe fn pull_and_launch_configured_disk_image<S: BlockStorage>(
        device: S,
        key: [u8; 32],
        fs_id: [u8; 16],
        scratch: &mut [u8],
        stack_base: u64,
        stack_top: u64,
        stack: &mut [u8],
        load_bias: OciElfLoadBias,
    ) -> Result<core::convert::Infallible, OciDiskImageBootError> {
        let pulled = unsafe { pull_configured_disk_image(device, key, fs_id).await? };
        match pulled {
            PulledConfiguredEdgeFs::Partition { mut fs, report, .. } => unsafe {
                launch_pulled_edgefs(
                    &mut fs, &report, scratch, stack_base, stack_top, stack, load_bias,
                )
                .map_err(Into::into)
            },
            PulledConfiguredEdgeFs::WholeDisk { mut fs, report, .. } => unsafe {
                launch_pulled_edgefs(
                    &mut fs, &report, scratch, stack_base, stack_top, stack, load_bias,
                )
                .map_err(Into::into)
            },
        }
    }
}

#[cfg(any(target_os = "none", test))]
#[allow(dead_code)]
mod boot_config {
    use alloc::string::{String, ToString};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EdgeFsBootTarget {
        ExistingPartition,
        FormatFirstPartition,
        FormatWholeDisk,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct BootConfig {
        pub image: String,
        pub edgefs: EdgeFsBootTarget,
        pub rootfs_path: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum BootConfigError {
        InvalidUtf8,
        InvalidLine(usize),
        UnknownKey(String),
        InvalidEdgeFsTarget(String),
        InvalidRootfsPath(String),
        MissingImage,
    }

    impl core::fmt::Display for BootConfigError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::InvalidUtf8 => f.write_str("boot config is not valid UTF-8"),
                Self::InvalidLine(line) => write!(f, "invalid boot config line {line}"),
                Self::UnknownKey(key) => write!(f, "unknown boot config key: {key}"),
                Self::InvalidEdgeFsTarget(target) => {
                    write!(f, "invalid edgefs boot target: {target}")
                }
                Self::InvalidRootfsPath(path) => write!(f, "invalid rootfs path: {path}"),
                Self::MissingImage => f.write_str("boot config is missing image"),
            }
        }
    }

    impl core::error::Error for BootConfigError {}

    impl BootConfig {
        pub fn parse(bytes: &[u8]) -> Result<Self, BootConfigError> {
            let text = core::str::from_utf8(bytes).map_err(|_| BootConfigError::InvalidUtf8)?;
            let mut image = None;
            let mut edgefs = EdgeFsBootTarget::ExistingPartition;
            let mut rootfs_path = "/".to_string();

            for (index, raw_line) in text.lines().enumerate() {
                let line_no = index + 1;
                let line = raw_line
                    .split_once('#')
                    .map_or(raw_line, |(line, _)| line)
                    .trim();
                if line.is_empty() {
                    continue;
                }

                let Some((key, value)) = line.split_once('=') else {
                    return Err(BootConfigError::InvalidLine(line_no));
                };
                let key = key.trim();
                let value = value.trim();
                if value.is_empty() {
                    return Err(BootConfigError::InvalidLine(line_no));
                }

                match key {
                    "image" | "oci_image" | "ref" => image = Some(value.to_string()),
                    "edgefs" => edgefs = parse_edgefs_target(value)?,
                    "rootfs" | "rootfs_path" => rootfs_path = parse_rootfs_path(value)?,
                    other => return Err(BootConfigError::UnknownKey(other.to_string())),
                }
            }

            let image = image.ok_or(BootConfigError::MissingImage)?;
            Ok(Self {
                image,
                edgefs,
                rootfs_path,
            })
        }
    }

    fn parse_edgefs_target(value: &str) -> Result<EdgeFsBootTarget, BootConfigError> {
        match value {
            "partition" | "existing-partition" | "existing_partition" => {
                Ok(EdgeFsBootTarget::ExistingPartition)
            }
            "format-partition" | "format_partition" | "first-partition" | "first_partition" => {
                Ok(EdgeFsBootTarget::FormatFirstPartition)
            }
            "whole-disk" | "whole_disk" | "format-whole-disk" | "format_whole_disk" => {
                Ok(EdgeFsBootTarget::FormatWholeDisk)
            }
            other => Err(BootConfigError::InvalidEdgeFsTarget(other.to_string())),
        }
    }

    fn parse_rootfs_path(value: &str) -> Result<String, BootConfigError> {
        if !value.starts_with('/') || value.as_bytes().contains(&0) {
            return Err(BootConfigError::InvalidRootfsPath(value.to_string()));
        }

        let without_root = &value[1..];
        if without_root.is_empty() {
            return Ok(value.to_string());
        }

        for component in without_root.split('/') {
            if component.is_empty() || component == "." || component == ".." {
                return Err(BootConfigError::InvalidRootfsPath(value.to_string()));
            }
        }

        Ok(value.to_string())
    }

    #[cfg(test)]
    mod tests {
        use super::{BootConfig, EdgeFsBootTarget};

        #[test]
        fn parses_required_image_and_defaults() {
            let config = BootConfig::parse(b"# edgeOS\nimage=registry.local/app:latest\n").unwrap();
            assert_eq!(config.image, "registry.local/app:latest");
            assert_eq!(config.edgefs, EdgeFsBootTarget::ExistingPartition);
            assert_eq!(config.rootfs_path, "/");
        }

        #[test]
        fn parses_edgefs_and_rootfs_options() {
            let config = BootConfig::parse(
                b"oci_image = example.com/ns/app:v1\nedgefs = whole-disk\nrootfs_path = /apps/app\n",
            )
            .unwrap();
            assert_eq!(config.image, "example.com/ns/app:v1");
            assert_eq!(config.edgefs, EdgeFsBootTarget::FormatWholeDisk);
            assert_eq!(config.rootfs_path, "/apps/app");
        }
    }
}

#[cfg(target_os = "none")]
#[allow(dead_code)]
mod disk_boot {
    use super::boot_config::{BootConfig, BootConfigError, EdgeFsBootTarget};
    use edgerun_edgefs::{EdgeFs, EdgeFsError, EdgeFsInfo};
    use edgerun_storage::{
        detect_partitions, probe_filesystem, BlockStorage, FatError, FatReadOnly, FileSystemKind,
        FileSystemProbe, FileSystemProbeError, PartitionBlockDevice, PartitionEntry,
        PartitionError, PartitionTable, StorageError,
    };

    pub const DEFAULT_BOOT_CONFIG_PATHS: &[&str] =
        &["/edgerun/boot.cfg", "/EDGERUN/BOOT.CFG", "/boot.cfg"];

    #[derive(Debug)]
    pub enum DiskBootError {
        Partition(PartitionError),
        Probe(FileSystemProbeError),
        Fat(FatError),
        BootConfig(BootConfigError),
        EdgeFs(EdgeFsError),
        NoEdgeFsPartition,
        NoFatPartition,
        NoBootConfig,
        NoPartition,
    }

    impl From<PartitionError> for DiskBootError {
        fn from(error: PartitionError) -> Self {
            Self::Partition(error)
        }
    }

    impl From<EdgeFsError> for DiskBootError {
        fn from(error: EdgeFsError) -> Self {
            Self::EdgeFs(error)
        }
    }

    impl From<FileSystemProbeError> for DiskBootError {
        fn from(error: FileSystemProbeError) -> Self {
            Self::Probe(error)
        }
    }

    impl From<FatError> for DiskBootError {
        fn from(error: FatError) -> Self {
            Self::Fat(error)
        }
    }

    impl From<BootConfigError> for DiskBootError {
        fn from(error: BootConfigError) -> Self {
            Self::BootConfig(error)
        }
    }

    #[derive(Debug, Clone)]
    pub struct DiskPartitionProbe {
        pub partition: PartitionEntry,
        pub filesystem: FileSystemProbe,
    }

    #[derive(Debug, Clone)]
    pub struct EdgeFsPartition {
        pub partition: PartitionEntry,
        pub info: EdgeFsInfo,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum MountPlanKind {
        EdgeFsReadWrite,
        ForeignReadOnly,
        FutureNative,
        Unknown,
    }

    #[derive(Debug, Clone)]
    pub struct MountPlanEntry {
        pub partition: PartitionEntry,
        pub filesystem: FileSystemProbe,
        pub edgefs: Option<EdgeFsInfo>,
        pub kind: MountPlanKind,
    }

    #[derive(Debug, Clone)]
    pub struct DiskMountPlan {
        pub table: PartitionTable,
        pub entries: alloc::vec::Vec<MountPlanEntry>,
    }

    pub enum OpenedEdgeFs<S: BlockStorage> {
        Partition(EdgeFs<PartitionBlockDevice<S>>),
        WholeDisk(EdgeFs<S>),
    }

    pub struct RtBlockDeviceStorage<T> {
        device: T,
    }

    impl<T> RtBlockDeviceStorage<T> {
        pub fn new(device: T) -> Self {
            Self { device }
        }

        pub fn into_inner(self) -> T {
            self.device
        }
    }

    impl BlockStorage for RtBlockDeviceStorage<edgerun_virtio::VirtBlk> {
        fn sector_size(&self) -> usize {
            edgerun_virtio::SECTOR_SIZE
        }

        fn sectors(&self) -> u64 {
            self.device.sectors()
        }

        fn read_sector(
            &mut self,
            sector: u64,
            buf: &mut [u8],
        ) -> core::result::Result<(), StorageError> {
            if buf.len() != self.sector_size() {
                return Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::InvalidInput,
                    "sector buffer size mismatch",
                )));
            }
            if self.device.read_sector(sector, buf) {
                Ok(())
            } else {
                Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::Other,
                    "bare block read failed",
                )))
            }
        }

        fn write_sector(
            &mut self,
            sector: u64,
            buf: &[u8],
        ) -> core::result::Result<(), StorageError> {
            if buf.len() != self.sector_size() {
                return Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::InvalidInput,
                    "sector buffer size mismatch",
                )));
            }
            if self.device.write_sector(sector, buf) {
                Ok(())
            } else {
                Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::Other,
                    "bare block write failed",
                )))
            }
        }
    }

    pub struct RtPartitionStorage<'a, S: BlockStorage> {
        parent: &'a mut S,
        start_lba: u64,
        sectors: u64,
    }

    impl<'a, S: BlockStorage> RtPartitionStorage<'a, S> {
        pub fn new(parent: &'a mut S, start_lba: u64, sectors: u64) -> Self {
            Self {
                parent,
                start_lba,
                sectors,
            }
        }
    }

    impl<S: BlockStorage> BlockStorage for RtPartitionStorage<'_, S> {
        fn sector_size(&self) -> usize {
            BlockStorage::sector_size(&*self.parent)
        }

        fn sectors(&self) -> u64 {
            self.sectors
        }

        fn read_sector(
            &mut self,
            sector: u64,
            buf: &mut [u8],
        ) -> core::result::Result<(), StorageError> {
            if sector >= self.sectors {
                return Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::UnexpectedEof,
                    "partition sector index out of range",
                )));
            }
            let Some(parent_sector) = self.start_lba.checked_add(sector) else {
                return Err(StorageError::Io(edgerun_storage::io::Error::other(
                    "partition sector overflow",
                )));
            };
            BlockStorage::read_sector(&mut *self.parent, parent_sector, buf)
        }

        fn write_sector(
            &mut self,
            sector: u64,
            buf: &[u8],
        ) -> core::result::Result<(), StorageError> {
            if sector >= self.sectors {
                return Err(StorageError::Io(edgerun_storage::io::Error::new(
                    edgerun_storage::io::ErrorKind::UnexpectedEof,
                    "partition sector index out of range",
                )));
            }
            let Some(parent_sector) = self.start_lba.checked_add(sector) else {
                return Err(StorageError::Io(edgerun_storage::io::Error::other(
                    "partition sector overflow",
                )));
            };
            BlockStorage::write_sector(&mut *self.parent, parent_sector, buf)
        }

        fn sync(&mut self) -> core::result::Result<(), StorageError> {
            BlockStorage::sync(&mut *self.parent)
        }
    }

    pub fn scan_partition_filesystems<S: BlockStorage>(
        device: &mut S,
    ) -> Result<(PartitionTable, alloc::vec::Vec<DiskPartitionProbe>), DiskBootError> {
        let table = detect_partitions(device)?;
        let mut probes = alloc::vec::Vec::new();
        for partition in &table.partitions {
            let mut slice =
                PartitionBlockDevice::new(&mut *device, partition.start_lba, partition.sectors)?;
            probes.push(DiskPartitionProbe {
                partition: partition.clone(),
                filesystem: probe_filesystem(&mut slice)?,
            });
        }
        Ok((table, probes))
    }

    pub fn plan_partition_mounts<S: BlockStorage>(
        device: &mut S,
        key: [u8; 32],
    ) -> Result<DiskMountPlan, DiskBootError> {
        let table = detect_partitions(device)?;
        let mut entries = alloc::vec::Vec::new();
        for partition in &table.partitions {
            let mut slice =
                PartitionBlockDevice::new(&mut *device, partition.start_lba, partition.sectors)?;
            let filesystem = probe_filesystem(&mut slice)?;

            let edgefs = if filesystem.kind == FileSystemKind::EdgeFs {
                let slice = PartitionBlockDevice::new(
                    &mut *device,
                    partition.start_lba,
                    partition.sectors,
                )?;
                EdgeFs::probe(slice, key).ok()
            } else {
                None
            };

            let kind = match (filesystem.kind, edgefs.is_some()) {
                (FileSystemKind::EdgeFs, true) => MountPlanKind::EdgeFsReadWrite,
                (FileSystemKind::Fat12 | FileSystemKind::Fat16 | FileSystemKind::Fat32, _) => {
                    MountPlanKind::ForeignReadOnly
                }
                (FileSystemKind::ExFat | FileSystemKind::Iso9660, _) => {
                    MountPlanKind::ForeignReadOnly
                }
                (FileSystemKind::Ext, _) => MountPlanKind::FutureNative,
                _ => MountPlanKind::Unknown,
            };

            entries.push(MountPlanEntry {
                partition: partition.clone(),
                filesystem,
                edgefs,
                kind,
            });
        }

        Ok(DiskMountPlan { table, entries })
    }

    pub fn scan_edgefs_partitions<S: BlockStorage>(
        device: &mut S,
        key: [u8; 32],
    ) -> Result<(PartitionTable, alloc::vec::Vec<EdgeFsPartition>), DiskBootError> {
        let table = detect_partitions(device)?;
        let mut matches = alloc::vec::Vec::new();
        for partition in &table.partitions {
            let slice =
                PartitionBlockDevice::new(&mut *device, partition.start_lba, partition.sectors)?;
            if let Ok(info) = EdgeFs::probe(slice, key) {
                matches.push(EdgeFsPartition {
                    partition: partition.clone(),
                    info,
                });
            }
        }
        Ok((table, matches))
    }

    pub fn open_first_edgefs_partition<S: BlockStorage>(
        device: S,
        key: [u8; 32],
    ) -> Result<EdgeFs<PartitionBlockDevice<S>>, DiskBootError> {
        let mut scan_device = device;
        let (_, matches) = scan_edgefs_partitions(&mut scan_device, key)?;
        let Some(first) = matches.first() else {
            return Err(DiskBootError::NoEdgeFsPartition);
        };
        let partition = PartitionBlockDevice::new(
            scan_device,
            first.partition.start_lba,
            first.partition.sectors,
        )?;
        EdgeFs::open(partition, key).map_err(Into::into)
    }

    pub fn open_whole_disk_edgefs<S: BlockStorage>(
        device: S,
        key: [u8; 32],
    ) -> Result<EdgeFs<S>, DiskBootError> {
        EdgeFs::open(device, key).map_err(Into::into)
    }

    pub fn open_first_fat_partition_readonly<S: BlockStorage>(
        device: S,
    ) -> Result<FatReadOnly<PartitionBlockDevice<S>>, DiskBootError> {
        let mut scan_device = device;
        let (_, probes) = scan_partition_filesystems(&mut scan_device)?;
        let Some(first) = probes.iter().find(|probe| {
            matches!(
                probe.filesystem.kind,
                FileSystemKind::Fat12 | FileSystemKind::Fat16 | FileSystemKind::Fat32
            )
        }) else {
            return Err(DiskBootError::NoFatPartition);
        };
        let partition = PartitionBlockDevice::new(
            scan_device,
            first.partition.start_lba,
            first.partition.sectors,
        )?;
        FatReadOnly::open(partition).map_err(Into::into)
    }

    pub fn read_first_fat_file_8_3<S: BlockStorage>(
        device: S,
        path: &str,
    ) -> Result<alloc::vec::Vec<u8>, DiskBootError> {
        read_first_fat_file(device, path)
    }

    pub fn read_first_fat_file<S: BlockStorage>(
        device: S,
        path: &str,
    ) -> Result<alloc::vec::Vec<u8>, DiskBootError> {
        let mut fat = open_first_fat_partition_readonly(device)?;
        fat.read_file(path).map_err(Into::into)
    }

    pub fn read_first_fat_boot_config<S: BlockStorage>(
        device: S,
        path: &str,
    ) -> Result<BootConfig, DiskBootError> {
        let bytes = read_first_fat_file_8_3(device, path)?;
        BootConfig::parse(&bytes).map_err(Into::into)
    }

    pub fn read_default_fat_boot_config<S: BlockStorage>(
        device: S,
    ) -> Result<BootConfig, DiskBootError> {
        let mut fat = open_first_fat_partition_readonly(device)?;
        for path in DEFAULT_BOOT_CONFIG_PATHS {
            match fat.read_file(path) {
                Ok(bytes) => return BootConfig::parse(&bytes).map_err(Into::into),
                Err(FatError::NotFound(_)) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(DiskBootError::NoBootConfig)
    }

    pub fn read_default_fat_boot_config_mut<S: BlockStorage>(
        device: &mut S,
    ) -> Result<BootConfig, DiskBootError> {
        let mut fat = open_first_fat_partition_readonly(&mut *device)?;
        for path in DEFAULT_BOOT_CONFIG_PATHS {
            match fat.read_file(path) {
                Ok(bytes) => return BootConfig::parse(&bytes).map_err(Into::into),
                Err(FatError::NotFound(_)) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(DiskBootError::NoBootConfig)
    }

    pub fn open_configured_edgefs<S: BlockStorage>(
        device: S,
        config: &BootConfig,
        key: [u8; 32],
        fs_id: [u8; 16],
    ) -> Result<OpenedEdgeFs<S>, DiskBootError> {
        match config.edgefs {
            EdgeFsBootTarget::ExistingPartition => {
                open_first_edgefs_partition(device, key).map(OpenedEdgeFs::Partition)
            }
            EdgeFsBootTarget::FormatFirstPartition => {
                format_first_partition_as_edgefs(device, key, fs_id).map(OpenedEdgeFs::Partition)
            }
            EdgeFsBootTarget::FormatWholeDisk => {
                format_whole_disk_edgefs(device, key, fs_id).map(OpenedEdgeFs::WholeDisk)
            }
        }
    }

    pub fn format_partition_as_edgefs<S: BlockStorage>(
        device: S,
        partition: &PartitionEntry,
        key: [u8; 32],
        fs_id: [u8; 16],
    ) -> Result<EdgeFs<PartitionBlockDevice<S>>, DiskBootError> {
        let partition = PartitionBlockDevice::new(device, partition.start_lba, partition.sectors)?;
        EdgeFs::format_with_id(partition, key, fs_id).map_err(Into::into)
    }

    pub fn format_first_partition_as_edgefs<S: BlockStorage>(
        device: S,
        key: [u8; 32],
        fs_id: [u8; 16],
    ) -> Result<EdgeFs<PartitionBlockDevice<S>>, DiskBootError> {
        let mut scan_device = device;
        let table = detect_partitions(&mut scan_device)?;
        let Some(first) = table.partitions.first() else {
            return Err(DiskBootError::NoPartition);
        };
        format_partition_as_edgefs(scan_device, first, key, fs_id)
    }

    pub fn format_whole_disk_edgefs<S: BlockStorage>(
        device: S,
        key: [u8; 32],
        fs_id: [u8; 16],
    ) -> Result<EdgeFs<S>, DiskBootError> {
        EdgeFs::format_with_id(device, key, fs_id).map_err(Into::into)
    }
}

#[cfg(target_os = "none")]
core::arch::global_asm!(
    r#"
    .section .text.entry,"ax"
    .global _start
_start:
    lea rsp, [rip + _stack]
    xor rbp, rbp

    lea rdi, [rip + _bss_start]
    lea rcx, [rip + _bss_end]
    sub rcx, rdi
    xor eax, eax
    rep stosb

    call kernel_main

1:
    hlt
    jmp 1b
"#
);

struct PumpStats {
    arp_replies: u32,
    icmp_replies: u32,
}

struct NetPump<'net, 'stack> {
    net: &'net mut edgerun_virtio::VirtNet,
    network: Network<'stack>,
    rx_buf: [u8; 1514],
    logged_start: bool,
    logged_arp: bool,
    logged_icmp: bool,
}

fn poll_network(
    net: &mut edgerun_virtio::VirtNet,
    network: &mut Network<'_>,
    rx_buf: &mut [u8; 1514],
) -> PumpStats {
    let mut stats = PumpStats {
        arp_replies: 0,
        icmp_replies: 0,
    };

    while let Some(len) = net.recv(rx_buf) {
        match network.recv(&rx_buf[..len]) {
            Some(ParsedPacket::Arp { header, .. }) => {
                if header.oper == ARP_OP_REQUEST
                    && header.tpa == *network.stack.ip.as_bytes()
                    && network
                        .send_arp_reply(&header)
                        .map(|packet| net.send(packet))
                        .unwrap_or(false)
                {
                    stats.arp_replies = stats.arp_replies.wrapping_add(1);
                }
            }
            Some(ParsedPacket::Icmp {
                eth,
                ip,
                header,
                payload,
            }) => {
                if header.icmp_type == ICMP_ECHO_REQUEST
                    && ip.dst == *network.stack.ip.as_bytes()
                    && network
                        .send_icmp_echo_reply(
                            eth.src,
                            IpAddr::from_slice(&ip.src),
                            &header,
                            payload,
                        )
                        .map(|packet| net.send(packet))
                        .unwrap_or(false)
                {
                    stats.icmp_replies = stats.icmp_replies.wrapping_add(1);
                }
            }
            _ => {}
        }
    }

    stats
}

fn dhcp_ipv4_to_rt(ip: edgerun_dhcp::Ipv4Addr) -> IpAddr {
    let octets = ip.octets();
    IpAddr::new(octets[0], octets[1], octets[2], octets[3])
}

#[cfg(target_os = "none")]
unsafe fn probe_tpm2() -> Option<[u8; 32]> {
    rt::log::log(1, "Looking for TPM2 ACPI table...");
    match unsafe { edgerun_tpm::CrbTpmTransport::discover_acpi() } {
        Ok(Some(transport)) => {
            rt::log::log(1, "TPM2 CRB transport discovered");
            return probe_tpm2_transport(transport.with_timeout_polls(10));
        }
        Ok(None) => {
            rt::log::log(1, "No TPM2 ACPI table found");
        }
        Err(_) => {
            rt::log::log(1, "TPM2 ACPI discovery failed");
        }
    }

    rt::log::log(1, "Trying TPM2 TIS transport...");
    let transport = unsafe { edgerun_tpm::TisTpmTransport::new_default_x86() };
    probe_tpm2_transport(transport.with_timeout_polls(100_000))
}

#[cfg(target_os = "none")]
fn fill_bare_random_source(out: &mut [u8]) -> edgerun_crypto::error::Result<()> {
    if out.is_empty() {
        return Ok(());
    }

    if let Some(mut rng) = edgerun_virtio::find_virtio_rng() {
        if rng.init() && rng.fill_bytes(out) {
            return Ok(());
        }
    }

    fill_tpm2_random_source(out)
}

#[cfg(target_os = "none")]
fn fill_tpm2_random_source(out: &mut [u8]) -> edgerun_crypto::error::Result<()> {
    if out.is_empty() {
        return Ok(());
    }

    if let Ok(Some(transport)) = unsafe { edgerun_tpm::CrbTpmTransport::discover_acpi() } {
        if fill_tpm2_random_transport(transport.with_timeout_polls(10), out) {
            return Ok(());
        }
    }

    let transport = unsafe { edgerun_tpm::TisTpmTransport::new_default_x86() };
    if fill_tpm2_random_transport(transport.with_timeout_polls(100_000), out) {
        Ok(())
    } else {
        Err(edgerun_crypto::error::CryptoError::TpmUnavailable)
    }
}

#[cfg(target_os = "none")]
fn fill_tpm2_random_transport<T>(transport: T, out: &mut [u8]) -> bool
where
    T: edgerun_tpm::FixedTpmTransport + edgerun_tpm::TpmTransport,
{
    let mut device = edgerun_tpm::TpmDevice::new(transport);
    let startup_code = device.startup_response_code(edgerun_tpm::TPM_SU_CLEAR);
    if startup_code != edgerun_tpm::TPM_RC_SUCCESS && startup_code != 0x100 && startup_code != 0x120
    {
        return false;
    }

    let mut offset = 0usize;
    while offset < out.len() {
        let end = core::cmp::min(offset + 2048, out.len());
        let n = device.get_random_into(&mut out[offset..end]);
        if n == 0 || offset + n > end {
            return false;
        }
        offset += n;
    }
    true
}

#[cfg(target_os = "none")]
fn probe_tpm2_transport<T>(transport: T) -> Option<[u8; 32]>
where
    T: edgerun_tpm::FixedTpmTransport + edgerun_tpm::TpmTransport,
{
    let mut device = edgerun_tpm::TpmDevice::new(transport);
    let startup_code = device.startup_response_code(edgerun_tpm::TPM_SU_CLEAR);
    if startup_code == edgerun_tpm::TPM_RC_SUCCESS || startup_code == 0x100 || startup_code == 0x120
    {
        rt::log::log(1, "TPM2 startup ok");
    } else if startup_code == 0x144 {
        rt::log::log(1, "TPM2 startup failed: command size");
        return None;
    } else if startup_code == 0xffff_fffb {
        rt::log::log(1, "TPM2 startup failed: transport");
        return None;
    } else if startup_code == 0xffff_fffc {
        rt::log::log(1, "TPM2 startup failed: malformed response");
        return None;
    } else {
        rt::log::log(1, "TPM2 startup failed");
        return None;
    }

    let mut tpm_random = [0u8; 32];
    let random_len = device.get_random_into(&mut tpm_random);
    let entropy = if random_len != 0 {
        rt::log::log(1, "TPM2 random ok");
        edgerun_crypto::rng::mix_entropy(&tpm_random[..random_len]);
        Some(tpm_random)
    } else {
        rt::log::log(1, "TPM2 random failed");
        None
    };

    let Some(entropy) = entropy else {
        return None;
    };

    let read_public_code = device.read_public_response_code(edgerun_tpm::TpmHandle(0x8100_0001));
    if read_public_code == edgerun_tpm::TPM_RC_SUCCESS {
        rt::log::log(1, "TPM2 persistent key readable");
    } else {
        rt::log::log(1, "TPM2 persistent key not readable");
        return Some(entropy);
    }

    let digest = [0u8; 32];
    let mut signature = [0u8; 64];
    if device
        .sign_p256_sha256_into(edgerun_tpm::TpmHandle(0x8100_0001), &digest, &mut signature)
        .is_ok()
    {
        rt::log::log(1, "TPM2 sign ok");
    } else {
        rt::log::log(1, "TPM2 sign failed");
    }

    Some(entropy)
}

impl Future for NetPump<'_, '_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if !this.logged_start {
            rt::log::log(1, "Net pump started");
            this.logged_start = true;
        }

        let stats = poll_network(this.net, &mut this.network, &mut this.rx_buf);
        if stats.arp_replies != 0 && !this.logged_arp {
            rt::log::log(1, "ARP reply sent");
            this.logged_arp = true;
        }
        if stats.icmp_replies != 0 && !this.logged_icmp {
            rt::log::log(1, "ICMP echo reply sent");
            this.logged_icmp = true;
        }

        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

#[cfg(target_os = "none")]
#[panic_handler]
unsafe fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::arch::asm!("hlt");
    }
}

#[used]
#[cfg(target_os = "none")]
#[link_section = ".multiboot"]
static MULTIBOOT_HEADER: [u32; 8] = [
    0x1BADB002, 0x00010000, 0xE4514FFE, 0x100000, 0x100000, 0, 0, 0,
];

#[no_mangle]
#[cfg(target_os = "none")]
pub unsafe extern "C" fn kernel_main() -> ! {
    rt::timer::set_now(0);
    rt::log::log(1, "Starting edgerun unikernel");

    edgerun_crypto::rng::register_random_source(fill_bare_random_source);

    let mut rng = Rng::new_from_entropy();
    if let Some(mut virtio_rng) = edgerun_virtio::find_virtio_rng() {
        if virtio_rng.init() {
            let mut virtio_entropy = [0u8; 32];
            if virtio_rng.fill_bytes(&mut virtio_entropy) {
                rng.mix_entropy(&virtio_entropy);
                edgerun_crypto::rng::mix_entropy(&virtio_entropy);
                rt::log::log(1, "RNG mixed VirtIO entropy");
            } else {
                rt::log::log(1, "VirtIO RNG read failed");
            }
        } else {
            rt::log::log(1, "VirtIO RNG init failed");
        }
    } else {
        rt::log::log(1, "No VirtIO RNG found");
    }

    if let Some(tpm_entropy) = unsafe { probe_tpm2() } {
        rng.mix_entropy(&tpm_entropy);
        rt::log::log(1, "RNG mixed TPM entropy");
    }

    let test_crc = crc32(b"hello");
    let _ = test_crc;

    let mut rx_buf = RingBuffer::new(1024);
    rx_buf.push_slice(b"test packet");
    let _ = rx_buf.len();

    rt::log::log(1, "Looking for VirtIO...");

    if let Some(mut console) = edgerun_virtio::find_virtio_console() {
        if console.init() {
            let _ = console.write_all(b"edgerun: virtio-console online\n");
            rt::log::log(1, "VirtIO console init ok");
        } else {
            rt::log::log(1, "VirtIO console init failed");
        }
    } else {
        rt::log::log(1, "No VirtIO console found");
    }

    if let Some(mut block) = edgerun_virtio::find_virtio_blk() {
        rt::log::log(1, "VirtIO block device found");
        if block.init() {
            rt::log::log(1, "VirtIO block init ok");
            let mut first_sector = [0u8; 512];
            if block.read_sector(0, &mut first_sector) {
                rt::log::log(1, "VirtIO block first sector read ok");
            } else {
                rt::log::log(1, "VirtIO block first sector read failed");
            }
            if block.read_sector(0, &mut first_sector) {
                rt::log::log(1, "VirtIO block second sector read ok");
            } else {
                rt::log::log(1, "VirtIO block second sector read failed");
            }
            if first_sector[510] == 0x55 && first_sector[511] == 0xaa {
                let mut partition_sector = [0u8; 512];
                if block.read_sector(2048, &mut partition_sector) {
                    rt::log::log(1, "VirtIO block partition sector read ok");
                } else {
                    rt::log::log(1, "VirtIO block partition sector read failed");
                }
                if block.read_sector(2112, &mut partition_sector) {
                    rt::log::log(1, "VirtIO block probe sector read ok");
                } else {
                    rt::log::log(1, "VirtIO block probe sector read failed");
                }
                if block.read_sector(2308, &mut partition_sector) {
                    rt::log::log(1, "VirtIO block FAT root sector read ok");
                } else {
                    rt::log::log(1, "VirtIO block FAT root sector read failed");
                }
            }
            rt::log::log(1, "VirtIO block scanning partitions");
            let mut storage = disk_boot::RtBlockDeviceStorage::new(block);
            match edgerun_storage::BlockStorage::read_sector(&mut storage, 0, &mut first_sector) {
                Ok(()) => rt::log::log(1, "VirtIO storage adapter read ok"),
                Err(_) => rt::log::log(1, "VirtIO storage adapter read failed"),
            }
            if first_sector[510] != 0x55 || first_sector[511] != 0xaa {
                rt::log::log(1, "VirtIO block has no partitions");
            } else {
                match edgerun_storage::detect_partitions(&mut storage) {
                    Ok(table) if !table.partitions.is_empty() => {
                        rt::log::log(1, "VirtIO block partition table detected");
                        for partition in &table.partitions {
                            let mut partition_boot_sector = [0u8; 512];
                            match edgerun_storage::BlockStorage::read_sector(
                                &mut storage,
                                partition.start_lba,
                                &mut partition_boot_sector,
                            ) {
                                Ok(()) if partition_boot_sector.iter().all(|byte| *byte == 0) => {
                                    rt::log::log(1, "VirtIO partition is blank")
                                }
                                Ok(()) => {
                                    rt::log::log(1, "VirtIO block partitions detected");
                                    let mut partition_storage = disk_boot::RtPartitionStorage::new(
                                        &mut storage,
                                        partition.start_lba,
                                        partition.sectors,
                                    );
                                    let mut fat_root_sector = [0u8; 512];
                                    match edgerun_storage::BlockStorage::read_sector(
                                        &mut partition_storage,
                                        260,
                                        &mut fat_root_sector,
                                    ) {
                                        Ok(()) => {
                                            rt::log::log(1, "VirtIO FAT relative root read ok")
                                        }
                                        Err(_) => {
                                            rt::log::log(1, "VirtIO FAT relative root read failed")
                                        }
                                    }
                                    rt::log::log(1, "VirtIO opening FAT boot partition");
                                    match edgerun_storage::FatReadOnly::open(partition_storage) {
                                        Ok(mut fat) => {
                                            rt::log::log(1, "VirtIO FAT boot partition open ok");
                                            match fat.root_entries() {
                                                Ok(_) => rt::log::log(
                                                    1,
                                                    "VirtIO FAT root directory read ok",
                                                ),
                                                Err(_) => rt::log::log(
                                                    1,
                                                    "VirtIO FAT root directory read failed",
                                                ),
                                            }
                                            rt::log::log(1, "VirtIO FAT boot config read start");
                                            match fat.read_file("/edgerun/boot.cfg") {
                                                Ok(bytes) => {
                                                    rt::log::log(
                                                        1,
                                                        "VirtIO FAT boot config bytes read ok",
                                                    );
                                                    match boot_config::BootConfig::parse(&bytes) {
                                                        Ok(_) => rt::log::log(
                                                            1,
                                                            "VirtIO FAT boot config read ok",
                                                        ),
                                                        Err(_) => rt::log::log(
                                                            1,
                                                            "VirtIO FAT boot config parse failed",
                                                        ),
                                                    }
                                                }
                                                Err(_) => rt::log::log(
                                                    1,
                                                    "VirtIO FAT boot config missing",
                                                ),
                                            }
                                        }
                                        Err(_) => rt::log::log(1, "VirtIO partition is not FAT"),
                                    }
                                }
                                Err(_) => rt::log::log(1, "VirtIO partition read failed"),
                            }
                        }
                    }
                    Ok(_) => {
                        rt::log::log(1, "VirtIO block has no partitions");
                    }
                    Err(_) => {
                        rt::log::log(1, "VirtIO block partition scan failed");
                    }
                }
            }
        } else {
            rt::log::log(1, "VirtIO block init failed");
        }
    } else {
        rt::log::log(1, "No VirtIO block device found");
    }

    let mut net = match edgerun_virtio::find_virtio_net() {
        Some(n) => n,
        None => {
            rt::log::log(1, "No VirtIO found");
            loop {
                core::arch::asm!("hlt");
            }
        }
    };

    rt::log::log(1, "VirtIO found");
    if !net.init() {
        rt::log::log(1, "VirtIO init failed");
        loop {
            core::arch::asm!("hlt");
        }
    }
    let mac = net.get_mac();

    let mut stack = IpStack::new();
    stack.configure(
        IpAddr::new(0, 0, 0, 0),
        IpAddr::new(255, 255, 255, 0),
        IpAddr::zero(),
        mac,
    );

    let dhcp_xid = 0x12345678;
    let mut dhcp_ip = IpAddr::zero();
    let mut dhcp_netmask = IpAddr::new(255, 255, 255, 0);
    let mut dhcp_gateway = IpAddr::zero();
    let mut offered_ip = None;
    let mut offered_netmask = None;
    let mut offered_gateway = None;
    let mut requested_lease = false;
    let mut network = Network::new(&mut stack);

    let discover = DhcpMessage::discover(dhcp_xid, mac).to_wire();
    rt::log::log(1, "Sending DHCP discover");
    if let Some(pkt) = network.send_udp(
        IpAddr::new(255, 255, 255, 255),
        DHCP_CLIENT_PORT,
        DHCP_SERVER_PORT,
        &discover,
    ) {
        if net.send(pkt) {
            rt::log::log(1, "DHCP discover queued");
        } else {
            rt::log::log(1, "DHCP discover send failed");
        }
    }

    let mut rx_buf = [0u8; 1514];
    let mut logged_rx = false;
    for _ in 0..1000 {
        if let Some(len) = net.recv(&mut rx_buf) {
            if !logged_rx {
                rt::log::log(1, "VirtIO RX packet observed");
                logged_rx = true;
            }
            if let Some(ParsedPacket::Udp {
                header, payload, ..
            }) = network.recv(&rx_buf[..len])
            {
                if header.src_port == DHCP_SERVER_PORT && header.dst_port == DHCP_CLIENT_PORT {
                    if let Ok(message) = DhcpMessage::from_wire(payload) {
                        if message.xid != dhcp_xid || message.chaddr[..6] != mac {
                            continue;
                        }

                        match message.options.message_type {
                            Some(DhcpMessageType::Offer) if !requested_lease => {
                                let Some(server_id) = message.options.server_id else {
                                    continue;
                                };
                                offered_ip = Some(message.yiaddr);
                                offered_netmask = message.options.subnet_mask;
                                offered_gateway = message.options.router;

                                let request =
                                    DhcpMessage::request(dhcp_xid, mac, message.yiaddr, server_id)
                                        .to_wire();
                                if let Some(pkt) = network.send_udp(
                                    IpAddr::new(255, 255, 255, 255),
                                    DHCP_CLIENT_PORT,
                                    DHCP_SERVER_PORT,
                                    &request,
                                ) {
                                    if net.send(pkt) {
                                        requested_lease = true;
                                        rt::log::log(1, "DHCP request queued");
                                    } else {
                                        rt::log::log(1, "DHCP request send failed");
                                    }
                                }
                            }
                            Some(DhcpMessageType::Ack) if requested_lease => {
                                dhcp_ip = dhcp_ipv4_to_rt(message.yiaddr);
                                if dhcp_ip == IpAddr::zero() {
                                    if let Some(ip) = offered_ip {
                                        dhcp_ip = dhcp_ipv4_to_rt(ip);
                                    }
                                }
                                if let Some(netmask) =
                                    message.options.subnet_mask.or(offered_netmask)
                                {
                                    dhcp_netmask = dhcp_ipv4_to_rt(netmask);
                                }
                                if let Some(gateway) = message.options.router.or(offered_gateway) {
                                    dhcp_gateway = dhcp_ipv4_to_rt(gateway);
                                }
                                rt::log::log(1, "DHCP lease accepted");
                                break;
                            }
                            Some(DhcpMessageType::Nak) => {
                                rt::log::log(1, "DHCP lease rejected");
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    let net_stats = net.stats();
    if net_stats.tx_completed != 0 {
        rt::log::log(1, "VirtIO TX completed");
    } else {
        rt::log::log(1, "VirtIO TX pending");
    }
    if net_stats.rx_received == 0 {
        rt::log::log(1, "VirtIO RX no packets");
    }

    drop(network);

    if dhcp_ip != IpAddr::zero() {
        stack.ip = dhcp_ip;
        stack.netmask = dhcp_netmask;
        stack.gateway = dhcp_gateway;
    } else {
        rt::log::log(1, "Using static fallback IP");
        stack.ip = IpAddr::new(192, 168, 1, 12);
    }

    let mut network = Network::new(&mut stack);
    let tftp_server = if network.stack.gateway != IpAddr::zero() {
        network.stack.gateway
    } else {
        IpAddr::new(192, 168, 1, 1)
    };
    let rrq = TftpMessage::rrq("edgerun.bin").to_wire();
    rt::log::log(1, "Sending TFTP RRQ");
    if let Some(pkt) = network.send_udp(tftp_server, 2070, TFTP_PORT, &rrq) {
        if net.send(pkt) {
            rt::log::log(1, "TFTP RRQ queued");
        } else {
            rt::log::log(1, "TFTP RRQ send failed");
        }
    }
    let mut tcp = TcpSocket::new();

    let addr = rt::SocketAddr::new(0xC0A8010C, 8080);
    let _ = tcp.bind(addr);
    let _ = tcp.listen(10);
    let _ = rng.next();

    if net.is_link_up() {}

    block_on(NetPump {
        net: &mut net,
        network,
        rx_buf: [0; 1514],
        logged_start: false,
        logged_arp: false,
        logged_icmp: false,
    });

    loop {
        core::arch::asm!("hlt");
    }
}

#[cfg(not(target_os = "none"))]
fn main() {}
