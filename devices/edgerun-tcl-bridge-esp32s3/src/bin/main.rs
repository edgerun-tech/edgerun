#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, Ordering};

use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::usb_serial_jtag::UsbSerialJtag;

use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{
    Config as NetConfig, IpAddress, IpEndpoint, Ipv4Address, Ipv4Cidr, Runner, Stack,
    StackResources, StaticConfigV4,
};
use esp_radio::wifi::{
    AccessPointConfig, AuthMethod, ModeConfig, WifiController, WifiDevice,
};
use embedded_io_async::Write;

use embassy_executor::Spawner;
use embassy_time::{with_timeout, Duration, Timer};
use static_cell::StaticCell;

use esp_backtrace as _;

extern crate alloc;

use alloc::string::ToString;
use alloc::vec::Vec;

#[path = "../display.rs"]
mod display;

const AP_NET_SOCKETS: usize = 4;
const AP_IP: Ipv4Address = Ipv4Address::new(10, 13, 38, 1);
const AP_LEASE_BASE: Ipv4Address = Ipv4Address::new(10, 13, 38, 2);
const MAX_CLIENTS: usize = 8;

esp_bootloader_esp_idf::esp_app_desc!();

#[derive(Clone, Copy)]
struct ClientRecord {
    mac: [u8; 6],
    ip: Ipv4Address,
}

struct ClientTable {
    entries: UnsafeCell<[Option<ClientRecord>; MAX_CLIENTS]>,
    count: AtomicU32,
}

unsafe impl Sync for ClientTable {}

impl ClientTable {
    const fn new() -> Self {
        const NONE: Option<ClientRecord> = None;
        ClientTable {
            entries: UnsafeCell::new([NONE; MAX_CLIENTS]),
            count: AtomicU32::new(0),
        }
    }

    fn add(&self, mac: [u8; 6], ip: Ipv4Address) {
        let idx = self.count.load(Ordering::Acquire) as usize;
        if idx >= MAX_CLIENTS {
            return;
        }
        let entry = ClientRecord { mac, ip };
        unsafe {
            (*self.entries.get())[idx] = Some(entry);
        }
        self.count.store((idx + 1) as u32, Ordering::Release);
    }

    fn snapshot(&self, buf: &mut [Option<ClientRecord>; MAX_CLIENTS]) -> usize {
        let count = self.count.load(Ordering::Acquire) as usize;
        unsafe {
            for i in 0..count {
                buf[i] = (*self.entries.get())[i];
            }
        }
        count
    }
}

static CLIENTS: ClientTable = ClientTable::new();

#[allow(clippy::large_stack_frames)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_println::println!("init display...");
    unsafe { display::Display::init() }
    esp_println::println!("init display done");

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    let usb = UsbSerialJtag::new(peripherals.USB_DEVICE).into_async();
    let (mut _usb_rx, mut usb_tx) = usb.split();

    write_line(&mut usb_tx, "event boot edgerun-ap-display").await;

    static RADIO: StaticCell<esp_radio::Controller<'static>> = StaticCell::new();
    let radio_init = RADIO.init(esp_radio::init().expect("Failed to initialize Wi-Fi controller"));
    let (mut wifi_controller, interfaces) =
        esp_radio::wifi::new(radio_init, peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");

    static AP_NET_RESOURCES_CELL: StaticCell<StackResources<AP_NET_SOCKETS>> = StaticCell::new();
    let (ap_stack, ap_runner) = embassy_net::new(
        interfaces.ap,
        NetConfig::ipv4_static(StaticConfigV4 {
            address: Ipv4Cidr::new(AP_IP, 24),
            gateway: Some(AP_IP),
            dns_servers: heapless::Vec::new(),
        }),
        AP_NET_RESOURCES_CELL.init(StackResources::new()),
        0x4150_5443_4c53_3355,
    );
    spawner
        .spawn(ap_net_task(ap_runner))
        .expect("failed to spawn ap network task");
    spawner
        .spawn(ap_dhcp_task(ap_stack))
        .expect("failed to spawn ap dhcp task");

    write_line(&mut usb_tx, "event starting-ap").await;
    ap_start(&mut wifi_controller, &mut usb_tx, "EdgeNet", "", 1).await;

    if wait_ap_link(ap_stack, &mut usb_tx).await {
        write_line(
            &mut usb_tx,
            &alloc::format!(
                "ok ap-started ssid=EdgeNet ip={} lease_base={}",
                format_ipv4(AP_IP),
                format_ipv4(AP_LEASE_BASE)
            ),
        )
        .await;
    }

    write_line(&mut usb_tx, "event ready").await;

    draw_boot_screen();

    let mut client_buf = [None; MAX_CLIENTS];
    loop {
        let count = CLIENTS.snapshot(&mut client_buf);

        write_line(
            &mut usb_tx,
            &alloc::format!("event display-update clients={}", count),
        )
        .await;

        update_display(count, &client_buf);

        Timer::after(Duration::from_secs(2)).await;
    }
}

