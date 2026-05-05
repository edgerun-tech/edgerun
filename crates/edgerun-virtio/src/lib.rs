//! Modern virtio PCI/MMIO device support for bare EdgeRun targets.

#![no_std]
#![allow(dead_code)]

use core::sync::atomic::{fence, AtomicBool, Ordering};

#[cfg(test)]
extern crate std;

pub const VIRTIO_VENDOR_ID: u16 = 0x1af4;
pub const VIRTIO_MODERN_DEVICE_ID_NET: u16 = 0x1041;
pub const VIRTIO_MODERN_DEVICE_ID_BLK: u16 = 0x1042;
pub const VIRTIO_MODERN_DEVICE_ID_CONSOLE: u16 = 0x1043;
pub const VIRTIO_MODERN_DEVICE_ID_RNG: u16 = 0x1044;

pub const VIRTIO_DEVICE_TYPE_NET: u32 = 1;
pub const VIRTIO_DEVICE_TYPE_BLK: u32 = 2;
pub const VIRTIO_DEVICE_TYPE_CONSOLE: u32 = 3;
pub const VIRTIO_DEVICE_TYPE_RNG: u32 = 4;

pub const VIRTIO_NET_F_CSUM: u64 = 1 << 0;
pub const VIRTIO_NET_F_GUEST_CSUM: u64 = 1 << 1;
pub const VIRTIO_NET_F_MAC: u64 = 1 << 5;
pub const VIRTIO_NET_F_STATUS: u64 = 1 << 16;
pub const VIRTIO_BLK_F_SIZE_MAX: u64 = 1 << 1;
pub const VIRTIO_BLK_F_SEG_MAX: u64 = 1 << 2;
pub const VIRTIO_BLK_F_RO: u64 = 1 << 5;
pub const VIRTIO_BLK_F_BLK_SIZE: u64 = 1 << 6;
pub const VIRTIO_BLK_F_FLUSH: u64 = 1 << 9;
pub const VIRTIO_F_VERSION_1: u64 = 1 << 32;

pub const VIRTIO_CONFIG_STATUS_ACKNOWLEDGE: u8 = 1;
pub const VIRTIO_CONFIG_STATUS_DRIVER: u8 = 2;
pub const VIRTIO_CONFIG_STATUS_DRIVER_OK: u8 = 4;
pub const VIRTIO_CONFIG_STATUS_FEATURES_OK: u8 = 8;
pub const VIRTIO_CONFIG_STATUS_FAILED: u8 = 0x80;

pub const VIRTIO_NET_S_LINK_UP: u16 = 1;

pub const VIRTIO_BLK_T_IN: u32 = 0;
pub const VIRTIO_BLK_T_OUT: u32 = 1;
pub const VIRTIO_BLK_T_FLUSH: u32 = 4;
pub const VIRTIO_BLK_S_OK: u8 = 0;
pub const SECTOR_SIZE: usize = 512;

const VIRTIO_PCI_CAP_VENDOR: u8 = 0x09;
const VIRTIO_PCI_CAP_COMMON_CFG: u8 = 1;
const VIRTIO_PCI_CAP_NOTIFY_CFG: u8 = 2;
const VIRTIO_PCI_CAP_ISR_CFG: u8 = 3;
const VIRTIO_PCI_CAP_DEVICE_CFG: u8 = 4;

const QUEUE_SIZE: usize = 16;
const RX_QUEUE: u16 = 0;
const TX_QUEUE: u16 = 1;
const CONSOLE_RX_QUEUE: u16 = 0;
const CONSOLE_TX_QUEUE: u16 = 1;
const NET_HDR_LEN: usize = core::mem::size_of::<VirtioNetHdr>();
const BUFFER_SIZE: usize = 2048;
const CONSOLE_BUFFER_SIZE: usize = 256;
const TX_FREE_ALL_MASK: u16 = u16::MAX >> (u16::BITS as usize - QUEUE_SIZE);
const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;
const VIRTIO_POLL_SPINS: usize = 5_000;

const PCI_CONFIG_ADDRESS: u16 = 0xcf8;
const PCI_CONFIG_DATA: u16 = 0xcfc;
const PCI_COMMAND: u8 = 0x04;
const PCI_STATUS: u8 = 0x06;
const PCI_HEADER_TYPE: u8 = 0x0e;
const PCI_BAR0: u8 = 0x10;
const PCI_CAPABILITY_LIST: u8 = 0x34;
const PCI_COMMAND_MEMORY: u16 = 0x0002;
const PCI_COMMAND_BUS_MASTER: u16 = 0x0004;
const PCI_STATUS_CAPABILITIES: u16 = 0x0010;

