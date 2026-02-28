// SPDX-License-Identifier: Apache-2.0
#![no_main]
#![no_std]

use core::panic::PanicInfo;

use uefi::prelude::*;

#[entry]
fn efi_main() -> Status {
    // Fail-closed placeholder for Phase B wiring.
    // Network policy fetch and Linux handoff will be added next.
    Status::SECURITY_VIOLATION
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}
