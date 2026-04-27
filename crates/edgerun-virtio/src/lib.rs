//! Modern virtio-net PCI driver for QEMU/KVM.

#![no_std]
#![allow(dead_code)]

use core::sync::atomic::{fence, Ordering};

pub const VIRTIO_VENDOR_ID: u16 = 0x1af4;
pub const VIRTIO_MODERN_DEVICE_ID_NET: u16 = 0x1041;
pub const VIRTIO_MODERN_DEVICE_ID_BLK: u16 = 0x1042;
pub const VIRTIO_MODERN_DEVICE_ID_CONSOLE: u16 = 0x1043;
pub const VIRTIO_MODERN_DEVICE_ID_RNG: u16 = 0x1044;

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

const VIRTIO_PCI_CAP_VENDOR: u8 = 0x09;
const VIRTIO_PCI_CAP_COMMON_CFG: u8 = 1;
const VIRTIO_PCI_CAP_NOTIFY_CFG: u8 = 2;
const VIRTIO_PCI_CAP_ISR_CFG: u8 = 3;
const VIRTIO_PCI_CAP_DEVICE_CFG: u8 = 4;

const QUEUE_SIZE: usize = 8;
const RX_QUEUE: u16 = 0;
const TX_QUEUE: u16 = 1;
const CONSOLE_TX_QUEUE: u16 = 1;
const NET_HDR_LEN: usize = core::mem::size_of::<VirtioNetHdr>();
const BUFFER_SIZE: usize = 2048;
const TX_FREE_ALL_MASK: u16 = (1u16 << QUEUE_SIZE) - 1;
const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;
const VIRTIO_POLL_SPINS: usize = 50_000;

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
struct BlkData([u8; edgerun_rt::storage::SECTOR_SIZE]);

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
static mut BLK_DATA: BlkData = BlkData([0; edgerun_rt::storage::SECTOR_SIZE]);
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
struct ConsoleData([u8; 256]);

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
static mut CONSOLE_DATA: ConsoleData = ConsoleData([0; 256]);

pub struct VirtNet {
    mac: [u8; 6],
    mtu: u16,
    status: u16,
    features: u64,
    host_features: u64,
    bus: u8,
    slot: u8,
    func: u8,
    common_cfg: *mut u8,
    notify_cfg: *mut u8,
    device_cfg: *mut u8,
    isr_cfg: *mut u8,
    notify_off_multiplier: u32,
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
    pub const fn new() -> Self {
        Self {
            mac: [0; 6],
            mtu: 1500,
            status: 0,
            features: 0,
            host_features: 0,
            bus: 0,
            slot: 0,
            func: 0,
            common_cfg: core::ptr::null_mut(),
            notify_cfg: core::ptr::null_mut(),
            device_cfg: core::ptr::null_mut(),
            isr_cfg: core::ptr::null_mut(),
            notify_off_multiplier: 0,
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
        }
    }