fn draw_boot_screen() {
    unsafe {
        display::Display::draw_text(8, 30, "EDGENET", 4, 0x07e0, 0x0000);
        display::Display::draw_text(8, 70, "AP DISPLAY", 3, 0xffff, 0x0000);
        display::Display::draw_text(8, 120, "10.13.38.1", 2, 0x07ff, 0x0000);
    }
}

fn update_display(count: usize, clients: &[Option<ClientRecord>; MAX_CLIENTS]) {
    let ip_str = format_ipv4(AP_IP);
    let count_str = alloc::format!("CLIENTS: {}", count);

    let mut strs: Vec<alloc::string::String> = Vec::with_capacity(3 + count * 2);
    strs.push(alloc::string::String::from("EDGENET AP"));
    strs.push(ip_str);
    strs.push(count_str);
    for i in 0..count {
        if let Some(client) = &clients[i] {
            strs.push(format_mac(client.mac));
            strs.push(format_ipv4(client.ip));
        }
    }

    let n = strs.len();
    let lines = strs.as_slice();

    unsafe {
        display::Display::draw_rgb565_with(display::LCD_WIDTH, display::LCD_HEIGHT, |x, y| {
            let bg: u16 = 0x0000;

            if n >= 1 {
                if let Some(c) = text_pixel(x, y, &lines[0], 8, 4, 3, 0x07e0) {
                    return c;
                }
            }
            if n >= 2 {
                if let Some(c) = text_pixel(x, y, &lines[1], 8, 32, 2, 0xffff) {
                    return c;
                }
            }
            if n >= 3 {
                if let Some(c) = text_pixel(x, y, &lines[2], 8, 58, 2, 0x07ff) {
                    return c;
                }
            }

            let mut si = 3usize;
            let mut cy = 84u16;
            while si + 1 < n {
                if let Some(c) = text_pixel(x, y, &lines[si], 8, cy, 2, 0xffe0) {
                    return c;
                }
                si += 1;
                if let Some(c) = text_pixel(x, y, &lines[si], 8, cy + 16, 2, 0xffff) {
                    return c;
                }
                si += 1;
                cy += 28;
            }

            bg
        });
    }
}

fn text_pixel(x: u16, y: u16, text: &str, tx: u16, ty: u16, scale: u8, fg: u16) -> Option<u16> {
    let cell_w = display::CHAR_STEP as u16 * scale as u16;
    let cell_h = display::CHAR_H as u16 * scale as u16;

    if y < ty || y >= ty + cell_h {
        return None;
    }
    if x < tx {
        return None;
    }
    let rel_x = x - tx;
    let rel_y = y - ty;

    let char_index = rel_x / cell_w;
    if char_index as usize >= text.len() {
        return None;
    }

    let col_in_char = (rel_x % cell_w) / scale as u16;
    let row_in_char = rel_y / scale as u16;

    if col_in_char as usize >= display::CHAR_W {
        return None;
    }
    if row_in_char as usize >= display::CHAR_H {
        return None;
    }

    let ch = text.as_bytes()[char_index as usize];
    let bits = display::glyph_column(ch, col_in_char as usize);
    if (bits >> (row_in_char as usize)) & 1 != 0 {
        Some(fg)
    } else {
        Some(0x0000)
    }
}