const VIRTIO_MMIO_MAGIC: u32 = 0x7472_6976;
const VIRTIO_MMIO_VERSION_MODERN: u32 = 2;
const VIRTIO_MMIO_VENDOR_ID: u16 = 0xffff;
const VIRTIO_MMIO_MAGIC_VALUE: usize = 0x000;
const VIRTIO_MMIO_VERSION: usize = 0x004;
const VIRTIO_MMIO_DEVICE_ID: usize = 0x008;
const VIRTIO_MMIO_VENDOR: usize = 0x00c;
const VIRTIO_MMIO_DEVICE_FEATURES: usize = 0x010;
const VIRTIO_MMIO_DEVICE_FEATURES_SEL: usize = 0x014;
const VIRTIO_MMIO_DRIVER_FEATURES: usize = 0x020;
const VIRTIO_MMIO_DRIVER_FEATURES_SEL: usize = 0x024;
const VIRTIO_MMIO_QUEUE_SEL: usize = 0x030;
const VIRTIO_MMIO_QUEUE_NUM_MAX: usize = 0x034;
const VIRTIO_MMIO_QUEUE_NUM: usize = 0x038;
const VIRTIO_MMIO_QUEUE_READY: usize = 0x044;
const VIRTIO_MMIO_QUEUE_NOTIFY: usize = 0x050;
const VIRTIO_MMIO_INTERRUPT_STATUS: usize = 0x060;
const VIRTIO_MMIO_INTERRUPT_ACK: usize = 0x064;
const VIRTIO_MMIO_STATUS: usize = 0x070;
const VIRTIO_MMIO_QUEUE_DESC_LOW: usize = 0x080;
const VIRTIO_MMIO_QUEUE_DESC_HIGH: usize = 0x084;
const VIRTIO_MMIO_QUEUE_DRIVER_LOW: usize = 0x090;
const VIRTIO_MMIO_QUEUE_DRIVER_HIGH: usize = 0x094;
const VIRTIO_MMIO_QUEUE_DEVICE_LOW: usize = 0x0a0;
const VIRTIO_MMIO_QUEUE_DEVICE_HIGH: usize = 0x0a4;
const VIRTIO_MMIO_CONFIG: usize = 0x100;
const VIRTIO_INTERRUPT_USED_RING: u8 = 1;
const VIRTIO_INTERRUPT_CONFIG_CHANGE: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtioTransportKind {
    ModernPci,
    Mmio,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtioDeviceInfo {
    pub transport: VirtioTransportKind,
    pub device_type: u32,
    pub vendor_id: u16,
    pub device_id: u16,
    pub bus: u8,
    pub slot: u8,
    pub func: u8,
    pub mmio_base: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirtioInterruptStatus {
    pub raw: u8,
    pub used_ring: bool,
    pub config_change: bool,
}

impl VirtioInterruptStatus {
    fn from_raw(raw: u8) -> Self {
        Self {
            raw,
            used_ring: raw & VIRTIO_INTERRUPT_USED_RING != 0,
            config_change: raw & VIRTIO_INTERRUPT_CONFIG_CHANGE != 0,
        }
    }
}

impl VirtioDeviceInfo {
    pub fn open_net(self) -> Option<VirtNet> {
        if self.device_type != VIRTIO_DEVICE_TYPE_NET {
            return None;
        }

        match self.transport {
            VirtioTransportKind::ModernPci => {
                let device = read_modern_virtio_device(self.bus, self.slot, self.func)?;
                VirtNet::from_modern_device(device)
            }
            VirtioTransportKind::Mmio => VirtNet::from_mmio_base(self.mmio_base),
        }
    }

    pub fn open_initialized_net(self) -> Option<VirtNet> {
        initialized_device(self.open_net()?, VirtNet::init)
    }

    pub fn open_blk(self) -> Option<VirtBlk> {
        if self.device_type != VIRTIO_DEVICE_TYPE_BLK {
            return None;
        }

        match self.transport {
            VirtioTransportKind::ModernPci => {
                let device = read_modern_virtio_device(self.bus, self.slot, self.func)?;
                VirtBlk::from_modern_device(device)
            }
            VirtioTransportKind::Mmio => VirtBlk::from_mmio_base(self.mmio_base),
        }
    }

    pub fn open_initialized_blk(self) -> Option<VirtBlk> {
        initialized_device(self.open_blk()?, VirtBlk::init)
    }

    pub fn open_rng(self) -> Option<VirtRng> {
        if self.device_type != VIRTIO_DEVICE_TYPE_RNG {
            return None;
        }

        match self.transport {
            VirtioTransportKind::ModernPci => {
                let device = read_modern_virtio_device(self.bus, self.slot, self.func)?;
                VirtRng::from_modern_device(device)
            }
            VirtioTransportKind::Mmio => VirtRng::from_mmio_base(self.mmio_base),
        }
    }

    pub fn open_initialized_rng(self) -> Option<VirtRng> {
        initialized_device(self.open_rng()?, VirtRng::init)
    }

    pub fn open_console(self) -> Option<VirtConsole> {
        if self.device_type != VIRTIO_DEVICE_TYPE_CONSOLE {
            return None;
        }

        match self.transport {
            VirtioTransportKind::ModernPci => {
                let device = read_modern_virtio_device(self.bus, self.slot, self.func)?;
                VirtConsole::from_modern_device(device)
            }
            VirtioTransportKind::Mmio => VirtConsole::from_mmio_base(self.mmio_base),
        }
    }

    pub fn open_initialized_console(self) -> Option<VirtConsole> {
        initialized_device(self.open_console()?, VirtConsole::init)
    }
}

#[derive(Clone, Copy)]
struct VirtioPciCap {
    cfg_type: u8,
    bar: u8,
    offset: u32,
    length: u32,
    notify_off_multiplier: u32,
}

#[derive(Clone, Copy)]
struct ModernVirtioDevice {
    bus: u8,
    slot: u8,
    func: u8,
    common: VirtioPciCap,
    notify: VirtioPciCap,
    device: Option<VirtioPciCap>,
    isr: Option<VirtioPciCap>,
}

#[derive(Clone, Copy)]
struct MappedModernVirtioDevice {
    bus: u8,
    slot: u8,
    func: u8,
    common_cfg: *mut u8,
    notify_cfg: *mut u8,
    device_cfg: *mut u8,
    isr_cfg: *mut u8,
    notify_off_multiplier: u32,
}

#[derive(Clone, Copy)]
struct DriverTransport {
    bus: u8,
    slot: u8,
    func: u8,
    mmio: bool,
    common_cfg: *mut u8,
    notify_cfg: *mut u8,
    device_cfg: *mut u8,
    isr_cfg: *mut u8,
    notify_off_multiplier: u32,
}

impl DriverTransport {
    const fn empty() -> Self {
        Self {
            bus: 0,
            slot: 0,
            func: 0,
            mmio: false,
            common_cfg: core::ptr::null_mut(),
            notify_cfg: core::ptr::null_mut(),
            device_cfg: core::ptr::null_mut(),
            isr_cfg: core::ptr::null_mut(),
            notify_off_multiplier: 0,
        }
    }

    fn from_modern(mapped: MappedModernVirtioDevice, device_cfg: *mut u8) -> Self {
        Self {
            bus: mapped.bus,
            slot: mapped.slot,
            func: mapped.func,
            mmio: false,
            common_cfg: mapped.common_cfg,
            notify_cfg: mapped.notify_cfg,
            device_cfg,
            isr_cfg: mapped.isr_cfg,
            notify_off_multiplier: mapped.notify_off_multiplier,
        }
    }

    fn from_mmio(base: *mut u8, device_cfg: *mut u8) -> Self {
        Self {
            mmio: true,
            common_cfg: base,
            notify_cfg: base,
            device_cfg,
            ..Self::empty()
        }
    }

    fn transport(self) -> Option<VirtioTransport> {
        if self.mmio {
            VirtioTransport::mmio(self.common_cfg)
        } else {
            VirtioTransport::modern_pci(
                self.bus,
                self.slot,
                self.func,
                self.common_cfg,
                self.notify_cfg,
                self.device_cfg,
                self.isr_cfg,
                self.notify_off_multiplier,
            )
        }
    }
}

#[derive(Clone, Copy)]
struct PciLocation {
    bus: u8,
    slot: u8,
    func: u8,
}

#[derive(Clone, Copy)]
enum VirtioTransport {
    ModernPci {
        location: PciLocation,
        common_cfg: *mut u8,
        notify_cfg: *mut u8,
        device_cfg: *mut u8,
        isr_cfg: *mut u8,
        notify_off_multiplier: u32,
    },
    Mmio {
        base: *mut u8,
    },
}

impl VirtioTransport {
    fn modern_pci(
        bus: u8,
        slot: u8,
        func: u8,
        common_cfg: *mut u8,
        notify_cfg: *mut u8,
        device_cfg: *mut u8,
        isr_cfg: *mut u8,
        notify_off_multiplier: u32,
    ) -> Option<Self> {
        if common_cfg.is_null() || notify_cfg.is_null() {
            return None;
        }

        Some(Self::ModernPci {
            location: PciLocation { bus, slot, func },
            common_cfg,
            notify_cfg,
            device_cfg,
            isr_cfg,
            notify_off_multiplier,
        })
    }

    fn mmio(base: *mut u8) -> Option<Self> {
        if base.is_null()
            || read_u32(unsafe { base.add(VIRTIO_MMIO_MAGIC_VALUE) }) != VIRTIO_MMIO_MAGIC
        {
            return None;
        }
        if read_u32(unsafe { base.add(VIRTIO_MMIO_VERSION) }) != VIRTIO_MMIO_VERSION_MODERN {
            return None;
        }

        Some(Self::Mmio { base })
    }

    fn common_cfg(self) -> *mut u8 {
        match self {
            Self::ModernPci { common_cfg, .. } => common_cfg,
            Self::Mmio { base } => base,
        }
    }

    fn device_cfg(self) -> *mut u8 {
        match self {
            Self::ModernPci { device_cfg, .. } => device_cfg,
            Self::Mmio { base } => unsafe { base.add(VIRTIO_MMIO_CONFIG) },
        }
    }

    fn status(self) -> u8 {
        match self {
            Self::ModernPci { common_cfg, .. } => read_u8(unsafe { common_cfg.add(20) }),
            Self::Mmio { base } => read_u32(unsafe { base.add(VIRTIO_MMIO_STATUS) }) as u8,
        }
    }

    fn write_status(self, status: u8) {
        match self {
            Self::ModernPci { common_cfg, .. } => write_u8(unsafe { common_cfg.add(20) }, status),
            Self::Mmio { base } => {
                write_u32(unsafe { base.add(VIRTIO_MMIO_STATUS) }, status as u32)
            }
        }
    }

    fn read_device_features(self) -> u64 {
        match self {
            Self::ModernPci { common_cfg, .. } => read_device_features(common_cfg),
            Self::Mmio { base } => {
                write_u32(unsafe { base.add(VIRTIO_MMIO_DEVICE_FEATURES_SEL) }, 0);
                let low = read_u32(unsafe { base.add(VIRTIO_MMIO_DEVICE_FEATURES) }) as u64;
                write_u32(unsafe { base.add(VIRTIO_MMIO_DEVICE_FEATURES_SEL) }, 1);
                let high = read_u32(unsafe { base.add(VIRTIO_MMIO_DEVICE_FEATURES) }) as u64;
                low | (high << 32)
            }
        }
    }

    fn write_driver_features(self, features: u64) {
        match self {
            Self::ModernPci { common_cfg, .. } => write_driver_features(common_cfg, features),
            Self::Mmio { base } => {
                write_u32(unsafe { base.add(VIRTIO_MMIO_DRIVER_FEATURES_SEL) }, 0);
                write_u32(
                    unsafe { base.add(VIRTIO_MMIO_DRIVER_FEATURES) },
                    features as u32,
                );
                write_u32(unsafe { base.add(VIRTIO_MMIO_DRIVER_FEATURES_SEL) }, 1);
                write_u32(
                    unsafe { base.add(VIRTIO_MMIO_DRIVER_FEATURES) },
                    (features >> 32) as u32,
                );
            }
        }
    }

    fn select_queue(self, queue: u16) {
        match self {
            Self::ModernPci { common_cfg, .. } => write_u16(unsafe { common_cfg.add(22) }, queue),
            Self::Mmio { base } => {
                write_u32(unsafe { base.add(VIRTIO_MMIO_QUEUE_SEL) }, queue as u32)
            }
        }
    }

    fn read_queue_size(self) -> u16 {
        match self {
            Self::ModernPci { common_cfg, .. } => read_u16(unsafe { common_cfg.add(24) }),
            Self::Mmio { base } => read_u32(unsafe { base.add(VIRTIO_MMIO_QUEUE_NUM_MAX) }) as u16,
        }
    }

    fn write_queue_ready(self, ready: bool) {
        match self {
            Self::ModernPci { common_cfg, .. } => {
                write_u16(unsafe { common_cfg.add(28) }, u16::from(ready))
            }
            Self::Mmio { base } => write_u32(
                unsafe { base.add(VIRTIO_MMIO_QUEUE_READY) },
                u32::from(ready),
            ),
        }
    }

    fn read_queue_notify_off(self) -> u16 {
        match self {
            Self::ModernPci { common_cfg, .. } => read_u16(unsafe { common_cfg.add(30) }),
            Self::Mmio { .. } => 0,
        }
    }

    fn notify_split_queue(self, queue_off: u16, queue: u16) {
        match self {
            Self::ModernPci {
                notify_cfg,
                notify_off_multiplier,
                ..
            } => notify_split_queue(notify_cfg, notify_off_multiplier, queue_off, queue),
            Self::Mmio { base } => {
                let _ = queue_off;
                write_u32(unsafe { base.add(VIRTIO_MMIO_QUEUE_NOTIFY) }, queue as u32);
            }
        }
    }

    fn take_interrupt_status(self) -> VirtioInterruptStatus {
        let raw = match self {
            Self::ModernPci { isr_cfg, .. } => {
                if isr_cfg.is_null() {
                    0
                } else {
                    read_u8(isr_cfg)
                }
            }
            Self::Mmio { base } => {
                let raw = read_u32(unsafe { base.add(VIRTIO_MMIO_INTERRUPT_STATUS) }) as u8;
                if raw != 0 {
                    write_u32(unsafe { base.add(VIRTIO_MMIO_INTERRUPT_ACK) }, raw as u32);
                }
                raw
            }
        };

        VirtioInterruptStatus::from_raw(raw)
    }

    fn enable(self) {
        match self {
            Self::ModernPci { location, .. } => {
                enable_pci_memory_and_bus_master(location.bus, location.slot, location.func)
            }
            Self::Mmio { .. } => {}
        }
    }

    fn fail(self) {
        self.write_status(self.status() | VIRTIO_CONFIG_STATUS_FAILED);
    }

    fn driver_ok(self) -> bool {
        self.status() & VIRTIO_CONFIG_STATUS_DRIVER_OK != 0
    }

    fn reset(self) {
        self.write_status(0);
    }

    fn negotiate_features(self, supported_features: u64) -> Option<NegotiatedFeatures> {
        self.enable();
        self.write_status(0);
        self.write_status(VIRTIO_CONFIG_STATUS_ACKNOWLEDGE);
        self.write_status(VIRTIO_CONFIG_STATUS_ACKNOWLEDGE | VIRTIO_CONFIG_STATUS_DRIVER);

        let host = self.read_device_features();
        let driver = host & supported_features;
        if driver & VIRTIO_F_VERSION_1 == 0 {
            self.fail();
            return None;
        }

        self.write_driver_features(driver);
        self.write_status(self.status() | VIRTIO_CONFIG_STATUS_FEATURES_OK);
        if self.status() & VIRTIO_CONFIG_STATUS_FEATURES_OK == 0 {
            self.fail();
            return None;
        }

        Some(NegotiatedFeatures { host, driver })
    }

    fn configure_split_queue(
        self,
        queue: u16,
        max_queue_size: u16,
        min_queue_size: u16,
        desc: u64,
        driver: u64,
        device: u64,
    ) -> Option<u16> {
        self.select_queue(queue);
        let host_queue_size = self.read_queue_size();
        let queue_size = core::cmp::min(host_queue_size, max_queue_size);
        if queue_size < min_queue_size {
            return None;
        }

        match self {
            Self::ModernPci { common_cfg, .. } => {
                write_u16(unsafe { common_cfg.add(24) }, queue_size);
                write_u64(unsafe { common_cfg.add(32) }, desc);
                write_u64(unsafe { common_cfg.add(40) }, driver);
                write_u64(unsafe { common_cfg.add(48) }, device);
            }
            Self::Mmio { base } => {
                write_u32(
                    unsafe { base.add(VIRTIO_MMIO_QUEUE_NUM) },
                    queue_size as u32,
                );
                write_u32(unsafe { base.add(VIRTIO_MMIO_QUEUE_DESC_LOW) }, desc as u32);
                write_u32(
                    unsafe { base.add(VIRTIO_MMIO_QUEUE_DESC_HIGH) },
                    (desc >> 32) as u32,
                );
                write_u32(
                    unsafe { base.add(VIRTIO_MMIO_QUEUE_DRIVER_LOW) },
                    driver as u32,
                );
                write_u32(
                    unsafe { base.add(VIRTIO_MMIO_QUEUE_DRIVER_HIGH) },
                    (driver >> 32) as u32,
                );
                write_u32(
                    unsafe { base.add(VIRTIO_MMIO_QUEUE_DEVICE_LOW) },
                    device as u32,
                );
                write_u32(
                    unsafe { base.add(VIRTIO_MMIO_QUEUE_DEVICE_HIGH) },
                    (device >> 32) as u32,
                );
            }
        }
        self.write_queue_ready(true);
        Some(queue_size)
    }
}

impl MappedModernVirtioDevice {
    fn map(device: ModernVirtioDevice) -> Option<Self> {
        let common_cfg = map_pci_cap(device.bus, device.slot, device.func, device.common)?;
        let notify_cfg = map_pci_cap(device.bus, device.slot, device.func, device.notify)?;
        let device_cfg = device
            .device
            .and_then(|cap| map_pci_cap(device.bus, device.slot, device.func, cap))
            .unwrap_or(core::ptr::null_mut());
        let isr_cfg = device
            .isr
            .and_then(|cap| map_pci_cap(device.bus, device.slot, device.func, cap))
            .unwrap_or(core::ptr::null_mut());

        Some(Self {
            bus: device.bus,
            slot: device.slot,
            func: device.func,
            common_cfg,
            notify_cfg,
            device_cfg,
            isr_cfg,
            notify_off_multiplier: device.notify.notify_off_multiplier,
        })
    }
}

// QEMU's modern virtio-net path consumes the 12-byte header layout here; using
// the shorter 10-byte base header shifts Ethernet frames and breaks DHCP.
#[repr(C)]
#[derive(Clone, Copy)]
struct VirtioNetHdr {
    flags: u8,
    gso_type: u8,
    hdr_len: u16,
    gso_size: u16,
    csum_start: u16,
    csum_offset: u16,
    num_buffers: u16,
}

const _: [(); 12] = [(); core::mem::size_of::<VirtioNetHdr>()];

const EMPTY_NET_HDR: VirtioNetHdr = VirtioNetHdr {
    flags: 0,
    gso_type: 0,
    hdr_len: 0,
    gso_size: 0,
    csum_start: 0,
    csum_offset: 0,
    num_buffers: 0,
};

#[repr(C)]
#[derive(Clone, Copy)]
struct VirtqDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

#[repr(C, align(2))]
struct VirtqAvail {
    flags: u16,
    idx: u16,
    ring: [u16; QUEUE_SIZE],
    used_event: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VirtqUsedElem {
    id: u32,
    len: u32,
}

#[repr(C, align(4))]
struct VirtqUsed {
    flags: u16,
    idx: u16,
    ring: [VirtqUsedElem; QUEUE_SIZE],
    avail_event: u16,
}

#[repr(align(16))]
struct DescTable([VirtqDesc; QUEUE_SIZE]);

#[repr(align(16))]
struct PacketBuffers([[u8; BUFFER_SIZE]; QUEUE_SIZE]);

const EMPTY_DESC: VirtqDesc = VirtqDesc {
    addr: 0,
    len: 0,
    flags: 0,
    next: 0,
};

const EMPTY_USED_ELEM: VirtqUsedElem = VirtqUsedElem { id: 0, len: 0 };

static mut RX_DESC: DescTable = DescTable([EMPTY_DESC; QUEUE_SIZE]);
static mut RX_AVAIL: VirtqAvail = VirtqAvail {
    flags: 0,
    idx: 0,
    ring: [0; QUEUE_SIZE],
    used_event: 0,
};
static mut RX_USED: VirtqUsed = VirtqUsed {
    flags: 0,
    idx: 0,
    ring: [EMPTY_USED_ELEM; QUEUE_SIZE],
    avail_event: 0,
};
static mut RX_BUFFERS: PacketBuffers = PacketBuffers([[0; BUFFER_SIZE]; QUEUE_SIZE]);

static mut TX_DESC: DescTable = DescTable([EMPTY_DESC; QUEUE_SIZE]);
static mut TX_AVAIL: VirtqAvail = VirtqAvail {
    flags: 0,
    idx: 0,
    ring: [0; QUEUE_SIZE],
    used_event: 0,
};
static mut TX_USED: VirtqUsed = VirtqUsed {
    flags: 0,
    idx: 0,
    ring: [EMPTY_USED_ELEM; QUEUE_SIZE],
    avail_event: 0,
};
static mut TX_BUFFERS: PacketBuffers = PacketBuffers([[0; BUFFER_SIZE]; QUEUE_SIZE]);

#[repr(C)]
#[derive(Clone, Copy)]
struct VirtioBlkReqHeader {
    request_type: u32,
    reserved: u32,
    sector: u64,
}

#[repr(align(16))]
struct BlkDescTable([VirtqDesc; QUEUE_SIZE]);

#[repr(align(512))]
struct BlkData([u8; SECTOR_SIZE]);

static mut BLK_DESC: BlkDescTable = BlkDescTable([EMPTY_DESC; QUEUE_SIZE]);
static mut BLK_AVAIL: VirtqAvail = VirtqAvail {
    flags: 0,
    idx: 0,
    ring: [0; QUEUE_SIZE],
    used_event: 0,
};
static mut BLK_USED: VirtqUsed = VirtqUsed {
    flags: 0,
    idx: 0,
    ring: [EMPTY_USED_ELEM; QUEUE_SIZE],
    avail_event: 0,
};
static mut BLK_HEADER: VirtioBlkReqHeader = VirtioBlkReqHeader {
    request_type: 0,
    reserved: 0,
    sector: 0,
};
static mut BLK_DATA: BlkData = BlkData([0; SECTOR_SIZE]);
static mut BLK_STATUS: u8 = 0xff;

#[repr(align(16))]
struct RngDescTable([VirtqDesc; QUEUE_SIZE]);

#[repr(align(16))]
struct RngData([u8; 256]);

static mut RNG_DESC: RngDescTable = RngDescTable([EMPTY_DESC; QUEUE_SIZE]);
static mut RNG_AVAIL: VirtqAvail = VirtqAvail {
    flags: 0,
    idx: 0,
    ring: [0; QUEUE_SIZE],
    used_event: 0,
};
static mut RNG_USED: VirtqUsed = VirtqUsed {
    flags: 0,
    idx: 0,
    ring: [EMPTY_USED_ELEM; QUEUE_SIZE],
    avail_event: 0,
};
static mut RNG_DATA: RngData = RngData([0; 256]);

#[repr(align(16))]
struct ConsoleDescTable([VirtqDesc; QUEUE_SIZE]);

#[repr(align(16))]
#[derive(Clone, Copy)]
struct ConsoleData([u8; CONSOLE_BUFFER_SIZE]);

static mut CONSOLE_RX_DESC: ConsoleDescTable = ConsoleDescTable([EMPTY_DESC; QUEUE_SIZE]);
static mut CONSOLE_RX_AVAIL: VirtqAvail = VirtqAvail {
    flags: 0,
    idx: 0,
    ring: [0; QUEUE_SIZE],
    used_event: 0,
};
static mut CONSOLE_RX_USED: VirtqUsed = VirtqUsed {
    flags: 0,
    idx: 0,
    ring: [EMPTY_USED_ELEM; QUEUE_SIZE],
    avail_event: 0,
};
static mut CONSOLE_RX_DATA: [ConsoleData; QUEUE_SIZE] =
    [ConsoleData([0; CONSOLE_BUFFER_SIZE]); QUEUE_SIZE];

static mut CONSOLE_DESC: ConsoleDescTable = ConsoleDescTable([EMPTY_DESC; QUEUE_SIZE]);
static mut CONSOLE_AVAIL: VirtqAvail = VirtqAvail {
    flags: 0,
    idx: 0,
    ring: [0; QUEUE_SIZE],
    used_event: 0,
};
static mut CONSOLE_USED: VirtqUsed = VirtqUsed {
    flags: 0,
    idx: 0,
    ring: [EMPTY_USED_ELEM; QUEUE_SIZE],
    avail_event: 0,
};
static mut CONSOLE_DATA: ConsoleData = ConsoleData([0; CONSOLE_BUFFER_SIZE]);

static NET_CLAIMED: AtomicBool = AtomicBool::new(false);
static BLK_CLAIMED: AtomicBool = AtomicBool::new(false);
static RNG_CLAIMED: AtomicBool = AtomicBool::new(false);
static CONSOLE_CLAIMED: AtomicBool = AtomicBool::new(false);

fn claim_driver(claimed: &AtomicBool) -> bool {
    claimed
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
}

fn release_driver(claimed: &AtomicBool) {
    claimed.store(false, Ordering::Release);
}

fn mmio_transport_for_device(base: *mut u8, device_type: u32) -> Option<VirtioTransport> {
    let transport = VirtioTransport::mmio(base)?;
    if read_u32(unsafe { base.add(VIRTIO_MMIO_DEVICE_ID) }) == device_type {
        Some(transport)
    } else {
        None
    }
}

macro_rules! init_fail {
    ($self:expr) => {{
        $self.release_claim();
        return false;
    }};
    ($self:expr, $transport:expr) => {{
        $transport.fail();
        $self.release_claim();
        return false;
    }};
}

macro_rules! impl_driver_common {
    ($claim:ident) => {
        pub fn is_initialized(&self) -> bool {
            self.transport().is_some_and(VirtioTransport::driver_ok)
        }

        pub fn take_interrupt_status(&self) -> VirtioInterruptStatus {
            self.transport()
                .map(VirtioTransport::take_interrupt_status)
                .unwrap_or_default()
        }

        fn transport(&self) -> Option<VirtioTransport> {
            self.transport.transport()
        }

        fn release_claim(&mut self) {
            if self.claimed {
                if let Some(transport) = self.transport() {
                    transport.reset();
                }
                release_driver(&$claim);
                self.claimed = false;
            }
        }
    };
}

pub struct VirtNet {
    mac: [u8; 6],
    mtu: u16,
    status: u16,
    features: u64,
    host_features: u64,
    transport: DriverTransport,
    queue_size: u16,
    rx_notify_off: u16,
    tx_notify_off: u16,
    rx_last_used_idx: u16,
    tx_last_used_idx: u16,
    tx_free_mask: u16,
    tx_submitted: u32,
    tx_completed: u32,
    rx_received: u32,
    rx_invalid: u32,
    rx_empty: u32,
    link_up: bool,
    claimed: bool,
}

#[derive(Clone, Copy, Default)]
pub struct VirtNetStats {
    pub tx_submitted: u32,
    pub tx_completed: u32,
    pub rx_received: u32,
    pub rx_invalid: u32,
    pub rx_empty: u32,
}

impl VirtNet {
    impl_driver_common!(NET_CLAIMED);

    pub const fn new() -> Self {
        Self {
            mac: [0; 6],
            mtu: 1500,
            status: 0,
            features: 0,
            host_features: 0,
            transport: DriverTransport::empty(),
            queue_size: 0,
            rx_notify_off: 0,
            tx_notify_off: 0,
            rx_last_used_idx: 0,
            tx_last_used_idx: 0,
            tx_free_mask: TX_FREE_ALL_MASK,
            tx_submitted: 0,
            tx_completed: 0,
            rx_received: 0,
            rx_invalid: 0,
            rx_empty: 0,
            link_up: false,
            claimed: false,
        }
    }

    pub fn init(&mut self) -> bool {
        let Some(transport) = self.transport() else {
            init_fail!(self);
        };
        if transport.device_cfg().is_null() {
            init_fail!(self);
        }

        let Some(features) = transport
            .negotiate_features(VIRTIO_F_VERSION_1 | VIRTIO_NET_F_MAC | VIRTIO_NET_F_STATUS)
        else {
            init_fail!(self);
        };
        self.host_features = features.host;
        self.features = features.driver;

        if self.features & VIRTIO_NET_F_MAC != 0 {
            for i in 0..6 {
                self.mac[i] = read_u8(unsafe { self.transport.device_cfg.add(i) });
            }
        }

        if self.features & VIRTIO_NET_F_STATUS != 0 {
            self.status = read_u16(unsafe { self.transport.device_cfg.add(6) });
            self.link_up = (self.status & VIRTIO_NET_S_LINK_UP) != 0;
        } else {
            self.status = VIRTIO_NET_S_LINK_UP;
            self.link_up = true;
        }

        transport.select_queue(RX_QUEUE);
        self.queue_size = transport.read_queue_size();
        self.rx_notify_off = transport.read_queue_notify_off();

        if self.queue_size < QUEUE_SIZE as u16 {
            init_fail!(self, transport);
        }

        transport.select_queue(TX_QUEUE);
        if transport.read_queue_size() < QUEUE_SIZE as u16 {
            init_fail!(self, transport);
        }
        self.tx_notify_off = transport.read_queue_notify_off();

        let (rx_desc, rx_avail, rx_used) = unsafe {
            (
                core::ptr::addr_of_mut!(RX_DESC.0) as u64,
                core::ptr::addr_of_mut!(RX_AVAIL) as u64,
                core::ptr::addr_of_mut!(RX_USED) as u64,
            )
        };
        let Some(rx_queue_size) = transport.configure_split_queue(
            RX_QUEUE,
            QUEUE_SIZE as u16,
            QUEUE_SIZE as u16,
            rx_desc,
            rx_avail,
            rx_used,
        ) else {
            init_fail!(self, transport);
        };
        self.queue_size = rx_queue_size;

        let (tx_desc, tx_avail, tx_used) = unsafe {
            (
                core::ptr::addr_of_mut!(TX_DESC.0) as u64,
                core::ptr::addr_of_mut!(TX_AVAIL) as u64,
                core::ptr::addr_of_mut!(TX_USED) as u64,
            )
        };
        if transport
            .configure_split_queue(
                TX_QUEUE,
                QUEUE_SIZE as u16,
                QUEUE_SIZE as u16,
                tx_desc,
                tx_avail,
                tx_used,
            )
            .is_none()
        {
            init_fail!(self, transport);
        }

        unsafe {
            self.init_rx_queue();
            self.init_tx_queue();
        }

        transport.write_status(transport.status() | VIRTIO_CONFIG_STATUS_DRIVER_OK);
        self.notify_queue(RX_QUEUE);

        true
    }

    pub fn get_mac(&self) -> [u8; 6] {
        self.mac
    }

    pub fn is_link_up(&self) -> bool {
        self.link_up
    }

    pub fn refresh_status(&mut self) -> bool {
        if !self.is_initialized() || self.transport.device_cfg.is_null() {
            return false;
        }

        if self.features & VIRTIO_NET_F_STATUS != 0 {
            self.status = read_u16(unsafe { self.transport.device_cfg.add(6) });
            self.link_up = (self.status & VIRTIO_NET_S_LINK_UP) != 0;
        } else {
            self.status = VIRTIO_NET_S_LINK_UP;
            self.link_up = true;
        }

        true
    }

    pub fn mtu(&self) -> u16 {
        self.mtu
    }

    pub fn stats(&mut self) -> VirtNetStats {
        if self.is_initialized() {
            unsafe {
                self.reap_tx_used();
            }
        }

        VirtNetStats {
            tx_submitted: self.tx_submitted,
            tx_completed: self.tx_completed,
            rx_received: self.rx_received,
            rx_invalid: self.rx_invalid,
            rx_empty: self.rx_empty,
        }
    }

    pub fn send(&mut self, data: &[u8]) -> bool {
        if !self.is_initialized() {
            return false;
        }

        let Some(frame_len) = net_tx_frame_len(data.len()) else {
            return false;
        };

        unsafe {
            self.reap_tx_used();
            let desc_id = match self.take_tx_descriptor() {
                Some(desc_id) => desc_id,
                None => return false,
            };

            let buffer = (core::ptr::addr_of_mut!(TX_BUFFERS.0) as *mut [u8; BUFFER_SIZE])
                .add(desc_id as usize) as *mut u8;
            core::ptr::write(buffer as *mut VirtioNetHdr, EMPTY_NET_HDR);
            core::ptr::copy_nonoverlapping(data.as_ptr(), buffer.add(NET_HDR_LEN), data.len());

            let desc = core::ptr::addr_of_mut!(TX_DESC.0) as *mut VirtqDesc;
            core::ptr::write(
                desc.add(desc_id as usize),
                VirtqDesc {
                    addr: buffer as u64,
                    len: frame_len,
                    flags: 0,
                    next: 0,
                },
            );

            post_split_queue_descriptor(
                core::ptr::addr_of_mut!(TX_AVAIL),
                QUEUE_SIZE as u16,
                desc_id,
            );
            self.tx_submitted = self.tx_submitted.wrapping_add(1);
        }

        self.notify_queue(TX_QUEUE);
        true
    }

    pub fn recv(&mut self, buf: &mut [u8]) -> Option<usize> {
        if !self.is_initialized() {
            return None;
        }

        unsafe {
            let used = core::ptr::addr_of_mut!(RX_USED);
            let used_idx = split_queue_used_idx(used);
            if used_idx == self.rx_last_used_idx {
                return None;
            }

            let elem = split_queue_used_elem(used, QUEUE_SIZE as u16, self.rx_last_used_idx);
            self.rx_last_used_idx = self.rx_last_used_idx.wrapping_add(1);

            let desc_id = elem.id as usize;
            if desc_id >= self.queue_size as usize {
                self.rx_invalid = self.rx_invalid.wrapping_add(1);
                return None;
            }

            let Some(payload_len) = net_rx_payload_len(elem.len) else {
                self.rx_invalid = self.rx_invalid.wrapping_add(1);
                self.post_rx_descriptor(desc_id as u16);
                self.notify_queue(RX_QUEUE);
                return None;
            };
            let len = core::cmp::min(payload_len, buf.len());
            if len == 0 {
                self.rx_empty = self.rx_empty.wrapping_add(1);
            } else {
                self.rx_received = self.rx_received.wrapping_add(1);
            }
            if len != 0 {
                let src = (core::ptr::addr_of_mut!(RX_BUFFERS.0) as *mut [u8; BUFFER_SIZE])
                    .add(desc_id) as *const u8;
                core::ptr::copy_nonoverlapping(src.add(NET_HDR_LEN), buf.as_mut_ptr(), len);
            }

            self.post_rx_descriptor(desc_id as u16);
            self.notify_queue(RX_QUEUE);
            Some(len)
        }
    }

    fn from_modern_device(device: ModernVirtioDevice) -> Option<Self> {
        let mapped = MappedModernVirtioDevice::map(device)?;
        if !claim_driver(&NET_CLAIMED) {
            return None;
        }
        let mut net = Self::new();
        net.claimed = true;
        net.transport = DriverTransport::from_modern(mapped, mapped.device_cfg);
        Some(net)
    }

    pub fn from_mmio_base(base: usize) -> Option<Self> {
        let base = base as *mut u8;
        let transport = mmio_transport_for_device(base, VIRTIO_DEVICE_TYPE_NET)?;
        if !claim_driver(&NET_CLAIMED) {
            return None;
        }

        let mut net = Self::new();
        net.claimed = true;
        net.transport = DriverTransport::from_mmio(base, transport.device_cfg());
        Some(net)
    }

    unsafe fn init_rx_queue(&mut self) {
        self.rx_last_used_idx = 0;
        self.rx_received = 0;
        self.rx_invalid = 0;
        self.rx_empty = 0;
        write_volatile_u16(core::ptr::addr_of_mut!(RX_AVAIL.idx), 0);
        write_volatile_u16(core::ptr::addr_of_mut!(RX_USED.idx), 0);

        for i in 0..QUEUE_SIZE {
            let buffer = (core::ptr::addr_of_mut!(RX_BUFFERS.0) as *mut [u8; BUFFER_SIZE]).add(i);
            core::ptr::write(
                (core::ptr::addr_of_mut!(RX_DESC.0) as *mut VirtqDesc).add(i),
                VirtqDesc {
                    addr: buffer as u64,
                    len: BUFFER_SIZE as u32,
                    flags: VIRTQ_DESC_F_WRITE,
                    next: 0,
                },
            );
            self.post_rx_descriptor(i as u16);
        }
    }

    unsafe fn init_tx_queue(&mut self) {
        self.tx_last_used_idx = 0;
        self.tx_free_mask = TX_FREE_ALL_MASK;
        self.tx_submitted = 0;
        self.tx_completed = 0;
        write_volatile_u16(core::ptr::addr_of_mut!(TX_AVAIL.idx), 0);
        write_volatile_u16(core::ptr::addr_of_mut!(TX_USED.idx), 0);
        core::ptr::write_bytes(
            core::ptr::addr_of_mut!(TX_BUFFERS.0) as *mut u8,
            0,
            QUEUE_SIZE * BUFFER_SIZE,
        );
    }

    fn take_tx_descriptor(&mut self) -> Option<u16> {
        if self.tx_free_mask == 0 {
            return None;
        }

        let desc_id = self.tx_free_mask.trailing_zeros() as u16;
        self.tx_free_mask &= !(1u16 << desc_id);
        Some(desc_id)
    }

    unsafe fn post_rx_descriptor(&self, desc_id: u16) {
        post_split_queue_descriptor(core::ptr::addr_of_mut!(RX_AVAIL), self.queue_size, desc_id);
    }

    unsafe fn reap_tx_used(&mut self) {
        let used = core::ptr::addr_of!(TX_USED);
        let used_idx = split_queue_used_idx(used);
        while self.tx_last_used_idx != used_idx {
            let elem = split_queue_used_elem(used, QUEUE_SIZE as u16, self.tx_last_used_idx);
            if elem.id < QUEUE_SIZE as u32 {
                self.tx_free_mask |= 1u16 << elem.id;
            }
            self.tx_last_used_idx = self.tx_last_used_idx.wrapping_add(1);
            self.tx_completed = self.tx_completed.wrapping_add(1);
        }
    }

    fn notify_queue(&self, queue: u16) {
        let notify_off = match queue {
            RX_QUEUE => self.rx_notify_off,
            TX_QUEUE => self.tx_notify_off,
            _ => return,
        };
        if let Some(transport) = self.transport() {
            transport.notify_split_queue(notify_off, queue);
        }
    }
}

impl Drop for VirtNet {
    fn drop(&mut self) {
        self.release_claim();
    }
}

impl Default for VirtNet {
    fn default() -> Self {
        Self::new()
    }
}

fn net_tx_frame_len(payload_len: usize) -> Option<u32> {
    if payload_len == 0 {
        return None;
    }

    let frame_len = payload_len.checked_add(NET_HDR_LEN)?;
    if frame_len > BUFFER_SIZE {
        return None;
    }

    Some(frame_len as u32)
}

fn net_rx_payload_len(frame_len: u32) -> Option<usize> {
    let frame_len = frame_len as usize;
    if !(NET_HDR_LEN..=BUFFER_SIZE).contains(&frame_len) {
        return None;
    }

    Some(frame_len - NET_HDR_LEN)
}

struct NegotiatedFeatures {
    host: u64,
    driver: u64,
}

fn enable_pci_memory_and_bus_master(bus: u8, slot: u8, func: u8) {
    let command = pci_read_u16(bus, slot, func, PCI_COMMAND);
    pci_write_u16(
        bus,
        slot,
        func,
        PCI_COMMAND,
        command | PCI_COMMAND_MEMORY | PCI_COMMAND_BUS_MASTER,
    );
}

fn read_device_features(common_cfg: *mut u8) -> u64 {
    write_u32(common_cfg, 0);
    let low = read_u32(unsafe { common_cfg.add(4) }) as u64;
    write_u32(common_cfg, 1);
    let high = read_u32(unsafe { common_cfg.add(4) }) as u64;
    low | (high << 32)
}

fn write_driver_features(common_cfg: *mut u8, features: u64) {
    write_u32(unsafe { common_cfg.add(8) }, 0);
    write_u32(unsafe { common_cfg.add(12) }, features as u32);
    write_u32(unsafe { common_cfg.add(8) }, 1);
    write_u32(unsafe { common_cfg.add(12) }, (features >> 32) as u32);
}

fn notify_split_queue(notify_cfg: *mut u8, notify_off_multiplier: u32, queue_off: u16, queue: u16) {
    let offset = (queue_off as u32).saturating_mul(notify_off_multiplier) as usize;
    write_u16(unsafe { notify_cfg.add(offset) }, queue);
}

unsafe fn post_split_queue_descriptor(avail: *mut VirtqAvail, queue_size: u16, desc_id: u16) {
    if queue_size == 0 {
        return;
    }

    let idx = read_volatile_u16(core::ptr::addr_of!((*avail).idx));
    core::ptr::write_volatile(
        (*avail)
            .ring
            .as_mut_ptr()
            .add((idx as usize) % queue_size as usize),
        desc_id,
    );
    fence(Ordering::SeqCst);
    write_volatile_u16(core::ptr::addr_of_mut!((*avail).idx), idx.wrapping_add(1));
}

unsafe fn split_queue_used_idx(used: *const VirtqUsed) -> u16 {
    read_volatile_u16(core::ptr::addr_of!((*used).idx))
}

unsafe fn split_queue_used_elem(
    used: *const VirtqUsed,
    queue_size: u16,
    used_idx: u16,
) -> VirtqUsedElem {
    let ring_idx = (used_idx as usize) % queue_size as usize;
    read_volatile_used_elem((*used).ring.as_ptr().add(ring_idx))
}

unsafe fn take_single_used_completion(
    used: *const VirtqUsed,
    queue_size: u16,
    last_used_idx: &mut u16,
) -> Option<VirtqUsedElem> {
    let used_idx = split_queue_used_idx(used);
    if used_idx == *last_used_idx {
        return None;
    }

    let elem = split_queue_used_elem(used, queue_size, *last_used_idx);
    let next_used_idx = last_used_idx.wrapping_add(1);
    if used_idx != next_used_idx {
        *last_used_idx = used_idx;
        return None;
    }

    *last_used_idx = next_used_idx;
    Some(elem)
}

unsafe fn wait_for_used_completion(used: *const VirtqUsed, last_used_idx: u16) -> bool {
    let mut spins = 0;
    while split_queue_used_idx(used) == last_used_idx {
        spins += 1;
        if spins > VIRTIO_POLL_SPINS {
            return false;
        }
        core::hint::spin_loop();
    }
    true
}

pub struct VirtBlk {
    features: u64,
    host_features: u64,
    transport: DriverTransport,
    queue_notify_off: u16,
    queue_size: u16,
    last_used_idx: u16,
    sectors: u64,
    read_only: bool,
    block_size: u32,
    claimed: bool,
}

unsafe impl Send for VirtBlk {}

impl VirtBlk {
    impl_driver_common!(BLK_CLAIMED);

    pub const fn new() -> Self {
        Self {
            features: 0,
            host_features: 0,
            transport: DriverTransport::empty(),
            queue_notify_off: 0,
            queue_size: 0,
            last_used_idx: 0,
            sectors: 0,
            read_only: false,
            block_size: SECTOR_SIZE as u32,
            claimed: false,
        }
    }

    pub fn init(&mut self) -> bool {
        let Some(transport) = self.transport() else {
            init_fail!(self);
        };
        if transport.device_cfg().is_null() {
            init_fail!(self);
        }

        let Some(features) = transport.negotiate_features(
            VIRTIO_F_VERSION_1 | VIRTIO_BLK_F_RO | VIRTIO_BLK_F_BLK_SIZE | VIRTIO_BLK_F_FLUSH,
        ) else {
            init_fail!(self);
        };
        self.host_features = features.host;
        self.features = features.driver;

        self.sectors = read_u64(self.transport.device_cfg);
        self.read_only = self.features & VIRTIO_BLK_F_RO != 0;
        if self.features & VIRTIO_BLK_F_BLK_SIZE != 0 {
            self.block_size = read_u32(unsafe { self.transport.device_cfg.add(20) });
        }
        if self.sectors == 0 || self.block_size != SECTOR_SIZE as u32 {
            init_fail!(self, transport);
        }

        transport.select_queue(0);
        self.queue_notify_off = transport.read_queue_notify_off();

        let (desc, avail, used) = unsafe {
            (
                core::ptr::addr_of_mut!(BLK_DESC.0) as u64,
                core::ptr::addr_of_mut!(BLK_AVAIL) as u64,
                core::ptr::addr_of_mut!(BLK_USED) as u64,
            )
        };
        let Some(queue_size) =
            transport.configure_split_queue(0, QUEUE_SIZE as u16, 3, desc, avail, used)
        else {
            init_fail!(self, transport);
        };
        self.queue_size = queue_size;

        unsafe {
            self.init_queue();
        }

        transport.write_status(transport.status() | VIRTIO_CONFIG_STATUS_DRIVER_OK);
        true
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    pub fn block_size(&self) -> u32 {
        self.block_size
    }

    pub fn sectors(&self) -> u64 {
        self.sectors
    }

    pub fn read_sector(&mut self, sector: u64, out: &mut [u8]) -> bool {
        if !self.is_initialized() || out.len() != SECTOR_SIZE || sector >= self.sectors {
            return false;
        }

        unsafe {
            if !self.submit_request(VIRTIO_BLK_T_IN, sector, SECTOR_SIZE, true) {
                return false;
            }
            core::ptr::copy_nonoverlapping(
                core::ptr::addr_of!(BLK_DATA.0) as *const u8,
                out.as_mut_ptr(),
                SECTOR_SIZE,
            );
        }
        true
    }

    pub fn read_sectors(&mut self, start_sector: u64, out: &mut [u8]) -> bool {
        if !self.is_initialized() || out.len() % SECTOR_SIZE != 0 {
            return false;
        }

        let sector_count = (out.len() / SECTOR_SIZE) as u64;
        if !self.sector_range_in_bounds(start_sector, sector_count) {
            return false;
        }

        let mut offset = 0;
        let mut sector = start_sector;
        while offset < out.len() {
            let chunk_len = core::cmp::min(SECTOR_SIZE, out.len() - offset);
            if !self.read_sectors_chunk(sector, &mut out[offset..offset + chunk_len]) {
                return false;
            }
            offset += chunk_len;
            sector += (chunk_len / SECTOR_SIZE) as u64;
        }
        true
    }

    pub fn write_sector(&mut self, sector: u64, data: &[u8]) -> bool {
        if !self.is_initialized()
            || self.read_only
            || data.len() != SECTOR_SIZE
            || sector >= self.sectors
        {
            return false;
        }

        unsafe {
            core::ptr::copy_nonoverlapping(
                data.as_ptr(),
                core::ptr::addr_of_mut!(BLK_DATA.0) as *mut u8,
                SECTOR_SIZE,
            );
            self.submit_request(VIRTIO_BLK_T_OUT, sector, SECTOR_SIZE, false)
        }
    }

    pub fn write_sectors(&mut self, start_sector: u64, data: &[u8]) -> bool {
        if !self.is_initialized() || data.len() % SECTOR_SIZE != 0 {
            return false;
        }

        let sector_count = (data.len() / SECTOR_SIZE) as u64;
        if !self.sector_range_in_bounds(start_sector, sector_count) {
            return false;
        }
        if sector_count == 0 {
            return true;
        }
        if self.read_only {
            return false;
        }

        let mut offset = 0;
        let mut sector = start_sector;
        while offset < data.len() {
            let chunk_len = core::cmp::min(SECTOR_SIZE, data.len() - offset);
            if !self.write_sectors_chunk(sector, &data[offset..offset + chunk_len]) {
                return false;
            }
            offset += chunk_len;
            sector += (chunk_len / SECTOR_SIZE) as u64;
        }
        true
    }

    pub fn flush(&mut self) -> bool {
        if !self.is_initialized() {
            return false;
        }
        if self.features & VIRTIO_BLK_F_FLUSH == 0 {
            return true;
        }
        unsafe { self.submit_request(VIRTIO_BLK_T_FLUSH, 0, 0, false) }
    }

    fn from_modern_device(device: ModernVirtioDevice) -> Option<Self> {
        let mapped = MappedModernVirtioDevice::map(device)?;
        if !claim_driver(&BLK_CLAIMED) {
            return None;
        }
        let mut blk = Self::new();
        blk.claimed = true;
        blk.transport = DriverTransport::from_modern(mapped, mapped.device_cfg);
        Some(blk)
    }

    pub fn from_mmio_base(base: usize) -> Option<Self> {
        let base = base as *mut u8;
        let transport = mmio_transport_for_device(base, VIRTIO_DEVICE_TYPE_BLK)?;
        if !claim_driver(&BLK_CLAIMED) {
            return None;
        }

        let mut blk = Self::new();
        blk.claimed = true;
        blk.transport = DriverTransport::from_mmio(base, transport.device_cfg());
        Some(blk)
    }

    fn sector_range_in_bounds(&self, start_sector: u64, sector_count: u64) -> bool {
        start_sector
            .checked_add(sector_count)
            .is_some_and(|end_sector| end_sector <= self.sectors)
    }

    fn chunk_sector_count(len: usize) -> Option<u32> {
        if len == 0 || len % SECTOR_SIZE != 0 || len > SECTOR_SIZE {
            return None;
        }

        Some((len / SECTOR_SIZE) as u32)
    }

    fn read_sectors_chunk(&mut self, sector: u64, out: &mut [u8]) -> bool {
        let Some(sector_count) = Self::chunk_sector_count(out.len()) else {
            return false;
        };
        if !self.sector_range_in_bounds(sector, sector_count as u64) {
            return false;
        }

        unsafe {
            if !self.submit_request(VIRTIO_BLK_T_IN, sector, out.len(), true) {
                return false;
            }
            core::ptr::copy_nonoverlapping(
                core::ptr::addr_of!(BLK_DATA.0) as *const u8,
                out.as_mut_ptr(),
                out.len(),
            );
        }
        true
    }

    fn write_sectors_chunk(&mut self, sector: u64, data: &[u8]) -> bool {
        let Some(sector_count) = Self::chunk_sector_count(data.len()) else {
            return false;
        };
        if self.read_only || !self.sector_range_in_bounds(sector, sector_count as u64) {
            return false;
        }

        unsafe {
            core::ptr::copy_nonoverlapping(
                data.as_ptr(),
                core::ptr::addr_of_mut!(BLK_DATA.0) as *mut u8,
                data.len(),
            );
            self.submit_request(VIRTIO_BLK_T_OUT, sector, data.len(), false)
        }
    }

    unsafe fn init_queue(&mut self) {
        self.last_used_idx = 0;
        write_volatile_u16(core::ptr::addr_of_mut!(BLK_AVAIL.idx), 0);
        write_volatile_u16(core::ptr::addr_of_mut!(BLK_USED.idx), 0);
        core::ptr::write_bytes(
            core::ptr::addr_of_mut!(BLK_DATA.0) as *mut u8,
            0,
            SECTOR_SIZE,
        );
        BLK_STATUS = 0xff;
    }

    unsafe fn submit_request(
        &mut self,
        request_type: u32,
        sector: u64,
        data_len: usize,
        read: bool,
    ) -> bool {
        if !self.prepare_request_descriptors(request_type, sector, data_len, read) {
            return false;
        }

        post_split_queue_descriptor(core::ptr::addr_of_mut!(BLK_AVAIL), self.queue_size, 0);
        self.notify_queue();

        if !wait_for_used_completion(core::ptr::addr_of!(BLK_USED), self.last_used_idx) {
            return false;
        }

        let Some(elem) = take_single_used_completion(
            core::ptr::addr_of!(BLK_USED),
            self.queue_size,
            &mut self.last_used_idx,
        ) else {
            return false;
        };
        elem.id == 0 && read_u8(core::ptr::addr_of!(BLK_STATUS)) == VIRTIO_BLK_S_OK
    }

    unsafe fn prepare_request_descriptors(
        &mut self,
        request_type: u32,
        sector: u64,
        data_len: usize,
        read: bool,
    ) -> bool {
        if data_len > SECTOR_SIZE {
            return false;
        }
        BLK_HEADER = VirtioBlkReqHeader {
            request_type,
            reserved: 0,
            sector,
        };
        BLK_STATUS = 0xff;

        let desc = core::ptr::addr_of_mut!(BLK_DESC.0) as *mut VirtqDesc;
        let has_data = data_len != 0;
        core::ptr::write(
            desc,
            VirtqDesc {
                addr: core::ptr::addr_of!(BLK_HEADER) as u64,
                len: core::mem::size_of::<VirtioBlkReqHeader>() as u32,
                flags: VIRTQ_DESC_F_NEXT,
                next: if has_data { 1 } else { 2 },
            },
        );
        if has_data {
            core::ptr::write(
                desc.add(1),
                VirtqDesc {
                    addr: core::ptr::addr_of_mut!(BLK_DATA.0) as u64,
                    len: data_len as u32,
                    flags: if read {
                        VIRTQ_DESC_F_WRITE | VIRTQ_DESC_F_NEXT
                    } else {
                        VIRTQ_DESC_F_NEXT
                    },
                    next: 2,
                },
            );
        }
        core::ptr::write(
            desc.add(2),
            VirtqDesc {
                addr: core::ptr::addr_of_mut!(BLK_STATUS) as u64,
                len: 1,
                flags: VIRTQ_DESC_F_WRITE,
                next: 0,
            },
        );
        true
    }

    fn notify_queue(&self) {
        if let Some(transport) = self.transport() {
            transport.notify_split_queue(self.queue_notify_off, 0);
        }
    }
}

impl Drop for VirtBlk {
    fn drop(&mut self) {
        self.release_claim();
    }
}

impl Default for VirtBlk {
    fn default() -> Self {
        Self::new()
    }
}

pub struct VirtRng {
    features: u64,
    host_features: u64,
    transport: DriverTransport,
    queue_notify_off: u16,
    queue_size: u16,
    last_used_idx: u16,
    claimed: bool,
}

unsafe impl Send for VirtRng {}

impl VirtRng {
    impl_driver_common!(RNG_CLAIMED);

    pub const fn new() -> Self {
        Self {
            features: 0,
            host_features: 0,
            transport: DriverTransport::empty(),
            queue_notify_off: 0,
            queue_size: 0,
            last_used_idx: 0,
            claimed: false,
        }
    }

    pub fn init(&mut self) -> bool {
        let Some(transport) = self.transport() else {
            init_fail!(self);
        };

        let Some(features) = transport.negotiate_features(VIRTIO_F_VERSION_1) else {
            init_fail!(self);
        };
        self.host_features = features.host;
        self.features = features.driver;

        transport.select_queue(0);
        self.queue_notify_off = transport.read_queue_notify_off();

        let (desc, avail, used) = unsafe {
            (
                core::ptr::addr_of_mut!(RNG_DESC.0) as u64,
                core::ptr::addr_of_mut!(RNG_AVAIL) as u64,
                core::ptr::addr_of_mut!(RNG_USED) as u64,
            )
        };
        let Some(queue_size) =
            transport.configure_split_queue(0, QUEUE_SIZE as u16, 1, desc, avail, used)
        else {
            init_fail!(self, transport);
        };
        self.queue_size = queue_size;

        unsafe {
            self.init_queue();
        }

        transport.write_status(transport.status() | VIRTIO_CONFIG_STATUS_DRIVER_OK);
        true
    }

    pub fn fill_bytes(&mut self, out: &mut [u8]) -> bool {
        if !self.is_initialized() {
            return false;
        }

        let mut offset = 0;
        while offset < out.len() {
            let written = unsafe { self.request_entropy(&mut out[offset..]) };
            if written == 0 {
                return false;
            }
            offset += written;
        }
        true
    }

    fn from_modern_device(device: ModernVirtioDevice) -> Option<Self> {
        let mapped = MappedModernVirtioDevice::map(device)?;
        if !claim_driver(&RNG_CLAIMED) {
            return None;
        }
        let mut rng = Self::new();
        rng.claimed = true;
        rng.transport = DriverTransport::from_modern(mapped, core::ptr::null_mut());
        Some(rng)
    }

    pub fn from_mmio_base(base: usize) -> Option<Self> {
        let base = base as *mut u8;
        mmio_transport_for_device(base, VIRTIO_DEVICE_TYPE_RNG)?;
        if !claim_driver(&RNG_CLAIMED) {
            return None;
        }

        let mut rng = Self::new();
        rng.claimed = true;
        rng.transport = DriverTransport::from_mmio(base, core::ptr::null_mut());
        Some(rng)
    }

    unsafe fn init_queue(&mut self) {
        self.last_used_idx = 0;
        write_volatile_u16(core::ptr::addr_of_mut!(RNG_AVAIL.idx), 0);
        write_volatile_u16(core::ptr::addr_of_mut!(RNG_USED.idx), 0);
        core::ptr::write_bytes(core::ptr::addr_of_mut!(RNG_DATA.0) as *mut u8, 0, 256);
    }

    unsafe fn request_entropy(&mut self, out: &mut [u8]) -> usize {
        let request_len = core::cmp::min(out.len(), 256);
        if request_len == 0 {
            return 0;
        }

        let desc = core::ptr::addr_of_mut!(RNG_DESC.0) as *mut VirtqDesc;
        core::ptr::write(
            desc,
            VirtqDesc {
                addr: core::ptr::addr_of_mut!(RNG_DATA.0) as u64,
                len: request_len as u32,
                flags: VIRTQ_DESC_F_WRITE,
                next: 0,
            },
        );

        post_split_queue_descriptor(core::ptr::addr_of_mut!(RNG_AVAIL), self.queue_size, 0);
        self.notify_queue();

        if !wait_for_used_completion(core::ptr::addr_of!(RNG_USED), self.last_used_idx) {
            return 0;
        }

        let Some(elem) = take_single_used_completion(
            core::ptr::addr_of!(RNG_USED),
            self.queue_size,
            &mut self.last_used_idx,
        ) else {
            return 0;
        };
        if elem.id != 0 {
            return 0;
        }

        let len = core::cmp::min(elem.len as usize, request_len);
        core::ptr::copy_nonoverlapping(
            core::ptr::addr_of!(RNG_DATA.0) as *const u8,
            out.as_mut_ptr(),
            len,
        );
        len
    }

    fn notify_queue(&self) {
        if let Some(transport) = self.transport() {
            transport.notify_split_queue(self.queue_notify_off, 0);
        }
    }
}

impl Drop for VirtRng {
    fn drop(&mut self) {
        self.release_claim();
    }
}

impl Default for VirtRng {
    fn default() -> Self {
        Self::new()
    }
}

pub struct VirtConsole {
    features: u64,
    host_features: u64,
    transport: DriverTransport,
    rx_notify_off: u16,
    rx_queue_size: u16,
    rx_last_used_idx: u16,
    tx_notify_off: u16,
    tx_queue_size: u16,
    tx_last_used_idx: u16,
    claimed: bool,
}

unsafe impl Send for VirtConsole {}

impl VirtConsole {
    impl_driver_common!(CONSOLE_CLAIMED);

    pub const fn new() -> Self {
        Self {
            features: 0,
            host_features: 0,
            transport: DriverTransport::empty(),
            rx_notify_off: 0,
            rx_queue_size: 0,
            rx_last_used_idx: 0,
            tx_notify_off: 0,
            tx_queue_size: 0,
            tx_last_used_idx: 0,
            claimed: false,
        }
    }

    pub fn init(&mut self) -> bool {
        let Some(transport) = self.transport() else {
            init_fail!(self);
        };

        let Some(features) = transport.negotiate_features(VIRTIO_F_VERSION_1) else {
            init_fail!(self);
        };
        self.host_features = features.host;
        self.features = features.driver;

        transport.select_queue(CONSOLE_RX_QUEUE);
        self.rx_notify_off = transport.read_queue_notify_off();

        let (rx_desc, rx_avail, rx_used) = unsafe {
            (
                core::ptr::addr_of_mut!(CONSOLE_RX_DESC.0) as u64,
                core::ptr::addr_of_mut!(CONSOLE_RX_AVAIL) as u64,
                core::ptr::addr_of_mut!(CONSOLE_RX_USED) as u64,
            )
        };
        let Some(rx_queue_size) = transport.configure_split_queue(
            CONSOLE_RX_QUEUE,
            QUEUE_SIZE as u16,
            1,
            rx_desc,
            rx_avail,
            rx_used,
        ) else {
            init_fail!(self, transport);
        };
        self.rx_queue_size = rx_queue_size;

        transport.select_queue(CONSOLE_TX_QUEUE);
        self.tx_notify_off = transport.read_queue_notify_off();

        let (desc, avail, used) = unsafe {
            (
                core::ptr::addr_of_mut!(CONSOLE_DESC.0) as u64,
                core::ptr::addr_of_mut!(CONSOLE_AVAIL) as u64,
                core::ptr::addr_of_mut!(CONSOLE_USED) as u64,
            )
        };
        let Some(tx_queue_size) = transport.configure_split_queue(
            CONSOLE_TX_QUEUE,
            QUEUE_SIZE as u16,
            1,
            desc,
            avail,
            used,
        ) else {
            init_fail!(self, transport);
        };
        self.tx_queue_size = tx_queue_size;

        unsafe {
            self.init_rx_queue();
            self.init_tx_queue();
        }

        transport.write_status(transport.status() | VIRTIO_CONFIG_STATUS_DRIVER_OK);
        true
    }

    pub fn recv(&mut self, buf: &mut [u8]) -> Option<usize> {
        if !self.is_initialized() {
            return None;
        }

        unsafe {
            let used = core::ptr::addr_of_mut!(CONSOLE_RX_USED);
            let used_idx = split_queue_used_idx(used);
            if used_idx == self.rx_last_used_idx {
                return None;
            }

            let elem = split_queue_used_elem(used, self.rx_queue_size, self.rx_last_used_idx);
            self.rx_last_used_idx = self.rx_last_used_idx.wrapping_add(1);

            let desc_id = elem.id as usize;
            if desc_id >= self.rx_queue_size as usize {
                return None;
            }

            let Some(rx_len) = console_rx_len(elem.len) else {
                self.post_rx_descriptor(desc_id as u16);
                self.notify_rx_queue();
                return None;
            };
            let len = core::cmp::min(rx_len, buf.len());
            if len != 0 {
                let src = (core::ptr::addr_of!(CONSOLE_RX_DATA) as *const ConsoleData).add(desc_id)
                    as *const u8;
                core::ptr::copy_nonoverlapping(src, buf.as_mut_ptr(), len);
            }

            self.post_rx_descriptor(desc_id as u16);
            self.notify_rx_queue();
            Some(len)
        }
    }

    pub fn write_all(&mut self, bytes: &[u8]) -> bool {
        if !self.is_initialized() {
            return false;
        }

        let mut offset = 0;
        while offset < bytes.len() {
            let written = unsafe { self.write_chunk(&bytes[offset..]) };
            if written == 0 {
                return false;
            }
            offset += written;
        }
        true
    }

    fn from_modern_device(device: ModernVirtioDevice) -> Option<Self> {
        let mapped = MappedModernVirtioDevice::map(device)?;
        if !claim_driver(&CONSOLE_CLAIMED) {
            return None;
        }
        let mut console = Self::new();
        console.claimed = true;
        console.transport = DriverTransport::from_modern(mapped, core::ptr::null_mut());
        Some(console)
    }

    pub fn from_mmio_base(base: usize) -> Option<Self> {
        let base = base as *mut u8;
        mmio_transport_for_device(base, VIRTIO_DEVICE_TYPE_CONSOLE)?;
        if !claim_driver(&CONSOLE_CLAIMED) {
            return None;
        }

        let mut console = Self::new();
        console.claimed = true;
        console.transport = DriverTransport::from_mmio(base, core::ptr::null_mut());
        Some(console)
    }

    unsafe fn init_tx_queue(&mut self) {
        self.tx_last_used_idx = 0;
        write_volatile_u16(core::ptr::addr_of_mut!(CONSOLE_AVAIL.idx), 0);
        write_volatile_u16(core::ptr::addr_of_mut!(CONSOLE_USED.idx), 0);
        core::ptr::write_bytes(
            core::ptr::addr_of_mut!(CONSOLE_DATA.0) as *mut u8,
            0,
            CONSOLE_BUFFER_SIZE,
        );
    }

    unsafe fn init_rx_queue(&mut self) {
        self.rx_last_used_idx = 0;
        write_volatile_u16(core::ptr::addr_of_mut!(CONSOLE_RX_AVAIL.idx), 0);
        write_volatile_u16(core::ptr::addr_of_mut!(CONSOLE_RX_USED.idx), 0);
        core::ptr::write_bytes(
            core::ptr::addr_of_mut!(CONSOLE_RX_DATA) as *mut u8,
            0,
            QUEUE_SIZE * CONSOLE_BUFFER_SIZE,
        );

        for i in 0..self.rx_queue_size as usize {
            let buffer = (core::ptr::addr_of_mut!(CONSOLE_RX_DATA) as *mut ConsoleData).add(i);
            core::ptr::write(
                (core::ptr::addr_of_mut!(CONSOLE_RX_DESC.0) as *mut VirtqDesc).add(i),
                VirtqDesc {
                    addr: buffer as u64,
                    len: CONSOLE_BUFFER_SIZE as u32,
                    flags: VIRTQ_DESC_F_WRITE,
                    next: 0,
                },
            );
            self.post_rx_descriptor(i as u16);
        }
    }

    unsafe fn write_chunk(&mut self, bytes: &[u8]) -> usize {
        let len = core::cmp::min(bytes.len(), CONSOLE_BUFFER_SIZE);
        if len == 0 {
            return 0;
        }

        core::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            core::ptr::addr_of_mut!(CONSOLE_DATA.0) as *mut u8,
            len,
        );

        let desc = core::ptr::addr_of_mut!(CONSOLE_DESC.0) as *mut VirtqDesc;
        core::ptr::write(
            desc,
            VirtqDesc {
                addr: core::ptr::addr_of!(CONSOLE_DATA.0) as u64,
                len: len as u32,
                flags: 0,
                next: 0,
            },
        );

        post_split_queue_descriptor(
            core::ptr::addr_of_mut!(CONSOLE_AVAIL),
            self.tx_queue_size,
            0,
        );
        self.notify_tx_queue();

        if !wait_for_used_completion(core::ptr::addr_of!(CONSOLE_USED), self.tx_last_used_idx) {
            return 0;
        }

        let Some(elem) = take_single_used_completion(
            core::ptr::addr_of!(CONSOLE_USED),
            self.tx_queue_size,
            &mut self.tx_last_used_idx,
        ) else {
            return 0;
        };
        if elem.id == 0 {
            len
        } else {
            0
        }
    }

    fn notify_tx_queue(&self) {
        if let Some(transport) = self.transport() {
            transport.notify_split_queue(self.tx_notify_off, CONSOLE_TX_QUEUE);
        }
    }

    unsafe fn post_rx_descriptor(&self, desc_id: u16) {
        post_split_queue_descriptor(
            core::ptr::addr_of_mut!(CONSOLE_RX_AVAIL),
            self.rx_queue_size,
            desc_id,
        );
    }

    fn notify_rx_queue(&self) {
        if let Some(transport) = self.transport() {
            transport.notify_split_queue(self.rx_notify_off, CONSOLE_RX_QUEUE);
        }
    }
}

impl Drop for VirtConsole {
    fn drop(&mut self) {
        self.release_claim();
    }
}

fn console_rx_len(len: u32) -> Option<usize> {
    let len = len as usize;
    if len > CONSOLE_BUFFER_SIZE {
        return None;
    }

    Some(len)
}

impl Default for VirtConsole {
    fn default() -> Self {
        Self::new()
    }
}

fn initialized_device<T>(mut device: T, init: fn(&mut T) -> bool) -> Option<T> {
    if init(&mut device) {
        Some(device)
    } else {
        None
    }
}

pub fn find_virtio_net() -> Option<VirtNet> {
    #[cfg(target_arch = "x86_64")]
    {
        let device = find_modern_virtio_device(VIRTIO_MODERN_DEVICE_ID_NET)?;
        return VirtNet::from_modern_device(device);
    }

    #[allow(unreachable_code)]
    None
}

pub fn find_initialized_virtio_net() -> Option<VirtNet> {
    initialized_device(find_virtio_net()?, VirtNet::init)
}

pub fn find_virtio_blk() -> Option<VirtBlk> {
    #[cfg(target_arch = "x86_64")]
    {
        let device = find_modern_virtio_device(VIRTIO_MODERN_DEVICE_ID_BLK)?;
        return VirtBlk::from_modern_device(device);
    }

    #[allow(unreachable_code)]
    None
}

pub fn find_initialized_virtio_blk() -> Option<VirtBlk> {
    initialized_device(find_virtio_blk()?, VirtBlk::init)
}

pub fn find_virtio_rng() -> Option<VirtRng> {
    #[cfg(target_arch = "x86_64")]
    {
        let device = find_modern_virtio_device(VIRTIO_MODERN_DEVICE_ID_RNG)?;
        return VirtRng::from_modern_device(device);
    }

    #[allow(unreachable_code)]
    None
}

pub fn find_initialized_virtio_rng() -> Option<VirtRng> {
    initialized_device(find_virtio_rng()?, VirtRng::init)
}

pub fn find_virtio_console() -> Option<VirtConsole> {
    #[cfg(target_arch = "x86_64")]
    {
        let device = find_modern_virtio_device(VIRTIO_MODERN_DEVICE_ID_CONSOLE)?;
        return VirtConsole::from_modern_device(device);
    }

    #[allow(unreachable_code)]
    None
}

pub fn find_initialized_virtio_console() -> Option<VirtConsole> {
    initialized_device(find_virtio_console()?, VirtConsole::init)
}

pub fn find_virtio_net_mmio(base: usize) -> Option<VirtNet> {
    VirtNet::from_mmio_base(base)
}

pub fn find_initialized_virtio_net_mmio(base: usize) -> Option<VirtNet> {
    initialized_device(find_virtio_net_mmio(base)?, VirtNet::init)
}

pub fn find_virtio_blk_mmio(base: usize) -> Option<VirtBlk> {
    VirtBlk::from_mmio_base(base)
}

pub fn find_initialized_virtio_blk_mmio(base: usize) -> Option<VirtBlk> {
    initialized_device(find_virtio_blk_mmio(base)?, VirtBlk::init)
}

pub fn find_virtio_rng_mmio(base: usize) -> Option<VirtRng> {
    VirtRng::from_mmio_base(base)
}

pub fn find_initialized_virtio_rng_mmio(base: usize) -> Option<VirtRng> {
    initialized_device(find_virtio_rng_mmio(base)?, VirtRng::init)
}

pub fn find_virtio_console_mmio(base: usize) -> Option<VirtConsole> {
    VirtConsole::from_mmio_base(base)
}

pub fn find_initialized_virtio_console_mmio(base: usize) -> Option<VirtConsole> {
    initialized_device(find_virtio_console_mmio(base)?, VirtConsole::init)
}

pub fn virtio_mmio_device_info(base: usize) -> Option<VirtioDeviceInfo> {
    let base_ptr = base as *mut u8;
    VirtioTransport::mmio(base_ptr)?;
    let device_type = read_u32(unsafe { base_ptr.add(VIRTIO_MMIO_DEVICE_ID) });
    if device_type == 0 {
        return None;
    }

    Some(VirtioDeviceInfo {
        transport: VirtioTransportKind::Mmio,
        device_type,
        vendor_id: read_u32(unsafe { base_ptr.add(VIRTIO_MMIO_VENDOR) }) as u16,
        device_id: 0,
        bus: 0,
        slot: 0,
        func: 0,
        mmio_base: base,
    })
}

pub fn scan_virtio_mmio_devices(bases: &[usize], out: &mut [VirtioDeviceInfo]) -> usize {
    let mut found = 0;
    for &base in bases {
        if let Some(info) = virtio_mmio_device_info(base) {
            if found < out.len() {
                out[found] = info;
            }
            found += 1;
        }
    }
    found
}

pub fn scan_virtio_devices(out: &mut [VirtioDeviceInfo]) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        return scan_modern_virtio_pci_devices(out);
    }

    #[allow(unreachable_code)]
    {
        let _ = out;
        0
    }
}

fn modern_pci_device_type(device_id: u16) -> Option<u32> {
    match device_id {
        VIRTIO_MODERN_DEVICE_ID_NET => Some(VIRTIO_DEVICE_TYPE_NET),
        VIRTIO_MODERN_DEVICE_ID_BLK => Some(VIRTIO_DEVICE_TYPE_BLK),
        VIRTIO_MODERN_DEVICE_ID_CONSOLE => Some(VIRTIO_DEVICE_TYPE_CONSOLE),
        VIRTIO_MODERN_DEVICE_ID_RNG => Some(VIRTIO_DEVICE_TYPE_RNG),
        _ => None,
    }
}

fn scan_modern_virtio_pci_devices(out: &mut [VirtioDeviceInfo]) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        let mut found = 0;
        for bus in 0..=0 {
            for slot in 0..32 {
                for func in 0..8 {
                    let vendor = pci_read_u16(bus, slot, func, 0x00);
                    if vendor == 0xffff {
                        if func == 0 {
                            break;
                        }
                        continue;
                    }

                    let device_id = pci_read_u16(bus, slot, func, 0x02);
                    if vendor == VIRTIO_VENDOR_ID {
                        if let Some(device_type) = modern_pci_device_type(device_id) {
                            if found < out.len() {
                                out[found] = VirtioDeviceInfo {
                                    transport: VirtioTransportKind::ModernPci,
                                    device_type,
                                    vendor_id: vendor,
                                    device_id,
                                    bus,
                                    slot,
                                    func,
                                    mmio_base: 0,
                                };
                            }
                            found += 1;
                        }
                    }

                    if func == 0 && !is_multifunction(bus, slot) {
                        break;
                    }
                }
            }
        }
        return found;
    }

    #[allow(unreachable_code)]
    {
        let _ = out;
        0
    }
}

