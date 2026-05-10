use crate::uefi::{
    self, EFI_DEVICE_ERROR, EFI_INVALID_PARAMETER, EFI_NOT_READY, EFI_SUCCESS, EFI_TIMEOUT,
    EfiStatus,
};
use crate::usb::BtUsb;

pub const OGF_CONTROLLER_BASEBAND: u16 = 0x03;
pub const OGF_INFORMATIONAL: u16 = 0x04;
pub const OGF_LE_CONTROLLER: u16 = 0x08;
pub const OGF_VENDOR: u16 = 0x3f;

pub const OCF_RESET: u16 = 0x0003;
pub const OCF_READ_LOCAL_VERSION: u16 = 0x0001;
pub const OCF_READ_BD_ADDR: u16 = 0x0009;

pub const HCI_RESET: u16 = hci_opcode(OGF_CONTROLLER_BASEBAND, OCF_RESET);
pub const HCI_READ_LOCAL_VERSION: u16 = hci_opcode(OGF_INFORMATIONAL, OCF_READ_LOCAL_VERSION);
pub const HCI_READ_BD_ADDR: u16 = hci_opcode(OGF_INFORMATIONAL, OCF_READ_BD_ADDR);

pub const EVENT_COMMAND_COMPLETE: u8 = 0x0e;
pub const EVENT_COMMAND_STATUS: u8 = 0x0f;

pub const fn hci_opcode(ogf: u16, ocf: u16) -> u16 {
    (ogf << 10) | ocf
}

#[derive(Clone, Copy, Default)]
pub struct LocalVersion {
    pub hci_version: u8,
    pub hci_revision: u16,
    pub lmp_pal_version: u8,
    pub manufacturer: u16,
    pub lmp_pal_subversion: u16,
}

pub struct Hci {
    usb: BtUsb,
}

impl Hci {
    pub const fn new(usb: BtUsb) -> Self {
        Self { usb }
    }

    pub fn send_command(&mut self, opcode: u16, params: &[u8]) -> EfiStatus {
        if params.len() > 255 {
            return EFI_INVALID_PARAMETER;
        }
        let mut packet = [0u8; 258];
        packet[0] = opcode as u8;
        packet[1] = (opcode >> 8) as u8;
        packet[2] = params.len() as u8;
        let mut i = 0usize;
        while i < params.len() {
            packet[3 + i] = params[i];
            i += 1;
        }
        self.usb.send_hci_command_packet(&mut packet[..3 + params.len()])
    }

    pub fn command(
        &mut self,
        opcode: u16,
        params: &[u8],
        out: &mut [u8],
    ) -> Result<usize, EfiStatus> {
        let status = self.send_command(opcode, params);
        if status != EFI_SUCCESS {
            return Err(status);
        }
        self.wait_command_complete(opcode, out, 200)
    }

    pub fn wait_command_complete(
        &mut self,
        expected_opcode: u16,
        out: &mut [u8],
        max_polls: usize,
    ) -> Result<usize, EfiStatus> {
        if out.len() < 6 {
            return Err(EFI_INVALID_PARAMETER);
        }

        let mut polls = 0usize;
        while polls < max_polls {
            match self.usb.read_hci_event(out, 50_000) {
                Ok(n) => {
                    if n < 3 {
                        polls += 1;
                        continue;
                    }
                    let event_code = out[0];
                    let param_len = out[1] as usize;
                    if n < 2 + param_len {
                        return Err(EFI_DEVICE_ERROR);
                    }

                    if event_code == EVENT_COMMAND_COMPLETE {
                        if param_len < 4 || n < 6 {
                            return Err(EFI_DEVICE_ERROR);
                        }
                        let opcode = u16::from_le_bytes([out[3], out[4]]);
                        let status = out[5];
                        if opcode == expected_opcode {
                            if status == 0 {
                                return Ok(n);
                            }
                            unsafe {
                                uefi::puts("HCI command failed opcode=0x");
                                uefi::put_hex_u16(opcode);
                                uefi::puts(" hci_status=0x");
                                uefi::put_hex_u8(status);
                                uefi::puts("\r\n");
                            }
                            return Err(EFI_DEVICE_ERROR);
                        }
                    } else if event_code == EVENT_COMMAND_STATUS {
                        if param_len < 4 || n < 6 {
                            return Err(EFI_DEVICE_ERROR);
                        }
                        let status = out[2];
                        let opcode = u16::from_le_bytes([out[4], out[5]]);
                        if opcode == expected_opcode && status != 0 {
                            return Err(EFI_DEVICE_ERROR);
                        }
                    }
                }
                Err(status) => {
                    if status != EFI_TIMEOUT && status != EFI_NOT_READY {
                        return Err(status);
                    }
                }
            }
            polls += 1;
        }
        Err(EFI_TIMEOUT)
    }

    pub fn reset(&mut self) -> Result<(), EfiStatus> {
        let mut event = [0u8; 260];
        self.command(HCI_RESET, &[], &mut event).map(|_| ())
    }

    pub fn read_local_version(&mut self) -> Result<LocalVersion, EfiStatus> {
        let mut event = [0u8; 260];
        let n = self.command(HCI_READ_LOCAL_VERSION, &[], &mut event)?;
        if n < 14 {
            return Err(EFI_DEVICE_ERROR);
        }
        Ok(LocalVersion {
            hci_version: event[6],
            hci_revision: u16::from_le_bytes([event[7], event[8]]),
            lmp_pal_version: event[9],
            manufacturer: u16::from_le_bytes([event[10], event[11]]),
            lmp_pal_subversion: u16::from_le_bytes([event[12], event[13]]),
        })
    }

    pub fn read_bd_addr(&mut self) -> Result<[u8; 6], EfiStatus> {
        let mut event = [0u8; 260];
        let n = self.command(HCI_READ_BD_ADDR, &[], &mut event)?;
        if n < 12 {
            return Err(EFI_DEVICE_ERROR);
        }
        Ok([event[6], event[7], event[8], event[9], event[10], event[11]])
    }

    pub fn write_acl(&mut self, data: &mut [u8]) -> Result<usize, EfiStatus> {
        self.usb.write_acl(data, 1_000_000)
    }

    pub fn read_acl(&mut self, out: &mut [u8]) -> Result<usize, EfiStatus> {
        self.usb.read_acl(out, 1_000_000)
    }
}
