//! Realtek RTL8125 2.5G NIC driver
//! PCI device ID: 10ec:8125

#![no_std]

use edgerun_bare_rt::SpinLock;

pub const VENDOR_ID: u16 = 0x10ec;
pub const DEVICE_ID: u16 = 0x8125;

pub const REG_MAC0: u16 = 0x0000;
pub const REG_MAC1: u16 = 0x0004;
pub const REG_MAC2: u16 = 0x0008;
pub const REG_MAC3: u16 = 0x000C;
pub const REG_MAC4: u16 = 0x0010;
pub const REG_MAC5: u16 = 0x0014;

pub const REG_TCR: u16 = 0x0040;
pub const REG_RCR: u16 = 0x0044;
pub const REG_CMD: u16 = 0x0037;
pub const REG_IMR: u16 = 0x003C;
pub const REG_ISR: u16 = 0x003E;

pub const CMD_RX_EN: u8 = 0x08;
pub const CMD_TX_EN: u8 = 0x04;
pub const CMD_RESET: u8 = 0x10;

pub const TCR_DMAMAX: u32 = 0x0F0000;

pub const RCR_ACCEPT_PHYS_MATCH: u32 = 0x00000001;
pub const RCR_ACCEPT_BROAD: u32 = 0x00000002;
pub const RCR_ACCEPT_MULTICAST: u32 = 0x00000004;
pub const RCR_ACCEPT_ALL_PHYS: u32 = 0x00000010;
pub const RCR_ACCEPT_LEN_ERROR: u32 = 0x00000020;
pub const RCR_ACCEPT_OK: u32 = 0x00000040;
pub const RCR_ACCEPT_MULTICAST2: u32 = 0x00000080;
pub const RCR_ACCEPT_ALL: u32 = 0x00000100;
pub const RCR_WRAP: u32 = 0x00002000;

pub const RX_BUF_SIZE: usize = 8192;
pub const TX_BUF_SIZE: usize = 8192;
pub const NUM_TX_DESC: usize = 4;
pub const NUM_RX_DESC: usize = 4;

const TX_DESC_SIZE: usize = 16;
const RX_DESC_SIZE: usize = 16;

#[repr(C)]
pub struct TxDesc {
    pub opts1: u32,
    pub opts2: u32,
    pub addr: u64,
    pub reserved: u64,
}

#[repr(C)]
pub struct RxDesc {
    pub opts1: u32,
    pub opts2: u32,
    pub addr: u64,
    pub reserved: u64,
}

pub struct Rtl8125 {
    iobase: usize,
    lock: SpinLock,
    tx_cur: usize,
    rx_cur: usize,
}

impl Rtl8125 {
    pub const fn new(iobase: usize) -> Self {
        Self {
            iobase,
            lock: SpinLock::new(),
            tx_cur: 0,
            rx_cur: 0,
        }
    }

    #[inline]
    fn read32(&self, reg: u16) -> u32 {
        let addr = (self.iobase + reg as usize) as *const u32;
        unsafe { addr.read_volatile() }
    }

    #[inline]
    fn write32(&self, reg: u16, val: u32) {
        let addr = (self.iobase + reg as usize) as *mut u32;
        unsafe { addr.write_volatile(val) }
    }

    #[inline]
    fn read8(&self, reg: u16) -> u8 {
        let addr = (self.iobase + reg as usize) as *const u8;
        unsafe { addr.read_volatile() }
    }

    #[inline]
    fn write8(&self, reg: u16, val: u8) {
        let addr = (self.iobase + reg as usize) as *mut u8;
        unsafe { addr.write_volatile(val) }
    }

    pub fn init(&self) {
        self.write8(REG_CMD, CMD_RESET);
        self.spin_wait();
        self.spin_wait();
        self.spin_wait();

        self.write32(REG_TCR, TCR_DMAMAX | 0x03000100);
        self.write32(REG_RCR, RCR_ACCEPT_BROAD | RCR_ACCEPT_MULTICAST | RCR_ACCEPT_ALL_PHYS | RCR_ACCEPT_OK);
        self.write8(REG_CMD, CMD_RX_EN | CMD_TX_EN);
    }

    pub fn reset(&self) {
        self.write8(REG_CMD, CMD_RESET);
        self.spin_wait();
    }

    #[inline]
    fn spin_wait(&self) {
        for _ in 0..8 {
            unsafe { core::arch::asm!("pause") };
        }
    }

    pub fn get_mac(&self) -> [u8; 6] {
        [
            self.read8(REG_MAC0) as u8,
            self.read8(REG_MAC1) as u8,
            self.read8(REG_MAC2) as u8,
            self.read8(REG_MAC3) as u8,
            self.read8(REG_MAC4) as u8,
            self.read8(REG_MAC5) as u8,
        ]
    }
}