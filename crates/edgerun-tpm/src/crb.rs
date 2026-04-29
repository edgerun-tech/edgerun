use crate::prelude::v1::*;
use alloc::vec;
use alloc::vec::Vec;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{compiler_fence, Ordering};
use edgerun_encoding::byteorder::read_u32_be;

use crate::acpi::AcpiTpm2Info;
use crate::traits::{FixedTpmTransport, TpmTransport};
use crate::types::TpmError;

const HEADER_SIZE: usize = 10;
const DEFAULT_TIMEOUT_POLLS: usize = 1_000_000;

const CTRL_REQ_OFFSET: usize = 0x40;
const CTRL_CANCEL_OFFSET: usize = 0x48;
const CTRL_START_OFFSET: usize = 0x4c;
const CTRL_CMD_SIZE_OFFSET: usize = 0x58;
const CTRL_CMD_ADDR_OFFSET: usize = 0x5c;
const CTRL_RSP_SIZE_OFFSET: usize = 0x64;
const CTRL_RSP_ADDR_OFFSET: usize = 0x68;

const CTRL_REQ_CMD_READY: u32 = 1 << 0;
const CTRL_START: u32 = 1 << 0;
const TPM_CRB_TIMEOUT_CODE: u32 = 0xffff_fffe;

/// TPM 2.0 Command Response Buffer transport over memory-mapped registers.
///
/// The `control_area` should come from the ACPI `TPM2` table. For early board
/// bring-up with a known CRB locality base such as `0xfed4_0000`, use
/// `from_register_base`.
#[derive(Clone, Copy, Debug)]
pub struct CrbTpmTransport {
    control_area: usize,
    command_buffer: usize,
    command_buffer_size: usize,
    response_buffer: usize,
    response_buffer_size: usize,
    timeout_polls: usize,
}

impl CrbTpmTransport {
    /// Create a CRB transport from explicit MMIO addresses.
    ///
    /// # Safety
    ///
    /// The caller must ensure all addresses are valid MMIO mappings for the
    /// active TPM CRB locality and remain accessible for the lifetime of this
    /// transport.
    pub const unsafe fn new(
        control_area: usize,
        command_buffer: usize,
        command_buffer_size: usize,
        response_buffer: usize,
        response_buffer_size: usize,
    ) -> Self {
        Self {
            control_area,
            command_buffer,
            command_buffer_size,
            response_buffer,
            response_buffer_size,
            timeout_polls: DEFAULT_TIMEOUT_POLLS,
        }
    }

    /// Create a CRB transport by reading buffer descriptors from an ACPI TPM2
    /// control area pointer.
    ///
    /// # Safety
    ///
    /// `control_area` must point at a TPM2 CRB control area. This performs
    /// volatile reads from that region.
    pub unsafe fn from_control_area(control_area: usize) -> Result<Self, TpmError> {
        let first_base = if control_area & 0xfff == 0 {
            control_area
        } else {
            control_area.saturating_sub(CTRL_REQ_OFFSET)
        };
        let second_base = if first_base == control_area {
            control_area.saturating_sub(CTRL_REQ_OFFSET)
        } else {
            control_area
        };

        let first = unsafe { Self::from_register_base(first_base) };
        if first.has_sane_descriptors() {
            return Ok(first);
        }

        let second = unsafe { Self::from_register_base(second_base) };
        if second.has_sane_descriptors() {
            return Ok(second);
        }

        Err(TpmError::Protocol("invalid TPM CRB descriptors".into()))
    }

