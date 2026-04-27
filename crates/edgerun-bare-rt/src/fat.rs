//! FAT file system

#![allow(dead_code)]

use crate::storage::BlockDevice;

pub const FAT_BOOT_SIG: u16 = 0xAA55;

pub struct FatFs<D: BlockDevice> {
    _dev: D,
}

impl<D: BlockDevice> FatFs<D> {
    pub fn open(mut dev: D) -> Option<Self> {
        let mut boot = [0u8; 512];
        if !dev.read_sector(0, &mut boot) {
            return None;
        }
        let sig = (boot[511] as u16) << 8 | boot[510] as u16;
        if sig != FAT_BOOT_SIG {
            return None;
        }
        Some(Self { _dev: dev })
    }

    pub fn read_boot(&mut self) -> [u8; 512] {
        let mut boot = [0u8; 512];
        let _ = self._dev.read_sector(0, &mut boot);
        boot
    }
}