fn find_modern_virtio_device(device_id: u16) -> Option<ModernVirtioDevice> {
    #[cfg(target_arch = "x86_64")]
    {
        for bus in 0..=0 {
            for slot in 0..32 {
                for func in 0..8 {
                    let vendor = pci_read_u16(bus, slot, func, 0x00);
                    if vendor == 0xffff {
                        if func == 0 {
                            break;
                        }
                        continue;
                    }

                    let device = pci_read_u16(bus, slot, func, 0x02);
                    if vendor == VIRTIO_VENDOR_ID && device == device_id {
                        if let Some(device) = read_modern_virtio_device(bus, slot, func) {
                            return Some(device);
                        }
                    }

                    if func == 0 && !is_multifunction(bus, slot) {
                        break;
                    }
                }
            }
        }
    }

    None
}

fn read_modern_virtio_device(bus: u8, slot: u8, func: u8) -> Option<ModernVirtioDevice> {
    if pci_read_u16(bus, slot, func, PCI_STATUS) & PCI_STATUS_CAPABILITIES == 0 {
        return None;
    }

    let mut common = None;
    let mut notify = None;
    let mut device = None;
    let mut isr = None;
    let mut cap_ptr = pci_read_u8(bus, slot, func, PCI_CAPABILITY_LIST) & !0x3;
    let mut guard = 0;

    while cap_ptr >= 0x40 && guard < 48 {
        guard += 1;
        let cap_vndr = pci_read_u8(bus, slot, func, cap_ptr);
        let cap_next = pci_read_u8(bus, slot, func, cap_ptr + 1) & !0x3;
        let cap_len = pci_read_u8(bus, slot, func, cap_ptr + 2);

        if cap_vndr == VIRTIO_PCI_CAP_VENDOR && cap_len >= 16 {
            let cfg_type = pci_read_u8(bus, slot, func, cap_ptr + 3);
            let cap = VirtioPciCap {
                cfg_type,
                bar: pci_read_u8(bus, slot, func, cap_ptr + 4),
                offset: pci_read_u32(bus, slot, func, cap_ptr + 8),
                length: pci_read_u32(bus, slot, func, cap_ptr + 12),
                notify_off_multiplier: if cfg_type == VIRTIO_PCI_CAP_NOTIFY_CFG && cap_len >= 20 {
                    pci_read_u32(bus, slot, func, cap_ptr + 16)
                } else {
                    0
                },
            };

            match cfg_type {
                VIRTIO_PCI_CAP_COMMON_CFG => common = Some(cap),
                VIRTIO_PCI_CAP_NOTIFY_CFG => notify = Some(cap),
                VIRTIO_PCI_CAP_ISR_CFG => isr = Some(cap),
                VIRTIO_PCI_CAP_DEVICE_CFG => device = Some(cap),
                _ => {}
            }
        }

        if cap_next == 0 {
            break;
        }
        cap_ptr = cap_next;
    }

    Some(ModernVirtioDevice {
        bus,
        slot,
        func,
        common: common?,
        notify: notify?,
        device,
        isr,
    })
}

