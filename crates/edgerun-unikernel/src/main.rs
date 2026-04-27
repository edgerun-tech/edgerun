//! Edgerun unikernel - bare shell with networking

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

extern crate edgerun_dhcp;
extern crate edgerun_platform;
extern crate edgerun_rt as rt;
extern crate edgerun_tftp;
extern crate edgerun_tpm;
extern crate edgerun_virtio;

use edgerun_dhcp::message::{DHCP_CLIENT_PORT, DHCP_SERVER_PORT};
use edgerun_dhcp::{DhcpMessage, DhcpMessageType};
use edgerun_tftp::message::{TftpMessage, TFTP_PORT};
use rt::ip::{ParsedPacket, ARP_OP_REQUEST, ICMP_ECHO_REQUEST};
use rt::{block_on, crc32, IpAddr, IpStack, Network, RingBuffer, Rng, TcpSocket};

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

#[cfg(target_os = "none")]
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

#[cfg(target_os = "none")]
unsafe fn probe_tpm2() -> Option<[u8; 32]> {
    rt::log::log(1, "Looking for TPM2 ACPI table...");
    match unsafe { edgerun_tpm::CrbTpmTransport::discover_acpi() } {
        Ok(Some(transport)) => {
            rt::log::log(1, "TPM2 CRB transport discovered");
            return probe_tpm2_transport(transport.with_timeout_polls(10));
        }
        Ok(None) => {
            rt::log::log(1, "No TPM2 ACPI table found");
        }
        Err(_) => {
            rt::log::log(1, "TPM2 ACPI discovery failed");
        }
    }

    rt::log::log(1, "Trying TPM2 TIS transport...");
    let transport = unsafe { edgerun_tpm::TisTpmTransport::new_default_x86() };
    probe_tpm2_transport(transport.with_timeout_polls(100_000))
}

#[cfg(target_os = "none")]
fn fill_tpm2_random_source(out: &mut [u8]) -> edgerun_crypto::error::Result<()> {
    if out.is_empty() {
        return Ok(());
    }

    if let Ok(Some(transport)) = unsafe { edgerun_tpm::CrbTpmTransport::discover_acpi() } {
        if fill_tpm2_random_transport(transport.with_timeout_polls(10), out) {
            return Ok(());
        }
    }

    let transport = unsafe { edgerun_tpm::TisTpmTransport::new_default_x86() };
    if fill_tpm2_random_transport(transport.with_timeout_polls(100_000), out) {
        Ok(())
    } else {
        Err(edgerun_crypto::error::CryptoError::TpmUnavailable)
    }
}

#[cfg(target_os = "none")]
fn fill_tpm2_random_transport<T>(transport: T, out: &mut [u8]) -> bool
where
    T: edgerun_tpm::FixedTpmTransport + edgerun_tpm::TpmTransport,
{
    let mut device = edgerun_tpm::TpmDevice::new(transport);
    let startup_code = device.startup_response_code(edgerun_tpm::TPM_SU_CLEAR);
    if startup_code != edgerun_tpm::TPM_RC_SUCCESS && startup_code != 0x100 && startup_code != 0x120
    {
        return false;
    }

    let mut offset = 0usize;
    while offset < out.len() {
        let end = core::cmp::min(offset + 2048, out.len());
        let n = device.get_random_into(&mut out[offset..end]);
        if n == 0 || offset + n > end {
            return false;
        }
        offset += n;
    }
    true
}

#[cfg(target_os = "none")]
fn probe_tpm2_transport<T>(transport: T) -> Option<[u8; 32]>
where
    T: edgerun_tpm::FixedTpmTransport + edgerun_tpm::TpmTransport,
{
    let mut device = edgerun_tpm::TpmDevice::new(transport);
    let startup_code = device.startup_response_code(edgerun_tpm::TPM_SU_CLEAR);
    if startup_code == edgerun_tpm::TPM_RC_SUCCESS || startup_code == 0x100 || startup_code == 0x120
    {
        rt::log::log(1, "TPM2 startup ok");
    } else if startup_code == 0x144 {
        rt::log::log(1, "TPM2 startup failed: command size");
        return None;
    } else if startup_code == 0xffff_fffb {
        rt::log::log(1, "TPM2 startup failed: transport");
        return None;
    } else if startup_code == 0xffff_fffc {
        rt::log::log(1, "TPM2 startup failed: malformed response");
        return None;
    } else {
        rt::log::log(1, "TPM2 startup failed");
        return None;
    }

    let mut tpm_random = [0u8; 32];
    let random_len = device.get_random_into(&mut tpm_random);
    let entropy = if random_len != 0 {
        rt::log::log(1, "TPM2 random ok");
        edgerun_crypto::rng::mix_entropy(&tpm_random[..random_len]);
        Some(tpm_random)
    } else {
        rt::log::log(1, "TPM2 random failed");
        None
    };

    let Some(entropy) = entropy else {
        return None;
    };

    let read_public_code = device.read_public_response_code(edgerun_tpm::TpmHandle(0x8100_0001));
    if read_public_code == edgerun_tpm::TPM_RC_SUCCESS {
        rt::log::log(1, "TPM2 persistent key readable");
    } else {
        rt::log::log(1, "TPM2 persistent key not readable");
        return Some(entropy);
    }

    let digest = [0u8; 32];
    let mut signature = [0u8; 64];
    if device
        .sign_p256_sha256_into(edgerun_tpm::TpmHandle(0x8100_0001), &digest, &mut signature)
        .is_ok()
    {
        rt::log::log(1, "TPM2 sign ok");
    } else {
        rt::log::log(1, "TPM2 sign failed");
    }

    Some(entropy)
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

#[cfg(target_os = "none")]
#[panic_handler]
unsafe fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::arch::asm!("hlt");
    }
}

