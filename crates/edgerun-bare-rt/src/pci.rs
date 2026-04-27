//! PCI bus enumeration

#![allow(dead_code)]

pub const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
pub const PCI_CONFIG_DATA: u16 = 0xCFC;

pub const PCI_VENDOR_ID: u16 = 0x00;
pub const PCI_DEVICE_ID: u16 = 0x02;
pub const PCI_COMMAND: u16 = 0x04;
pub const PCI_STATUS: u16 = 0x06;
pub const PCI_REVISION_ID: u16 = 0x08;
pub const PCI_PROG_IF: u16 = 0x09;
pub const PCI_SUBCLASS: u16 = 0x0A;
pub const PCI_CLASS: u16 = 0x0B;
pub const PCI_HEADER_TYPE: u16 = 0x0E;
pub const PCI_BASE_ADDRESS_0: u16 = 0x10;
pub const PCI_BASE_ADDRESS_1: u16 = 0x14;
pub const PCI_BASE_ADDRESS_2: u16 = 0x18;
pub const PCI_BASE_ADDRESS_3: u16 = 0x1C;
pub const PCI_BASE_ADDRESS_4: u16 = 0x20;
pub const PCI_BASE_ADDRESS_5: u16 = 0x24;
pub const PCI_INTERRUPT_LINE: u16 = 0x3C;

pub const PCI_COMMAND_IO_SPACE: u16 = 0x0001;
pub const PCI_COMMAND_MEMORY_SPACE: u16 = 0x0002;
pub const PCI_COMMAND_BUS_MASTER: u16 = 0x0004;

pub const PCI_HEADER_TYPE_NORMAL: u8 = 0x00;
pub const PCI_CLASS_STORAGE: u8 = 0x01;
pub const PCI_CLASS_NETWORK: u8 = 0x02;
pub const PCI_CLASS_DISPLAY: u8 = 0x03;
pub const PCI_CLASS_BRIDGE: u8 = 0x06;

pub const PCI_SUBCLASS_STORAGE_SATA: u8 = 0x01;
pub const PCI_SUBCLASS_STORAGE_NVME: u8 = 0x08;
pub const PCI_SUBCLASS_NET_ETHERNET: u8 = 0x00;

#[derive(Clone, Copy, Debug)]
pub struct PciDevice {
    pub bus: u8,
    pub slot: u8,
    pub func: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class: u8,
    pub subclass: u8,
    pub prog_if: u8,
    pub revision: u8,
    pub bars: [u32; 6],
}

impl PciDevice {
    pub fn is_storage(&self) -> bool {
        self.class == PCI_CLASS_STORAGE
            && (self.subclass == PCI_SUBCLASS_STORAGE_SATA || self.subclass == PCI_SUBCLASS_STORAGE_NVME)
    }

    pub fn is_nvme(&self) -> bool {
        self.class == PCI_CLASS_STORAGE && self.subclass == PCI_SUBCLASS_STORAGE_NVME
    }

    pub fn is_sata(&self) -> bool {
        self.class == PCI_CLASS_STORAGE && self.subclass == PCI_SUBCLASS_STORAGE_SATA
    }

    pub fn is_ethernet(&self) -> bool {
        self.class == PCI_CLASS_NETWORK && self.subclass == PCI_SUBCLASS_NET_ETHERNET
    }

    pub fn bar_addr(&self, bar: usize) -> u64 {
        if bar < 6 {
            self.bars[bar] as u64 & !0xF
        } else {
            0
        }
    }

    pub fn bar_is_io(&self, bar: usize) -> bool {
        bar < 6 && (self.bars[bar] & 0x1) != 0
    }

    pub fn bar_is_mem64(&self, bar: usize) -> bool {
        bar < 6 && (self.bars[bar] & 0x6) == 0x6
    }
}

pub struct PciBus;

impl PciBus {
    pub fn read_config(&self, bus: u8, slot: u8, func: u8, reg: u8) -> u32 {
        let addr = ((bus as u32) << 16)
            | ((slot as u32) << 11)
            | ((func as u32) << 8)
            | ((reg as u32) & 0xFC)
            | 0x80000000u32;
        
        unsafe {
            core::ptr::write_volatile(PCI_CONFIG_ADDRESS as *mut u32, addr);
            core::ptr::read_volatile(PCI_CONFIG_DATA as *const u32)
        }
    }

    pub fn write_config(&self, bus: u8, slot: u8, func: u8, reg: u8, val: u32) {
        let addr = ((bus as u32) << 16)
            | ((slot as u32) << 11)
            | ((func as u32) << 8)
            | ((reg as u32) & 0xFC)
            | 0x80000000u32;
        
        unsafe {
            core::ptr::write_volatile(PCI_CONFIG_ADDRESS as *mut u32, addr);
            core::ptr::write_volatile(PCI_CONFIG_DATA as *mut u32, val);
        }
    }

    pub fn read_device(&self, bus: u8, slot: u8, func: u8) -> Option<PciDevice> {
        let vendor = self.read_config(bus, slot, func, 0) as u16;
        if vendor == 0xFFFF || vendor == 0 {
            return None;
        }

        let device_id = (self.read_config(bus, slot, func, 0) >> 16) as u16;
        let class = (self.read_config(bus, slot, func, 0x0B) >> 24) as u8;
        let subclass = (self.read_config(bus, slot, func, 0x0A) >> 16) as u8;
        let prog_if = (self.read_config(bus, slot, func, 0x09) >> 24) as u8;
        let revision = self.read_config(bus, slot, func, 0x08) as u8;

        let mut bars = [0u32; 6];
        for i in 0..6 {
            bars[i] = self.read_config(bus, slot, func, PCI_BASE_ADDRESS_0 as u8 + (i as u8 * 4));
        }

        Some(PciDevice {
            bus,
            slot,
            func,
            vendor_id: vendor,
            device_id,
            class,
            subclass,
            prog_if,
            revision,
            bars,
        })
    }

    pub fn find_nvme(&self) -> Option<(PciDevice, u64)> {
        for bus in 0..1 {
            for slot in 0..32 {
                if let Some(dev) = self.read_device(bus, slot, 0) {
                    if dev.is_nvme() {
                        let bar0 = dev.bar_addr(0);
                        if bar0 != 0 {
                            return Some((dev, bar0));
                        }
                    }
                }
            }
        }
        None
    }

    pub fn find_sata(&self) -> Option<(PciDevice, u64)> {
        for bus in 0..1 {
            for slot in 0..32 {
                if let Some(dev) = self.read_device(bus, slot, 0) {
                    if dev.is_sata() {
                        let bar0 = dev.bar_addr(0);
                        if bar0 != 0 {
                            return Some((dev, bar0));
                        }
                    }
                }
            }
        }
        None
    }

    pub fn find_ethernet(&self) -> Option<(PciDevice, u64)> {
        for bus in 0..1 {
            for slot in 0..32 {
                if let Some(dev) = self.read_device(bus, slot, 0) {
                    if dev.is_ethernet() {
                        let bar0 = dev.bar_addr(0);
                        if bar0 != 0 {
                            return Some((dev, bar0));
                        }
                    }
                }
            }
        }
        None
    }
}