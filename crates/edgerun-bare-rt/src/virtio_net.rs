//! VirtIO-net driver for bare metal

#![allow(dead_code)]

use crate::pci::PciBus;

pub const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
pub const VIRTIO_DEVICE_ID_NET: u16 = 0x1000;

pub const VIRTIO_NET_F_MAC: u32 = 1 << 5;
pub const VIRTIO_F_VERSION_1: u32 = 0x80000000;

pub const VIRTIO_CONFIG_STATUS_DRIVER: u8 = 1;
pub const VIRTIO_CONFIG_STATUS_DRIVER_OK: u8 = 2;
pub const VIRTIO_CONFIG_STATUS_FEATURES_OK: u8 = 4;

pub const VIRTIO_NET_S_LINK_UP: u16 = 1;

const VIRTIO_PCI_STATUS: usize = 0x0E;
const VIRTIO_PCI_CONFIG: usize = 0x14;

pub struct VirtioNet {
    base: *mut u32,
    pub mac: [u8; 6],
    pub status: u16,
    pub link_up: bool,
}

impl VirtioNet {
    pub fn new(base: *mut u32) -> Self {
        Self {
            base,
            mac: [0; 6],
            status: 0,
            link_up: false,
        }
    }

    pub fn init(&mut self) -> bool {
        unsafe {
            self.base.add(VIRTIO_PCI_STATUS).write(VIRTIO_CONFIG_STATUS_DRIVER as u32);
            
            let features = self.base.add(0).read();
            self.base.add(0).write(features | VIRTIO_NET_F_MAC | VIRTIO_F_VERSION_1);
            
            let status = self.base.add(VIRTIO_PCI_STATUS).read() as u8;
            if status & 4 == 0 {
                return false;
            }
            
            self.base.add(VIRTIO_PCI_STATUS).write(VIRTIO_CONFIG_STATUS_DRIVER as u32 | VIRTIO_CONFIG_STATUS_DRIVER_OK as u32);
            
            self.status = self.base.add(VIRTIO_PCI_CONFIG).read() as u16;
            self.link_up = (self.status & VIRTIO_NET_S_LINK_UP) != 0;
            
            for i in 0..6 {
                self.mac[i] = (self.base.add(VIRTIO_PCI_CONFIG + 6 + i).read() & 0xFF) as u8;
            }
            
            true
        }
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

    pub fn send(&self, _data: &[u8]) -> bool {
        true
    }

    pub fn recv(&self, _buf: &mut [u8]) -> Option<usize> {
        None
    }
}

pub fn find_from_pci(pci: &PciBus) -> Option<(VirtioNet, u64)> {
    for bus in 0..1 {
        for slot in 0..32 {
            if let Some(dev) = pci.read_device(bus, slot, 0) {
                if dev.vendor_id == VIRTIO_VENDOR_ID && dev.device_id == VIRTIO_DEVICE_ID_NET {
                    let bar0 = dev.bar_addr(0);
                    if bar0 != 0 {
                        let mut net = VirtioNet::new(bar0 as *mut u32);
                        if net.init() {
                            return Some((net, bar0));
                        }
                    }
                }
            }
        }
    }
    None
}