    pub fn init(&mut self) -> bool {
        if self.common_cfg.is_null() || self.device_cfg.is_null() || self.notify_cfg.is_null() {
            return false;
        }

        enable_pci_memory_and_bus_master(self.bus, self.slot, self.func);
        write_common_status(self.common_cfg, 0);
        write_common_status(self.common_cfg, VIRTIO_CONFIG_STATUS_ACKNOWLEDGE);
        write_common_status(
            self.common_cfg,
            VIRTIO_CONFIG_STATUS_ACKNOWLEDGE | VIRTIO_CONFIG_STATUS_DRIVER,
        );

        self.host_features = read_device_features(self.common_cfg);
        self.features =
            self.host_features & (VIRTIO_F_VERSION_1 | VIRTIO_NET_F_MAC | VIRTIO_NET_F_STATUS);
        if self.features & VIRTIO_F_VERSION_1 == 0 {
            fail_device(self.common_cfg);
            return false;
        }

        write_driver_features(self.common_cfg, self.features);
        write_common_status(
            self.common_cfg,
            common_status(self.common_cfg) | VIRTIO_CONFIG_STATUS_FEATURES_OK,
        );
        if common_status(self.common_cfg) & VIRTIO_CONFIG_STATUS_FEATURES_OK == 0 {
            fail_device(self.common_cfg);
            return false;
        }

        if self.features & VIRTIO_NET_F_MAC != 0 {
            for i in 0..6 {
                self.mac[i] = read_u8(unsafe { self.device_cfg.add(i) });
            }
        }

        if self.features & VIRTIO_NET_F_STATUS != 0 {
            self.status = read_u16(unsafe { self.device_cfg.add(6) });
            self.link_up = (self.status & VIRTIO_NET_S_LINK_UP) != 0;
        } else {
            self.status = VIRTIO_NET_S_LINK_UP;
            self.link_up = true;
        }

        select_queue(self.common_cfg, RX_QUEUE);
        self.queue_size = read_queue_size(self.common_cfg);
        self.rx_notify_off = read_queue_notify_off(self.common_cfg);

        if self.queue_size < QUEUE_SIZE as u16 {
            fail_device(self.common_cfg);
            return false;
        }

        select_queue(self.common_cfg, TX_QUEUE);
        if read_queue_size(self.common_cfg) < QUEUE_SIZE as u16 {
            fail_device(self.common_cfg);
            return false;
        }
        self.tx_notify_off = read_queue_notify_off(self.common_cfg);

        let (rx_desc, rx_avail, rx_used) = unsafe {
            (
                core::ptr::addr_of_mut!(RX_DESC.0) as u64,
                core::ptr::addr_of_mut!(RX_AVAIL) as u64,
                core::ptr::addr_of_mut!(RX_USED) as u64,
            )
        };
        if !setup_split_queue(
            self.common_cfg,
            RX_QUEUE,
            QUEUE_SIZE as u16,
            QUEUE_SIZE as u16,
            rx_desc,
            rx_avail,
            rx_used,
        ) {
            fail_device(self.common_cfg);
            return false;
        }

        let (tx_desc, tx_avail, tx_used) = unsafe {
            (
                core::ptr::addr_of_mut!(TX_DESC.0) as u64,
                core::ptr::addr_of_mut!(TX_AVAIL) as u64,
                core::ptr::addr_of_mut!(TX_USED) as u64,
            )
        };
        if !setup_split_queue(
            self.common_cfg,
            TX_QUEUE,
            QUEUE_SIZE as u16,
            QUEUE_SIZE as u16,
            tx_desc,
            tx_avail,
            tx_used,
        ) {
            fail_device(self.common_cfg);
            return false;
        }

        unsafe {
            self.init_rx_queue();
            self.init_tx_queue();
        }

        write_common_status(
            self.common_cfg,
            common_status(self.common_cfg) | VIRTIO_CONFIG_STATUS_DRIVER_OK,
        );
        self.notify_queue(RX_QUEUE);

        true
    }

    pub fn get_mac(&self) -> [u8; 6] {
        self.mac
    }

    pub fn is_link_up(&self) -> bool {
        self.link_up
    }

    pub fn mtu(&self) -> u16 {
        self.mtu
    }

    pub fn stats(&mut self) -> VirtNetStats {
        unsafe {
            self.reap_tx_used();
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
        if data.is_empty() || data.len() + NET_HDR_LEN > BUFFER_SIZE {
            return false;
        }

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
                    len: (data.len() + NET_HDR_LEN) as u32,
                    flags: 0,
                    next: 0,
                },
            );

