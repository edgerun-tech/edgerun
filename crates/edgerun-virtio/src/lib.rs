//! Modern virtio-net PCI driver for QEMU/KVM.

#![no_std]
#![allow(dead_code)]

pub const VIRTIO_VENDOR_ID: u16 = 0x1af4;
pub const VIRTIO_MODERN_DEVICE_ID_NET: u16 = 0x1041;

pub const VIRTIO_NET_F_CSUM: u64 = 1 << 0;
pub const VIRTIO_NET_F_GUEST_CSUM: u64 = 1 << 1;
pub const VIRTIO_NET_F_MAC: u64 = 1 << 5;
pub const VIRTIO_NET_F_STATUS: u64 = 1 << 16;
pub const VIRTIO_F_VERSION_1: u64 = 1 << 32;

pub const VIRTIO_CONFIG_STATUS_ACKNOWLEDGE: u8 = 1;
pub const VIRTIO_CONFIG_STATUS_DRIVER: u8 = 2;
pub const VIRTIO_CONFIG_STATUS_DRIVER_OK: u8 = 4;
pub const VIRTIO_CONFIG_STATUS_FEATURES_OK: u8 = 8;
pub const VIRTIO_CONFIG_STATUS_FAILED: u8 = 0x80;

pub const VIRTIO_NET_S_LINK_UP: u16 = 1;

const VIRTIO_PCI_CAP_VENDOR: u8 = 0x09;
const VIRTIO_PCI_CAP_COMMON_CFG: u8 = 1;
const VIRTIO_PCI_CAP_NOTIFY_CFG: u8 = 2;
const VIRTIO_PCI_CAP_ISR_CFG: u8 = 3;
const VIRTIO_PCI_CAP_DEVICE_CFG: u8 = 4;

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
struct ModernNetDevice {
    bus: u8,
    slot: u8,
    func: u8,
    common: VirtioPciCap,
    notify: VirtioPciCap,
    device: VirtioPciCap,
    isr: Option<VirtioPciCap>,
}

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
    queue_notify_off: u16,
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
            bus: 0,
            slot: 0,
            func: 0,
            common_cfg: core::ptr::null_mut(),
            notify_cfg: core::ptr::null_mut(),
            device_cfg: core::ptr::null_mut(),
            isr_cfg: core::ptr::null_mut(),
            notify_off_multiplier: 0,
            queue_size: 0,
            queue_notify_off: 0,
            link_up: false,
        }
    }

    pub fn init(&mut self) -> bool {
        if self.common_cfg.is_null() || self.device_cfg.is_null() || self.notify_cfg.is_null() {
            return false;
        }

        self.enable_pci_memory_and_bus_master();
        self.write_status(0);
        self.write_status(VIRTIO_CONFIG_STATUS_ACKNOWLEDGE);
        self.write_status(VIRTIO_CONFIG_STATUS_ACKNOWLEDGE | VIRTIO_CONFIG_STATUS_DRIVER);

        self.host_features = self.read_device_features();
        self.features = self.host_features & (VIRTIO_F_VERSION_1 | VIRTIO_NET_F_MAC | VIRTIO_NET_F_STATUS);
        if self.features & VIRTIO_F_VERSION_1 == 0 {
            self.fail();
            return false;
        }

        self.write_driver_features(self.features);
        self.write_status(self.read_status() | VIRTIO_CONFIG_STATUS_FEATURES_OK);
        if self.read_status() & VIRTIO_CONFIG_STATUS_FEATURES_OK == 0 {
            self.fail();
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

        self.select_queue(0);
        self.queue_size = self.read_queue_size();
        self.queue_notify_off = self.read_queue_notify_off();

        if self.queue_size == 0 {
            self.fail();
            return false;
        }

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
        false
    }

    pub fn recv(&mut self, _buf: &mut [u8]) -> Option<usize> {
        None
    }

    fn from_modern_device(device: ModernNetDevice) -> Option<Self> {
        let common_cfg = map_pci_cap(device.bus, device.slot, device.func, device.common)?;
        let notify_cfg = map_pci_cap(device.bus, device.slot, device.func, device.notify)?;
        let device_cfg = map_pci_cap(device.bus, device.slot, device.func, device.device)?;
        let isr_cfg = device
            .isr
            .and_then(|cap| map_pci_cap(device.bus, device.slot, device.func, cap))
            .unwrap_or(core::ptr::null_mut());

        let mut net = Self::new();
        net.bus = device.bus;
        net.slot = device.slot;
        net.func = device.func;
        net.common_cfg = common_cfg;
        net.notify_cfg = notify_cfg;
        net.device_cfg = device_cfg;
        net.isr_cfg = isr_cfg;
        net.notify_off_multiplier = device.notify.notify_off_multiplier;
        Some(net)
    }

    fn enable_pci_memory_and_bus_master(&self) {
        let command = pci_read_u16(self.bus, self.slot, self.func, PCI_COMMAND);
        pci_write_u16(
            self.bus,
            self.slot,
            self.func,
            PCI_COMMAND,
            command | PCI_COMMAND_MEMORY | PCI_COMMAND_BUS_MASTER,
        );
    }

    fn fail(&self) {
        self.write_status(self.read_status() | VIRTIO_CONFIG_STATUS_FAILED);
    }

    fn read_status(&self) -> u8 {
        read_u8(unsafe { self.common_cfg.add(20) })
    }

    fn write_status(&self, status: u8) {
        write_u8(unsafe { self.common_cfg.add(20) }, status);
    }

    fn read_device_features(&self) -> u64 {
        write_u32(self.common_cfg, 0);
        let low = read_u32(unsafe { self.common_cfg.add(4) }) as u64;
        write_u32(self.common_cfg, 1);
        let high = read_u32(unsafe { self.common_cfg.add(4) }) as u64;
        low | (high << 32)
    }

    fn write_driver_features(&self, features: u64) {
        write_u32(unsafe { self.common_cfg.add(8) }, 0);
        write_u32(unsafe { self.common_cfg.add(12) }, features as u32);
        write_u32(unsafe { self.common_cfg.add(8) }, 1);
        write_u32(unsafe { self.common_cfg.add(12) }, (features >> 32) as u32);
    }

    fn select_queue(&self, queue: u16) {
        write_u16(unsafe { self.common_cfg.add(22) }, queue);
    }

    fn read_queue_size(&self) -> u16 {
        read_u16(unsafe { self.common_cfg.add(24) })
    }

    fn read_queue_notify_off(&self) -> u16 {
        read_u16(unsafe { self.common_cfg.add(30) })
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
                    if vendor == VIRTIO_VENDOR_ID && device == VIRTIO_MODERN_DEVICE_ID_NET {
                        if let Some(device) = read_modern_net_device(bus, slot, func) {
                            if let Some(mut net) = VirtNet::from_modern_device(device) {
                                if net.init() {
                                    return Some(net);
                                }
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

fn read_modern_net_device(bus: u8, slot: u8, func: u8) -> Option<ModernNetDevice> {
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

    Some(ModernNetDevice {
        bus,
        slot,
        func,
        common: common?,
        notify: notify?,
        device: device?,
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

    if base == 0 { None } else { Some(base) }
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
    pci_write_u32(bus, slot, func, offset, (current & mask) | ((value as u32) << shift));
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