#[embassy_executor::task]
async fn ap_net_task(mut runner: Runner<'static, WifiDevice<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn ap_dhcp_task(ap_stack: Stack<'static>) -> ! {
    let mut rx_meta = [PacketMetadata::EMPTY; 4];
    let mut tx_meta = [PacketMetadata::EMPTY; 4];
    let mut rx_buf = [0u8; 1536];
    let mut tx_buf = [0u8; 1536];
    let mut packet = [0u8; 1536];
    let mut socket = UdpSocket::new(
        ap_stack,
        &mut rx_meta,
        &mut rx_buf,
        &mut tx_meta,
        &mut tx_buf,
    );

    loop {
        if socket.bind(67).is_ok() {
            break;
        }
        Timer::after(Duration::from_secs(1)).await;
    }

    let mut lease_index: u32 = 0;
    loop {
        let Ok((len, _meta)) = socket.recv_from(&mut packet).await else {
            continue;
        };
        let Some(reply_len) = build_dhcp_reply(&mut packet, len, &mut lease_index) else {
            continue;
        };
        let dest = IpEndpoint::new(IpAddress::Ipv4(Ipv4Address::BROADCAST), 68);
        let _ = socket.send_to(&packet[..reply_len], dest).await;
    }
}

fn build_dhcp_reply(packet: &mut [u8], len: usize, lease_index: &mut u32) -> Option<usize> {
    const DHCP_FIXED_LEN: usize = 240;
    const OPTION_MSG_TYPE: u8 = 53;
    const OPTION_SERVER_ID: u8 = 54;
    const OPTION_LEASE_TIME: u8 = 51;
    const OPTION_SUBNET_MASK: u8 = 1;
    const OPTION_ROUTER: u8 = 3;
    const OPTION_DNS: u8 = 6;
    const OPTION_END: u8 = 255;
    const DHCP_DISCOVER: u8 = 1;
    const DHCP_OFFER: u8 = 2;
    const DHCP_REQUEST: u8 = 3;
    const DHCP_ACK: u8 = 5;

    if len < DHCP_FIXED_LEN || packet.first().copied()? != 1 {
        return None;
    }
    if packet.get(236..240)? != [99, 130, 83, 99] {
        return None;
    }

    let mut msg_type = None;
    let mut pos = DHCP_FIXED_LEN;
    while pos < len {
        let option = packet[pos];
        pos += 1;
        match option {
            0 => {}
            OPTION_END => break,
            option => {
                let option_len = *packet.get(pos)? as usize;
                pos += 1;
                let value = packet.get(pos..pos + option_len)?;
                if option == OPTION_MSG_TYPE && option_len == 1 {
                    msg_type = Some(value[0]);
                }
                pos += option_len;
            }
        }
    }

    let reply_type = match msg_type? {
        DHCP_DISCOVER => DHCP_OFFER,
        DHCP_REQUEST => DHCP_ACK,
        _ => return None,
    };

    let client_mac = [
        packet[28], packet[29], packet[30], packet[31], packet[32], packet[33],
    ];

    let lease_ip = {
        let idx = *lease_index;
        *lease_index = idx.wrapping_add(1);
        let n = idx as u16;
        Ipv4Address::new(
            AP_LEASE_BASE.octets()[0],
            AP_LEASE_BASE.octets()[1],
            AP_LEASE_BASE.octets()[2],
            AP_LEASE_BASE.octets()[3].wrapping_add((n & 0xff) as u8),
        )
    };

    if reply_type == DHCP_ACK {
        CLIENTS.add(client_mac, lease_ip);
    }

    packet[0] = 2;
    packet[1] = 1;
    packet[2] = 6;
    packet[3] = 0;
    packet[8..12].copy_from_slice(&[0, 0, 0, 0]);
    packet[16..20].copy_from_slice(&lease_ip.octets());
    packet[20..24].copy_from_slice(&AP_IP.octets());
    packet[24..28].copy_from_slice(&[0, 0, 0, 0]);
    packet[236..240].copy_from_slice(&[99, 130, 83, 99]);

    let mut out = DHCP_FIXED_LEN;
    out = push_dhcp_option(packet, out, OPTION_MSG_TYPE, &[reply_type])?;
    out = push_dhcp_option(packet, out, OPTION_SERVER_ID, &AP_IP.octets())?;
    out = push_dhcp_option(packet, out, OPTION_LEASE_TIME, &3600u32.to_be_bytes())?;
    out = push_dhcp_option(packet, out, OPTION_SUBNET_MASK, &[255, 255, 255, 0])?;
    out = push_dhcp_option(packet, out, OPTION_ROUTER, &AP_IP.octets())?;
    out = push_dhcp_option(packet, out, OPTION_DNS, &AP_IP.octets())?;
    *packet.get_mut(out)? = OPTION_END;
    out += 1;

    Some(out.max(300))
}