    /// Create a CRB transport by reading buffer descriptors from a CRB locality
    /// register base, usually `0xfed4_0000` on x86.
    ///
    /// # Safety
    ///
    /// `register_base` must point at a TPM2 CRB locality register block. This
    /// performs volatile reads from that region.
    pub unsafe fn from_register_base(register_base: usize) -> Self {
        let command_buffer_size =
            unsafe { read_u32(register_base + CTRL_CMD_SIZE_OFFSET) } as usize;
        let command_buffer = unsafe { read_u64(register_base + CTRL_CMD_ADDR_OFFSET) } as usize;
        let advertised_response_buffer_size =
            unsafe { read_u32(register_base + CTRL_RSP_SIZE_OFFSET) } as usize;
        let advertised_response_buffer =
            unsafe { read_u64(register_base + CTRL_RSP_ADDR_OFFSET) } as usize;
        let response_buffer_size = if advertised_response_buffer == command_buffer {
            advertised_response_buffer_size
        } else {
            command_buffer_size
        };
        let response_buffer = if advertised_response_buffer == command_buffer {
            advertised_response_buffer
        } else {
            command_buffer
        };

        Self {
            control_area: register_base,
            command_buffer,
            command_buffer_size,
            response_buffer,
            response_buffer_size,
            timeout_polls: DEFAULT_TIMEOUT_POLLS,
        }
    }

    /// Create a CRB transport from parsed ACPI `TPM2` table information.
    ///
    /// # Safety
    ///
    /// The ACPI control area must be valid and identity-mapped.
    pub unsafe fn from_acpi_tpm2(info: AcpiTpm2Info) -> Result<Self, TpmError> {
        if !info.is_crb() {
            return Err(TpmError::Protocol("ACPI TPM2 table is not CRB".into()));
        }
        unsafe { Self::from_control_area(info.control_area as usize) }
    }

    /// Discover a CRB TPM from ACPI firmware tables.
    ///
    /// # Safety
    ///
    /// This scans and reads firmware memory using physical addresses.
    pub unsafe fn discover_acpi() -> Result<Option<Self>, TpmError> {
        let Some(info) = (unsafe { crate::acpi::discover_tpm2_info()? }) else {
            return Ok(None);
        };
        unsafe { Self::from_acpi_tpm2(info) }.map(Some)
    }

    pub fn with_timeout_polls(mut self, timeout_polls: usize) -> Self {
        self.timeout_polls = timeout_polls;
        self
    }

    pub fn control_area(&self) -> usize {
        self.control_area
    }

    pub fn command_buffer(&self) -> usize {
        self.command_buffer
    }

    pub fn command_buffer_size(&self) -> usize {
        self.command_buffer_size
    }

    pub fn response_buffer(&self) -> usize {
        self.response_buffer
    }

    pub fn response_buffer_size(&self) -> usize {
        self.response_buffer_size
    }

    fn has_sane_descriptors(&self) -> bool {
        const MAX_CRB_BUFFER_SIZE: usize = 64 * 1024;
        const MAX_CRB_MMIO_SPAN: usize = 64 * 1024;

        self.command_buffer >= 0x1000
            && self.response_buffer >= 0x1000
            && self.command_buffer >= self.control_area
            && self.response_buffer >= self.control_area
            && self.command_buffer - self.control_area < MAX_CRB_MMIO_SPAN
            && self.response_buffer - self.control_area < MAX_CRB_MMIO_SPAN
            && (HEADER_SIZE..=MAX_CRB_BUFFER_SIZE).contains(&self.command_buffer_size)
            && (HEADER_SIZE..=MAX_CRB_BUFFER_SIZE).contains(&self.response_buffer_size)
            && self
                .command_buffer
                .checked_add(self.command_buffer_size)
                .is_some()
            && self
                .response_buffer
                .checked_add(self.response_buffer_size)
                .is_some()
    }

    fn request_command_ready(&self) -> Result<(), TpmError> {
        unsafe {
            write_u32(self.control_area + CTRL_REQ_OFFSET, CTRL_REQ_CMD_READY);
        }
        self.wait_u32_clear(self.control_area + CTRL_REQ_OFFSET, CTRL_REQ_CMD_READY)
    }

    fn start_command(&self) -> Result<(), TpmError> {
        unsafe {
            write_u32(self.control_area + CTRL_CANCEL_OFFSET, 0);
            compiler_fence(Ordering::SeqCst);
            write_u32(self.control_area + CTRL_START_OFFSET, CTRL_START);
        }
        Ok(())
    }