            post_split_queue_descriptor(core::ptr::addr_of_mut!(TX_AVAIL), desc_id);
            self.tx_submitted = self.tx_submitted.wrapping_add(1);
        }

        self.notify_queue(TX_QUEUE);
        true
    }

    pub fn recv(&mut self, buf: &mut [u8]) -> Option<usize> {
        unsafe {
            let used = core::ptr::addr_of_mut!(RX_USED);
            let used_idx = split_queue_used_idx(used);
            if used_idx == self.rx_last_used_idx {
                return None;
            }

            let elem = split_queue_used_elem(used, self.rx_last_used_idx);
            self.rx_last_used_idx = self.rx_last_used_idx.wrapping_add(1);

            let desc_id = elem.id as usize;
            if desc_id >= QUEUE_SIZE {
                self.rx_invalid = self.rx_invalid.wrapping_add(1);
                return None;
            }

            let payload_len = (elem.len as usize).saturating_sub(NET_HDR_LEN);
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
        let mut net = Self::new();
        net.bus = mapped.bus;
        net.slot = mapped.slot;
        net.func = mapped.func;
        net.common_cfg = mapped.common_cfg;
        net.notify_cfg = mapped.notify_cfg;
        net.device_cfg = mapped.device_cfg;
        net.isr_cfg = mapped.isr_cfg;
        net.notify_off_multiplier = mapped.notify_off_multiplier;
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
        post_split_queue_descriptor(core::ptr::addr_of_mut!(RX_AVAIL), desc_id);
    }

    unsafe fn reap_tx_used(&mut self) {
        let used = core::ptr::addr_of!(TX_USED);
        let used_idx = split_queue_used_idx(used);
        while self.tx_last_used_idx != used_idx {
            let elem = split_queue_used_elem(used, self.tx_last_used_idx);
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
        notify_split_queue(
            self.notify_cfg,
            self.notify_off_multiplier,
            notify_off,
            queue,
        );
    }
}

impl Default for VirtNet {
    fn default() -> Self {
        Self::new()
    }
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

fn common_status(common_cfg: *const u8) -> u8 {
    read_u8(unsafe { common_cfg.add(20) })
}

fn write_common_status(common_cfg: *mut u8, status: u8) {
    write_u8(unsafe { common_cfg.add(20) }, status);
}

fn fail_device(common_cfg: *mut u8) {
    write_common_status(
        common_cfg,
        common_status(common_cfg) | VIRTIO_CONFIG_STATUS_FAILED,
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

fn select_queue(common_cfg: *mut u8, queue: u16) {
    write_u16(unsafe { common_cfg.add(22) }, queue);
}

fn read_queue_size(common_cfg: *const u8) -> u16 {
    read_u16(unsafe { common_cfg.add(24) })
}

fn read_queue_notify_off(common_cfg: *const u8) -> u16 {
    read_u16(unsafe { common_cfg.add(30) })
}

fn setup_split_queue(
    common_cfg: *mut u8,
    queue: u16,
    queue_size: u16,
    min_queue_size: u16,
    desc: u64,
    driver: u64,
    device: u64,
) -> bool {
    select_queue(common_cfg, queue);
    if read_queue_size(common_cfg) < min_queue_size {
        return false;
    }

    write_u16(unsafe { common_cfg.add(24) }, queue_size);
    write_u64(unsafe { common_cfg.add(32) }, desc);
    write_u64(unsafe { common_cfg.add(40) }, driver);
    write_u64(unsafe { common_cfg.add(48) }, device);
    write_u16(unsafe { common_cfg.add(28) }, 1);
    true
}

fn notify_split_queue(notify_cfg: *mut u8, notify_off_multiplier: u32, queue_off: u16, queue: u16) {
    let offset = (queue_off as u32).saturating_mul(notify_off_multiplier) as usize;
    write_u16(unsafe { notify_cfg.add(offset) }, queue);
}

unsafe fn post_split_queue_descriptor(avail: *mut VirtqAvail, desc_id: u16) {
    let idx = read_volatile_u16(core::ptr::addr_of!((*avail).idx));
    core::ptr::write_volatile(
        (*avail).ring.as_mut_ptr().add((idx as usize) % QUEUE_SIZE),
        desc_id,
    );
    fence(Ordering::SeqCst);
    write_volatile_u16(core::ptr::addr_of_mut!((*avail).idx), idx.wrapping_add(1));
}

unsafe fn split_queue_used_idx(used: *const VirtqUsed) -> u16 {
    read_volatile_u16(core::ptr::addr_of!((*used).idx))
}

unsafe fn split_queue_used_elem(used: *const VirtqUsed, used_idx: u16) -> VirtqUsedElem {
    let ring_idx = (used_idx as usize) % QUEUE_SIZE;
    read_volatile_used_elem((*used).ring.as_ptr().add(ring_idx))
}

pub struct VirtBlk {
    features: u64,
    host_features: u64,
    bus: u8,
    slot: u8,
    func: u8,
    common_cfg: *mut u8,
    notify_cfg: *mut u8,
    device_cfg: *mut u8,
    isr_cfg: *mut u8,
    notify_off_multiplier: u32,
    queue_notify_off: u16,
    queue_size: u16,
    last_used_idx: u16,
    sectors: u64,
    read_only: bool,
    block_size: u32,
}

unsafe impl Send for VirtBlk {}

impl VirtBlk {
    pub const fn new() -> Self {
        Self {
            features: 0,
            host_features: 0,
            bus: 0,
            slot: 0,
            func: 0,
            common_cfg: core::ptr::null_mut(),
            notify_cfg: core::ptr::null_mut(),
            device_cfg: core::ptr::null_mut(),
            isr_cfg: core::ptr::null_mut(),
            notify_off_multiplier: 0,
            queue_notify_off: 0,
            queue_size: 0,
            last_used_idx: 0,
            sectors: 0,
            read_only: false,
            block_size: edgerun_rt::storage::SECTOR_SIZE as u32,
        }
    }

    pub fn init(&mut self) -> bool {
        if self.common_cfg.is_null() || self.device_cfg.is_null() || self.notify_cfg.is_null() {
            return false;
        }

        enable_pci_memory_and_bus_master(self.bus, self.slot, self.func);
        write_common_status(self.common_cfg, 0);
        write_common_status(self.common_cfg, VIRTIO_CONFIG_STATUS_ACKNOWLEDGE);
        write_common_status(
            self.common_cfg,
            VIRTIO_CONFIG_STATUS_ACKNOWLEDGE | VIRTIO_CONFIG_STATUS_DRIVER,
        );

        self.host_features = read_device_features(self.common_cfg);
        self.features = self.host_features
            & (VIRTIO_F_VERSION_1 | VIRTIO_BLK_F_RO | VIRTIO_BLK_F_BLK_SIZE | VIRTIO_BLK_F_FLUSH);
        if self.features & VIRTIO_F_VERSION_1 == 0 {
            fail_device(self.common_cfg);
            return false;
        }

        write_driver_features(self.common_cfg, self.features);
        write_common_status(
            self.common_cfg,
            common_status(self.common_cfg) | VIRTIO_CONFIG_STATUS_FEATURES_OK,
        );
        if common_status(self.common_cfg) & VIRTIO_CONFIG_STATUS_FEATURES_OK == 0 {
            fail_device(self.common_cfg);
            return false;
        }

        self.sectors = read_u64(self.device_cfg);
        self.read_only = self.features & VIRTIO_BLK_F_RO != 0;
        if self.features & VIRTIO_BLK_F_BLK_SIZE != 0 {
            self.block_size = read_u32(unsafe { self.device_cfg.add(20) });
        }
        if self.sectors == 0 || self.block_size != edgerun_rt::storage::SECTOR_SIZE as u32 {
            fail_device(self.common_cfg);
            return false;
        }

        select_queue(self.common_cfg, 0);
        self.queue_size = read_queue_size(self.common_cfg);
        self.queue_notify_off = read_queue_notify_off(self.common_cfg);
        if self.queue_size < 3 {
            fail_device(self.common_cfg);
            return false;
        }

        let (desc, avail, used) = unsafe {
            (
                core::ptr::addr_of_mut!(BLK_DESC.0) as u64,
                core::ptr::addr_of_mut!(BLK_AVAIL) as u64,
                core::ptr::addr_of_mut!(BLK_USED) as u64,
            )
        };
        if !setup_split_queue(self.common_cfg, 0, QUEUE_SIZE as u16, 3, desc, avail, used) {
            fail_device(self.common_cfg);
            return false;
        }

        unsafe {
            self.init_queue();
        }

        write_common_status(
            self.common_cfg,
            common_status(self.common_cfg) | VIRTIO_CONFIG_STATUS_DRIVER_OK,
        );
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
        if out.len() != edgerun_rt::storage::SECTOR_SIZE || sector >= self.sectors {
            return false;
        }

        unsafe {
            if !self.submit_request(VIRTIO_BLK_T_IN, sector, true) {
                return false;
            }
            core::ptr::copy_nonoverlapping(
                core::ptr::addr_of!(BLK_DATA.0) as *const u8,
                out.as_mut_ptr(),
                edgerun_rt::storage::SECTOR_SIZE,
            );
        }
        true
    }

    pub fn write_sector(&mut self, sector: u64, data: &[u8]) -> bool {
        if self.read_only
            || data.len() != edgerun_rt::storage::SECTOR_SIZE
            || sector >= self.sectors
        {
            return false;
        }

        unsafe {
            core::ptr::copy_nonoverlapping(
                data.as_ptr(),
                core::ptr::addr_of_mut!(BLK_DATA.0) as *mut u8,
                edgerun_rt::storage::SECTOR_SIZE,
            );
            self.submit_request(VIRTIO_BLK_T_OUT, sector, false)
        }
    }

    pub fn flush(&mut self) -> bool {
        if self.features & VIRTIO_BLK_F_FLUSH == 0 {
            return true;
        }
        unsafe { self.submit_request(VIRTIO_BLK_T_FLUSH, 0, false) }
    }

    fn from_modern_device(device: ModernVirtioDevice) -> Option<Self> {
        let mapped = MappedModernVirtioDevice::map(device)?;
        let mut blk = Self::new();
        blk.bus = mapped.bus;
        blk.slot = mapped.slot;
        blk.func = mapped.func;
        blk.common_cfg = mapped.common_cfg;
        blk.notify_cfg = mapped.notify_cfg;
        blk.device_cfg = mapped.device_cfg;
        blk.isr_cfg = mapped.isr_cfg;
        blk.notify_off_multiplier = mapped.notify_off_multiplier;
        Some(blk)
    }

    unsafe fn init_queue(&mut self) {
        self.last_used_idx = 0;
        write_volatile_u16(core::ptr::addr_of_mut!(BLK_AVAIL.idx), 0);
        write_volatile_u16(core::ptr::addr_of_mut!(BLK_USED.idx), 0);
        core::ptr::write_bytes(
            core::ptr::addr_of_mut!(BLK_DATA.0) as *mut u8,
            0,
            edgerun_rt::storage::SECTOR_SIZE,
        );
        BLK_STATUS = 0xff;
    }

    unsafe fn submit_request(&mut self, request_type: u32, sector: u64, read: bool) -> bool {
        BLK_HEADER = VirtioBlkReqHeader {
            request_type,
            reserved: 0,
            sector,
        };
        BLK_STATUS = 0xff;

        let desc = core::ptr::addr_of_mut!(BLK_DESC.0) as *mut VirtqDesc;
        core::ptr::write(
            desc,
            VirtqDesc {
                addr: core::ptr::addr_of!(BLK_HEADER) as u64,
                len: core::mem::size_of::<VirtioBlkReqHeader>() as u32,
                flags: VIRTQ_DESC_F_NEXT,
                next: 1,
            },
        );
        core::ptr::write(
            desc.add(1),
            VirtqDesc {
                addr: core::ptr::addr_of_mut!(BLK_DATA.0) as u64,
                len: edgerun_rt::storage::SECTOR_SIZE as u32,
                flags: if read {
                    VIRTQ_DESC_F_WRITE | VIRTQ_DESC_F_NEXT
                } else {
                    VIRTQ_DESC_F_NEXT
                },
                next: 2,
            },
        );
        core::ptr::write(
            desc.add(2),
            VirtqDesc {
                addr: core::ptr::addr_of_mut!(BLK_STATUS) as u64,
                len: 1,
                flags: VIRTQ_DESC_F_WRITE,
                next: 0,
            },
        );

        post_split_queue_descriptor(core::ptr::addr_of_mut!(BLK_AVAIL), 0);
        self.notify_queue();

        let mut spins = 0;
        while split_queue_used_idx(core::ptr::addr_of!(BLK_USED)) == self.last_used_idx {
            spins += 1;
            if spins > VIRTIO_POLL_SPINS {
                return false;
            }
            core::hint::spin_loop();
        }

        self.last_used_idx = self.last_used_idx.wrapping_add(1);
        read_u8(core::ptr::addr_of!(BLK_STATUS)) == VIRTIO_BLK_S_OK
    }

    fn notify_queue(&self) {
        notify_split_queue(
            self.notify_cfg,
            self.notify_off_multiplier,
            self.queue_notify_off,
            0,
        );
    }
}

impl Default for VirtBlk {
    fn default() -> Self {
        Self::new()
    }
}

impl edgerun_rt::storage::BlockDevice for VirtBlk {
    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> bool {
        Self::read_sector(self, sector, buf)
    }

    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> bool {
        Self::write_sector(self, sector, buf)
    }

    fn sectors(&self) -> u64 {
        self.sectors
    }
}

pub struct VirtRng {
    features: u64,
    host_features: u64,
    bus: u8,
    slot: u8,
    func: u8,
    common_cfg: *mut u8,
    notify_cfg: *mut u8,
    isr_cfg: *mut u8,
    notify_off_multiplier: u32,
    queue_notify_off: u16,
    queue_size: u16,
    last_used_idx: u16,
}

unsafe impl Send for VirtRng {}

impl VirtRng {
    pub const fn new() -> Self {
        Self {
            features: 0,
            host_features: 0,
            bus: 0,
            slot: 0,
            func: 0,
            common_cfg: core::ptr::null_mut(),
            notify_cfg: core::ptr::null_mut(),
            isr_cfg: core::ptr::null_mut(),
            notify_off_multiplier: 0,
            queue_notify_off: 0,
            queue_size: 0,
            last_used_idx: 0,
        }
    }

    pub fn init(&mut self) -> bool {
        if self.common_cfg.is_null() || self.notify_cfg.is_null() {
            return false;
        }

        enable_pci_memory_and_bus_master(self.bus, self.slot, self.func);
        write_common_status(self.common_cfg, 0);
        write_common_status(self.common_cfg, VIRTIO_CONFIG_STATUS_ACKNOWLEDGE);
        write_common_status(
            self.common_cfg,
            VIRTIO_CONFIG_STATUS_ACKNOWLEDGE | VIRTIO_CONFIG_STATUS_DRIVER,
        );

        self.host_features = read_device_features(self.common_cfg);
        self.features = self.host_features & VIRTIO_F_VERSION_1;
        if self.features & VIRTIO_F_VERSION_1 == 0 {
            fail_device(self.common_cfg);
            return false;
        }

        write_driver_features(self.common_cfg, self.features);
        write_common_status(
            self.common_cfg,
            common_status(self.common_cfg) | VIRTIO_CONFIG_STATUS_FEATURES_OK,
        );
        if common_status(self.common_cfg) & VIRTIO_CONFIG_STATUS_FEATURES_OK == 0 {
            fail_device(self.common_cfg);
            return false;
        }

        select_queue(self.common_cfg, 0);
        self.queue_size = read_queue_size(self.common_cfg);
        self.queue_notify_off = read_queue_notify_off(self.common_cfg);
        if self.queue_size < 1 {
            fail_device(self.common_cfg);
            return false;
        }

        let (desc, avail, used) = unsafe {
            (
                core::ptr::addr_of_mut!(RNG_DESC.0) as u64,
                core::ptr::addr_of_mut!(RNG_AVAIL) as u64,
                core::ptr::addr_of_mut!(RNG_USED) as u64,
            )
        };
        if !setup_split_queue(self.common_cfg, 0, QUEUE_SIZE as u16, 1, desc, avail, used) {
            fail_device(self.common_cfg);
            return false;
        }

        unsafe {
            self.init_queue();
        }

        write_common_status(
            self.common_cfg,
            common_status(self.common_cfg) | VIRTIO_CONFIG_STATUS_DRIVER_OK,
        );
        true
    }

    pub fn fill_bytes(&mut self, out: &mut [u8]) -> bool {
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
        let mut rng = Self::new();
        rng.bus = mapped.bus;
        rng.slot = mapped.slot;
        rng.func = mapped.func;
        rng.common_cfg = mapped.common_cfg;
        rng.notify_cfg = mapped.notify_cfg;
        rng.isr_cfg = mapped.isr_cfg;
        rng.notify_off_multiplier = mapped.notify_off_multiplier;
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

        post_split_queue_descriptor(core::ptr::addr_of_mut!(RNG_AVAIL), 0);
        self.notify_queue();

        let mut spins = 0;
        while split_queue_used_idx(core::ptr::addr_of!(RNG_USED)) == self.last_used_idx {
            spins += 1;
            if spins > VIRTIO_POLL_SPINS {
                return 0;
            }
            core::hint::spin_loop();
        }

        let elem = split_queue_used_elem(core::ptr::addr_of!(RNG_USED), self.last_used_idx);
        self.last_used_idx = self.last_used_idx.wrapping_add(1);
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
        notify_split_queue(
            self.notify_cfg,
            self.notify_off_multiplier,
            self.queue_notify_off,
            0,
        );
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
    bus: u8,
    slot: u8,
    func: u8,
    common_cfg: *mut u8,
    notify_cfg: *mut u8,
    isr_cfg: *mut u8,
    notify_off_multiplier: u32,
    tx_notify_off: u16,
    tx_queue_size: u16,
    tx_last_used_idx: u16,
}

unsafe impl Send for VirtConsole {}

impl VirtConsole {
    pub const fn new() -> Self {
        Self {
            features: 0,
            host_features: 0,
            bus: 0,
            slot: 0,
            func: 0,
            common_cfg: core::ptr::null_mut(),
            notify_cfg: core::ptr::null_mut(),
            isr_cfg: core::ptr::null_mut(),
            notify_off_multiplier: 0,
            tx_notify_off: 0,
            tx_queue_size: 0,
            tx_last_used_idx: 0,
        }
    }

    pub fn init(&mut self) -> bool {
        if self.common_cfg.is_null() || self.notify_cfg.is_null() {
            return false;
        }

        enable_pci_memory_and_bus_master(self.bus, self.slot, self.func);
        write_common_status(self.common_cfg, 0);
        write_common_status(self.common_cfg, VIRTIO_CONFIG_STATUS_ACKNOWLEDGE);
        write_common_status(
            self.common_cfg,
            VIRTIO_CONFIG_STATUS_ACKNOWLEDGE | VIRTIO_CONFIG_STATUS_DRIVER,
        );

        self.host_features = read_device_features(self.common_cfg);
        self.features = self.host_features & VIRTIO_F_VERSION_1;
        if self.features & VIRTIO_F_VERSION_1 == 0 {
            fail_device(self.common_cfg);
            return false;
        }

        write_driver_features(self.common_cfg, self.features);
        write_common_status(
            self.common_cfg,
            common_status(self.common_cfg) | VIRTIO_CONFIG_STATUS_FEATURES_OK,
        );
        if common_status(self.common_cfg) & VIRTIO_CONFIG_STATUS_FEATURES_OK == 0 {
            fail_device(self.common_cfg);
            return false;
        }

        select_queue(self.common_cfg, CONSOLE_TX_QUEUE);
        self.tx_queue_size = read_queue_size(self.common_cfg);
        self.tx_notify_off = read_queue_notify_off(self.common_cfg);
        if self.tx_queue_size < 1 {
            fail_device(self.common_cfg);
            return false;
        }

        let (desc, avail, used) = unsafe {
            (
                core::ptr::addr_of_mut!(CONSOLE_DESC.0) as u64,
                core::ptr::addr_of_mut!(CONSOLE_AVAIL) as u64,
                core::ptr::addr_of_mut!(CONSOLE_USED) as u64,
            )
        };
        if !setup_split_queue(
            self.common_cfg,
            CONSOLE_TX_QUEUE,
            QUEUE_SIZE as u16,
            1,
            desc,
            avail,
            used,
        ) {
            fail_device(self.common_cfg);
            return false;
        }

        unsafe {
            self.init_tx_queue();
        }

        write_common_status(
            self.common_cfg,
            common_status(self.common_cfg) | VIRTIO_CONFIG_STATUS_DRIVER_OK,
        );
        true
    }

    pub fn write_all(&mut self, bytes: &[u8]) -> bool {
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
        let mut console = Self::new();
        console.bus = mapped.bus;
        console.slot = mapped.slot;
        console.func = mapped.func;
        console.common_cfg = mapped.common_cfg;
        console.notify_cfg = mapped.notify_cfg;
        console.isr_cfg = mapped.isr_cfg;
        console.notify_off_multiplier = mapped.notify_off_multiplier;
        Some(console)
    }

    unsafe fn init_tx_queue(&mut self) {
        self.tx_last_used_idx = 0;
        write_volatile_u16(core::ptr::addr_of_mut!(CONSOLE_AVAIL.idx), 0);
        write_volatile_u16(core::ptr::addr_of_mut!(CONSOLE_USED.idx), 0);
        core::ptr::write_bytes(core::ptr::addr_of_mut!(CONSOLE_DATA.0) as *mut u8, 0, 256);
    }

    unsafe fn write_chunk(&mut self, bytes: &[u8]) -> usize {
        let len = core::cmp::min(bytes.len(), 256);
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

        post_split_queue_descriptor(core::ptr::addr_of_mut!(CONSOLE_AVAIL), 0);
        self.notify_tx_queue();

        let mut spins = 0;
        while split_queue_used_idx(core::ptr::addr_of!(CONSOLE_USED)) == self.tx_last_used_idx {
            spins += 1;
            if spins > VIRTIO_POLL_SPINS {
                return 0;
            }
            core::hint::spin_loop();
        }

        let elem = split_queue_used_elem(core::ptr::addr_of!(CONSOLE_USED), self.tx_last_used_idx);
        self.tx_last_used_idx = self.tx_last_used_idx.wrapping_add(1);
        if elem.id == 0 {
            len
        } else {
            0
        }
    }

    fn notify_tx_queue(&self) {
        notify_split_queue(
            self.notify_cfg,
            self.notify_off_multiplier,
            self.tx_notify_off,
            CONSOLE_TX_QUEUE,
        );
    }
}

impl Default for VirtConsole {
    fn default() -> Self {
        Self::new()
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

pub fn find_virtio_blk() -> Option<VirtBlk> {
    #[cfg(target_arch = "x86_64")]
    {
        let device = find_modern_virtio_device(VIRTIO_MODERN_DEVICE_ID_BLK)?;
        return VirtBlk::from_modern_device(device);
    }

    #[allow(unreachable_code)]
    None
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

pub fn find_virtio_console() -> Option<VirtConsole> {
    #[cfg(target_arch = "x86_64")]
    {
        let device = find_modern_virtio_device(VIRTIO_MODERN_DEVICE_ID_CONSOLE)?;
        return VirtConsole::from_modern_device(device);
    }

    #[allow(unreachable_code)]
    None
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
fn port_inl(port: u16) -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!("in eax, dx", out("eax") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    value
}

#[inline]
fn port_outl(port: u16, value: u32) {
    unsafe {
        core::arch::asm!("out dx, eax", in("dx") port, in("eax") value, options(nomem, nostack, preserves_flags));
    }
}
