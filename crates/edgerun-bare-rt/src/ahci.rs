//! AHCI/SATA driver

#![allow(dead_code)]

use crate::storage::{BlockDevice, SECTOR_SIZE};

pub const AHCI_BASE: usize = 0x00;
pub const AHCI_GHC: usize = 0x00;
pub const AHCI_IS: usize = 0x02;
pub const AHCI_PI: usize = 0x04;
pub const AHCI_PORTS: usize = 0x00;

pub const AHCI_CMD_LIST: usize = 0x00;
pub const AHCI_CMD_TABLE: usize = 0x08;
pub const AHCI_FIS: usize = 0x00;

pub const AHCI_CMD_START: u32 = 1 << 0;
pub const AHCI_CMD_FRE: u32 = 1 << 4;
pub const AHCI_CMD_FR: u32 = 1 << 5;
pub const AHCI_CMD_CL: u32 = 1 << 6;
pub const AHCI_CMD_PMA: u32 = 1 << 7;

pub const AHCI_TFD_ERR: u16 = 1 << 0;
pub const AHCI_TFD_DRQ: u16 = 1 << 3;
pub const AHCI_TFD_BSY: u16 = 1 << 7;

pub const SATA_CMD_READ_FIS: u8 = 0x25;
pub const SATA_CMD_WRITE_FIS: u8 = 0x35;
pub const SATA_CMD_IDENTIFY: u8 = 0xEC;

pub const FIS_TYPE_REG_H2D: u8 = 0x27;
pub const FIS_TYPE_REG_D2H: u8 = 0x34;
pub const FIS_TYPE_SETUP: u8 = 0x90;
pub const FIS_TYPE_PIO: u8 = 0x5F;
pub const FIS_TYPE_DMA: u8 = 0x41;

#[repr(C, packed)]
pub struct FisRegH2D {
    fis_type: u8,
    flags: u8,
    cmd: u8,
    feature_low: u8,
    lba0: u32,
    lba1: u16,
    device: u8,
    lba2: u8,
    feature_high: u8,
    sectors: u8,
    control: u8,
    reserved: [u8; 6],
}

#[repr(C, packed)]
pub struct AhciCommandList {
    prdtl: u16,
    pRD: u8,
    rsv1: u8,
    ctba: u32,
    ctbau: u32,
    reserved: [u32; 4],
}

#[repr(C, packed)]
pub struct AhciPrdt {
    dba: u32,
    dbau: u32,
    reserved: u32,
    dbc: u32,
}

#[repr(C, packed)]
pub struct AhciPort {
    cmd: u32,
    tfd: u32,
    sig: u32,
    ist: u32,
    ie: u32,
    cmd2: u32,
    rsv: [u32; 2],
    tf: [u32; 10],
    rsv2: [u32; 4],
    done: u32,
}

pub struct AhciController {
    ptr: *mut u32,
    ports: [*mut AhciPort; 32],
    port_count: u32,
}

impl AhciController {
    pub fn new(base: *mut u32) -> Self {
        unsafe {
            let pi = base.offset(AHCI_PI as isize / 4).read();
            let count = pi.count_ones();
            Self {
                ptr: base,
                ports: [core::ptr::null_mut(); 32],
                port_count: count,
            }
        }
    }

    pub fn init(&mut self) -> bool {
        unsafe {
            let ghc = self.ptr.offset(AHCI_GHC as isize / 4);
            let val = ghc.read();
            ghc.write(val | 1);
            let pi = self.ptr.offset(AHCI_PI as isize / 4).read();
            if pi == 0 {
                return false;
            }
            for i in 0..32 {
                if pi & (1 << i) != 0 {
                    let port_ptr = self.ptr.add(AHCI_PORTS as usize + (i * 0x80));
                    self.ports[i as usize] = port_ptr as *mut AhciPort;
                }
            }
            true
        }
    }

    pub fn read_sector(&mut self, port: u32, lba: u64, buf: &mut [u8]) -> bool {
        unsafe {
            if port as usize >= 32 || self.ports[port as usize].is_null() {
                return false;
            }
            let p = &mut *self.ports[port as usize];
            let mut tfd = p.tfd;
            while tfd & (AHCI_TFD_BSY as u32) != 0 {
                tfd = p.tfd;
            }
            if tfd & (AHCI_TFD_ERR as u32) != 0 {
                return false;
            }
            let mut fis = FisRegH2D {
                fis_type: FIS_TYPE_REG_H2D,
                flags: 0x80,
                cmd: SATA_CMD_READ_FIS,
                feature_low: 0,
                lba0: lba as u32,
                lba1: (lba >> 32) as u16,
                device: 0x40,
                lba2: 0,
                feature_high: 0,
                sectors: 1,
                control: 0,
                reserved: [0; 6],
            };
            let prdt = buf.as_mut_ptr() as u32;
            let prdtl = (buf.len() / SECTOR_SIZE) as u16;
            core::ptr::write_volatile(p as *mut AhciPort as *mut u64, 0);
            p.cmd |= AHCI_CMD_START;
            true
        }
    }

    pub fn write_sector(&mut self, port: u32, lba: u64, buf: &[u8]) -> bool {
        unsafe {
            if port as usize >= 32 || self.ports[port as usize].is_null() {
                return false;
            }
            let p = &mut *self.ports[port as usize];
            let mut tfd = p.tfd;
            while tfd & (AHCI_TFD_BSY as u32) != 0 {
                tfd = p.tfd;
            }
            if tfd & (AHCI_TFD_ERR as u32) != 0 {
                return false;
            }
            let mut fis = FisRegH2D {
                fis_type: FIS_TYPE_REG_H2D,
                flags: 0x80,
                cmd: SATA_CMD_WRITE_FIS,
                feature_low: 0,
                lba0: lba as u32,
                lba1: (lba >> 32) as u16,
                device: 0x40,
                lba2: 0,
                feature_high: 0,
                sectors: 1,
                control: 0,
                reserved: [0; 6],
            };
            core::ptr::write_volatile(p as *mut AhciPort as *mut u64, 0);
            p.cmd |= AHCI_CMD_START;
            true
        }
    }

    pub fn identify(&mut self, port: u32) -> bool {
        unsafe {
            if port as usize >= 32 || self.ports[port as usize].is_null() {
                return false;
            }
            let p = &mut *self.ports[port as usize];
            p.cmd |= AHCI_CMD_START;
            true
        }
    }

    pub fn port_count(&self) -> u32 {
        self.port_count
    }
}

impl BlockDevice for AhciController {
    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> bool {
        self.read_sector(0, sector, buf)
    }

    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> bool {
        self.write_sector(0, sector, buf)
    }

    fn sectors(&self) -> u64 {
        0
    }
}