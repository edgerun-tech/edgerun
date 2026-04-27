//! Edgerun unikernel - bare shell with networking

#![no_std]
#![no_main]

extern crate edgerun_bare_rt as rt;
extern crate edgerun_virtio;
extern crate edgerun_platform;

use rt::{DhcpClient, TftpConfig, TcpSocket, block_on, runtime::spawn, Rng, crc32, RingBuffer, IpStack, Network, IpAddr, DhcpStateMachine};

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

struct NetworkTask;

impl Future for NetworkTask {
    type Output = ();
    
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let _ = cx;
        rt::log::log(3, "Network task");
        Poll::Pending
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! { 
    loop { unsafe { core::arch::asm!("hlt", options(noreturn)); } }
}

#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    main();
    loop { unsafe { core::arch::asm!("hlt", options(noreturn)); } }
}

#[no_mangle]
pub unsafe extern "C" fn main() {
    rt::timer::set_now(0);
    
    let mut rng = Rng::new_from_entropy();
    let test_crc = crc32(b"hello");
    let _ = test_crc;
    
    let mut rx_buf = RingBuffer::new(1024);
    rx_buf.push_slice(b"test packet");
    let _ = rx_buf.len();
    
    let mut net = match edgerun_virtio::find_virtio_net() {
        Some(n) => n,
        None => loop { core::arch::asm!("hlt") },
    };
    
    net.init();
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
    if let Some(pkt) = network.send_ip(IpAddr::new(255, 255, 255, 255), 17, &discover) {
        net.send(pkt);
    }
    
    drop(network);
    
    let mut rx_buf = [0u8; 1514];
    let len = net.recv(&mut rx_buf);
    if len.is_some() {
        let _ = dhcp.parse(&rx_buf);
    }
    
    if dhcp.ip != IpAddr::zero() {
        stack.ip = dhcp.ip;
        stack.netmask = dhcp.netmask;
        stack.gateway = dhcp.gateway;
    } else {
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
    
    let (ip, _, _) = (stack.ip, stack.netmask, stack.gateway);
    
    block_on(NetworkTask);
}