fn map_pci_cap(bus: u8, slot: u8, func: u8, cap: VirtioPciCap) -> Option<*mut u8> {
    if cap.bar >= 6 {
        return None;
    }

    let bar = pci_bar_addr(bus, slot, func, cap.bar)?;
    Some((bar.checked_add(cap.offset as u64)?) as usize as *mut u8)
}

fn pci_bar_addr(bus: u8, slot: u8, func: u8, bar: u8) -> Option<u64> {
    let offset = PCI_BAR0 + bar * 4;
    let raw = pci_read_u32(bus, slot, func, offset);
    if raw == 0 || raw == 0xffff_ffff || raw & 0x1 != 0 {
        return None;
    }

    let base = if raw & 0x6 == 0x4 {
        let high = pci_read_u32(bus, slot, func, offset + 4) as u64;
        (high << 32) | ((raw & !0xf) as u64)
    } else {
        (raw & !0xf) as u64
    };

    if base == 0 {
        None
    } else {
        Some(base)
    }
}

fn is_multifunction(bus: u8, slot: u8) -> bool {
    (pci_read_u8(bus, slot, 0, PCI_HEADER_TYPE) & 0x80) != 0
}

fn pci_address(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    0x8000_0000
        | ((bus as u32) << 16)
        | ((slot as u32) << 11)
        | ((func as u32) << 8)
        | ((offset as u32) & 0xfc)
}

