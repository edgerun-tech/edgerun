//! Realtek RTL8125 2.5G NIC driver.
//!
//! This is a small polling driver for the bare-metal Edgerun path. It programs
//! one normal-priority TX ring and one RX ring, disables interrupts, and exposes
//! the same `init`/`send`/`recv` shape as the virtio-net driver.

#![no_std]

use core::sync::atomic::{fence, Ordering};

pub const VENDOR_ID: u16 = 0x10ec;
pub const DEVICE_ID: u16 = 0x8125;

pub const REG_MAC0: u16 = 0x0000;
pub const REG_TX_DESC_START_LOW: u16 = 0x0020;
pub const REG_TX_DESC_START_HIGH: u16 = 0x0024;
pub const REG_CMD: u16 = 0x0037;
pub const REG_INTR_MASK_8125: u16 = 0x0038;
pub const REG_INTR_STATUS_8125: u16 = 0x003c;
pub const REG_TCR: u16 = 0x0040;
pub const REG_RCR: u16 = 0x0044;
pub const REG_PHY_STATUS: u16 = 0x006c;
pub const REG_TX_POLL_8125: u16 = 0x0090;
pub const REG_RX_MAX_SIZE: u16 = 0x00da;
pub const REG_CPLUS_CMD: u16 = 0x00e0;
pub const REG_RX_DESC_ADDR_LOW: u16 = 0x00e4;
pub const REG_RX_DESC_ADDR_HIGH: u16 = 0x00e8;

pub const CMD_RX_EN: u8 = 0x08;
pub const CMD_TX_EN: u8 = 0x04;
pub const CMD_RESET: u8 = 0x10;

const TX_POLL_NORMAL_PRIORITY: u8 = 0x40;
const TX_DMA_BURST_UNLIMITED: u32 = 7 << 8;
const TX_INTERFRAME_GAP: u32 = 3 << 24;
const RX_DMA_BURST_UNLIMITED: u32 = 7 << 8;
const RX_FIFO_THRESHOLD_NONE: u32 = 7 << 13;

const CPLUS_RX_CHECKSUM: u16 = 1 << 5;
const CPLUS_PCI_MULTI_RW: u16 = 1 << 3;

pub const RCR_ACCEPT_PHYS_MATCH: u32 = 0x0000_0002;
pub const RCR_ACCEPT_BROADCAST: u32 = 0x0000_0008;
pub const RCR_ACCEPT_MULTICAST: u32 = 0x0000_0004;
pub const RCR_ACCEPT_ALL_PHYS: u32 = 0x0000_0001;

const PHY_STATUS_LINK_UP: u8 = 0x02;

pub const RX_BUF_SIZE: usize = 2048;
pub const TX_BUF_SIZE: usize = 2048;
pub const NUM_TX_DESC: usize = 32;
pub const NUM_RX_DESC: usize = 32;
pub const ETHERNET_FCS_LEN: usize = 4;
pub const MTU: u16 = 1500;

const DESC_OWN: u32 = 1 << 31;
const DESC_RING_END: u32 = 1 << 30;
const DESC_FIRST_FRAG: u32 = 1 << 29;
const DESC_LAST_FRAG: u32 = 1 << 28;
const DESC_LEN_MASK: u32 = 0x3fff;
const RX_ERROR_MASK: u32 = (1 << 22) | (1 << 21) | (1 << 20) | (1 << 19);

const PCI_CONFIG_ADDRESS: u16 = 0x0cf8;
const PCI_CONFIG_DATA: u16 = 0x0cfc;
const PCI_COMMAND: u8 = 0x04;
const PCI_HEADER_TYPE: u8 = 0x0e;
const PCI_BAR0: u8 = 0x10;
const PCI_COMMAND_MEMORY: u16 = 0x0002;
const PCI_COMMAND_BUS_MASTER: u16 = 0x0004;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RtlDesc {
    pub opts1: u32,
    pub opts2: u32,
    pub addr: u64,
}

const _: [(); 16] = [(); core::mem::size_of::<RtlDesc>()];

const EMPTY_DESC: RtlDesc = RtlDesc {
    opts1: 0,
    opts2: 0,
    addr: 0,
};

#[repr(C, align(256))]
struct TxDescRing([RtlDesc; NUM_TX_DESC]);

#[repr(C, align(256))]
struct RxDescRing([RtlDesc; NUM_RX_DESC]);

