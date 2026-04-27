//! Edgerun unikernel - bare shell with networking

#![no_std]
#![no_main]

extern crate edgerun_bare_rt as rt;
extern crate edgerun_dhcp;
extern crate edgerun_virtio;
extern crate edgerun_platform;

use edgerun_dhcp::message::{DHCP_CLIENT_PORT, DHCP_SERVER_PORT};
use edgerun_dhcp::{DhcpMessage, DhcpMessageType};
use rt::{
    block_on, crc32, IpAddr, IpStack, Network, Rng, RingBuffer, TcpSocket, TftpConfig,
};
use rt::ip::{ParsedPacket, ARP_OP_REQUEST, ICMP_ECHO_REQUEST};

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

struct PumpStats {
    arp_replies: u32,
    icmp_replies: u32,
}

struct NetPump<'net, 'stack> {
    net: &'net mut edgerun_virtio::VirtNet,
    network: Network<'stack>,
    rx_buf: [u8; 1514],
    logged_start: bool,
    logged_arp: bool,
    logged_icmp: bool,
}

fn poll_network(
    net: &mut edgerun_virtio::VirtNet,
    network: &mut Network<'_>,
    rx_buf: &mut [u8; 1514],
) -> PumpStats {
    let mut stats = PumpStats {
        arp_replies: 0,
        icmp_replies: 0,
    };

    while let Some(len) = net.recv(rx_buf) {
        match network.recv(&rx_buf[..len]) {
            Some(ParsedPacket::Arp { header, .. }) => {
                if header.oper == ARP_OP_REQUEST
                    && header.tpa == *network.stack.ip.as_bytes()
                    && network
                        .send_arp_reply(&header)
                        .map(|packet| net.send(packet))
                        .unwrap_or(false)
                {
                    stats.arp_replies = stats.arp_replies.wrapping_add(1);
                }
            }
            Some(ParsedPacket::Icmp {
                eth,
                ip,
                header,
                payload,
            }) => {
                if header.icmp_type == ICMP_ECHO_REQUEST
                    && ip.dst == *network.stack.ip.as_bytes()
                    && network
                        .send_icmp_echo_reply(
                            eth.src,
                            IpAddr::from_slice(&ip.src),
                            &header,
                            payload,
                        )
                        .map(|packet| net.send(packet))
                        .unwrap_or(false)
                {
                    stats.icmp_replies = stats.icmp_replies.wrapping_add(1);
                }
            }
            _ => {}
        }
    }

    stats
}

fn dhcp_ipv4_to_rt(ip: edgerun_dhcp::Ipv4Addr) -> IpAddr {
    let octets = ip.octets();
    IpAddr::new(octets[0], octets[1], octets[2], octets[3])
}

impl Future for NetPump<'_, '_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if !this.logged_start {
            rt::log::log(1, "Net pump started");
            this.logged_start = true;
        }

        let stats = poll_network(this.net, &mut this.network, &mut this.rx_buf);
        if stats.arp_replies != 0 && !this.logged_arp {
            rt::log::log(1, "ARP reply sent");
            this.logged_arp = true;
        }
        if stats.icmp_replies != 0 && !this.logged_icmp {
            rt::log::log(1, "ICMP echo reply sent");
            this.logged_icmp = true;
        }

        cx.waker().wake_by_ref();
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
    
    let dhcp_xid = 0x12345678;
    let mut dhcp_ip = IpAddr::zero();
    let mut dhcp_netmask = IpAddr::new(255, 255, 255, 0);
    let mut dhcp_gateway = IpAddr::zero();
    let mut network = Network::new(&mut stack);
    
    let discover = DhcpMessage::discover(dhcp_xid, mac).to_wire();
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
    let mut logged_rx = false;
    for _ in 0..1000 {
        if let Some(len) = net.recv(&mut rx_buf) {
            if !logged_rx {
                rt::log::log(1, "VirtIO RX packet observed");
                logged_rx = true;
            }
            if let Some(ParsedPacket::Udp { header, payload, .. }) = network.recv(&rx_buf[..len]) {
                if header.src_port == DHCP_SERVER_PORT && header.dst_port == DHCP_CLIENT_PORT {
                    if let Ok(message) = DhcpMessage::from_wire(payload) {
                        let lease_response = matches!(
                            message.options.message_type,
                            Some(DhcpMessageType::Offer) | Some(DhcpMessageType::Ack)
                        );
                        if message.xid == dhcp_xid && lease_response && message.chaddr[..6] == mac {
                            dhcp_ip = dhcp_ipv4_to_rt(message.yiaddr);
                            if let Some(netmask) = message.options.subnet_mask {
                                dhcp_netmask = dhcp_ipv4_to_rt(netmask);
                            }
                            if let Some(gateway) = message.options.router {
                                dhcp_gateway = dhcp_ipv4_to_rt(gateway);
                            }
                            rt::log::log(1, "DHCP lease accepted");
                            break;
                        }
                    }
                }
            }
        }
    }

    let net_stats = net.stats();
    if net_stats.tx_completed != 0 {
        rt::log::log(1, "VirtIO TX completed");
    } else {
        rt::log::log(1, "VirtIO TX pending");
    }
    if net_stats.rx_received == 0 {
        rt::log::log(1, "VirtIO RX no packets");
    }

    drop(network);
    
    if dhcp_ip != IpAddr::zero() {
        stack.ip = dhcp_ip;
        stack.netmask = dhcp_netmask;
        stack.gateway = dhcp_gateway;
    } else {
        rt::log::log(1, "Using static fallback IP");
        stack.ip = IpAddr::new(192, 168, 1, 12);
    }

    let network = Network::new(&mut stack);
    
    let _tftp = TftpConfig::new(0xC0A80101, "edgerun.bin");
    let mut tcp = TcpSocket::new();
    
    let addr = rt::SocketAddr::new(0xC0A8010C, 8080);
    let _ = tcp.bind(addr);
    let _ = tcp.listen(10);
    let _ = rng.next();
    
    if net.is_link_up() {
    }

    block_on(NetPump {
        net: &mut net,
        network,
        rx_buf: [0; 1514],
        logged_start: false,
        logged_arp: false,
        logged_icmp: false,
    });

    loop {
        core::arch::asm!("hlt");
    }
}
