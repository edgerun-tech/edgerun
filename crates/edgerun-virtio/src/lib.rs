//! Virtio-net driver for QEMU/KVM

#![no_std]

pub const VIRTIO_NET_F_CSUM: u32 = 1 << 0;
pub const VIRTIO_NET_F_MAC: u32 = 1 << 5;
pub const VIRTIO_F_VERSION_1: u32 = 0x80000000;
pub const VIRTIO_NET_S_LINK_UP: u16 = 1;

pub const VIRTIO_CONFIG_S_DRIVER_OK: u8 = 4;

pub struct VirtNet {
    mac: [u8; 6],
}

impl VirtNet {
    pub const fn new() -> Self {
        Self { mac: [0; 6] }
    }

    pub fn init(&mut self) {}

    pub fn get_mac(&self) -> [u8; 6] {
        self.mac
    }

    pub fn is_link_up(&self) -> bool {
        true
    }
}

impl Default for VirtNet {
    fn default() -> Self {
        Self::new()
    }
}

pub fn find_virtio_net() -> Option<VirtNet> {
    Some(VirtNet::new())
}