#[repr(C, align(16))]
struct TxBuffers([[u8; TX_BUF_SIZE]; NUM_TX_DESC]);

#[repr(C, align(16))]
struct RxBuffers([[u8; RX_BUF_SIZE]; NUM_RX_DESC]);

static mut TX_DESC: TxDescRing = TxDescRing([EMPTY_DESC; NUM_TX_DESC]);
static mut RX_DESC: RxDescRing = RxDescRing([EMPTY_DESC; NUM_RX_DESC]);
static mut TX_BUFFERS: TxBuffers = TxBuffers([[0; TX_BUF_SIZE]; NUM_TX_DESC]);
static mut RX_BUFFERS: RxBuffers = RxBuffers([[0; RX_BUF_SIZE]; NUM_RX_DESC]);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rtl8125Stats {
    pub tx_submitted: u32,
    pub tx_completed: u32,
    pub tx_dropped: u32,
    pub rx_received: u32,
    pub rx_dropped: u32,
    pub rx_errors: u32,
}

pub struct Rtl8125 {
    mmio_base: usize,
    bus: u8,
    slot: u8,
    func: u8,
    mac: [u8; 6],
    tx_cur: usize,
    tx_dirty: usize,
    rx_cur: usize,
    tx_submitted: u32,
    tx_completed: u32,
    tx_dropped: u32,
    rx_received: u32,
    rx_dropped: u32,
    rx_errors: u32,
}

unsafe impl Send for Rtl8125 {}

impl Rtl8125 {
    pub const fn new(mmio_base: usize) -> Self {
        Self {
            mmio_base,
            bus: 0,
            slot: 0,
            func: 0,
            mac: [0; 6],
            tx_cur: 0,
            tx_dirty: 0,
            rx_cur: 0,
            tx_submitted: 0,
            tx_completed: 0,
            tx_dropped: 0,
            rx_received: 0,
            rx_dropped: 0,
            rx_errors: 0,
        }
    }

    pub const fn with_pci_location(mut self, bus: u8, slot: u8, func: u8) -> Self {
        self.bus = bus;
        self.slot = slot;
        self.func = func;
        self
    }

    pub fn init(&mut self) -> bool {
        if self.mmio_base == 0 {
            return false;
        }

        enable_pci_memory_and_bus_master(self.bus, self.slot, self.func);
        self.disable_interrupts();
        if !self.reset() {
            return false;
        }

        self.mac = self.read_mac();

        unsafe {
            self.init_rings();
        }

        self.write16(REG_RX_MAX_SIZE, RX_BUF_SIZE as u16 + 1);
        self.write16(REG_CPLUS_CMD, CPLUS_PCI_MULTI_RW | CPLUS_RX_CHECKSUM);
        self.write32(REG_TCR, TX_DMA_BURST_UNLIMITED | TX_INTERFRAME_GAP);
        self.write32(
            REG_RCR,
            RX_DMA_BURST_UNLIMITED
                | RX_FIFO_THRESHOLD_NONE
                | RCR_ACCEPT_PHYS_MATCH
                | RCR_ACCEPT_BROADCAST
                | RCR_ACCEPT_MULTICAST,
        );

        unsafe {
            let tx_addr = core::ptr::addr_of!(TX_DESC.0) as u64;
            let rx_addr = core::ptr::addr_of!(RX_DESC.0) as u64;
            self.write32(REG_TX_DESC_START_HIGH, (tx_addr >> 32) as u32);
            self.write32(REG_TX_DESC_START_LOW, tx_addr as u32);
            self.write32(REG_RX_DESC_ADDR_HIGH, (rx_addr >> 32) as u32);
            self.write32(REG_RX_DESC_ADDR_LOW, rx_addr as u32);
        }

        fence(Ordering::SeqCst);
        self.write32(REG_INTR_STATUS_8125, 0xffff_ffff);
        self.write8(REG_CMD, CMD_RX_EN | CMD_TX_EN);
        self.kick_tx();
        true
    }

    pub fn reset(&self) -> bool {
        self.write8(REG_CMD, CMD_RESET);
        for _ in 0..100_000 {
            if self.read8(REG_CMD) & CMD_RESET == 0 {
                return true;
            }
            core::hint::spin_loop();
        }
        false
    }

    pub fn get_mac(&self) -> [u8; 6] {
        self.mac
    }

    pub fn is_link_up(&self) -> bool {
        self.read8(REG_PHY_STATUS) & PHY_STATUS_LINK_UP != 0
    }