fn pci_read_u32(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    port_outl(PCI_CONFIG_ADDRESS, pci_address(bus, slot, func, offset));
    port_inl(PCI_CONFIG_DATA)
}

fn pci_write_u32(bus: u8, slot: u8, func: u8, offset: u8, value: u32) {
    port_outl(PCI_CONFIG_ADDRESS, pci_address(bus, slot, func, offset));
    port_outl(PCI_CONFIG_DATA, value);
}

fn pci_read_u16(bus: u8, slot: u8, func: u8, offset: u8) -> u16 {
    let shift = ((offset & 0x2) * 8) as u32;
    ((pci_read_u32(bus, slot, func, offset) >> shift) & 0xffff) as u16
}

fn pci_write_u16(bus: u8, slot: u8, func: u8, offset: u8, value: u16) {
    let shift = ((offset & 0x2) * 8) as u32;
    let mask = !(0xffffu32 << shift);
    let current = pci_read_u32(bus, slot, func, offset);
    pci_write_u32(
        bus,
        slot,
        func,
        offset,
        (current & mask) | ((value as u32) << shift),
    );
}

fn pci_read_u8(bus: u8, slot: u8, func: u8, offset: u8) -> u8 {
    let shift = ((offset & 0x3) * 8) as u32;
    ((pci_read_u32(bus, slot, func, offset) >> shift) & 0xff) as u8
}