fn push_dhcp_option(packet: &mut [u8], pos: usize, option: u8, value: &[u8]) -> Option<usize> {
    let end = pos.checked_add(2)?.checked_add(value.len())?;
    if end > packet.len() || value.len() > u8::MAX as usize {
        return None;
    }
    packet[pos] = option;
    packet[pos + 1] = value.len() as u8;
    packet[pos + 2..end].copy_from_slice(value);
    Some(end)
}

async fn wait_ap_link<W>(ap_stack: Stack<'static>, tx: &mut W) -> bool
where
    W: Write,
{
    if ap_stack.is_link_up() {
        return true;
    }

    match with_timeout(Duration::from_secs(5), ap_stack.wait_link_up()).await {
        Ok(()) => true,
        Err(_) => {
            write_line(tx, "err ap-link-timeout").await;
            false
        }
    }
}

async fn ap_start<W>(
    wifi: &mut WifiController<'_>,
    tx: &mut W,
    ssid: &str,
    password: &str,
    channel: u8,
) where
    W: Write,
{
    if let Ok(true) = wifi.is_started() {
        let _ = wifi.stop_async().await;
    }

    let auth = if password.is_empty() {
        AuthMethod::None
    } else {
        AuthMethod::Wpa2Personal
    };
    let ap = AccessPointConfig::default()
        .with_ssid(ssid.to_string())
        .with_password(password.to_string())
        .with_auth_method(auth)
        .with_channel(channel);

    if let Err(err) = wifi.set_config(&ModeConfig::AccessPoint(ap)) {
        write_line(tx, &alloc::format!("err ap-config {:?}", err)).await;
        return;
    }
    match wifi.start_async().await {
        Ok(()) => write_line(tx, "ok ap-started").await,
        Err(err) => write_line(tx, &alloc::format!("err ap-start {:?}", err)).await,
    }
}

async fn write_line<W>(tx: &mut W, line: &str)
where
    W: Write,
{
    let _ = tx.write_all(line.as_bytes()).await;
    let _ = tx.write_all(b"\r\n").await;
    let _ = tx.flush().await;
}

fn format_mac(mac: [u8; 6]) -> alloc::string::String {
    alloc::format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    )
}

fn format_ipv4(ip: Ipv4Address) -> alloc::string::String {
    let octets = ip.octets();
    alloc::format!("{}.{}.{}.{}", octets[0], octets[1], octets[2], octets[3])
}
