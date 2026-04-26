//! Edgerun unikernel - bare shell with networking

#![no_std]
#![no_main]

extern crate edgerun_bare_rt as rt;
extern crate edgerun_virtio;
extern crate edgerun_platform;

use rt::{DhcpClient, TftpConfig, TcpSocket, block_on, runtime::spawn};
use rt::Ipv4Addr;

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
    
    let mut net = match edgerun_virtio::find_virtio_net() {
        Some(n) => n,
        None => loop { core::arch::asm!("hlt") },
    };
    
    net.init();
    let mac = net.get_mac();
    let _dhcp = DhcpClient::new(mac);
    let _tftp = TftpConfig::new(0xC0A80101, "edgerun.bin");
    let mut tcp = TcpSocket::new();
    
    let local_ip = Ipv4Addr::new(192, 168, 1, 12);
    let server_ip = Ipv4Addr::new(192, 168, 1, 1);
    let mask = Ipv4Addr::new(255, 255, 255, 0);
    
    let addr = rt::SocketAddr::new(local_ip.0, 8080);
    let _ = tcp.bind(addr);
    let _ = tcp.listen(10);
    
    spawn(NetworkTask);
    
    if net.is_link_up() {
        rt::log::log(3, "VirtIO Net: OK");
        rt::log::log(3, "IP: 192.168.1.12");
        rt::log::log(3, "TCP 8080");
    }
    
    block_on(NetworkTask);
}