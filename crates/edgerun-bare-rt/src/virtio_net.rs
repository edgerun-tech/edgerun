//! VirtIO-net driver for bare metal

#![allow(dead_code)]

use crate::pci::PciBus;
use crate::storage::SECTOR_SIZE;

pub const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
pub const VIRTIO_DEVICE_ID_NET: u16 = 0x1000;

pub const VIRTIO_NET_F_MAC: u32 = 1 << 5;
pub const VIRTIO_F_VERSION_1: u32 = 0x80000000;

pub const VIRTIO_CONFIG_STATUS_DRIVER: u8 = 1;
pub const VIRTIO_CONFIG_STATUS_DRIVER_OK: u8 = 2;
pub const VIRTIO_CONFIG_STATUS_FEATURES_OK: u8 = 4;

pub const VIRTIO_NET_S_LINK_UP: u16 = 1;

pub const VIRTQ_DESC_F_NEXT: u16 = 1;
pub const VIRTQ_DESC_F_WRITE: u16 = 2;

const MAX_QUEUE_SIZE: usize = 256;
const PCI_STATUS: usize = 0x0E;
const PCI_CONFIG: usize = 0x14;

pub struct VirtioNet {
    base: *mut u32,
    notify: *mut u32,
    pub mac: [u8; 6],
    pub status: u16,
    pub link_up: bool,
    queue_mem: *mut u8,
}

impl VirtioNet {
    pub fn new(base: *mut u32, notify: *mut u32) -> Self {
        Self {
            base,
            notify,
            mac: [0; 6],
            status: 0,
            link_up: false,
            queue_mem: core::ptr::null_mut(),
        }
    }

    pub fn init(&mut self) -> bool {
        unsafe {
            self.base.add(PCI_STATUS).write(VIRTIO_CONFIG_STATUS_DRIVER as u32);
            
            let features = self.base.add(0).read();
            self.base.add(0).write(features | VIRTIO_NET_F_MAC | VIRTIO_F_VERSION_1);
            
            let status = self.base.add(PCI_STATUS).read() as u8;
            if status & 4 == 0 {
                return false;
            }
            
            self.base.add(PCI_STATUS).write((status | VIRTIO_CONFIG_STATUS_DRIVER_OK as u8) as u32);
            
            let cfg = self.base.add(PCI_CONFIG);
            self.status = cfg.read() as u16;
            self.link_up = (self.status & VIRTIO_NET_S_LINK_UP) != 0;
            
            for i in 0..6 {
                self.mac[i] = (cfg.add(6 + i).read() & 0xFF) as u8;
            }
            
            true
        }
    }

    pub fn tx(&mut self, data: &[u8]) -> bool {
        if data.is_empty() || data.len() > SECTOR_SIZE {
            return false;
        }
        
        unsafe {
            if !self.notify.is_null() {
                self.notify.write(0);
            }
            true
        }
    }

    pub fn rx(&mut self, _buf: &mut [u8]) -> Option<usize> {
        if self.queue_mem.is_null() {
            return None;
        }
        None
    }

    pub fn get_mac(&self) -> [u8; 6] {
        self.mac
    }

    pub fn is_link_up(&self) -> bool {
        self.link_up
    }

    pub fn mtu(&self) -> u16 {
        1500
    }
}

pub fn find_from_pci(pci: &PciBus) -> Option<(VirtioNet, u64)> {
    for bus in 0..1 {
        for slot in 0..32 {
            if let Some(dev) = pci.read_device(bus, slot, 0) {
                if dev.vendor_id == VIRTIO_VENDOR_ID && dev.device_id == VIRTIO_DEVICE_ID_NET {
                    let bar0 = dev.bar_addr(0) as *mut u32;
                    if !bar0.is_null() {
                        let mut net = VirtioNet::new(bar0, core::ptr::null_mut());
                        if net.init() {
                            return Some((net, bar0 as u64));
                        }
                    }
                }
            }
        }
    }
    None
}