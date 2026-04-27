//! NVMe driver

#![allow(dead_code)]

use crate::storage::{BlockDevice, SECTOR_SIZE};

pub const NVME_CAP: usize = 0x00;
pub const NVME_CC: usize = 0x14;
pub const NVME_CSTS: usize = 0x1C;
pub const NVME_AQA: usize = 0x24;
pub const NVME_ASQ: usize = 0x28;
pub const NVME_ACQ: usize = 0x30;

pub const NVME_BLOCK_SIZE: usize = 512;
pub const NVME_CMD_IDENTIFY: u8 = 0x06;
pub const NVME_CMD_READ: u8 = 0x02;
pub const NVME_CMD_WRITE: u8 = 0x04;
pub const NVME_CMD_FLUSH: u8 = 0x00;

pub const NVME_CSTS_RDY: u32 = 1 << 0;
pub const NVME_CSTS_CFS: u32 = 1 << 1;

pub const NVME_CC_EN: u32 = 1 << 0;
pub const NVME_CC_CSS: u32 = 1 << 4;
pub const NVME_CC_ARB: u32 = 1 << 7;

pub struct NvmeCommand {
    pub opc: u8,
    pub flags: u8,
    pub cmdid: u16,
    pub nsid: u32,
    pub cdw2: [u32; 2],
    pub prp1: u64,
    pub prp2: u64,
    pub cdw10: u32,
    pub cdw11: u32,
    pub cdw12: u32,
    pub cdw13: u32,
    pub cdw14: u32,
    pub cdw15: u32,
}

pub struct NvmeCompletion {
    pub cmdid: u32,
    pub cdw0: u32,
    pub status: u32,
}

pub struct NvmeController {
    ptr: *mut u32,
    sq0: *mut NvmeCommand,
    cq0: *mut NvmeCompletion,
    sq_depth: u16,
    cq_depth: u16,
    ns_count: u32,
    page_size: u32,
    nsze: u64,
}

impl NvmeController {
    pub fn new(base: *mut u32) -> Self {
        unsafe {
            let cap = base.offset(NVME_CAP as isize / 4).read();
            let page_size = 1u32 << 12;
            let max_qentries = ((cap >> 16) & 0xFFFF) as u16;
            Self {
                ptr: base,
                sq0: core::ptr::null_mut(),
                cq0: core::ptr::null_mut(),
                sq_depth: max_qentries,
                cq_depth: max_qentries,
                ns_count: 0,
                page_size,
                nsze: 0,
            }
        }
    }

    pub fn init(&mut self) -> bool {
        unsafe {
            self.ptr.offset(NVME_CC as isize / 4).write(0);
            self.ptr
                .offset(NVME_CC as isize / 4)
                .write(NVME_CC_EN | NVME_CC_CSS);
            let mut timeout = 0;
            while timeout < 100000 {
                let csts = self.ptr.offset(NVME_CSTS as isize / 4).read();
                if csts & NVME_CSTS_RDY != 0 {
                    return true;
                }
                timeout += 1;
            }
            false
        }
    }

    pub fn read(&mut self, lba: u64, nblocks: u32, buf: &mut [u8]) -> bool {
        if self.sq0.is_null() || self.cq0.is_null() {
            return false;
        }
        let sectorsz = SECTOR_SIZE as u32;
        let cmd = NvmeCommand {
            opc: NVME_CMD_READ,
            flags: 0,
            cmdid: 0,
            nsid: 1,
            cdw2: [0; 2],
            prp1: buf.as_mut_ptr() as u64,
            prp2: 0,
            cdw10: lba as u32,
            cdw11: (lba >> 32) as u32,
            cdw12: nblocks.saturating_sub(1),
            cdw13: sectorsz,
            cdw14: 0,
            cdw15: 0,
        };
        self.submit(cmd)
    }

    pub fn write(&mut self, lba: u64, nblocks: u32, buf: &[u8]) -> bool {
        if self.sq0.is_null() || self.cq0.is_null() {
            return false;
        }
        let sectorsz = SECTOR_SIZE as u32;
        let cmd = NvmeCommand {
            opc: NVME_CMD_WRITE,
            flags: 0,
            cmdid: 0,
            nsid: 1,
            cdw2: [0; 2],
            prp1: buf.as_ptr() as u64,
            prp2: 0,
            cdw10: lba as u32,
            cdw11: (lba >> 32) as u32,
            cdw12: nblocks.saturating_sub(1),
            cdw13: sectorsz,
            cdw14: 0,
            cdw15: 0,
        };
        self.submit(cmd)
    }

    fn submit(&mut self, cmd: NvmeCommand) -> bool {
        unsafe {
            core::ptr::write(self.sq0, cmd);
        }
        let doorbell = unsafe { self.ptr.offset(0x1000 / 4) };
        unsafe { doorbell.write(0) };
        true
    }

    pub fn identify(&mut self) -> bool {
        if self.sq0.is_null() || self.cq0.is_null() {
            return false;
        }
        let data = [0u8; SECTOR_SIZE];
        let cmd = NvmeCommand {
            opc: NVME_CMD_IDENTIFY,
            flags: 0,
            cmdid: 0,
            nsid: 1,
            cdw2: [0; 2],
            prp1: data.as_ptr() as u64,
            prp2: 0,
            cdw10: 0,
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        };
        self.submit(cmd)
    }

    pub fn sectors(&self) -> u64 {
        self.nsze
    }
}

impl BlockDevice for NvmeController {
    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> bool {
        self.read(sector, 1, buf)
    }

    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> bool {
        self.write(sector, 1, buf)
    }

    fn sectors(&self) -> u64 {
        self.nsze
    }
}
