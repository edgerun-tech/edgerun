#![no_std]
#![no_main]

mod ble;
mod hci;
mod realtek;
mod uefi;
mod usb;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    unsafe {
        uefi::puts("\r\nPANIC\r\n");
    }
    loop {
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(
    image_handle: uefi::EfiHandle,
    system_table: *mut uefi::EfiSystemTable,
) -> uefi::EfiStatus {
    unsafe {
        uefi::init(image_handle, system_table);
        uefi::puts("\r\nEdgeRun UEFI Bluetooth bring-up\r\n");
        uefi::puts("target: RTL8922AE USB Bluetooth, no external crates\r\n");
    }

    let device = match unsafe { usb::find_rtl8922_controller() } {
        Ok(device) => device,
        Err(status) => {
            unsafe {
                uefi::puts("No usable RTL8922AE Bluetooth USB function found: ");
                uefi::put_status(status);
                uefi::puts("\r\n");
            }
            return status;
        }
    };

    unsafe { device.describe() };

    let mut hci = hci::Hci::new(device);

    match realtek::init_rtl8922a(&mut hci) {
        Ok(()) => unsafe { uefi::puts("Realtek HCI init path completed\r\n") },
        Err(status) => {
            unsafe {
                uefi::puts("Realtek HCI init failed: ");
                uefi::put_status(status);
                uefi::puts("\r\n");
            }
            return status;
        }
    }

    match ble::start_advertising(&mut hci, b"ER-EFI-ADMIN") {
        Ok(()) => unsafe { uefi::puts("BLE advertising enabled as ER-EFI-ADMIN\r\n") },
        Err(status) => {
            unsafe {
                uefi::puts("BLE advertising failed: ");
                uefi::put_status(status);
                uefi::puts("\r\n");
            }
            return status;
        }
    }

    unsafe {
        uefi::puts("Bluetooth controller is alive; staying in firmware loop.\r\n");
    }

    loop {
        unsafe { uefi::stall_us(250_000) };
    }
}
