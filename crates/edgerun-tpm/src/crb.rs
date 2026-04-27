use alloc::vec;
use alloc::vec::Vec;
use core::ptr::{read_volatile, write_volatile};

use crate::acpi::AcpiTpm2Info;
use crate::traits::TpmTransport;
use crate::types::TpmError;

const HEADER_SIZE: usize = 10;
const DEFAULT_TIMEOUT_POLLS: usize = 1_000_000;

const CTRL_REQ_OFFSET: usize = 0x40;
const CTRL_START_OFFSET: usize = 0x4c;
const CTRL_CMD_SIZE_OFFSET: usize = 0x58;
const CTRL_CMD_ADDR_OFFSET: usize = 0x5c;
const CTRL_RSP_SIZE_OFFSET: usize = 0x64;
const CTRL_RSP_ADDR_OFFSET: usize = 0x68;

const CTRL_REQ_CMD_READY: u32 = 1 << 0;
const CTRL_START: u32 = 1 << 0;

/// TPM 2.0 Command Response Buffer transport over memory-mapped registers.
///
/// The `control_area` should come from the ACPI `TPM2` table. For early board
/// bring-up, `from_control_area` can also be pointed at a known CRB base such
/// as `0xfed4_0000` on many x86 systems.
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

    /// Create a CRB transport by reading buffer descriptors from the control area.
    ///
    /// # Safety
    ///
    /// `control_area` must point at a TPM2 CRB control area. This performs
    /// volatile reads from that region.
    pub unsafe fn from_control_area(control_area: usize) -> Self {
        let command_buffer_size = unsafe { read_u32(control_area + CTRL_CMD_SIZE_OFFSET) } as usize;
        let command_buffer = unsafe { read_u64(control_area + CTRL_CMD_ADDR_OFFSET) } as usize;
        let response_buffer_size =
            unsafe { read_u32(control_area + CTRL_RSP_SIZE_OFFSET) } as usize;
        let response_buffer = unsafe { read_u64(control_area + CTRL_RSP_ADDR_OFFSET) } as usize;

        Self {
            control_area,
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
        Ok(unsafe { Self::from_control_area(info.control_area as usize) })
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

    pub fn response_buffer(&self) -> usize {
        self.response_buffer
    }

    fn request_command_ready(&self) -> Result<(), TpmError> {
        unsafe {
            write_u32(self.control_area + CTRL_REQ_OFFSET, CTRL_REQ_CMD_READY);
        }
        self.wait_u32_clear(self.control_area + CTRL_REQ_OFFSET, CTRL_REQ_CMD_READY)
    }

    fn start_command(&self) -> Result<(), TpmError> {
        unsafe {
            write_u32(self.control_area + CTRL_START_OFFSET, CTRL_START);
        }
        self.wait_u32_clear(self.control_area + CTRL_START_OFFSET, CTRL_START)
    }

    fn wait_u32_clear(&self, address: usize, mask: u32) -> Result<(), TpmError> {
        for _ in 0..self.timeout_polls {
            let value = unsafe { read_u32(address) };
            if value & mask == 0 {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(TpmError::Io("TPM CRB timeout".into()))
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
        unsafe {
            for (offset, byte) in header.iter_mut().enumerate() {
                *byte = read_volatile((self.response_buffer + offset) as *const u8);
            }
        }

        let response_size =
            u32::from_be_bytes([header[2], header[3], header[4], header[5]]) as usize;
        if response_size < HEADER_SIZE {
            return Err(TpmError::Protocol(
                "TPM response shorter than header".into(),
            ));
        }
        if response_size > self.response_buffer_size {
            return Err(TpmError::Protocol("TPM response exceeds CRB buffer".into()));
        }

        let mut response = vec![0u8; response_size];
        response[..HEADER_SIZE].copy_from_slice(&header);
        unsafe {
            for (offset, byte) in response.iter_mut().enumerate().skip(HEADER_SIZE) {
                *byte = read_volatile((self.response_buffer + offset) as *const u8);
            }
        }
        Ok(response)
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
