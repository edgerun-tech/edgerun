use crate::prelude::v1::*;
use alloc::vec;
use alloc::vec::Vec;
use core::ptr::{read_volatile, write_volatile};
use edgerun_encoding::byteorder::read_u32_be;

use crate::traits::{FixedTpmTransport, TpmTransport};
use crate::types::TpmError;

const HEADER_SIZE: usize = 10;
const MAX_RESPONSE_SIZE: usize = 4096;
const DEFAULT_TIMEOUT_POLLS: usize = 1_000_000;
const DEFAULT_X86_TIS_BASE: usize = 0xfed4_0000;
const TPM_TIS_TIMEOUT_CODE: u32 = 0xffff_fffd;

const ACCESS_OFFSET: usize = 0x0000;
const STS_OFFSET: usize = 0x0018;
const DATA_FIFO_OFFSET: usize = 0x0024;

const ACCESS_REQUEST_USE: u8 = 1 << 1;
const ACCESS_ACTIVE_LOCALITY: u8 = 1 << 5;
const ACCESS_VALID: u8 = 1 << 7;

const STS_EXPECT: u32 = 1 << 3;
const STS_DATA_AVAIL: u32 = 1 << 4;
const STS_GO: u32 = 1 << 5;
const STS_COMMAND_READY: u32 = 1 << 6;
const STS_VALID: u32 = 1 << 7;
const STS_BURST_COUNT_MASK: u32 = 0xffff << 8;
const STS_BURST_COUNT_SHIFT: u32 = 8;

/// TPM Interface Specification FIFO transport over the PC Client MMIO window.
#[derive(Clone, Copy, Debug)]
pub struct TisTpmTransport {
    base: usize,
    timeout_polls: usize,
}

impl TisTpmTransport {
    /// Create a TIS transport at the conventional x86 locality-0 base.
    ///
    /// # Safety
    ///
    /// The caller must ensure the TPM TIS MMIO window is identity-mapped and
    /// that locality 0 is safe to claim.
    pub const unsafe fn new_default_x86() -> Self {
        Self::new(DEFAULT_X86_TIS_BASE)
    }

    /// Create a TIS transport at an explicit locality base.
    ///
    /// # Safety
    ///
    /// `base` must point at a valid TPM TIS locality register block.
    pub const unsafe fn new(base: usize) -> Self {
        Self {
            base,
            timeout_polls: DEFAULT_TIMEOUT_POLLS,
        }
    }

    pub fn with_timeout_polls(mut self, timeout_polls: usize) -> Self {
        self.timeout_polls = timeout_polls;
        self
    }

    pub fn base(&self) -> usize {
        self.base
    }

    fn claim_locality(&self) -> Result<(), TpmError> {
        unsafe {
            write_u8(self.base + ACCESS_OFFSET, ACCESS_REQUEST_USE);
        }

        self.wait_for_access(
            ACCESS_VALID | ACCESS_ACTIVE_LOCALITY,
            ACCESS_VALID | ACCESS_ACTIVE_LOCALITY,
        )
    }

    fn prepare_command(&self) -> Result<(), TpmError> {
        unsafe {
            write_u32(self.base + STS_OFFSET, STS_COMMAND_READY);
        }
        self.wait_for_sts(STS_COMMAND_READY, STS_COMMAND_READY)
    }

    fn write_command(&self, command: &[u8]) -> Result<(), TpmError> {
        for byte in command {
            self.wait_for_burst()?;
            unsafe {
                write_u8(self.base + DATA_FIFO_OFFSET, *byte);
            }
        }
        Ok(())
    }

    fn start_command(&self) {
        unsafe {
            write_u32(self.base + STS_OFFSET, STS_GO);
        }
    }

    fn read_response(&self) -> Result<Vec<u8>, TpmError> {
        let mut header = [0u8; HEADER_SIZE];
        self.read_fifo_exact(&mut header)?;

        let response_size = read_u32_be(&header, 2) as usize;
        if !(HEADER_SIZE..=MAX_RESPONSE_SIZE).contains(&response_size) {
            return Err(TpmError::TpmResponseCode(TPM_TIS_TIMEOUT_CODE));
        }

        let mut response = vec![0u8; response_size];
        response[..HEADER_SIZE].copy_from_slice(&header);
        self.read_fifo_exact(&mut response[HEADER_SIZE..])?;
        Ok(response)
    }

