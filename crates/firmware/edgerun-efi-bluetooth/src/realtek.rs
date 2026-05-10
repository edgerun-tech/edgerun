use crate::hci::{self, Hci, LocalVersion};
use crate::uefi::{self, EFI_DEVICE_ERROR, EFI_SUCCESS, EfiStatus};

const REALTEK_COMPANY_ID: u16 = 0x005d;

// Realtek vendor opcodes used by the Linux btrtl path. The download command is
// intentionally not fully wired yet: RTL8922A firmware is EPATCH-formatted and
// must be parsed/fragmented exactly, not guessed.
const OCF_RTK_READ_ROM_VERSION: u16 = 0x006d;
const OCF_RTK_DOWNLOAD_FW: u16 = 0x0020;

const HCI_RTK_READ_ROM_VERSION: u16 = hci::hci_opcode(hci::OGF_VENDOR, OCF_RTK_READ_ROM_VERSION);
#[allow(dead_code)]
const HCI_RTK_DOWNLOAD_FW: u16 = hci::hci_opcode(hci::OGF_VENDOR, OCF_RTK_DOWNLOAD_FW);

#[cfg(feature = "embedded-rtl8922au-fw")]
static RTL8922AU_FW: &[u8] = include_bytes!("../firmware/rtl8922au_fw.bin");

#[cfg(feature = "embedded-rtl8922au-fw")]
static RTL8922AU_CONFIG: &[u8] = include_bytes!("../firmware/rtl8922au_config.bin");

#[cfg(not(feature = "embedded-rtl8922au-fw"))]
static RTL8922AU_FW: &[u8] = &[];

#[cfg(not(feature = "embedded-rtl8922au-fw"))]
static RTL8922AU_CONFIG: &[u8] = &[];

pub fn init_rtl8922a(hci: &mut Hci) -> Result<(), EfiStatus> {
    unsafe { uefi::puts("HCI reset...\r\n") };
    hci.reset()?;

    let version = hci.read_local_version()?;
    print_version(version);

    if version.manufacturer != REALTEK_COMPANY_ID {
        unsafe { uefi::puts("warning: controller manufacturer is not Realtek\r\n") };
    }

    match read_rom_version(hci) {
        Ok(rom) => {
            unsafe {
                uefi::puts("Realtek ROM version: 0x");
                uefi::put_hex_u8(rom);
                uefi::puts("\r\n");
            }
        }
        Err(status) => {
            unsafe {
                uefi::puts("Realtek ROM version read failed: ");
                uefi::put_status(status);
                uefi::puts("\r\n");
            }
        }
    }

    if !RTL8922AU_FW.is_empty() {
        unsafe {
            uefi::puts("embedded rtl8922au_fw.bin bytes: 0x");
            uefi::put_hex_usize(RTL8922AU_FW.len());
            uefi::puts(" config bytes: 0x");
            uefi::put_hex_usize(RTL8922AU_CONFIG.len());
            uefi::puts("\r\n");
        }
        upload_rtl8922a_firmware_stub(hci, RTL8922AU_FW, RTL8922AU_CONFIG)?;
        hci.reset()?;
    } else {
        unsafe {
            uefi::puts("rtl8922au firmware not embedded; trying controller as-is\r\n");
            uefi::puts("enable feature embedded-rtl8922au-fw after adding firmware files\r\n");
        }
    }

    match hci.read_bd_addr() {
        Ok(addr) => print_bd_addr(addr),
        Err(status) => {
            unsafe {
                uefi::puts("BD_ADDR read failed: ");
                uefi::put_status(status);
                uefi::puts("\r\n");
            }
            return Err(status);
        }
    }

    Ok(())
}

fn read_rom_version(hci: &mut Hci) -> Result<u8, EfiStatus> {
    let mut event = [0u8; 260];
    let n = hci.command(HCI_RTK_READ_ROM_VERSION, &[], &mut event)?;
    if n < 7 {
        return Err(EFI_DEVICE_ERROR);
    }
    Ok(event[6])
}

fn upload_rtl8922a_firmware_stub(
    _hci: &mut Hci,
    _fw: &[u8],
    _config: &[u8],
) -> Result<(), EfiStatus> {
    // TODO: Port the minimal RTL8922A branch from Linux drivers/bluetooth/btrtl.c:
    // - verify EPATCH signature
    // - select the patch by ROM/LMP subversion
    // - append/use rtl8922au_config.bin exactly as Realtek expects
    // - send fragments through HCI vendor opcode 0xfc20
    // - mark the final fragment with the Realtek end marker
    //
    // This is deliberately not guessed. Sending malformed vendor firmware chunks
    // can wedge the controller until power-cycle and makes debugging miserable in
    // UEFI. The current crate gets USB/HCI discovery, reset, version, BD_ADDR, and
    // BLE advertising path in place first.
    unsafe { uefi::puts("RTL8922A firmware upload parser is not implemented yet\r\n") };
    Err(EFI_DEVICE_ERROR)
}

fn print_version(version: LocalVersion) {
    unsafe {
        uefi::puts("HCI version=0x");
        uefi::put_hex_u8(version.hci_version);
        uefi::puts(" hci_rev=0x");
        uefi::put_hex_u16(version.hci_revision);
        uefi::puts(" lmp=0x");
        uefi::put_hex_u8(version.lmp_pal_version);
        uefi::puts(" manufacturer=0x");
        uefi::put_hex_u16(version.manufacturer);
        uefi::puts(" lmp_sub=0x");
        uefi::put_hex_u16(version.lmp_pal_subversion);
        uefi::puts("\r\n");
    }
}

fn print_bd_addr(addr: [u8; 6]) {
    unsafe {
        uefi::puts("BD_ADDR=");
        let mut i = 6usize;
        while i > 0 {
            i -= 1;
            uefi::put_hex_u8(addr[i]);
            if i != 0 {
                uefi::puts(":");
            }
        }
        uefi::puts("\r\n");
    }
}
