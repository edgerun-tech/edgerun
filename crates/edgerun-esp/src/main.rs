#![no_main]

use esp_alloc::heap_allocator;
use esp_hal::clock::CpuClock;
use esp_hal::init;
use esp_hal::time::Duration;
use esp_hal::timer::TimerGroup;
use esp_hal::EspDeviceInit;
use esp_wifi::wifi::WifiController;
use esp_wifi::wifi::WifiMode;
use esp_wifi::wifi::WifiState;
use esp_wifi::{initialize, EspWifiInit};
use log::info;
use smoltcp::socket::SocketSet;
use smoltcp::socket::TcpSocket;
use smoltcp::socket::TcpSocketBuffer;
use smoltcp::socket::UdpSocket;
use smoltcp::socket::UdpSocketBuffer;
use smoltcp::time::Instant;
use smoltcp::wire::IpAddress;
use smoltcp::wire::IpCidr;
use smoltcp::wire::IpEndpoint;

const ESP_AP_SSID: &str = "TCL_AC_SETUP";
const ESP_AP_PASSWORD: &str = "";
const AC_AP_SSID_PREFIX: &str = "tcl_AC_";
const AC_IP: &str = "192.168.1.1";
const AC_PORT: u16 = 10000;

#[entry]
fn main() -> ! {
    heap_allocator!(72 * 1024);

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = EspDeviceInit::take();

    info!("ESP32-S3 TCL AC Provisioning");
    info!("Initializing WiFi...");

    let init = EspWifiInit::new();
    let mut wifi = esp_wifi::wifi::newWifiController(init, peripherals.wifi);

    let ap_ssid = ESP_AP_SSID;
    let ap_password = ESP_AP_PASSWORD;

    loop {
        match wifi.get_wifi_state() {
            WifiState::ApStarted => {
                info!("AP already started");
            }
            WifiState::Idle | WifiState::Disconnected => {
                info!("Starting AP: {}...", ap_ssid);
                let config = esp_wifi::wifi::WifiApConfig {
                    ssid: ap_ssid.try_into().unwrap(),
                    password: ap_password.try_into().unwrap(),
                    channel: 6,
                    ..Default::default()
                };
                wifi.start_ap(&config);
            }
            _ => {
                info!("WiFi state: {:?}", wifi.get_wifi_state());
            }
        }

        if wifi.is_started() {
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    info!("AP started - waiting for station connection to send credentials");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}