#[inline]
fn read_u8(ptr: *const u8) -> u8 {
    unsafe { core::ptr::read_volatile(ptr) }
}

#[inline]
fn read_u16(ptr: *const u8) -> u16 {
    unsafe { core::ptr::read_volatile(ptr as *const u16) }
}

#[inline]
fn read_u32(ptr: *const u8) -> u32 {
    unsafe { core::ptr::read_volatile(ptr as *const u32) }
}

#[inline]
fn read_u64(ptr: *const u8) -> u64 {
    unsafe { core::ptr::read_volatile(ptr as *const u64) }
}

#[inline]
fn write_u8(ptr: *mut u8, value: u8) {
    unsafe { core::ptr::write_volatile(ptr, value) }
}

#[inline]
fn write_u16(ptr: *mut u8, value: u16) {
    unsafe { core::ptr::write_volatile(ptr as *mut u16, value) }
}

#[inline]
fn write_u32(ptr: *mut u8, value: u32) {
    unsafe { core::ptr::write_volatile(ptr as *mut u32, value) }
}

#[inline]
fn write_u64(ptr: *mut u8, value: u64) {
    unsafe { core::ptr::write_volatile(ptr as *mut u64, value) }
}

#[inline]
fn read_volatile_u16(ptr: *const u16) -> u16 {
    unsafe { core::ptr::read_volatile(ptr) }
}