    fn read_response_into(&self, response: &mut [u8]) -> Result<usize, TpmError> {
        if response.len() < HEADER_SIZE {
            return Err(TpmError::TpmResponseCode(TPM_TIS_TIMEOUT_CODE));
        }

        let mut header = [0u8; HEADER_SIZE];
        self.read_fifo_exact(&mut header)?;

        let response_size = read_u32_be(&header, 2) as usize;
        if !(HEADER_SIZE..=MAX_RESPONSE_SIZE).contains(&response_size) {
            return Err(TpmError::TpmResponseCode(TPM_TIS_TIMEOUT_CODE));
        }
        if response_size > response.len() {
            return Err(TpmError::TpmResponseCode(TPM_TIS_TIMEOUT_CODE));
        }

        response[..HEADER_SIZE].copy_from_slice(&header);
        self.read_fifo_exact(&mut response[HEADER_SIZE..response_size])?;
        Ok(response_size)
    }

    fn read_fifo_exact(&self, out: &mut [u8]) -> Result<(), TpmError> {
        let mut offset = 0;
        while offset < out.len() {
            self.wait_for_sts(STS_VALID | STS_DATA_AVAIL, STS_VALID | STS_DATA_AVAIL)?;
            let mut burst = self.burst_count();
            if burst == 0 {
                self.wait_for_burst()?;
                burst = self.burst_count();
            }

            while burst > 0 && offset < out.len() {
                out[offset] = unsafe { read_u8(self.base + DATA_FIFO_OFFSET) };
                offset += 1;
                burst -= 1;
            }
        }
        Ok(())
    }

    fn wait_for_access(&self, mask: u8, expected: u8) -> Result<(), TpmError> {
        for _ in 0..self.timeout_polls {
            let access = unsafe { read_u8(self.base + ACCESS_OFFSET) };
            if access & mask == expected {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(TpmError::TpmResponseCode(TPM_TIS_TIMEOUT_CODE))
    }

    fn wait_for_sts(&self, mask: u32, expected: u32) -> Result<(), TpmError> {
        for _ in 0..self.timeout_polls {
            let sts = unsafe { read_u32(self.base + STS_OFFSET) };
            if sts & mask == expected {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(TpmError::TpmResponseCode(TPM_TIS_TIMEOUT_CODE))
    }

    fn wait_for_burst(&self) -> Result<(), TpmError> {
        for _ in 0..self.timeout_polls {
            if self.burst_count() > 0 {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(TpmError::TpmResponseCode(TPM_TIS_TIMEOUT_CODE))
    }

    fn burst_count(&self) -> usize {
        let sts = unsafe { read_u32(self.base + STS_OFFSET) };
        ((sts & STS_BURST_COUNT_MASK) >> STS_BURST_COUNT_SHIFT) as usize
    }
}

impl TpmTransport for TisTpmTransport {
    fn transact(&mut self, command: &[u8]) -> Result<Vec<u8>, TpmError> {
        self.claim_locality()?;
        self.prepare_command()?;
        self.write_command(command)?;
        self.start_command();
        self.read_response()
    }
}

impl FixedTpmTransport for TisTpmTransport {
    fn transact_into(&mut self, command: &[u8], response: &mut [u8]) -> Result<usize, TpmError> {
        self.claim_locality()?;
        self.prepare_command()?;
        self.write_command(command)?;
        self.start_command();
        self.read_response_into(response)
    }
}

unsafe fn read_u8(address: usize) -> u8 {
    unsafe { read_volatile(address as *const u8) }
}

unsafe fn write_u8(address: usize, value: u8) {
    unsafe {
        write_volatile(address as *mut u8, value);
    }
}

unsafe fn read_u32(address: usize) -> u32 {
    unsafe { read_volatile(address as *const u32) }
}

unsafe fn write_u32(address: usize, value: u32) {
    unsafe {
        write_volatile(address as *mut u32, value);
    }
}
