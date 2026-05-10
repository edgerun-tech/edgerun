#![no_std]
#![no_main]

extern crate alloc;

use core::cell::UnsafeCell;
use core::panic::PanicInfo;

use edgerun_platform as _;
use edgerun_protocols::ethernet_ipv4::IpAddr;
use edgerun_relay::nostd_virtio::{EthernetRelay, EthernetRelayEvent, VirtioRelay};
use edgerun_virtio::{VirtRng, find_initialized_virtio_net, find_initialized_virtio_rng};

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    r#"
.section .multiboot
.align 4
.long 0x1BADB002
.long 0x00000003
.long -(0x1BADB002 + 0x00000003)

.section .text.entry
.global _start
_start:
    cli
    lea rsp, [rip + _stack]
    call edgerun_unikernel_relay_virtio_main
1:
    hlt
    jmp 1b
"#
);

const RELAY_IP: IpAddr = IpAddr::new(10, 0, 2, 15);
const RELAY_NETMASK: IpAddr = IpAddr::new(255, 255, 255, 0);
const RELAY_GATEWAY: IpAddr = IpAddr::new(10, 0, 2, 2);
const RELAY_PORT: u16 = 7999;

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_unikernel_relay_virtio_main() -> ! {
    debug_write(b"edgerun-unikernel relay virtio: start\n");

    let mut rng = match find_initialized_virtio_rng() {
        Some(rng) => rng,
        None => {
            debug_write(b"edgerun-unikernel relay virtio: missing virtio-rng\n");
            qemu_exit(2);
        }
    };
    let mut rng_probe = [0u8; 32];
    if !rng.fill_bytes(&mut rng_probe) {
        debug_write(b"edgerun-unikernel relay virtio: virtio-rng failed\n");
        qemu_exit(3);
    }
    install_rng(rng);
    debug_write(b"edgerun-unikernel relay virtio: rng ready\n");

    let mut net = match find_initialized_virtio_net() {
        Some(net) => net,
        None => {
            debug_write(b"edgerun-unikernel relay virtio: missing virtio-net\n");
            qemu_exit(4);
        }
    };
    let mac = net.get_mac();
    debug_write(b"edgerun-unikernel relay virtio: net ready\n");

    let wss_config = match edgerun_relay::self_signed_wss_config(&["localhost"]) {
        Ok(config) => config,
        Err(_) => {
            debug_write(b"edgerun-unikernel relay virtio: wss config failed\n");
            qemu_exit(5);
        }
    };
    debug_write(b"edgerun-unikernel relay virtio: wss ready\n");

    let ethernet = EthernetRelay::new(RELAY_IP, RELAY_NETMASK, RELAY_GATEWAY, mac, RELAY_PORT);
    let mut relay = VirtioRelay::with_wss_config(ethernet, wss_config);
    let mut events = alloc::vec::Vec::<EthernetRelayEvent>::new();
    debug_write(b"edgerun-unikernel relay virtio: relay ready\n");

    #[cfg(not(feature = "relay-virtio-smoke"))]
    let mut now_ms = 0u64;
    #[cfg(feature = "relay-virtio-smoke")]
    let now_ms = 0u64;
    loop {
        events.clear();
        relay.poll(&mut net, now_ms, &mut events);

        #[cfg(feature = "relay-virtio-smoke")]
        {
            debug_write(b"edgerun-unikernel relay virtio: poll ok\n");
            qemu_exit(0);
        }

        #[cfg(not(feature = "relay-virtio-smoke"))]
        {
            now_ms = now_ms.wrapping_add(10);
            spin();
        }
    }
}

struct RngSlot(UnsafeCell<Option<VirtRng>>);

unsafe impl Sync for RngSlot {}

static VIRTIO_RNG: RngSlot = RngSlot(UnsafeCell::new(None));

fn install_rng(rng: VirtRng) {
    unsafe {
        *VIRTIO_RNG.0.get() = Some(rng);
    }
    edgerun_crypto::register_random_source(virtio_random);
}

fn virtio_random(buf: &mut [u8]) -> edgerun_crypto::error::Result<()> {
    unsafe {
        let slot = &mut *VIRTIO_RNG.0.get();
        let Some(rng) = slot.as_mut() else {
            return Err(edgerun_crypto::error::CryptoError::RandomGenerationFailed);
        };
        rng.try_fill_bytes(buf)
            .map_err(|_| edgerun_crypto::error::CryptoError::RandomGenerationFailed)
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    debug_write(b"edgerun-unikernel relay virtio: panic\n");
    qemu_exit(1);
}

fn debug_write(bytes: &[u8]) {
    for &byte in bytes {
        unsafe {
            outb(0xe9, byte);
        }
    }
}

#[cfg(not(feature = "relay-virtio-smoke"))]
fn spin() {
    for _ in 0..10_000 {
        core::hint::spin_loop();
    }
}

fn qemu_exit(code: u32) -> ! {
    unsafe {
        core::arch::asm!(
            "mov dx, 0xf4",
            "mov eax, {code:e}",
            "out dx, eax",
            code = in(reg) code,
            options(noreturn)
        );
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn outb(port: u16, value: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") value);
    }
}