    fn wait_u32_clear(&self, address: usize, mask: u32) -> Result<(), TpmError> {
        for _ in 0..self.timeout_polls {
            let value = unsafe { read_u32(address) };
            if value & mask == 0 {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(TpmError::TpmResponseCode(TPM_CRB_TIMEOUT_CODE))
    }

    fn copy_to_command_buffer(&self, command: &[u8]) -> Result<(), TpmError> {
        if command.len() > self.command_buffer_size {
            return Err(TpmError::Protocol("TPM command exceeds CRB buffer".into()));
        }

        unsafe {
            for (offset, byte) in command.iter().enumerate() {
                write_volatile((self.command_buffer + offset) as *mut u8, *byte);
            }
        }

        Ok(())
    }

    fn read_response(&self) -> Result<Vec<u8>, TpmError> {
        if self.response_buffer_size < HEADER_SIZE {
            return Err(TpmError::Protocol(
                "TPM CRB response buffer too small".into(),
            ));
        }

        let mut header = [0u8; HEADER_SIZE];
        let response_size = 'wait_response: loop {
            for _ in 0..self.timeout_polls {
                unsafe { read_mmio_bytes(self.response_buffer, &mut header) };

                let size = read_u32_be(&header, 2) as usize;
                if (HEADER_SIZE..=self.response_buffer_size).contains(&size) {
                    break 'wait_response size;
                }

                core::hint::spin_loop();
            }

            return Err(TpmError::TpmResponseCode(TPM_CRB_TIMEOUT_CODE));
        };

        let mut response = vec![0u8; response_size];
        response[..HEADER_SIZE].copy_from_slice(&header);
        unsafe {
            read_mmio_bytes(
                self.response_buffer + HEADER_SIZE,
                &mut response[HEADER_SIZE..],
            )
        };
        Ok(response)
    }

    fn read_response_into(&self, response: &mut [u8]) -> Result<usize, TpmError> {
        if self.response_buffer_size < HEADER_SIZE || response.len() < HEADER_SIZE {
            return Err(TpmError::TpmResponseCode(TPM_CRB_TIMEOUT_CODE));
        }

        let mut header = [0u8; HEADER_SIZE];
        let response_size = 'wait_response: loop {
            for _ in 0..self.timeout_polls {
                unsafe { read_mmio_bytes(self.response_buffer, &mut header) };

                let size = read_u32_be(&header, 2) as usize;
                if (HEADER_SIZE..=self.response_buffer_size).contains(&size) {
                    break 'wait_response size;
                }

                core::hint::spin_loop();
            }

            return Err(TpmError::TpmResponseCode(TPM_CRB_TIMEOUT_CODE));
        };

        if response_size > response.len() {
            return Err(TpmError::TpmResponseCode(TPM_CRB_TIMEOUT_CODE));
        }

        response[..HEADER_SIZE].copy_from_slice(&header);
        unsafe {
            read_mmio_bytes(
                self.response_buffer + HEADER_SIZE,
                &mut response[HEADER_SIZE..response_size],
            )
        };
        Ok(response_size)
    }
}

impl TpmTransport for CrbTpmTransport {
    fn transact(&mut self, command: &[u8]) -> Result<Vec<u8>, TpmError> {
        self.request_command_ready()?;
        self.copy_to_command_buffer(command)?;
        self.start_command()?;
        self.read_response()
    }
}

impl FixedTpmTransport for CrbTpmTransport {
    fn transact_into(&mut self, command: &[u8], response: &mut [u8]) -> Result<usize, TpmError> {
        self.request_command_ready()?;
        self.copy_to_command_buffer(command)?;
        self.start_command()?;
        self.read_response_into(response)
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

unsafe fn read_u64(address: usize) -> u64 {
    let low = unsafe { read_u32(address) } as u64;
    let high = unsafe { read_u32(address + 4) } as u64;
    low | (high << 32)
}

unsafe fn read_mmio_bytes(address: usize, out: &mut [u8]) {
    let mut offset = 0;
    while offset < out.len() {
        let aligned = (address + offset) & !0x3;
        let word = unsafe { read_u32(aligned) }.to_le_bytes();
        while offset < out.len() && (address + offset) < aligned + 4 {
            let byte_index = (address + offset) - aligned;
            out[offset] = word[byte_index];
            offset += 1;
        }
    }
}
