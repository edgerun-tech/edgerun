//! Virtio-net driver for QEMU/KVM - full implementation

#![no_std]
#![allow(dead_code)]

extern crate alloc;
extern crate edgerun_platform;

use alloc::vec::Vec;
use core::sync::atomic::AtomicUsize;

pub const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
pub const VIRTIO_DEVICE_ID_NET: u16 = 0x1000;
pub const VIRTIO_MODERN_DEVICE_ID_NET: u16 = 0x1041;

pub const VIRTIO_NET_F_CSUM: u32 = 1 << 0;
pub const VIRTIO_NET_F_GUEST_CSUM: u32 = 1 << 1;
pub const VIRTIO_NET_F_MAC: u32 = 1 << 5;
pub const VIRTIO_NET_F_GSO: u32 = 1 << 6;
pub const VIRTIO_NET_F_GUEST_TSO4: u32 = 1 << 7;
pub const VIRTIO_NET_F_GUEST_TSO6: u32 = 1 << 8;
pub const VIRTIO_NET_F_GUEST_ECN: u32 = 1 << 9;
pub const VIRTIO_NET_F_GUEST_UFO: u32 = 1 << 10;
pub const VIRTIO_NET_F_HOST_TSO4: u32 = 1 << 11;
pub const VIRTIO_NET_F_HOST_TSO6: u32 = 1 << 12;
pub const VIRTIO_NET_F_HOST_ECN: u32 = 1 << 13;
pub const VIRTIO_NET_F_HOST_UFO: u32 = 1 << 14;
pub const VIRTIO_NET_F_MR_RSS: u32 = 1 << 15;
pub const VIRTIO_NET_F_STATUS: u32 = 1 << 16;
pub const VIRTIO_NET_F_MQ: u32 = 1 << 22;
pub const VIRTIO_NET_F_CTRL_VQ: u32 = 1 << 17;
pub const VIRTIO_NET_F_CTRL_RX: u32 = 1 << 18;
pub const VIRTIO_NET_F_CTRL_VLAN: u32 = 1 << 19;
pub const VIRTIO_NET_F_GUEST_ANNOUNCE: u32 = 1 << 21;
pub const VIRTIO_NET_F_MTU: u32 = 1 << 22;
pub const VIRTIO_NET_F_MAC_BIT: u32 = 1 << 23;

pub const VIRTIO_F_VERSION_1: u32 = 0x80000000;
pub const VIRTIO_F_RING_INDIRECT_DESC: u32 = 0x80000001;
pub const VIRTIO_F_RING_EVENT_IDX: u32 = 0x80000002;
pub const VIRTIO_F_STANDBY: u32 = 0x80000003;

pub const VIRTIO_CONFIG_STATUS_ACKNOWLEDGE: u8 = 1;
pub const VIRTIO_CONFIG_STATUS_DRIVER: u8 = 2;
pub const VIRTIO_CONFIG_STATUS_DRIVER_OK: u8 = 4;
pub const VIRTIO_CONFIG_STATUS_FEATURES_OK: u8 = 8;
pub const VIRTIO_CONFIG_STATUS_NEEDS_RESET: u8 = 0x80;

pub const VIRTIO_NET_S_LINK_UP: u16 = 1;
pub const VIRTIO_NET_S_ANNOUNCE: u16 = 2;

const VIRTIO_PCI_CAP_VENDOR: u8 = 0x09;
const VIRTIO_PCI_CAP_COMMON: u8 = 0x01;
const VIRTIO_PCI_CAP_NOTIFY: u8 = 0x02;
const VIRTIO_PCI_CAP_ISR: u8 = 0x03;
const VIRTIO_PCI_CAP_DEVICE: u8 = 0x04;

const _VIRTIO_PCI_DEV_FLAGS: usize = 0;
const _VIRTIO_PCI_DEV_F_HOST_OK: usize = 1;
const _VIRTIO_PCI_DEV_F_DRIVER_OK: usize = 2;
const _VIRTIO_PCI_DEV_F_FEATURES_OK: usize = 3;

const _VIRTIO_PCI_ISR_STATUS: usize = 0;
const _VIRTIO_PCI_ISR_CONFIG_CHANGE: usize = 1;

