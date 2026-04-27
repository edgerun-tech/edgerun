//! ATA/PATA driver

#![allow(dead_code)]

use crate::storage::{BlockDevice, SECTOR_SIZE};

pub const ATA_DATA: usize = 0x00;
pub const ATA_ERROR: usize = 0x02;
pub const ATA_FEATURES: usize = 0x02;
pub const ATA_SECTORS: usize = 0x02;
pub const ATA_LBA_LOW: usize = 0x03;
pub const ATA_LBA_MID: usize = 0x04;
pub const ATA_LBA_HIGH: usize = 0x05;
pub const ATA_DRIVE: usize = 0x06;
pub const ATA_STATUS: usize = 0x07;
pub const ATA_CMD: usize = 0x07;

pub const ATA_CTRL: usize = 0x00;
pub const ATA_ASTATUS: usize = 0x02;

pub const ATA_CMD_READ_DMA: u8 = 0xC8;
pub const ATA_CMD_WRITE_DMA: u8 = 0xCA;
pub const ATA_CMD_IDENTIFY: u8 = 0xEC;
pub const ATA_CMD_READ: u8 = 0x20;
pub const ATA_CMD_WRITE: u8 = 0x30;
pub const ATA_CMD_FLUSH_CACHE: u8 = 0xE7;

pub const ATA_STATUS_ERR: u8 = 1 << 0;
pub const ATA_STATUS_DRQ: u8 = 1 << 3;
pub const ATA_STATUS_SRV: u8 = 1 << 4;
pub const ATA_STATUS_DF: u8 = 1 << 5;
pub const ATA_STATUS_DRDY: u8 = 1 << 6;
pub const ATA_STATUS_BSY: u8 = 1 << 7;

pub struct AtaDevice {
    base: *mut u8,
    ctrl: *mut u8,
    slave: bool,
}

impl AtaDevice {
    pub fn new(base: *mut u8, ctrl: *mut u8, slave: bool) -> Self {
        Self { base, ctrl, slave }
    }

    pub fn init(&mut self) -> bool {
        self.write_reg(ATA_DRIVE, if self.slave { 0xB0 } else { 0xA0 });
        let status = self.read_reg(ATA_STATUS);
        status & ATA_STATUS_DRDY != 0
    }

    fn read_reg(&self, off: usize) -> u8 {
        unsafe { self.base.add(off).read() }
    }

    fn write_reg(&self, off: usize, val: u8) {
        unsafe { self.base.add(off).write(val) }
    }

    pub fn read_sectors(&mut self, lba: u32, count: u8, buf: &mut [u8]) -> bool {
        unsafe {
            self.write_reg(ATA_FEATURES, 0);
            self.write_reg(ATA_SECTORS, count);
            self.write_reg(ATA_LBA_LOW, lba as u8);
            self.write_reg(ATA_LBA_MID, (lba >> 8) as u8);
            self.write_reg(ATA_LBA_HIGH, (lba >> 16) as u8);
            self.write_reg(
                ATA_DRIVE,
                if self.slave { 0xB0 } else { 0xA0 } | ((lba >> 24) as u8 & 0x0F),
            );
            self.write_reg(ATA_CMD, ATA_CMD_READ);

            let mut timeout = 0;
            while timeout < 100000 {
                let status = self.read_reg(ATA_STATUS);
                if status & ATA_STATUS_ERR != 0 {
                    return false;
                }
                if status & ATA_STATUS_DRQ != 0 {
                    break;
                }
                timeout += 1;
            }

            let data = self.base as *mut u16;
            let len = (count as usize * SECTOR_SIZE) / 2;
            for i in 0..len {
                let val = data.offset(i as isize).read();
                buf.as_mut_ptr().add(i * 2).write(val as u8);
                buf.as_mut_ptr().add(i * 2 + 1).write((val >> 8) as u8);
            }
            true
        }
    }

    pub fn write_sectors(&mut self, lba: u32, count: u8, buf: &[u8]) -> bool {
        unsafe {
            self.write_reg(ATA_FEATURES, 0);
            self.write_reg(ATA_SECTORS, count);
            self.write_reg(ATA_LBA_LOW, lba as u8);
            self.write_reg(ATA_LBA_MID, (lba >> 8) as u8);
            self.write_reg(ATA_LBA_HIGH, (lba >> 16) as u8);
            self.write_reg(
                ATA_DRIVE,
                if self.slave { 0xB0 } else { 0xA0 } | ((lba >> 24) as u8 & 0x0F),
            );
            self.write_reg(ATA_CMD, ATA_CMD_WRITE);

            let mut timeout = 0;
            while timeout < 100000 {
                let status = self.read_reg(ATA_STATUS);
                if status & ATA_STATUS_ERR != 0 {
                    return false;
                }
                if status & ATA_STATUS_DRQ != 0 {
                    break;
                }
                timeout += 1;
            }

            let data = self.base as *mut u16;
            let len = (count as usize * SECTOR_SIZE) / 2;
            for i in 0..len {
                let lo = buf.as_ptr().add(i * 2).read();
                let hi = buf.as_ptr().add(i * 2 + 1).read();
                data.offset(i as isize).write((hi as u16) << 8 | lo as u16);
            }
            true
        }
    }

    pub fn identify(&mut self) -> bool {
        self.write_reg(ATA_DRIVE, if self.slave { 0xB0 } else { 0xA0 });
        self.write_reg(ATA_CMD, ATA_CMD_IDENTIFY);
        let mut timeout = 0;
        while timeout < 100000 {
            let status = self.read_reg(ATA_STATUS);
            if status & ATA_STATUS_ERR != 0 {
                return false;
            }
            if status & ATA_STATUS_DRQ != 0 {
                return true;
            }
            timeout += 1;
        }
        false
    }

    pub fn flush(&mut self) -> bool {
        self.write_reg(ATA_CMD, ATA_CMD_FLUSH_CACHE);
        let mut timeout = 0;
        while timeout < 100000 {
            let status = self.read_reg(ATA_STATUS);
            if status & ATA_STATUS_ERR != 0 {
                return false;
            }
            if status & ATA_STATUS_BSY == 0 {
                return true;
            }
            timeout += 1;
        }
        false
    }
}

impl BlockDevice for AtaDevice {
    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> bool {
        self.read_sectors(sector as u32, 1, buf)
    }

    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> bool {
        self.write_sectors(sector as u32, 1, buf)
    }

    fn sectors(&self) -> u64 {
        0
    }
}
