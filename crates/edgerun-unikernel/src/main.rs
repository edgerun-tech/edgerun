//! Edgerun unikernel - bare shell with networking

#![no_std]
#![no_main]

extern crate edgerun_bare_rt as rt;
extern crate edgerun_virtio;
extern crate edgerun_platform;

use rt::{
    block_on, crc32, runtime::spawn, DhcpClient, DhcpStateMachine, IpAddr, IpStack, Network, Rng,
    RingBuffer, TcpSocket, TftpConfig, DHCP_CLIENT_PORT, DHCP_SERVER_PORT,
};
use rt::ip::ParsedPacket;

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

core::arch::global_asm!(
    r#"
    .section .text.entry,"ax"
    .global _start
_start:
    lea rsp, [rip + _stack]
    xor rbp, rbp

    lea rdi, [rip + _bss_start]
    lea rcx, [rip + _bss_end]
    sub rcx, rdi
    xor eax, eax
    rep stosb

    call kernel_main

1:
    hlt
    jmp 1b
"#
);

struct NetworkTask;

impl Future for NetworkTask {
    type Output = ();
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        rt::log::log(3, "Network task");
        Poll::Pending
    }
}

#[panic_handler]
unsafe fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::arch::asm!("hlt");
    }
}

#[used]
#[link_section = ".multiboot"]
static MULTIBOOT_HEADER: [u32; 8] = [
    0x1BADB002,
    0x00010000,
    0xE4514FFE,
    0x100000,
    0x100000,
    0,
    0,
    0,
];

#[no_mangle]
pub unsafe extern "C" fn kernel_main() -> ! {
    rt::timer::set_now(0);
    rt::log::log(1, "Starting edgerun unikernel");
    
    let mut rng = Rng::new_from_entropy();
    let test_crc = crc32(b"hello");
    let _ = test_crc;
    
    let mut rx_buf = RingBuffer::new(1024);
    rx_buf.push_slice(b"test packet");
    let _ = rx_buf.len();
    
    rt::log::log(1, "Looking for VirtIO...");
    
    let mut net = match edgerun_virtio::find_virtio_net() {
        Some(n) => n,
        None => {
            rt::log::log(1, "No VirtIO found");
            loop {
                core::arch::asm!("hlt");
            }
        }
    };
    
    rt::log::log(1, "VirtIO found");
    if !net.init() {
        rt::log::log(1, "VirtIO init failed");
        loop {
            core::arch::asm!("hlt");
        }
    }
    let mac = net.get_mac();
    
    let mut stack = IpStack::new();
    stack.configure(
        IpAddr::new(0, 0, 0, 0),
        IpAddr::new(255, 255, 255, 0),
        IpAddr::zero(),
        mac,
    );
    
    let mut dhcp = DhcpStateMachine::new(mac);
    let mut network = Network::new(&mut stack);
    
    let discover = dhcp.discover();
    rt::log::log(1, "Sending DHCP discover");
    if let Some(pkt) = network.send_udp(
        IpAddr::new(255, 255, 255, 255),
        DHCP_CLIENT_PORT,
        DHCP_SERVER_PORT,
        &discover,
    ) {
        if net.send(pkt) {
            rt::log::log(1, "DHCP discover queued");
        } else {
            rt::log::log(1, "DHCP discover send failed");
        }
    }
    
    let mut rx_buf = [0u8; 1514];
    for _ in 0..1000 {
        if let Some(len) = net.recv(&mut rx_buf) {
            if let Some(ParsedPacket::Udp { header, payload }) = network.recv(&rx_buf[..len]) {
                if header.src_port == DHCP_SERVER_PORT && header.dst_port == DHCP_CLIENT_PORT {
                    if dhcp.parse(payload) {
                        rt::log::log(1, "DHCP lease accepted");
                        break;
                    }
                }
            }
        }
    }

    drop(network);
    
    if dhcp.ip != IpAddr::zero() {
        stack.ip = dhcp.ip;
        stack.netmask = dhcp.netmask;
        stack.gateway = dhcp.gateway;
    } else {
        rt::log::log(1, "Using static fallback IP");
        stack.ip = IpAddr::new(192, 168, 1, 12);
    }

    let mut network = Network::new(&mut stack);
    
    for _ in 0..100 {
        let mut rx_buf = [0u8; 1514];
        if let Some(len) = net.recv(&mut rx_buf) {
            if let Some(pkt) = network.recv(&rx_buf[..len]) {
                if pkt.is_icmp() {
                    break;
                }
            }
        }
    }
    
    let _dhcp = DhcpClient::new(mac);
    let _tftp = TftpConfig::new(0xC0A80101, "edgerun.bin");
    let mut tcp = TcpSocket::new();
    
    let addr = rt::SocketAddr::new(0xC0A8010C, 8080);
    let _ = tcp.bind(addr);
    let _ = tcp.listen(10);
    let _ = rng.next();
    
    spawn(NetworkTask);
    
    if net.is_link_up() {
    }
    
    let (_ip, _, _) = (stack.ip, stack.netmask, stack.gateway);
    
    block_on(NetworkTask);

    loop {
        core::arch::asm!("hlt");
    }
}