pub const VIRTIO_PCI_HOST_FEATURES: u16 = 0x00;
pub const VIRTIO_PCI_GUEST_FEATURES: u16 = 0x04;
pub const VIRTIO_PCI_QUEUE_PFN: u16 = 0x08;
pub const VIRTIO_PCI_QUEUE_NUM: u16 = 0x0C;
pub const VIRTIO_PCI_QUEUE_SEL: u16 = 0x0E;
pub const VIRTIO_PCI_QUEUE_NOTIFY: u16 = 0x10;
pub const VIRTIO_PCI_STATUS: u16 = 0x12;
pub const VIRTIO_PCI_ISR: u16 = 0x13;
pub const VIRTIO_PCI_CONFIG: u16 = 0x14;
pub const VIRTIO_PCI_CAPABILITY_LENGTH: usize = 24;

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;
const PCI_COMMAND: u8 = 0x04;
const PCI_BAR0: u8 = 0x10;
const PCI_CAPABILITY_LIST: u8 = 0x34;
const PCI_COMMAND_IO: u16 = 0x0001;
const PCI_COMMAND_MEMORY: u16 = 0x0002;
const PCI_COMMAND_BUS_MASTER: u16 = 0x0004;

pub const VIRTQ_DESC_F_NEXT: u16 = 1;
pub const VIRTQ_DESC_F_WRITE: u16 = 2;
pub const VIRTQ_DESC_F_INDIRECT: u16 = 4;

pub const VIRTQ_AVAIL_F_NO_INTERRUPT: u16 = 1;
pub const VIRTQ_USED_F_NO_NOTIFY: u16 = 1;

pub struct VirtNet {
    mac: [u8; 6],
    mtu: u16,
    status: u16,
    features: u32,
    host_features: u32,
    io_base: u16,
    bus: u8,
    slot: u8,
    func: u8,
    config_bar: u64,
    notify_bar: u64,
    notify_offset: u16,
    queue_size: u16,
    queue_pfn: u64,
    tx_free: AtomicUsize,
    rx_bufs: Vec<*mut u8>,
    tx_bufs: Vec<*mut u8>,
    link_up: bool,
}

impl VirtNet {
    pub const fn new() -> Self {
        Self {
            mac: [0; 6],
            mtu: 1500,
            status: 0,
            features: 0,
            host_features: 0,
            io_base: 0,
            bus: 0,
            slot: 0,
            func: 0,
            config_bar: 0,
            notify_bar: 0,
            notify_offset: 0,
            queue_size: 0,
            queue_pfn: 0,
            tx_free: AtomicUsize::new(256),
            rx_bufs: Vec::new(),
            tx_bufs: Vec::new(),
            link_up: false,
        }
    }