    pub fn mtu(&self) -> u16 {
        MTU
    }

    pub fn stats(&mut self) -> Rtl8125Stats {
        self.reap_tx();
        Rtl8125Stats {
            tx_submitted: self.tx_submitted,
            tx_completed: self.tx_completed,
            tx_dropped: self.tx_dropped,
            rx_received: self.rx_received,
            rx_dropped: self.rx_dropped,
            rx_errors: self.rx_errors,
        }
    }

    pub fn send(&mut self, data: &[u8]) -> bool {
        if data.is_empty() || data.len() > TX_BUF_SIZE || data.len() > DESC_LEN_MASK as usize {
            self.tx_dropped = self.tx_dropped.wrapping_add(1);
            return false;
        }

        self.reap_tx();
        if self.tx_cur.wrapping_sub(self.tx_dirty) >= NUM_TX_DESC {
            self.tx_dropped = self.tx_dropped.wrapping_add(1);
            return false;
        }

        let entry = self.tx_cur % NUM_TX_DESC;
        unsafe {
            let desc = descriptor_mut(core::ptr::addr_of_mut!(TX_DESC.0) as *mut RtlDesc, entry);
            if read_desc_opts1(desc) & DESC_OWN != 0 {
                self.tx_dropped = self.tx_dropped.wrapping_add(1);
                return false;
            }

            let buffer = (core::ptr::addr_of_mut!(TX_BUFFERS.0) as *mut [u8; TX_BUF_SIZE])
                .add(entry) as *mut u8;
            core::ptr::copy_nonoverlapping(data.as_ptr(), buffer, data.len());

            write_desc(
                desc,
                RtlDesc {
                    opts1: tx_opts1(entry, data.len()),
                    opts2: 0,
                    addr: buffer as u64,
                },
            );
        }

        fence(Ordering::SeqCst);
        self.tx_cur = self.tx_cur.wrapping_add(1);
        self.tx_submitted = self.tx_submitted.wrapping_add(1);
        self.kick_tx();
        true
    }

    pub fn recv(&mut self, buf: &mut [u8]) -> Option<usize> {
        let entry = self.rx_cur % NUM_RX_DESC;

        unsafe {
            let desc = descriptor_mut(core::ptr::addr_of_mut!(RX_DESC.0) as *mut RtlDesc, entry);
            let status = read_desc_opts1(desc);
            if status & DESC_OWN != 0 {
                return None;
            }
            fence(Ordering::Acquire);

            let len = rx_payload_len(status);
            let complete_frame =
                status & (DESC_FIRST_FRAG | DESC_LAST_FRAG) == (DESC_FIRST_FRAG | DESC_LAST_FRAG);
            let has_error = status & RX_ERROR_MASK != 0;
            let copied = if !complete_frame || has_error || len == 0 {
                self.rx_errors = self.rx_errors.wrapping_add(1);
                0
            } else {
                let copied = core::cmp::min(len, buf.len());
                let src = (core::ptr::addr_of!(RX_BUFFERS.0) as *const [u8; RX_BUF_SIZE]).add(entry)
                    as *const u8;
                core::ptr::copy_nonoverlapping(src, buf.as_mut_ptr(), copied);
                self.rx_received = self.rx_received.wrapping_add(1);
                copied
            };

            self.release_rx_desc(entry);
            self.rx_cur = (self.rx_cur + 1) % NUM_RX_DESC;

            if copied == 0 {
                self.rx_dropped = self.rx_dropped.wrapping_add(1);
                None
            } else {
                Some(copied)
            }
        }
    }

