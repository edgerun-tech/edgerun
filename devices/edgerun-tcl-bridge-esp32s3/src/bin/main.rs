#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering};

use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::usb_serial_jtag::UsbSerialJtag;

use embassy_net::tcp::TcpSocket;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{
    Config as NetConfig, IpAddress, IpEndpoint, Ipv4Address, Ipv4Cidr, Runner, Stack,
    StackResources, StaticConfigV4,
};
use esp_radio::wifi::{
    AccessPointConfig, AuthMethod, ClientConfig, ModeConfig, ScanConfig, WifiController, WifiDevice,
};
use embedded_io_async::{Read, Write};

use embassy_executor::Spawner;
use embassy_time::{with_timeout, Duration, Timer};
use static_cell::StaticCell;

use esp_backtrace as _;

extern crate alloc;

use alloc::{format, string::ToString};

const COMMAND_BUF_LEN: usize = 256;
const NET_SOCKETS: usize = 4;
const AP_NET_SOCKETS: usize = 4;
const AP_IP: Ipv4Address = Ipv4Address::new(192, 168, 4, 1);
const AP_LEASE_IP: Ipv4Address = Ipv4Address::new(192, 168, 4, 2);
static AP_LEASE_SEEN: AtomicBool = AtomicBool::new(false);
static AP_LEASE_MAC_HI: AtomicU32 = AtomicU32::new(0);
static AP_LEASE_MAC_LO: AtomicU16 = AtomicU16::new(0);

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.2.0

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);
    // COEX needs more RAM - so we've added some more
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    let usb = UsbSerialJtag::new(peripherals.USB_DEVICE).into_async();
    let (mut usb_rx, mut usb_tx) = usb.split();
    let mut line_buf = [0u8; COMMAND_BUF_LEN];

    write_line(&mut usb_tx, "event boot edgerun-tcl-bridge-esp32s3").await;
    write_line(&mut usb_tx, "event init-radio").await;

    static RADIO: StaticCell<esp_radio::Controller<'static>> = StaticCell::new();
    let radio_init = RADIO.init(esp_radio::init().expect("Failed to initialize Wi-Fi controller"));
    let (mut wifi_controller, interfaces) =
        esp_radio::wifi::new(radio_init, peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");

    static STA_NET_RESOURCES: StaticCell<StackResources<NET_SOCKETS>> = StaticCell::new();
    let (sta_stack, sta_runner) = embassy_net::new(
        interfaces.sta,
        NetConfig::dhcpv4(Default::default()),
        STA_NET_RESOURCES.init(StackResources::new()),
        0x5443_4c41_4353_3355,
    );
    spawner
        .spawn(sta_net_task(sta_runner))
        .expect("failed to spawn station network task");

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

    write_line(&mut usb_tx, "event ready").await;
    write_help(&mut usb_tx).await;

    loop {
        write_prompt(&mut usb_tx).await;
        let line = read_line(&mut usb_rx, &mut line_buf).await;
        let Some(line) = line else {
            write_line(&mut usb_tx, "err usb-read").await;
            Timer::after(Duration::from_millis(50)).await;
            continue;
        };

        handle_command(line, &mut wifi_controller, sta_stack, ap_stack, &mut usb_tx).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples
}

#[embassy_executor::task]
async fn sta_net_task(mut runner: Runner<'static, WifiDevice<'static>>) -> ! {
    runner.run().await
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

    loop {
        let Ok((len, _meta)) = socket.recv_from(&mut packet).await else {
            continue;
        };
        let Some(reply_len) = build_dhcp_reply(&mut packet, len) else {
            continue;
        };
        let dest = IpEndpoint::new(IpAddress::Ipv4(Ipv4Address::BROADCAST), 68);
        let _ = socket.send_to(&packet[..reply_len], dest).await;
    }
}

async fn handle_command<W>(
    line: &str,
    wifi: &mut WifiController<'_>,
    sta_stack: Stack<'static>,
    ap_stack: Stack<'static>,
    tx: &mut W,
)
where
    W: Write,
{
    let mut parts = line.split_whitespace();
    let Some(cmd) = parts.next() else {
        return;
    };

    match cmd {
        "help" | "?" => write_help(tx).await,
        "ping" => write_line(tx, "ok pong").await,
        "wifi-scan" => {
            let max = parts
                .next()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(32);
            wifi_scan(wifi, tx, max).await;
        }
        "wifi-connect" => {
            let ssid = parts.next();
            let password = parts.next().unwrap_or("");
            match ssid {
                Some(ssid) => wifi_connect(wifi, tx, ssid, password).await,
                None => write_line(tx, "err usage wifi-connect <ssid> [password]").await,
            }
        }
        "wifi-status" => wifi_status(wifi, tx).await,
        "wifi-stop" => wifi_stop(wifi, tx).await,
        "hotspot-status" => hotspot_status(ap_stack, tx).await,
        "ap-start" | "hotspot-start" => {
            let ssid = parts.next();
            let password = parts.next().map(dash_empty).unwrap_or("");
            let channel = parts
                .next()
                .and_then(|value| value.parse::<u8>().ok())
                .unwrap_or(1);
            match ssid {
                Some(ssid) => hotspot_start(wifi, ap_stack, tx, ssid, password, channel).await,
                None => {
                    write_line(
                        tx,
                        "err usage hotspot-start <ssid> [password-or-dash] [channel]",
                    )
                    .await
                }
            }
        }
        "ble-scan" | "ble-provision" => {
            write_line(tx, "err ble bridge commands are not wired yet").await
        }
        "tcl-provision" => {
            let ap_ssid = parts.next();
            let ap_password = parts.next();
            let home_ssid = parts.next();
            let home_password = parts.next();
            let bind_code = parts.next();
            let device_ip = parts.next().unwrap_or("192.168.1.1");
            match (ap_ssid, ap_password, home_ssid, home_password, bind_code) {
                (
                    Some(ap_ssid),
                    Some(ap_password),
                    Some(home_ssid),
                    Some(home_password),
                    Some(bind_code),
                ) => {
                    tcl_provision(
                        wifi,
                        sta_stack,
                        tx,
                        ap_ssid,
                        dash_empty(ap_password),
                        home_ssid,
                        dash_empty(home_password),
                        bind_code,
                        device_ip,
                    )
                    .await
                }
                _ => {
                    write_line(
                        tx,
                        "err usage tcl-provision <ac-ap-ssid> <ac-ap-password-or-dash> <home-ssid> <home-password-or-dash> <bind-code> [device-ip]",
                    )
                    .await
                }
            }
        }
        _ => write_line(tx, "err unknown-command").await,
    }
}

async fn wifi_scan<W>(wifi: &mut WifiController<'_>, tx: &mut W, max: usize)
where
    W: Write,
{
    let client = ClientConfig::default()
        .with_ssid("".to_string())
        .with_password("".to_string())
        .with_auth_method(AuthMethod::None);

    if let Err(err) = wifi.set_config(&ModeConfig::Client(client)) {
        write_line(tx, &format!("err wifi-config {:?}", err)).await;
        return;
    }

    match wifi.is_started() {
        Ok(true) => {}
        Ok(false) => {
            if let Err(err) = wifi.start_async().await {
                write_line(tx, &format!("err wifi-start {:?}", err)).await;
                return;
            }
        }
        Err(err) => {
            write_line(tx, &format!("err wifi-state {:?}", err)).await;
            return;
        }
    }

    let scan = ScanConfig::default().with_max(max);
    match wifi.scan_with_config_async(scan).await {
        Ok(aps) => {
            write_line(tx, &format!("ok wifi-scan count={}", aps.len())).await;
            for ap in aps {
                let auth = ap.auth_method.unwrap_or(AuthMethod::None);
                write_line(
                    tx,
                    &format!(
                        "ap ssid=\"{}\" bssid={} channel={} rssi={} auth={:?}",
                        ap.ssid.as_str(),
                        format_mac(ap.bssid),
                        ap.channel,
                        ap.signal_strength,
                        auth
                    ),
                )
                .await;
            }
        }
        Err(err) => write_line(tx, &format!("err wifi-scan {:?}", err)).await,
    }
}

async fn wifi_connect<W>(wifi: &mut WifiController<'_>, tx: &mut W, ssid: &str, password: &str)
where
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
    let client = ClientConfig::default()
        .with_ssid(ssid.to_string())
        .with_password(password.to_string())
        .with_auth_method(auth);

    if let Err(err) = wifi.set_config(&ModeConfig::Client(client)) {
        write_line(tx, &format!("err wifi-config {:?}", err)).await;
        return;
    }
    if let Err(err) = wifi.start_async().await {
        write_line(tx, &format!("err wifi-start {:?}", err)).await;
        return;
    }
    write_line(tx, "event wifi-connecting").await;
    match wifi.connect_async().await {
        Ok(()) => write_line(tx, "ok wifi-connected").await,
        Err(err) => write_line(tx, &format!("err wifi-connect {:?}", err)).await,
    }
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

async fn hotspot_start<W>(
    wifi: &mut WifiController<'_>,
    ap_stack: Stack<'static>,
    tx: &mut W,
    ssid: &str,
    password: &str,
    channel: u8,
) where
    W: Write,
{
    ap_start(wifi, tx, ssid, password, channel).await;
    if wait_ap_link(ap_stack, tx).await {
        write_line(
            tx,
            &format!(
                "ok hotspot-started ssid=\"{}\" ip={} lease={}",
                ssid,
                format_ipv4(AP_IP),
                format_ipv4(AP_LEASE_IP)
            ),
        )
        .await;
    }
}

async fn hotspot_status<W>(ap_stack: Stack<'static>, tx: &mut W)
where
    W: Write,
{
    let lease = if AP_LEASE_SEEN.load(Ordering::Relaxed) {
        format!(
            " lease={} client={}",
            format_ipv4(AP_LEASE_IP),
            format_stored_lease_mac()
        )
    } else {
        " lease=none".to_string()
    };
    write_line(
        tx,
        &format!(
            "ok hotspot-status link_up={} ip={}{}",
            ap_stack.is_link_up(),
            format_ipv4(AP_IP),
            lease
        ),
    )
    .await;
}

async fn wait_net_config<W>(net_stack: Stack<'static>, tx: &mut W) -> bool
where
    W: Write,
{
    if net_stack.is_config_up() {
        return true;
    }

    write_line(tx, "event net-wait-config").await;
    match with_timeout(Duration::from_secs(20), net_stack.wait_config_up()).await {
        Ok(()) => {
            write_line(tx, "ok net-config-up").await;
            true
        }
        Err(_) => {
            write_line(tx, "err net-config-timeout").await;
            false
        }
    }
}

async fn tcl_provision<W>(
    wifi: &mut WifiController<'_>,
    net_stack: Stack<'static>,
    tx: &mut W,
    ap_ssid: &str,
    ap_password: &str,
    home_ssid: &str,
    home_password: &str,
    bind_code: &str,
    device_ip: &str,
) where
    W: Write,
{
    wifi_connect(wifi, tx, ap_ssid, ap_password).await;
    if !wifi.is_connected().unwrap_or(false) {
        write_line(tx, "err tcl-provision wifi-not-connected").await;
        return;
    }
    if !wait_net_config(net_stack, tx).await {
        return;
    }

    let Some(ip) = parse_ipv4(device_ip) else {
        write_line(tx, "err invalid-device-ip").await;
        return;
    };
    let payload = build_tcl_legacy_payload(home_ssid, home_password, bind_code);
    softap_udp_provision(net_stack, tx, ip, &payload).await;
    softap_tcp_provision(net_stack, tx, ip, home_ssid, home_password).await;
}

async fn softap_udp_provision<W>(
    net_stack: Stack<'static>,
    tx: &mut W,
    device_ip: Ipv4Address,
    payload: &str,
) where
    W: Write,
{
    let mut rx_meta = [PacketMetadata::EMPTY; 4];
    let mut tx_meta = [PacketMetadata::EMPTY; 4];
    let mut rx_buf = [0u8; 2048];
    let mut tx_buf = [0u8; 2048];
    let mut recv_buf = [0u8; 2048];
    let mut socket = UdpSocket::new(
        net_stack,
        &mut rx_meta,
        &mut rx_buf,
        &mut tx_meta,
        &mut tx_buf,
    );

    if let Err(err) = socket.bind(10000) {
        write_line(tx, &format!("err tcl-udp-bind {:?}", err)).await;
        return;
    }

    let endpoint = IpEndpoint::new(IpAddress::Ipv4(device_ip), 10000);
    for attempt in 1..=3 {
        write_line(tx, &format!("event tcl-udp-send attempt={}", attempt)).await;
        if let Err(err) = socket.send_to(payload.as_bytes(), endpoint).await {
            write_line(tx, &format!("err tcl-udp-send {:?}", err)).await;
            continue;
        }
        match with_timeout(Duration::from_secs(2), socket.recv_from(&mut recv_buf)).await {
            Ok(Ok((len, meta))) => {
                write_line(
                    tx,
                    &format!(
                        "ok tcl-udp-response from={:?}:{} bytes={}",
                        meta.endpoint.addr, meta.endpoint.port, len
                    ),
                )
                .await;
                write_lossy_response(tx, "data tcl-udp ", &recv_buf[..len]).await;
                return;
            }
            Ok(Err(err)) => write_line(tx, &format!("err tcl-udp-recv {:?}", err)).await,
            Err(_) => write_line(tx, "event tcl-udp-timeout").await,
        }
        Timer::after(Duration::from_secs(1)).await;
    }
    write_line(tx, "err tcl-udp-no-response").await;
}

async fn softap_tcp_provision<W>(
    net_stack: Stack<'static>,
    tx: &mut W,
    device_ip: Ipv4Address,
    ssid: &str,
    password: &str,
) where
    W: Write,
{
    let mut rx_buf = [0u8; 2048];
    let mut tx_buf = [0u8; 2048];
    let mut read_buf = [0u8; 2048];
    let mut socket = TcpSocket::new(net_stack, &mut rx_buf, &mut tx_buf);
    socket.set_timeout(Some(Duration::from_secs(5)));
    let endpoint = IpEndpoint::new(IpAddress::Ipv4(device_ip), 10000);

    write_line(tx, "event tcl-tcp-connect").await;
    match with_timeout(Duration::from_secs(5), socket.connect(endpoint)).await {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            write_line(tx, &format!("err tcl-tcp-connect {:?}", err)).await;
            return;
        }
        Err(_) => {
            write_line(tx, "err tcl-tcp-connect-timeout").await;
            return;
        }
    }

    let request = format!(
        "<setReq><ssid>{}</ssid><password>{}</password></setReq>",
        ssid, password
    );
    if let Err(err) = tcp_write_all(&mut socket, request.as_bytes()).await {
        write_line(tx, &format!("err tcl-tcp-write {:?}", err)).await;
        socket.abort();
        return;
    }
    let _ = socket.flush().await;

    match with_timeout(Duration::from_secs(5), socket.read(&mut read_buf)).await {
        Ok(Ok(0)) => write_line(tx, "err tcl-tcp-empty-response").await,
        Ok(Ok(len)) => {
            write_line(tx, &format!("ok tcl-tcp-response bytes={}", len)).await;
            write_lossy_response(tx, "data tcl-tcp ", &read_buf[..len]).await;
        }
        Ok(Err(err)) => write_line(tx, &format!("err tcl-tcp-read {:?}", err)).await,
        Err(_) => write_line(tx, "err tcl-tcp-read-timeout").await,
    }
    socket.close();
    let _ = socket.flush().await;
}

async fn tcp_write_all(
    socket: &mut TcpSocket<'_>,
    mut bytes: &[u8],
) -> Result<(), embassy_net::tcp::Error> {
    while !bytes.is_empty() {
        let written = socket.write(bytes).await?;
        if written == 0 {
            return Err(embassy_net::tcp::Error::ConnectionReset);
        }
        bytes = &bytes[written..];
    }
    Ok(())
}

async fn wifi_status<W>(wifi: &mut WifiController<'_>, tx: &mut W)
where
    W: Write,
{
    let started = wifi.is_started().unwrap_or(false);
    let connected = wifi.is_connected().unwrap_or(false);
    let rssi = wifi.rssi().ok();
    write_line(
        tx,
        &format!(
            "ok wifi-status started={} connected={} rssi={:?}",
            started, connected, rssi
        ),
    )
    .await;
}

async fn wifi_stop<W>(wifi: &mut WifiController<'_>, tx: &mut W)
where
    W: Write,
{
    match wifi.is_started() {
        Ok(true) => match wifi.stop_async().await {
            Ok(()) => write_line(tx, "ok wifi-stopped").await,
            Err(err) => write_line(tx, &format!("err wifi-stop {:?}", err)).await,
        },
        Ok(false) => write_line(tx, "ok wifi-stopped").await,
        Err(err) => write_line(tx, &format!("err wifi-state {:?}", err)).await,
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
        write_line(tx, &format!("err ap-config {:?}", err)).await;
        return;
    }
    match wifi.start_async().await {
        Ok(()) => write_line(tx, "ok ap-started").await,
        Err(err) => write_line(tx, &format!("err ap-start {:?}", err)).await,
    }
}

async fn read_line<'a, R>(rx: &mut R, buf: &'a mut [u8]) -> Option<&'a str>
where
    R: Read,
{
    let mut len = 0usize;
    loop {
        let mut byte = [0u8; 1];
        if rx.read(&mut byte).await.ok()? == 0 {
            continue;
        }

        match byte[0] {
            b'\r' => {}
            b'\n' => return core::str::from_utf8(&buf[..len]).ok(),
            8 | 127 => len = len.saturating_sub(1),
            b if len < buf.len() => {
                buf[len] = b;
                len += 1;
            }
            _ => {}
        }
    }
}

async fn write_help<W>(tx: &mut W)
where
    W: Write,
{
    write_line(tx, "ok commands: help ping wifi-scan [max] wifi-connect <ssid> [password] wifi-status wifi-stop hotspot-start <ssid> [password-or-dash] [channel] hotspot-status tcl-provision <ac-ap-ssid> <ac-ap-password-or-dash> <home-ssid> <home-password-or-dash> <bind-code> [device-ip] ble-scan ble-provision").await;
}

async fn write_prompt<W>(tx: &mut W)
where
    W: Write,
{
    let _ = tx.write_all(b"> ").await;
    let _ = tx.flush().await;
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
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    )
}

fn format_stored_lease_mac() -> alloc::string::String {
    let hi = AP_LEASE_MAC_HI.load(Ordering::Relaxed).to_be_bytes();
    let lo = AP_LEASE_MAC_LO.load(Ordering::Relaxed).to_be_bytes();
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        hi[0], hi[1], hi[2], hi[3], lo[0], lo[1]
    )
}

fn format_ipv4(ip: Ipv4Address) -> alloc::string::String {
    let octets = ip.octets();
    format!("{}.{}.{}.{}", octets[0], octets[1], octets[2], octets[3])
}

fn build_dhcp_reply(packet: &mut [u8], len: usize) -> Option<usize> {
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
    if len >= 34 {
        AP_LEASE_MAC_HI.store(
            u32::from_be_bytes([packet[28], packet[29], packet[30], packet[31]]),
            Ordering::Relaxed,
        );
        AP_LEASE_MAC_LO.store(u16::from_be_bytes([packet[32], packet[33]]), Ordering::Relaxed);
        AP_LEASE_SEEN.store(true, Ordering::Relaxed);
    }

    packet[0] = 2;
    packet[1] = 1;
    packet[2] = 6;
    packet[3] = 0;
    packet[8..12].copy_from_slice(&[0, 0, 0, 0]);
    packet[16..20].copy_from_slice(&AP_LEASE_IP.octets());
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

async fn write_lossy_response<W>(tx: &mut W, prefix: &str, bytes: &[u8])
where
    W: Write,
{
    let _ = tx.write_all(prefix.as_bytes()).await;
    for &byte in bytes {
        match byte {
            b'\r' => {}
            b'\n' => {
                let _ = tx.write_all(b"\\n").await;
            }
            0x20..=0x7e => {
                let _ = tx.write_all(&[byte]).await;
            }
            _ => {
                let hex = b"0123456789abcdef";
                let escaped = [b'\\', b'x', hex[(byte >> 4) as usize], hex[(byte & 0x0f) as usize]];
                let _ = tx.write_all(&escaped).await;
            }
        }
    }
    let _ = tx.write_all(b"\r\n").await;
    let _ = tx.flush().await;
}

fn parse_ipv4(value: &str) -> Option<Ipv4Address> {
    let mut octets = [0u8; 4];
    let mut count = 0usize;
    for part in value.split('.') {
        if count == octets.len() {
            return None;
        }
        octets[count] = part.parse::<u8>().ok()?;
        count += 1;
    }
    if count == octets.len() {
        Some(Ipv4Address::new(octets[0], octets[1], octets[2], octets[3]))
    } else {
        None
    }
}

fn dash_empty(value: &str) -> &str {
    if value == "-" {
        ""
    } else {
        value
    }
}

fn build_tcl_legacy_payload(ssid: &str, password: &str, bind_code: &str) -> alloc::string::String {
    let timestamp = embassy_time::Instant::now().as_secs();
    format!(
        "{{\"msgId\":\"{}\",\"method\":\"setReq\",\"version\":\"1\",\"params\":{{\"bindCode\":\"{}\",\"ssid\":\"{}\",\"password\":\"{}\",\"timestamp\":{},\"timezone\":7,\"timearea\":\"Asia/Bangkok\",\"serverPort\":443,\"cloudType\":\"AWS\",\"caType\":\"release\",\"serverHost\":\"prod-center.aws.tcljd.com\",\"serverHostV2\":\"prod-center.aws.tcljd.com\"}}}}",
        timestamp % 900 + 100,
        JsonEscaped(bind_code),
        JsonEscaped(ssid),
        JsonEscaped(password),
        timestamp
    )
}

struct JsonEscaped<'a>(&'a str);

impl core::fmt::Display for JsonEscaped<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for ch in self.0.chars() {
            match ch {
                '"' => f.write_str("\\\"")?,
                '\\' => f.write_str("\\\\")?,
                '\n' => f.write_str("\\n")?,
                '\r' => f.write_str("\\r")?,
                '\t' => f.write_str("\\t")?,
                ch if ch < ' ' => write!(f, "\\u{:04x}", ch as u32)?,
                ch => f.write_fmt(format_args!("{}", ch))?,
            }
        }
        Ok(())
    }
}