#[inline]
fn write_volatile_u16(ptr: *mut u16, value: u16) {
    unsafe { core::ptr::write_volatile(ptr, value) }
}

#[inline]
fn read_volatile_used_elem(ptr: *const VirtqUsedElem) -> VirtqUsedElem {
    unsafe { core::ptr::read_volatile(ptr) }
}

#[inline]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn port_inl(port: u16) -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!("in eax, dx", out("eax") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    value
}

#[inline]
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn port_inl(_port: u16) -> u32 {
    u32::MAX
}

#[inline]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn port_outl(port: u16, value: u32) {
    unsafe {
        core::arch::asm!("out dx, eax", in("dx") port, in("eax") value, options(nomem, nostack, preserves_flags));
    }
}

#[inline]
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn port_outl(_port: u16, _value: u32) {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    static DRIVER_CLAIM_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn driver_claim_test_lock() -> MutexGuard<'static, ()> {
        match DRIVER_CLAIM_TEST_LOCK.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    #[repr(C, align(8))]
    struct TestCommonConfig([u8; 64]);

    #[repr(C, align(8))]
    struct TestMmio([u8; 0x200]);

    fn init_test_mmio(mmio: &mut TestMmio, device_type: u32) -> *mut u8 {
        let base = mmio.0.as_mut_ptr();
        write_u32(
            unsafe { base.add(VIRTIO_MMIO_MAGIC_VALUE) },
            VIRTIO_MMIO_MAGIC,
        );
        write_u32(
            unsafe { base.add(VIRTIO_MMIO_VERSION) },
            VIRTIO_MMIO_VERSION_MODERN,
        );
        write_u32(unsafe { base.add(VIRTIO_MMIO_DEVICE_ID) }, device_type);
        write_u32(
            unsafe { base.add(VIRTIO_MMIO_VENDOR) },
            VIRTIO_MMIO_VENDOR_ID as u32,
        );
        base
    }

    fn set_mmio_modern_features(base: *mut u8, features: u64) {
        write_u32(unsafe { base.add(VIRTIO_MMIO_DEVICE_FEATURES_SEL) }, 0);
        write_u32(
            unsafe { base.add(VIRTIO_MMIO_DEVICE_FEATURES) },
            features as u32,
        );
        write_u32(unsafe { base.add(VIRTIO_MMIO_DEVICE_FEATURES_SEL) }, 1);
        write_u32(
            unsafe { base.add(VIRTIO_MMIO_DEVICE_FEATURES) },
            (features >> 32) as u32,
        );
    }

    #[test]
    fn virtio_net_header_matches_modern_layout() {
        assert_eq!(core::mem::size_of::<VirtioNetHdr>(), 12);
        assert_eq!(NET_HDR_LEN, 12);
    }

    #[test]
    fn split_virtqueue_layout_matches_spec_basics() {
        assert_eq!(core::mem::size_of::<VirtqDesc>(), 16);
        assert_eq!(core::mem::align_of::<VirtqDesc>(), 8);
        assert_eq!(core::mem::align_of::<VirtqAvail>(), 2);
        assert_eq!(core::mem::align_of::<VirtqUsed>(), 4);
    }

    #[test]
    fn single_used_completion_accepts_exactly_one_entry() {
        let used = VirtqUsed {
            flags: 0,
            idx: 1,
            ring: [VirtqUsedElem { id: 7, len: 11 }; QUEUE_SIZE],
            avail_event: 0,
        };
        let mut last = 0;

        let elem =
            unsafe { take_single_used_completion(&used, QUEUE_SIZE as u16, &mut last) }.unwrap();

        assert_eq!(elem.id, 7);
        assert_eq!(elem.len, 11);
        assert_eq!(last, 1);
        assert!(
            unsafe { take_single_used_completion(&used, QUEUE_SIZE as u16, &mut last) }.is_none()
        );
        assert_eq!(last, 1);
    }

    #[test]
    fn single_used_completion_rejects_unexpected_jump_and_resyncs() {
        let used = VirtqUsed {
            flags: 0,
            idx: 3,
            ring: [VirtqUsedElem { id: 7, len: 11 }; QUEUE_SIZE],
            avail_event: 0,
        };
        let mut last = 0;

        assert!(
            unsafe { take_single_used_completion(&used, QUEUE_SIZE as u16, &mut last) }.is_none()
        );
        assert_eq!(last, 3);
    }

    #[test]
    fn pci_address_masks_register_offset() {
        assert_eq!(pci_address(1, 2, 3, 0x10), 0x8001_1310);
        assert_eq!(pci_address(1, 2, 3, 0x13), 0x8001_1310);
    }

    #[test]
    fn net_frame_lengths_reject_invalid_bounds() {
        assert_eq!(net_tx_frame_len(0), None);
        assert_eq!(net_tx_frame_len(1), Some((NET_HDR_LEN + 1) as u32));
        assert_eq!(
            net_tx_frame_len(BUFFER_SIZE - NET_HDR_LEN),
            Some(BUFFER_SIZE as u32)
        );
        assert_eq!(net_tx_frame_len(BUFFER_SIZE - NET_HDR_LEN + 1), None);

        assert_eq!(net_rx_payload_len((NET_HDR_LEN - 1) as u32), None);
        assert_eq!(net_rx_payload_len(NET_HDR_LEN as u32), Some(0));
        assert_eq!(
            net_rx_payload_len(BUFFER_SIZE as u32),
            Some(BUFFER_SIZE - NET_HDR_LEN)
        );
        assert_eq!(net_rx_payload_len((BUFFER_SIZE + 1) as u32), None);
    }

    #[test]
    fn console_rx_lengths_reject_buffer_overflow() {
        assert_eq!(console_rx_len(0), Some(0));
        assert_eq!(
            console_rx_len(CONSOLE_BUFFER_SIZE as u32),
            Some(CONSOLE_BUFFER_SIZE)
        );
        assert_eq!(console_rx_len((CONSOLE_BUFFER_SIZE + 1) as u32), None);
    }

    #[test]
    fn interrupt_status_decodes_known_bits() {
        assert_eq!(
            VirtioInterruptStatus::from_raw(0),
            VirtioInterruptStatus::default()
        );
        assert_eq!(
            VirtioInterruptStatus::from_raw(VIRTIO_INTERRUPT_USED_RING),
            VirtioInterruptStatus {
                raw: VIRTIO_INTERRUPT_USED_RING,
                used_ring: true,
                config_change: false,
            }
        );
        assert_eq!(
            VirtioInterruptStatus::from_raw(
                VIRTIO_INTERRUPT_USED_RING | VIRTIO_INTERRUPT_CONFIG_CHANGE
            ),
            VirtioInterruptStatus {
                raw: VIRTIO_INTERRUPT_USED_RING | VIRTIO_INTERRUPT_CONFIG_CHANGE,
                used_ring: true,
                config_change: true,
            }
        );
    }

    #[test]
    fn modern_pci_device_types_map_known_devices() {
        assert_eq!(
            modern_pci_device_type(VIRTIO_MODERN_DEVICE_ID_NET),
            Some(VIRTIO_DEVICE_TYPE_NET)
        );
        assert_eq!(
            modern_pci_device_type(VIRTIO_MODERN_DEVICE_ID_BLK),
            Some(VIRTIO_DEVICE_TYPE_BLK)
        );
        assert_eq!(
            modern_pci_device_type(VIRTIO_MODERN_DEVICE_ID_CONSOLE),
            Some(VIRTIO_DEVICE_TYPE_CONSOLE)
        );
        assert_eq!(
            modern_pci_device_type(VIRTIO_MODERN_DEVICE_ID_RNG),
            Some(VIRTIO_DEVICE_TYPE_RNG)
        );
        assert_eq!(modern_pci_device_type(0x1045), None);
    }

    #[test]
    fn modern_pci_transport_requires_common_and_notify_config() {
        let mut common = TestCommonConfig([0; 64]);
        let mut notify = [0u8; 8];
        let common_cfg = common.0.as_mut_ptr();
        let notify_cfg = notify.as_mut_ptr();

        assert!(VirtioTransport::modern_pci(
            0,
            1,
            0,
            common_cfg,
            notify_cfg,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            4,
        )
        .is_some());
        assert!(VirtioTransport::modern_pci(
            0,
            1,
            0,
            core::ptr::null_mut(),
            notify_cfg,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            4,
        )
        .is_none());
        assert!(VirtioTransport::modern_pci(
            0,
            1,
            0,
            common_cfg,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            4,
        )
        .is_none());
    }

    #[test]
    fn mmio_transport_requires_magic_and_modern_version() {
        let mut mmio = TestMmio([0; 0x200]);
        let base = init_test_mmio(&mut mmio, VIRTIO_DEVICE_TYPE_NET);

        assert!(VirtioTransport::mmio(base).is_some());
        write_u32(unsafe { base.add(VIRTIO_MMIO_MAGIC_VALUE) }, 0);
        assert!(VirtioTransport::mmio(base).is_none());

        write_u32(
            unsafe { base.add(VIRTIO_MMIO_MAGIC_VALUE) },
            VIRTIO_MMIO_MAGIC,
        );
        write_u32(unsafe { base.add(VIRTIO_MMIO_VERSION) }, 1);
        assert!(VirtioTransport::mmio(base).is_none());
    }

    #[test]
    fn mmio_device_info_and_constructors_validate_device_type() {
        let _guard = driver_claim_test_lock();
        let mut mmio = TestMmio([0; 0x200]);
        let base = init_test_mmio(&mut mmio, VIRTIO_DEVICE_TYPE_RNG) as usize;

        let info = virtio_mmio_device_info(base).unwrap();
        assert_eq!(info.transport, VirtioTransportKind::Mmio);
        assert_eq!(info.device_type, VIRTIO_DEVICE_TYPE_RNG);
        assert_eq!(info.vendor_id, VIRTIO_MMIO_VENDOR_ID);
        assert_eq!(info.mmio_base, base);

        assert!(VirtRng::from_mmio_base(base).is_some());
        assert!(VirtBlk::from_mmio_base(base).is_none());
        assert!(info.open_rng().is_some());
        assert!(info.open_net().is_none());
        assert!(VirtNet::from_mmio_base(0).is_none());
    }

    #[test]
    fn scan_mmio_devices_counts_all_matches_and_stores_prefix() {
        let mut rng_mmio = TestMmio([0; 0x200]);
        let mut blk_mmio = TestMmio([0; 0x200]);
        let rng_base = init_test_mmio(&mut rng_mmio, VIRTIO_DEVICE_TYPE_RNG) as usize;
        let blk_base = init_test_mmio(&mut blk_mmio, VIRTIO_DEVICE_TYPE_BLK) as usize;
        let bases = [0, rng_base, blk_base];
        let mut out = [VirtioDeviceInfo {
            transport: VirtioTransportKind::ModernPci,
            device_type: 0,
            vendor_id: 0,
            device_id: 0,
            bus: 0,
            slot: 0,
            func: 0,
            mmio_base: 0,
        }];

        assert_eq!(scan_virtio_mmio_devices(&bases, &mut out), 2);
        assert_eq!(out[0].transport, VirtioTransportKind::Mmio);
        assert_eq!(out[0].device_type, VIRTIO_DEVICE_TYPE_RNG);
        assert_eq!(out[0].mmio_base, rng_base);
    }

    #[test]
    fn mmio_constructors_reject_second_handle_until_drop() {
        let _guard = driver_claim_test_lock();
        let mut net_mmio = TestMmio([0; 0x200]);
        let base = init_test_mmio(&mut net_mmio, VIRTIO_DEVICE_TYPE_NET) as usize;

        let first = VirtNet::from_mmio_base(base).unwrap();
        assert!(VirtNet::from_mmio_base(base).is_none());
        drop(first);
        assert!(VirtNet::from_mmio_base(base).is_some());
    }

    #[test]
    fn init_failure_releases_driver_claim() {
        let _guard = driver_claim_test_lock();
        let mut net_mmio = TestMmio([0; 0x200]);
        let base_ptr = init_test_mmio(&mut net_mmio, VIRTIO_DEVICE_TYPE_NET);
        let base = base_ptr as usize;

        let mut first = VirtNet::from_mmio_base(base).unwrap();
        assert!(!first.init());
        assert_eq!(read_u32(unsafe { base_ptr.add(VIRTIO_MMIO_STATUS) }), 0);
        assert!(VirtNet::from_mmio_base(base).is_some());
    }

    #[test]
    fn initialized_mmio_rng_helper_returns_ready_device() {
        let _guard = driver_claim_test_lock();
        let mut rng_mmio = TestMmio([0; 0x200]);
        let base_ptr = init_test_mmio(&mut rng_mmio, VIRTIO_DEVICE_TYPE_RNG);
        let base = base_ptr as usize;

        set_mmio_modern_features(base_ptr, VIRTIO_F_VERSION_1);
        write_u32(unsafe { base_ptr.add(VIRTIO_MMIO_QUEUE_NUM_MAX) }, 1);

        let rng = find_initialized_virtio_rng_mmio(base).unwrap();

        assert!(rng.is_initialized());
        assert_eq!(
            read_u32(unsafe { base_ptr.add(VIRTIO_MMIO_STATUS) }) as u8,
            VIRTIO_CONFIG_STATUS_ACKNOWLEDGE
                | VIRTIO_CONFIG_STATUS_DRIVER
                | VIRTIO_CONFIG_STATUS_FEATURES_OK
                | VIRTIO_CONFIG_STATUS_DRIVER_OK
        );
    }

    #[test]
    fn mmio_feature_negotiation_writes_status_and_driver_features() {
        let mut mmio = TestMmio([0; 0x200]);
        let base = init_test_mmio(&mut mmio, VIRTIO_DEVICE_TYPE_RNG);
        let transport = VirtioTransport::mmio(base).unwrap();

        set_mmio_modern_features(base, VIRTIO_F_VERSION_1);

        let negotiated = transport.negotiate_features(VIRTIO_F_VERSION_1).unwrap();
        assert_eq!(negotiated.driver, VIRTIO_F_VERSION_1);
        assert_eq!(
            read_u32(unsafe { base.add(VIRTIO_MMIO_DRIVER_FEATURES_SEL) }),
            1
        );
        assert_eq!(
            read_u32(unsafe { base.add(VIRTIO_MMIO_DRIVER_FEATURES) }),
            (VIRTIO_F_VERSION_1 >> 32) as u32
        );
        assert_eq!(
            read_u32(unsafe { base.add(VIRTIO_MMIO_STATUS) }) as u8,
            VIRTIO_CONFIG_STATUS_ACKNOWLEDGE
                | VIRTIO_CONFIG_STATUS_DRIVER
                | VIRTIO_CONFIG_STATUS_FEATURES_OK
        );
    }

    #[test]
    fn mmio_queue_setup_writes_split_queue_registers() {
        let mut mmio = TestMmio([0; 0x200]);
        let base = init_test_mmio(&mut mmio, VIRTIO_DEVICE_TYPE_RNG);
        let transport = VirtioTransport::mmio(base).unwrap();

        write_u32(unsafe { base.add(VIRTIO_MMIO_QUEUE_NUM_MAX) }, 8);
        let queue_size = transport
            .configure_split_queue(3, 16, 1, 0x1122_3344_5566_7788, 0x2233_4455, 0x3344_5566)
            .unwrap();

        assert_eq!(queue_size, 8);
        assert_eq!(read_u32(unsafe { base.add(VIRTIO_MMIO_QUEUE_SEL) }), 3);
        assert_eq!(read_u32(unsafe { base.add(VIRTIO_MMIO_QUEUE_NUM) }), 8);
        assert_eq!(
            read_u32(unsafe { base.add(VIRTIO_MMIO_QUEUE_DESC_LOW) }),
            0x5566_7788
        );
        assert_eq!(
            read_u32(unsafe { base.add(VIRTIO_MMIO_QUEUE_DESC_HIGH) }),
            0x1122_3344
        );
        assert_eq!(
            read_u32(unsafe { base.add(VIRTIO_MMIO_QUEUE_DRIVER_LOW) }),
            0x2233_4455
        );
        assert_eq!(
            read_u32(unsafe { base.add(VIRTIO_MMIO_QUEUE_DEVICE_LOW) }),
            0x3344_5566
        );
        assert_eq!(read_u32(unsafe { base.add(VIRTIO_MMIO_QUEUE_READY) }), 1);

        transport.notify_split_queue(0, 3);
        assert_eq!(read_u32(unsafe { base.add(VIRTIO_MMIO_QUEUE_NOTIFY) }), 3);
    }

    #[test]
    fn mmio_take_interrupt_status_reads_and_acks() {
        let mut mmio = TestMmio([0; 0x200]);
        let base = init_test_mmio(&mut mmio, VIRTIO_DEVICE_TYPE_RNG);
        let transport = VirtioTransport::mmio(base).unwrap();

        write_u32(
            unsafe { base.add(VIRTIO_MMIO_INTERRUPT_STATUS) },
            (VIRTIO_INTERRUPT_USED_RING | VIRTIO_INTERRUPT_CONFIG_CHANGE) as u32,
        );

        let status = transport.take_interrupt_status();
        assert!(status.used_ring);
        assert!(status.config_change);
        assert_eq!(
            read_u32(unsafe { base.add(VIRTIO_MMIO_INTERRUPT_ACK) }),
            (VIRTIO_INTERRUPT_USED_RING | VIRTIO_INTERRUPT_CONFIG_CHANGE) as u32
        );
    }

    #[test]
    fn release_resets_mmio_device_status() {
        let _guard = driver_claim_test_lock();
        let mut rng_mmio = TestMmio([0; 0x200]);
        let base_ptr = init_test_mmio(&mut rng_mmio, VIRTIO_DEVICE_TYPE_RNG);
        let base = base_ptr as usize;

        write_u32(
            unsafe { base_ptr.add(VIRTIO_MMIO_STATUS) },
            VIRTIO_CONFIG_STATUS_DRIVER_OK as u32,
        );
        let rng = VirtRng::from_mmio_base(base).unwrap();
        drop(rng);

        assert_eq!(read_u32(unsafe { base_ptr.add(VIRTIO_MMIO_STATUS) }), 0);
    }

    #[test]
    fn split_queue_setup_returns_configured_size_not_host_max() {
        let mut mmio = TestMmio([0; 0x200]);
        let base = init_test_mmio(&mut mmio, VIRTIO_DEVICE_TYPE_NET);
        let transport = VirtioTransport::mmio(base).unwrap();

        write_u32(unsafe { base.add(VIRTIO_MMIO_QUEUE_NUM_MAX) }, 64);
        let queue_size = transport
            .configure_split_queue(
                0,
                QUEUE_SIZE as u16,
                QUEUE_SIZE as u16,
                0x1000,
                0x2000,
                0x3000,
            )
            .unwrap();

        assert_eq!(queue_size, QUEUE_SIZE as u16);
        assert_eq!(
            read_u32(unsafe { base.add(VIRTIO_MMIO_QUEUE_NUM) }),
            QUEUE_SIZE as u32
        );
    }

    #[test]
    fn split_queue_setup_clamps_to_host_queue_size() {
        let mut common = TestCommonConfig([0; 64]);
        let mut notify = [0u8; 8];
        let common_cfg = common.0.as_mut_ptr();
        let transport = VirtioTransport::modern_pci(
            0,
            1,
            0,
            common_cfg,
            notify.as_mut_ptr(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            4,
        )
        .unwrap();

        write_u16(unsafe { common_cfg.add(24) }, 4);
        let configured =
            transport.configure_split_queue(2, QUEUE_SIZE as u16, 3, 0x1000, 0x2000, 0x3000);

        assert_eq!(configured, Some(4));
        assert_eq!(read_u16(unsafe { common_cfg.add(22) }), 2);
        assert_eq!(read_u16(unsafe { common_cfg.add(24) }), 4);
        assert_eq!(read_u16(unsafe { common_cfg.add(28) }), 1);
        assert_eq!(read_u64(unsafe { common_cfg.add(32) }), 0x1000);
        assert_eq!(read_u64(unsafe { common_cfg.add(40) }), 0x2000);
        assert_eq!(read_u64(unsafe { common_cfg.add(48) }), 0x3000);
    }

    #[test]
    fn split_queue_setup_rejects_too_small_host_queue() {
        let mut common = TestCommonConfig([0; 64]);
        let mut notify = [0u8; 8];
        let common_cfg = common.0.as_mut_ptr();
        let transport = VirtioTransport::modern_pci(
            0,
            1,
            0,
            common_cfg,
            notify.as_mut_ptr(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            4,
        )
        .unwrap();

        write_u16(unsafe { common_cfg.add(24) }, 2);

        assert_eq!(
            transport.configure_split_queue(2, QUEUE_SIZE as u16, 3, 0x1000, 0x2000, 0x3000),
            None
        );
        assert_eq!(read_u16(unsafe { common_cfg.add(28) }), 0);
    }

    #[test]
    fn virtnet_defaults_are_uninitialized_and_link_down() {
        let mut net = VirtNet::new();

        assert_eq!(net.get_mac(), [0; 6]);
        assert_eq!(net.mtu(), 1500);
        assert!(!net.is_link_up());
        assert!(!net.refresh_status());
        assert_eq!(net.tx_free_mask, TX_FREE_ALL_MASK);
        assert!(!net.send(&[]));
    }

    #[test]
    fn virtblk_defaults_reject_io_until_initialized() {
        let mut blk = VirtBlk::new();
        let sector = [0u8; SECTOR_SIZE];
        let mut out = [0u8; SECTOR_SIZE];
        let mut empty = [];

        assert!(!blk.is_read_only());
        assert_eq!(blk.block_size(), SECTOR_SIZE as u32);
        assert_eq!(blk.sectors(), 0);
        assert!(!blk.read_sector(0, &mut out));
        assert!(!blk.write_sector(0, &sector));
        assert!(!blk.read_sectors(0, &mut empty));
        assert!(!blk.write_sectors(0, &empty));
        assert!(!blk.read_sectors(1, &mut empty));
        assert!(!blk.read_sectors(0, &mut out[..SECTOR_SIZE - 1]));
        assert!(!blk.write_sectors(0, &sector[..SECTOR_SIZE - 1]));
    }

    #[test]
    fn virtblk_multi_sector_helpers_validate_ranges() {
        let mut blk = VirtBlk::new();
        let mut two_sectors = [0u8; SECTOR_SIZE * 2];

        blk.sectors = 4;
        assert_eq!(VirtBlk::chunk_sector_count(0), None);
        assert_eq!(VirtBlk::chunk_sector_count(SECTOR_SIZE), Some(1));
        assert_eq!(VirtBlk::chunk_sector_count(SECTOR_SIZE - 1), None);
        assert_eq!(VirtBlk::chunk_sector_count(SECTOR_SIZE * 2), None);
        assert!(!blk.sector_range_in_bounds(3, 2));
        assert!(!blk.sector_range_in_bounds(u64::MAX, 1));
        assert!(!blk.read_sectors(3, &mut two_sectors));
        assert!(!blk.write_sectors(3, &two_sectors));

        blk.read_only = true;
        assert!(!blk.write_sectors(0, &two_sectors));
    }

    #[test]
    fn virtblk_flush_descriptor_chain_has_no_data_buffer() {
        let _guard = driver_claim_test_lock();
        let mut blk = VirtBlk::new();

        unsafe {
            assert!(blk.prepare_request_descriptors(VIRTIO_BLK_T_FLUSH, 0, 0, false));

            let desc = core::ptr::addr_of!(BLK_DESC.0) as *const VirtqDesc;
            let header = core::ptr::read(desc);
            let status = core::ptr::read(desc.add(2));

            assert_eq!(header.flags, VIRTQ_DESC_F_NEXT);
            assert_eq!(header.next, 2);
            assert_eq!(status.len, 1);
            assert_eq!(status.flags, VIRTQ_DESC_F_WRITE);
            assert_eq!(status.next, 0);
        }
    }

    #[test]
    fn uninitialized_rng_and_console_reject_io() {
        let mut rng = VirtRng::new();
        let mut console = VirtConsole::new();
        let mut empty = [];
        let mut input = [0u8; 8];

        assert!(!rng.fill_bytes(&mut empty));
        assert_eq!(console.recv(&mut input), None);
        assert!(!console.write_all(&[]));
    }
}