    unsafe fn init_rings(&mut self) {
        self.tx_cur = 0;
        self.tx_dirty = 0;
        self.rx_cur = 0;
        self.tx_submitted = 0;
        self.tx_completed = 0;
        self.tx_dropped = 0;
        self.rx_received = 0;
        self.rx_dropped = 0;
        self.rx_errors = 0;

        core::ptr::write_bytes(
            core::ptr::addr_of_mut!(TX_BUFFERS.0) as *mut u8,
            0,
            NUM_TX_DESC * TX_BUF_SIZE,
        );
        core::ptr::write_bytes(
            core::ptr::addr_of_mut!(RX_BUFFERS.0) as *mut u8,
            0,
            NUM_RX_DESC * RX_BUF_SIZE,
        );

        for i in 0..NUM_TX_DESC {
            let desc = descriptor_mut(core::ptr::addr_of_mut!(TX_DESC.0) as *mut RtlDesc, i);
            let buffer =
                (core::ptr::addr_of_mut!(TX_BUFFERS.0) as *mut [u8; TX_BUF_SIZE]).add(i) as *mut u8;
            write_desc(
                desc,
                RtlDesc {
                    opts1: if i == NUM_TX_DESC - 1 {
                        DESC_RING_END
                    } else {
                        0
                    },
                    opts2: 0,
                    addr: buffer as u64,
                },
            );
        }

        for i in 0..NUM_RX_DESC {
            let buffer =
                (core::ptr::addr_of_mut!(RX_BUFFERS.0) as *mut [u8; RX_BUF_SIZE]).add(i) as *mut u8;
            write_desc(
                descriptor_mut(core::ptr::addr_of_mut!(RX_DESC.0) as *mut RtlDesc, i),
                RtlDesc {
                    opts1: rx_owned_opts1(i),
                    opts2: 0,
                    addr: buffer as u64,
                },
            );
        }
        fence(Ordering::SeqCst);
    }

    unsafe fn release_rx_desc(&self, entry: usize) {
        let desc = descriptor_mut(core::ptr::addr_of_mut!(RX_DESC.0) as *mut RtlDesc, entry);
        let addr = read_desc_addr(desc);
        write_desc(
            desc,
            RtlDesc {
                opts1: rx_owned_opts1(entry),
                opts2: 0,
                addr,
            },
        );
        fence(Ordering::Release);
    }

    fn reap_tx(&mut self) {
        while self.tx_dirty != self.tx_cur {
            let entry = self.tx_dirty % NUM_TX_DESC;
            let owned = unsafe {
                read_desc_opts1(descriptor_mut(
                    core::ptr::addr_of_mut!(TX_DESC.0) as *mut RtlDesc,
                    entry,
                )) & DESC_OWN
                    != 0
            };
            if owned {
                break;
            }
            self.tx_dirty = self.tx_dirty.wrapping_add(1);
            self.tx_completed = self.tx_completed.wrapping_add(1);
        }
    }

    fn kick_tx(&self) {
        self.write8(REG_TX_POLL_8125, TX_POLL_NORMAL_PRIORITY);
    }

    fn disable_interrupts(&self) {
        self.write32(REG_INTR_MASK_8125, 0);
        self.write32(REG_INTR_STATUS_8125, 0xffff_ffff);
    }

    fn read_mac(&self) -> [u8; 6] {
        [
            self.read8(REG_MAC0),
            self.read8(REG_MAC0 + 1),
            self.read8(REG_MAC0 + 2),
            self.read8(REG_MAC0 + 3),
            self.read8(REG_MAC0 + 4),
            self.read8(REG_MAC0 + 5),
        ]
    }

    #[inline]
    fn read8(&self, reg: u16) -> u8 {
        let addr = (self.mmio_base + reg as usize) as *const u8;
        unsafe { addr.read_volatile() }
    }

    #[inline]
    fn write8(&self, reg: u16, val: u8) {
        let addr = (self.mmio_base + reg as usize) as *mut u8;
        unsafe { addr.write_volatile(val) }
    }

    #[inline]
    fn write16(&self, reg: u16, val: u16) {
        let addr = (self.mmio_base + reg as usize) as *mut u16;
        unsafe { addr.write_volatile(val) }
    }

    #[inline]
    fn write32(&self, reg: u16, val: u32) {
        let addr = (self.mmio_base + reg as usize) as *mut u32;
        unsafe { addr.write_volatile(val) }
    }
}

impl Default for Rtl8125 {
    fn default() -> Self {
        Self::new(0)
    }
}

pub fn find_rtl8125() -> Option<Rtl8125> {
    for bus in 0..=255 {
        for slot in 0..32 {
            let functions = if pci_read_u8(bus, slot, 0, PCI_HEADER_TYPE) & 0x80 != 0 {
                8
            } else {
                1
            };

            for func in 0..functions {
                if pci_read_u16(bus, slot, func, 0x00) != VENDOR_ID
                    || pci_read_u16(bus, slot, func, 0x02) != DEVICE_ID
                {
                    continue;
                }

                let mmio_base = find_memory_bar(bus, slot, func)?;
                return Some(Rtl8125::new(mmio_base).with_pci_location(bus, slot, func));
            }
        }
    }
    None
}