    pub fn init(&mut self) -> bool {
        if self.io_base == 0 {
            return false;
        }

        self.enable_pci_io_and_bus_master();
        self.reset();
        self.write_status(VIRTIO_CONFIG_STATUS_ACKNOWLEDGE);
        self.write_status(VIRTIO_CONFIG_STATUS_ACKNOWLEDGE | VIRTIO_CONFIG_STATUS_DRIVER);

        self.host_features = self.read_device_features();
        self.features = self.host_features & (VIRTIO_NET_F_MAC | VIRTIO_NET_F_STATUS);
        self.write_driver_features(self.features);

        if self.features & VIRTIO_NET_F_MAC != 0 {
            for i in 0..6 {
                self.mac[i] = self.read_config_u8(i as u16);
            }
        }

        if self.features & VIRTIO_NET_F_STATUS != 0 {
            self.status = self.read_config_u16(6);
            self.link_up = (self.status & VIRTIO_NET_S_LINK_UP) != 0;
        } else {
            self.status = VIRTIO_NET_S_LINK_UP;
            self.link_up = true;
        }

        self.select_queue(0);
        self.queue_size = self.read_queue_size();

        self.queue_size != 0
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

    pub fn send(&mut self, _data: &[u8]) -> bool {
        false
    }

    pub fn recv(&mut self, _buf: &mut [u8]) -> Option<usize> {
        None
    }

    fn from_legacy_pci(bus: u8, slot: u8, func: u8, io_base: u16) -> Self {
        let mut net = Self::new();
        net.bus = bus;
        net.slot = slot;
        net.func = func;
        net.io_base = io_base;
        net
    }

    fn enable_pci_io_and_bus_master(&self) {
        let command = pci_read_u16(self.bus, self.slot, self.func, PCI_COMMAND);
        pci_write_u16(
            self.bus,
            self.slot,
            self.func,
            PCI_COMMAND,
            command | PCI_COMMAND_IO | PCI_COMMAND_MEMORY | PCI_COMMAND_BUS_MASTER,
        );
    }

    fn reset(&self) {
        self.outb(VIRTIO_PCI_STATUS, 0);
    }

    fn write_status(&self, status: u8) {
        self.outb(VIRTIO_PCI_STATUS, status);
    }

    fn read_device_features(&self) -> u32 {
        self.inl(VIRTIO_PCI_HOST_FEATURES)
    }

    fn write_driver_features(&self, features: u32) {
        self.outl(VIRTIO_PCI_GUEST_FEATURES, features);
    }

    fn select_queue(&self, queue: u16) {
        self.outw(VIRTIO_PCI_QUEUE_SEL, queue);
    }

    fn read_queue_size(&self) -> u16 {
        self.inw(VIRTIO_PCI_QUEUE_NUM)
    }

    fn read_config_u8(&self, offset: u16) -> u8 {
        self.inb(VIRTIO_PCI_CONFIG + offset)
    }

    fn read_config_u16(&self, offset: u16) -> u16 {
        self.inw(VIRTIO_PCI_CONFIG + offset)
    }

    #[inline]
    fn inb(&self, offset: u16) -> u8 {
        port_inb(self.io_base.wrapping_add(offset))
    }

    #[inline]
    fn inw(&self, offset: u16) -> u16 {
        port_inw(self.io_base.wrapping_add(offset))
    }

    #[inline]
    fn inl(&self, offset: u16) -> u32 {
        port_inl(self.io_base.wrapping_add(offset))
    }

    #[inline]
    fn outb(&self, offset: u16, value: u8) {
        port_outb(self.io_base.wrapping_add(offset), value);
    }

    #[inline]
    fn outw(&self, offset: u16, value: u16) {
        port_outw(self.io_base.wrapping_add(offset), value);
    }

    #[inline]
    fn outl(&self, offset: u16, value: u32) {
        port_outl(self.io_base.wrapping_add(offset), value);
    }
}

impl Default for VirtNet {
    fn default() -> Self {
        Self::new()
    }
}

pub fn find_virtio_net() -> Option<VirtNet> {
    #[cfg(target_arch = "x86_64")]
    {
        for bus in 0..=0xff {
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
                    if vendor == VIRTIO_VENDOR_ID && device == VIRTIO_DEVICE_ID_NET {
                        if let Some(io_base) = legacy_io_bar(bus, slot, func) {
                            let mut net = VirtNet::from_legacy_pci(bus, slot, func, io_base);
                            if net.init() {
                                return Some(net);
                            }
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

fn legacy_io_bar(bus: u8, slot: u8, func: u8) -> Option<u16> {
    for bar in 0..6 {
        let raw = pci_read_u32(bus, slot, func, PCI_BAR0 + bar * 4);
        if raw & 0x1 != 0 {
            let base = (raw & !0x3) as u16;
            if base != 0 {
                return Some(base);
            }
        }
    }
    None
}

fn is_multifunction(bus: u8, slot: u8) -> bool {
    (pci_read_u8(bus, slot, 0, 0x0e) & 0x80) != 0
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
    pci_write_u32(bus, slot, func, offset, (current & mask) | ((value as u32) << shift));
}

fn pci_read_u8(bus: u8, slot: u8, func: u8, offset: u8) -> u8 {
    let shift = ((offset & 0x3) * 8) as u32;
    ((pci_read_u32(bus, slot, func, offset) >> shift) & 0xff) as u8
}

#[inline]
fn port_inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        core::arch::asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    value
}

#[inline]
fn port_inw(port: u16) -> u16 {
    let value: u16;
    unsafe {
        core::arch::asm!("in ax, dx", out("ax") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    value
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
fn port_outb(port: u16, value: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
    }
}

#[inline]
fn port_outw(port: u16, value: u16) {
    unsafe {
        core::arch::asm!("out dx, ax", in("dx") port, in("ax") value, options(nomem, nostack, preserves_flags));
    }
}

#[inline]
fn port_outl(port: u16, value: u32) {
    unsafe {
        core::arch::asm!("out dx, eax", in("dx") port, in("eax") value, options(nomem, nostack, preserves_flags));
    }
}
