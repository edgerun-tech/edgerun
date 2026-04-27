//! Block device storage interface

pub const SECTOR_SIZE: usize = 512;

pub trait BlockDevice {
    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> bool;
    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> bool;
    fn sectors(&self) -> u64;
}