fn tx_opts1(entry: usize, len: usize) -> u32 {
    DESC_OWN
        | DESC_FIRST_FRAG
        | DESC_LAST_FRAG
        | if entry == NUM_TX_DESC - 1 {
            DESC_RING_END
        } else {
            0
        }
        | len as u32
}

fn rx_owned_opts1(entry: usize) -> u32 {
    DESC_OWN
        | if entry == NUM_RX_DESC - 1 {
            DESC_RING_END
        } else {
            0
        }
        | RX_BUF_SIZE as u32
}

fn rx_payload_len(status: u32) -> usize {
    ((status & DESC_LEN_MASK) as usize).saturating_sub(ETHERNET_FCS_LEN)
}

unsafe fn descriptor_mut(base: *mut RtlDesc, entry: usize) -> *mut RtlDesc {
    base.add(entry)
}

unsafe fn read_desc_opts1(desc: *const RtlDesc) -> u32 {
    core::ptr::addr_of!((*desc).opts1).read_volatile()
}

unsafe fn read_desc_addr(desc: *const RtlDesc) -> u64 {
    core::ptr::addr_of!((*desc).addr).read_volatile()
}

unsafe fn write_desc(desc: *mut RtlDesc, value: RtlDesc) {
    core::ptr::write_volatile(desc, value);
}

fn find_memory_bar(bus: u8, slot: u8, func: u8) -> Option<usize> {
    let mut offset = PCI_BAR0;
    while offset <= PCI_BAR0 + 5 * 4 {
        let bar = pci_read_u32(bus, slot, func, offset);
        if bar != 0 && bar != u32::MAX && bar & 0x1 == 0 {
            let base = (bar & 0xffff_fff0) as usize;
            if base != 0 {
                return Some(base);
            }
        }

        offset += if bar & 0x6 == 0x4 { 8 } else { 4 };
    }
    None
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

fn pci_address(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    0x8000_0000
        | ((bus as u32) << 16)
        | ((slot as u32) << 11)
        | ((func as u32) << 8)
        | ((offset as u32) & 0xfc)
}

fn pci_read_u32(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    outl(PCI_CONFIG_ADDRESS, pci_address(bus, slot, func, offset));
    inl(PCI_CONFIG_DATA)
}

fn pci_read_u16(bus: u8, slot: u8, func: u8, offset: u8) -> u16 {
    let value = pci_read_u32(bus, slot, func, offset);
    ((value >> ((offset & 2) * 8)) & 0xffff) as u16
}

fn pci_read_u8(bus: u8, slot: u8, func: u8, offset: u8) -> u8 {
    let value = pci_read_u32(bus, slot, func, offset);
    ((value >> ((offset & 3) * 8)) & 0xff) as u8
}

fn pci_write_u16(bus: u8, slot: u8, func: u8, offset: u8, value: u16) {
    let shift = ((offset & 2) * 8) as u32;
    let mask = !(0xffffu32 << shift);
    let current = pci_read_u32(bus, slot, func, offset);
    let next = (current & mask) | ((value as u32) << shift);
    outl(PCI_CONFIG_ADDRESS, pci_address(bus, slot, func, offset));
    outl(PCI_CONFIG_DATA, next);
}

#[inline]
fn outl(port: u16, value: u32) {
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

#[inline]
fn inl(port: u16) -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!(
            "in eax, dx",
            out("eax") value,
            in("dx") port,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_layout_matches_hardware() {
        assert_eq!(core::mem::size_of::<RtlDesc>(), 16);
        assert_eq!(core::mem::align_of::<TxDescRing>(), 256);
        assert_eq!(core::mem::align_of::<RxDescRing>(), 256);
    }

    #[test]
    fn tx_opts_sets_ownership_fragments_and_ring_end() {
        assert_eq!(
            tx_opts1(0, 60),
            DESC_OWN | DESC_FIRST_FRAG | DESC_LAST_FRAG | 60
        );
        assert_eq!(
            tx_opts1(NUM_TX_DESC - 1, 1514),
            DESC_OWN | DESC_FIRST_FRAG | DESC_LAST_FRAG | DESC_RING_END | 1514
        );
    }

    #[test]
    fn rx_payload_len_strips_fcs() {
        assert_eq!(rx_payload_len(64), 60);
        assert_eq!(rx_payload_len(2), 0);
    }

    #[test]
    fn pci_config_address_is_type_one_address() {
        assert_eq!(pci_address(1, 2, 3, 0x10), 0x8001_1310);
    }
}