#[used]
#[cfg(target_os = "none")]
#[link_section = ".multiboot"]
static MULTIBOOT_HEADER: [u32; 8] = [
    0x1BADB002, 0x00010000, 0xE4514FFE, 0x100000, 0x100000, 0, 0, 0,
];

#[no_mangle]
#[cfg(target_os = "none")]
pub unsafe extern "C" fn kernel_main() -> ! {
    rt::timer::set_now(0);
    rt::log::log(1, "Starting edgerun unikernel");

    edgerun_crypto::rng::register_random_source(fill_tpm2_random_source);

    let mut rng = Rng::new_from_entropy();
    if let Some(tpm_entropy) = unsafe { probe_tpm2() } {
        rng.mix_entropy(&tpm_entropy);
        rt::log::log(1, "RNG mixed TPM entropy");
    }

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
    let mut offered_ip = None;
    let mut offered_netmask = None;
    let mut offered_gateway = None;
    let mut requested_lease = false;
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
            if let Some(ParsedPacket::Udp {
                header, payload, ..
            }) = network.recv(&rx_buf[..len])
            {
                if header.src_port == DHCP_SERVER_PORT && header.dst_port == DHCP_CLIENT_PORT {
                    if let Ok(message) = DhcpMessage::from_wire(payload) {
                        if message.xid != dhcp_xid || message.chaddr[..6] != mac {
                            continue;
                        }

                        match message.options.message_type {
                            Some(DhcpMessageType::Offer) if !requested_lease => {
                                let Some(server_id) = message.options.server_id else {
                                    continue;
                                };
                                offered_ip = Some(message.yiaddr);
                                offered_netmask = message.options.subnet_mask;
                                offered_gateway = message.options.router;

                                let request =
                                    DhcpMessage::request(dhcp_xid, mac, message.yiaddr, server_id)
                                        .to_wire();
                                if let Some(pkt) = network.send_udp(
                                    IpAddr::new(255, 255, 255, 255),
                                    DHCP_CLIENT_PORT,
                                    DHCP_SERVER_PORT,
                                    &request,
                                ) {
                                    if net.send(pkt) {
                                        requested_lease = true;
                                        rt::log::log(1, "DHCP request queued");
                                    } else {
                                        rt::log::log(1, "DHCP request send failed");
                                    }
                                }
                            }
                            Some(DhcpMessageType::Ack) if requested_lease => {
                                dhcp_ip = dhcp_ipv4_to_rt(message.yiaddr);
                                if dhcp_ip == IpAddr::zero() {
                                    if let Some(ip) = offered_ip {
                                        dhcp_ip = dhcp_ipv4_to_rt(ip);
                                    }
                                }
                                if let Some(netmask) =
                                    message.options.subnet_mask.or(offered_netmask)
                                {
                                    dhcp_netmask = dhcp_ipv4_to_rt(netmask);
                                }
                                if let Some(gateway) = message.options.router.or(offered_gateway) {
                                    dhcp_gateway = dhcp_ipv4_to_rt(gateway);
                                }
                                rt::log::log(1, "DHCP lease accepted");
                                break;
                            }
                            Some(DhcpMessageType::Nak) => {
                                rt::log::log(1, "DHCP lease rejected");
                                break;
                            }
                            _ => {}
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

    let mut network = Network::new(&mut stack);
    let tftp_server = if network.stack.gateway != IpAddr::zero() {
        network.stack.gateway
    } else {
        IpAddr::new(192, 168, 1, 1)
    };
    let rrq = TftpMessage::rrq("edgerun.bin").to_wire();
    rt::log::log(1, "Sending TFTP RRQ");
    if let Some(pkt) = network.send_udp(tftp_server, 2070, TFTP_PORT, &rrq) {
        if net.send(pkt) {
            rt::log::log(1, "TFTP RRQ queued");
        } else {
            rt::log::log(1, "TFTP RRQ send failed");
        }
    }
    let mut tcp = TcpSocket::new();

    let addr = rt::SocketAddr::new(0xC0A8010C, 8080);
    let _ = tcp.bind(addr);
    let _ = tcp.listen(10);
    let _ = rng.next();

    if net.is_link_up() {}

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

#[cfg(not(target_os = "none"))]
fn main() {}
