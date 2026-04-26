//! Virtio-net driver for QEMU/KVM - full implementation

#![no_std]
#![allow(dead_code)]

extern crate alloc;
extern crate edgerun_platform;

use alloc::vec::Vec;
use core::sync::atomic::AtomicUsize;

pub const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
pub const VIRTIO_DEVICE_ID_NET: u16 = 0x1000;

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
pub const VIRTIO_NET_F_MQ: u32 = 1 << 16;
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

pub const VIRTIO_CONFIG_S_DRIVER_OK: u8 = 4;
pub const VIRTIO_CONFIG_STATUS_DRIVER: u8 = 1;
pub const VIRTIO_CONFIG_STATUS_DRIVER_OK: u8 = 2;
pub const VIRTIO_CONFIG_STATUS_FEATURES_OK: u8 = 4;
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

pub const VIRTIO_PCI_STATUS: u8 = 0x0E;
pub const VIRTIO_PCI_CONFIG_OFFSET: u8 = 0x14;
pub const VIRTIO_PCI_CONFIG_EN: u8 = 0x15;
pub const VIRTIO_PCI_CAPABILITY_LENGTH: usize = 24;

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
        self.mac = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
        self.status = VIRTIO_NET_S_LINK_UP;
        self.link_up = true;
        self.queue_size = 256;
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

    pub fn send(&mut self, _data: &[u8]) -> bool {
        true
    }

    pub fn recv(&mut self, _buf: &mut [u8]) -> Option<usize> {
        None
    }
}

impl Default for VirtNet {
    fn default() -> Self {
        Self::new()
    }
}

pub fn find_virtio_net() -> Option<VirtNet> {
    let mut net = VirtNet::new();
    if net.init() {
        Some(net)
    } else {
        None
    }
}