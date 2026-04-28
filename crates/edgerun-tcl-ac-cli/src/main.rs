use edgerun_bluetooth_gatt::{format_gatt_uuid, AttProtocol, L2capSocket};
use edgerun_mgmt_bluetooth::{MgmtBluetoothBackend, MgmtDiscoveryTransport};
use edgerun_tcl_ac::{AcMode, AcState, FanSpeed, TclAcClient};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::process::{self, Command as ProcessCommand, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
#[cfg(unix)]
use std::{
    fs::OpenOptions,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
};

const STATE_FILE: &str = "/tmp/tcl-ac-state.json";
const DEFAULT_TCL_SESSION_FILE: &str = "~/.config/edgerun/tcl-home-session.json";
const DEFAULT_TCL_COUNTRY_CODE: &str = "TH";
const DEFAULT_TCL_ACCOUNT_HOST: &str = "https://sg.account.tcl.com";
const DEFAULT_TCL_CLIENT_ID: &str = "54148614";
const DEFAULT_TCL_IOT_CENTER: &str = "https://prod-center.aws.tcljd.com";

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedState {
    address: String,
    service_handle: Option<u16>,
    char_handle: Option<u16>,
}

enum Command {
    Scan {
        timeout: Option<u8>,
    },
    Connect {
        address: String,
    },
    Inspect {
        address: String,
    },
    Pair {
        address: String,
    },
    Provision {
        ssid: String,
        password: String,
        bind_code: String,
    },
    ProvisionFile {
        path: String,
        bind_code: String,
    },
    ProvisionBindFile {
        wifi_path: String,
        bind_path: String,
    },
    FetchBindCode {
        out_path: String,
    },
    Login {
        account: String,
        out_path: String,
    },
    LoginGoogleCode {
        auth_code: String,
        out_path: String,
    },
    DiagnoseProvision {
        address: String,
        wifi_path: String,
        bind_path: Option<String>,
    },
    SearchLan {
        timeout: Option<u64>,
        target_mac: Option<String>,
    },
    Status,
    Power {
        power_on: bool,
        power_off: bool,
    },
    Temp {
        temperature: i8,
    },
    Mode {
        mode: String,
    },
    Fan {
        speed: String,
    },
    Swing {
        swing_on: bool,
        swing_off: bool,
    },
    Eco {
        eco_on: bool,
        eco_off: bool,
    },
    Full {
        power: Option<bool>,
        temp: Option<i8>,
        mode: Option<String>,
        fan: Option<String>,
    },
    Disconnect,
}

fn main() {
    let cmd = parse_args();

    match cmd {
        Command::Scan { timeout } => do_scan(timeout.unwrap_or(10) as u64),
        Command::Connect { address } => do_connect(&address),
        Command::Inspect { address } => do_inspect(&address),
        Command::Pair { address } => do_pair(&address),
        Command::Provision {
            ssid,
            password,
            bind_code,
        } => do_provision(&ssid, &password, &bind_code),
        Command::ProvisionFile { path, bind_code } => do_provision_file(&path, &bind_code),
        Command::ProvisionBindFile {
            wifi_path,
            bind_path,
        } => do_provision_bind_file(&wifi_path, &bind_path),
        Command::FetchBindCode { out_path } => do_fetch_bind_code(&out_path),
        Command::Login { account, out_path } => do_login(&account, &out_path),
        Command::LoginGoogleCode {
            auth_code,
            out_path,
        } => do_login_google_code(&auth_code, &out_path),
        Command::DiagnoseProvision {
            address,
            wifi_path,
            bind_path,
        } => do_diagnose_provision(&address, &wifi_path, bind_path.as_deref()),
        Command::SearchLan {
            timeout,
            target_mac,
        } => do_search_lan(timeout.unwrap_or(60), target_mac.as_deref()),
        Command::Status => do_status(),
        Command::Power {
            power_on,
            power_off,
        } => do_power(power_on, power_off),
        Command::Temp { temperature } => do_temp(temperature),
        Command::Mode { mode } => do_mode(&mode),
        Command::Fan { speed } => do_fan(&speed),
        Command::Swing {
            swing_on,
            swing_off,
        } => do_swing(swing_on, swing_off),
        Command::Eco { eco_on, eco_off } => do_eco(eco_on, eco_off),
        Command::Full {
            power,
            temp,
            mode,
            fan,
        } => do_full(power, temp, mode.as_deref(), fan.as_deref()),
        Command::Disconnect => do_disconnect(),
    }
}

fn parse_args() -> Command {
    let args: Vec<String> = env::args().skip(1).collect();
    let sub = args.first().map(String::as_str).unwrap_or("status");
    match sub {
        "scan" => Command::Scan {
            timeout: args.get(1).and_then(|s| s.parse().ok()),
        },
        "connect" => Command::Connect {
            address: args.get(1).cloned().unwrap_or_default(),
        },
        "inspect" => Command::Inspect {
            address: args.get(1).cloned().unwrap_or_default(),
        },
        "pair" => Command::Pair {
            address: args.get(1).cloned().unwrap_or_default(),
        },
        "provision" => Command::Provision {
            ssid: args.get(1).cloned().unwrap_or_default(),
            password: args.get(2).cloned().unwrap_or_default(),
            bind_code: args.get(3).cloned().unwrap_or_default(),
        },
        "provision-file" => Command::ProvisionFile {
            path: args
                .get(1)
                .cloned()
                .unwrap_or_else(|| "~/wifi.txt".to_string()),
            bind_code: args.get(2).cloned().unwrap_or_default(),
        },
        "provision-bind-file" => Command::ProvisionBindFile {
            wifi_path: args
                .get(1)
                .cloned()
                .unwrap_or_else(|| "~/wifi.txt".to_string()),
            bind_path: args.get(2).cloned().unwrap_or_default(),
        },
        "fetch-bind-code" => Command::FetchBindCode {
            out_path: args
                .get(1)
                .cloned()
                .unwrap_or_else(|| "/tmp/tcl-bind.json".to_string()),
        },
        "login" => Command::Login {
            account: args.get(1).cloned().unwrap_or_default(),
            out_path: args
                .get(2)
                .cloned()
                .unwrap_or_else(|| default_session_path().to_string_lossy().to_string()),
        },
        "login-google-code" => Command::LoginGoogleCode {
            auth_code: args
                .get(1)
                .cloned()
                .or_else(|| env::var("TCL_GOOGLE_AUTH_CODE").ok())
                .unwrap_or_default(),
            out_path: args
                .get(2)
                .cloned()
                .unwrap_or_else(|| default_session_path().to_string_lossy().to_string()),
        },
        "diagnose-provision" => Command::DiagnoseProvision {
            address: args.get(1).cloned().unwrap_or_default(),
            wifi_path: args
                .get(2)
                .cloned()
                .unwrap_or_else(|| "~/wifi.txt".to_string()),
            bind_path: args.get(3).cloned(),
        },
        "search-lan" => Command::SearchLan {
            timeout: args.get(1).and_then(|s| s.parse().ok()),
            target_mac: args.get(2).cloned(),
        },
        "status" => Command::Status,
        "power" => Command::Power {
            power_on: args.iter().any(|s| s == "on" || s == "--on"),
            power_off: args.iter().any(|s| s == "off" || s == "--off"),
        },
        "temp" => Command::Temp {
            temperature: args.get(1).and_then(|s| s.parse().ok()).unwrap_or(25),
        },
        "mode" => Command::Mode {
            mode: args.get(1).cloned().unwrap_or_default(),
        },
        "fan" => Command::Fan {
            speed: args.get(1).cloned().unwrap_or_default(),
        },
        "swing" => Command::Swing {
            swing_on: args.iter().any(|s| s == "on" || s == "--on"),
            swing_off: args.iter().any(|s| s == "off" || s == "--off"),
        },
        "eco" => Command::Eco {
            eco_on: args.iter().any(|s| s == "on" || s == "--on"),
            eco_off: args.iter().any(|s| s == "off" || s == "--off"),
        },
        "full" => parse_full_args(&args[1..]),
        "disconnect" => Command::Disconnect,
        _ => Command::Status,
    }
}

fn parse_full_args(args: &[String]) -> Command {
    let mut power = None;
    let mut temp = None;
    let mut mode = None;
    let mut fan = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--power" | "power" => {
                power = args.get(i + 1).and_then(|s| parse_bool(s));
                i += 1;
            }
            "--temp" | "temp" => {
                temp = args.get(i + 1).and_then(|s| s.parse().ok());
                i += 1;
            }
            "--mode" | "mode" => {
                mode = args.get(i + 1).cloned();
                i += 1;
            }
            "--fan" | "fan" => {
                fan = args.get(i + 1).cloned();
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    Command::Full {
        power,
        temp,
        mode,
        fan,
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn load_saved_state() -> Option<SavedState> {
    let data = fs::read_to_string(STATE_FILE).ok()?;
    edgerun_json::from_str(&data).ok()
}

fn save_state(state: &SavedState) {
    if let Ok(data) = edgerun_json::to_string(state) {
        let _ = fs::write(state_file(), data);
    }
}

fn state_file() -> PathBuf {
    PathBuf::from(STATE_FILE)
}

fn do_scan(timeout_secs: u64) {
    println!("Scanning for TCL AC devices...");
    println!("Timeout: {}s", timeout_secs);
    println!();

    let controllers = match edgerun_mgmt_bluetooth::discover_controllers() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to discover Bluetooth controllers: {}", e);
            eprintln!("Make sure Bluetooth is available and you have permissions.");
            process::exit(1);
        }
    };

    if controllers.is_empty() {
        eprintln!("No Bluetooth controllers found.");
        process::exit(1);
    }

    let backend = MgmtBluetoothBackend {
        controller: controllers[0].clone(),
    };

    println!("Using controller: {}", backend.controller.address);
    println!();

    let result = match backend.discover_nearby(
        MgmtDiscoveryTransport::LowEnergy,
        (timeout_secs * 1000) as u32,
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Kernel mgmt scan failed: {}", e);
            eprintln!("Falling back to bluetoothctl LE scan.");
            do_bluetoothctl_scan(timeout_secs);
            return;
        }
    };

    let tcl_service_uuid = "0000f100-0000-1000-8000-00805f9b34fb";

    println!("Found {} device(s):", result.observations.len());
    println!();

    let mut tcl_devices: Vec<_> = Vec::new();

    for obs in &result.observations {
        let is_tcl = obs.service_uuids.iter().any(|uuid| {
            let normalized = uuid.replace("-", "").to_lowercase();
            normalized.ends_with(&tcl_service_uuid.replace("-", "").to_lowercase())
        });

        let name = obs.local_name.as_deref().unwrap_or("Unknown");
        let rssi = obs.rssi_dbm;

        println!("  {} ({}) - RSSI: {} dBm", obs.device_id, name, rssi);
        if !obs.service_uuids.is_empty() {
            println!(
                "    Services: {:?}",
                obs.service_uuids.iter().take(3).collect::<Vec<_>>()
            );
        }

        if is_tcl || name.to_lowercase().contains("tcl") || name.to_lowercase().contains("ac") {
            tcl_devices.push(obs.clone());
        }
    }

    println!();

    if tcl_devices.is_empty() {
        println!("No TCL AC devices found.");
        println!("Make sure your AC is powered on and in pairing mode.");
    } else {
        println!("TCL AC device(s) found:");
        for dev in &tcl_devices {
            let name = dev.local_name.as_deref().unwrap_or("Unknown");
            println!("  {} ({})", dev.device_id, name);
        }
        println!();
        println!("To connect, run:");
        println!("  tcl-ac connect <address>");
    }
}

fn do_bluetoothctl_scan(timeout_secs: u64) {
    let output = ProcessCommand::new("bluetoothctl")
        .arg("--timeout")
        .arg(timeout_secs.to_string())
        .arg("scan")
        .arg("le")
        .output();
    match output {
        Ok(output) => {
            print!("{}", String::from_utf8_lossy(&output.stdout));
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
            println!();
            println!("Run `bluetoothctl info <address>` for candidates, then:");
            println!("  cargo run -p edgerun-tcl-ac-cli --features std --bin tcl-ac -- connect <address>");
        }
        Err(err) => {
            eprintln!("bluetoothctl scan failed: {}", err);
            process::exit(1);
        }
    }
}

fn do_connect(address: &str) {
    let client = TclAcClient::new();

    println!("Connecting to TCL AC at {}...", address);

    match client.connect(address) {
        Ok(()) => {
            println!("Connected successfully!");

            if let Some(sh) = client.service_handle() {
                println!("  Service handle: 0x{:04x}", sh);
            }
            if let Some(ch) = client.characteristic_handle() {
                println!("  Characteristic handle: 0x{:04x}", ch);
            }

            let state = SavedState {
                address: address.to_string(),
                service_handle: client.service_handle(),
                char_handle: client.characteristic_handle(),
            };
            save_state(&state);
        }
        Err(e) => {
            eprintln!("Connection failed: {}", e);
            process::exit(1);
        }
    }
}

fn do_inspect(address: &str) {
    if address.is_empty() {
        eprintln!("Usage: tcl-ac inspect <address>");
        process::exit(2);
    }

    println!("Inspecting BLE GATT database at {}...", address);

    let socket = match connect_ble_socket(address) {
        Ok(socket) => socket,
        Err(err) => {
            eprintln!("Connection failed: {}", err);
            process::exit(1);
        }
    };

    let mut proto = AttProtocol::new(socket);
    proto.set_mtu(512);

    let primary_service_type = [0x00, 0x28];
    let data = match proto.read_by_group_type(0x0001, 0xffff, &primary_service_type) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Service discovery failed: {}", err);
            process::exit(1);
        }
    };

    let services = proto.parse_read_by_group_response(&data);
    if services.is_empty() {
        println!("No primary services discovered.");
        return;
    }

    println!("Services:");
    for (start, end, uuid) in services {
        let service_uuid = format_gatt_uuid(&uuid);
        println!("  0x{start:04x}-0x{end:04x}  {service_uuid}");

        let char_type = [0x03, 0x28];
        let chars = match proto.read_by_type(start, end, &char_type) {
            Ok(data) => proto.parse_read_by_type_response(&data),
            Err(err) => {
                println!("    characteristic discovery failed: {}", err);
                continue;
            }
        };

        for (decl_handle, value) in chars {
            if value.len() < 5 {
                println!("    0x{decl_handle:04x}  malformed characteristic declaration");
                continue;
            }
            let props = value[0];
            let value_handle = u16::from_le_bytes([value[1], value[2]]);
            let char_uuid = format_gatt_uuid(&value[3..]);
            println!(
                "    decl 0x{decl_handle:04x}, value 0x{value_handle:04x}, props 0x{props:02x}  {char_uuid}"
            );

            let desc_start = value_handle.saturating_add(1);
            if desc_start <= end {
                if let Ok(desc_data) = proto.find_information(desc_start, end) {
                    for (handle, desc_uuid) in proto.parse_find_information_response(&desc_data) {
                        println!(
                            "      desc 0x{handle:04x}  {}",
                            format_gatt_uuid(&desc_uuid)
                        );
                    }
                }
            }
        }
    }
}

fn connect_ble_socket(address: &str) -> Result<L2capSocket, String> {
    let public = L2capSocket::new()
        .and_then(|socket| {
            socket.connect_device(address, 0x01)?;
            Ok(socket)
        })
        .map_err(|err| err.to_string());
    match public {
        Ok(socket) => Ok(socket),
        Err(public_err) => L2capSocket::new()
            .and_then(|socket| {
                socket.connect_device(address, 0x02)?;
                Ok(socket)
            })
            .map_err(|random_err| {
                format!(
                    "failed as public ({}) and random ({})",
                    public_err, random_err
                )
            }),
    }
}

fn do_pair(address: &str) {
    let controllers = match edgerun_mgmt_bluetooth::discover_controllers() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to discover Bluetooth controllers: {}", e);
            process::exit(1);
        }
    };

    if controllers.is_empty() {
        eprintln!("No Bluetooth controllers found.");
        process::exit(1);
    }

    let backend = MgmtBluetoothBackend {
        controller: controllers[0].clone(),
    };

    println!("Pairing with TCL AC at {}...", address);

    if let Err(e) = backend.pair_device(address, 0x00) {
        eprintln!("Pairing failed: {}", e);
        process::exit(1);
    }

    println!("Pairing initiated. Waiting for completion...");

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(30);

    while start.elapsed() < timeout {
        match backend.monitor_events(1000) {
            Ok(events) => {
                for event in events {
                    match event {
                        edgerun_mgmt_bluetooth::MgmtControllerEvent::DeviceConnected {
                            device_id,
                            ..
                        } if device_id.eq_ignore_ascii_case(address) => {
                            println!("Pairing successful!");
                            return;
                        }
                        edgerun_mgmt_bluetooth::MgmtControllerEvent::ConnectFailed {
                            device_id,
                            status,
                            ..
                        } if device_id.eq_ignore_ascii_case(address) => {
                            eprintln!("Pairing failed with status: 0x{:02x}", status);
                            process::exit(1);
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                eprintln!("Monitor error: {}", e);
            }
        }
    }

    eprintln!("Pairing timed out.");
    process::exit(1);
}

fn with_client<F>(f: F)
where
    F: FnOnce(&TclAcClient) -> Result<(), edgerun_capabilities::CapabilityError>,
{
    let saved = load_saved_state().expect("Not connected - run 'tcl-ac connect <addr>' first");
    let client = TclAcClient::new();

    // Reconnect using saved address
    if let Err(e) = client.connect(&saved.address) {
        eprintln!("Reconnection failed: {}", e);
        process::exit(1);
    }

    f(&client).unwrap_or_else(|e| {
        eprintln!("Operation failed: {}", e);
        process::exit(1);
    });
}

fn do_status() {
    with_client(|client| match client.get_device_info() {
        Ok(info) => {
            println!("{}", info);
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to read device info: {}", e);
            Err(e)
        }
    })
}

fn do_provision(ssid: &str, password: &str, bind_code: &str) {
    if ssid.is_empty() || password.is_empty() {
        eprintln!("Usage: tcl-ac provision <ssid> <password> [bind-code]");
        process::exit(1);
    }

    with_client(|client| {
        match client.provision_wifi_responses(ssid, password, bind_code, None, None, 60_000) {
            Ok(responses) if responses.is_empty() => {
                println!("Provisioning payload accepted; no indication response was received.");
                Ok(())
            }
            Ok(responses) => {
                for response in responses {
                    println!("{}", response);
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("Provisioning failed: {}", e);
                Err(e)
            }
        }
    })
}

fn do_provision_file(path: &str, bind_code: &str) {
    let (ssid, password) = read_wifi_credentials(path);
    do_provision(&ssid, &password, bind_code);
}

fn do_provision_bind_file(wifi_path: &str, bind_path: &str) {
    if bind_path.is_empty() {
        eprintln!("Usage: tcl-ac provision-bind-file <wifi-file> <bind-response-json>");
        process::exit(1);
    }
    let (ssid, password) = read_wifi_credentials(wifi_path);
    let bind_path = expand_home(bind_path);
    let bind_data = match fs::read_to_string(&bind_path) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to read bind response file: {}", err);
            process::exit(1);
        }
    };
    let bind_code = json_string_value(&bind_data, "bindCode").unwrap_or_default();
    if bind_code.is_empty() {
        eprintln!("Bind response JSON does not contain bindCode");
        process::exit(1);
    }
    let server_host = json_string_value(&bind_data, "deviceMqttEndpoint");
    let server_host_v2 = json_string_value(&bind_data, "deviceMqttEndpointV2");

    with_client(|client| {
        match client.provision_wifi_with_commission_responses(
            &ssid,
            &password,
            &bind_code,
            server_host.as_deref(),
            server_host_v2.as_deref(),
            None,
            None,
            60_000,
        ) {
            Ok(responses) if responses.is_empty() => {
                println!("Provisioning payload accepted; no indication response was received.");
                Ok(())
            }
            Ok(responses) => {
                for response in responses {
                    println!("{}", response);
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("Provisioning failed: {}", e);
                Err(e)
            }
        }
    })
}

struct CommissioningData {
    bind_code: String,
    server_host: Option<String>,
    server_host_v2: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
struct TclSession {
    account: String,
    country_code: String,
    account_host: String,
    client_id: String,
    iot_center_url: String,
    iot_cloud_url: Option<String>,
    iot_cloud_url_emq: Option<String>,
    sso_id: Option<String>,
    access_token: String,
    refresh_token: Option<String>,
    saas_token: Option<String>,
    iot_sso_token: Option<String>,
    mqtt_endpoint: Option<String>,
    mqtt_endpoint_emq: Option<String>,
    saved_at_ms: u128,
}

#[derive(serde::Serialize)]
struct LoginRequest {
    channel: String,
    username: String,
    password: String,
    #[serde(rename = "captchaRule")]
    captcha_rule: i32,
    #[serde(rename = "osType")]
    os_type: i32,
    #[serde(rename = "osVersion")]
    os_version: String,
    equipment: i32,
    #[serde(rename = "clientVersion")]
    client_version: String,
    #[serde(rename = "deviceModel")]
    device_model: String,
}

#[derive(serde::Serialize)]
struct IotHostRequest {
    #[serde(rename = "ssoId")]
    sso_id: String,
    #[serde(rename = "ssoToken")]
    sso_token: String,
}

#[derive(serde::Serialize)]
struct IotRefreshRequest {
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "ssoToken")]
    sso_token: String,
    #[serde(rename = "appId")]
    app_id: String,
}

#[derive(serde::Serialize)]
struct BindCodeRequest {
    #[serde(rename = "jobId")]
    job_id: String,
    add_soure: String,
    entrance_id: String,
    #[serde(rename = "productKey")]
    product_key: String,
    #[serde(rename = "cloudType")]
    cloud_type: i32,
    #[serde(rename = "channelType")]
    channel_type: i32,
    #[serde(rename = "wifiMd5", skip_serializing_if = "Option::is_none")]
    wifi_md5: Option<String>,
}

fn read_commissioning_data(bind_path: Option<&str>) -> CommissioningData {
    let Some(bind_path) = bind_path.filter(|path| !path.is_empty()) else {
        return CommissioningData {
            bind_code: String::new(),
            server_host: None,
            server_host_v2: None,
        };
    };

    let bind_path = expand_home(bind_path);
    let bind_data = match fs::read_to_string(&bind_path) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to read bind response file: {}", err);
            process::exit(1);
        }
    };
    let bind_code = json_string_value(&bind_data, "bindCode").unwrap_or_default();
    if bind_code.is_empty() {
        eprintln!("Bind response JSON does not contain bindCode");
        process::exit(1);
    }

    CommissioningData {
        bind_code,
        server_host: json_string_value(&bind_data, "deviceMqttEndpoint"),
        server_host_v2: json_string_value(&bind_data, "deviceMqttEndpointV2"),
    }
}

fn do_login(account: &str, out_path: &str) {
    if account.is_empty() {
        eprintln!("Usage: tcl-ac login <email-or-phone> [session-json]");
        process::exit(2);
    }

    let login_config = LoginConfig::from_env();
    let password = env::var("TCL_PASSWORD")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| read_secret_prompt("TCL password: "));

    let request = LoginRequest {
        channel: "app".to_string(),
        username: account.to_string(),
        password: md5_hex(&password),
        captcha_rule: 2,
        os_type: 1,
        os_version: env::var("TCL_OS_VERSION").unwrap_or_else(|_| "Android 15".to_string()),
        equipment: 1,
        client_version: env::var("TCL_HOME_VERSION").unwrap_or_else(|_| "6.1.1".to_string()),
        device_model: env::var("TCL_DEVICE_MODEL").unwrap_or_else(|_| "Linux laptop".to_string()),
    };
    drop(password);

    let body = json_or_exit(&request, "login request");
    let url = format!(
        "{}/account/login?clientId={}",
        login_config.account_host.trim_end_matches('/'),
        login_config.client_id
    );
    let headers = vec![
        ("Content-Type", "application/json".to_string()),
        ("Accept", "application/json".to_string()),
    ];

    println!("Logging in to TCL account...");
    println!("  Account host: {}", login_config.account_host);
    let login_response = curl_post_json_with_headers(&url, &headers, &body);
    save_session_from_login_response(account, login_config, &login_response, out_path);
}

fn do_login_google_code(auth_code: &str, out_path: &str) {
    if auth_code.is_empty() {
        eprintln!("Usage: tcl-ac login-google-code <google-server-auth-code> [session-json]");
        eprintln!("You can also set TCL_GOOGLE_AUTH_CODE.");
        process::exit(2);
    }

    let login_config = LoginConfig::from_env();
    let url = format!(
        "{}/account/thirdParty/thirdLoginByAccessToken?platformId=6&token={}&flowTag=0&countryAbbr={}&appId={}&osType=1&osVersion={}&equipment=1&clientVersion={}&deviceModel={}",
        login_config.account_host.trim_end_matches('/'),
        url_query_escape(auth_code),
        url_query_escape(&login_config.country_code),
        url_query_escape(&login_config.client_id),
        url_query_escape(&env::var("TCL_OS_VERSION").unwrap_or_else(|_| "Android 15".to_string())),
        url_query_escape(&env::var("TCL_HOME_VERSION").unwrap_or_else(|_| "6.1.1".to_string())),
        url_query_escape(&env::var("TCL_DEVICE_MODEL").unwrap_or_else(|_| "Linux laptop".to_string())),
    );
    let headers = vec![
        ("Content-Type", "application/json".to_string()),
        ("Accept", "application/json".to_string()),
    ];

    println!("Logging in to TCL account with Google auth code...");
    println!("  Account host: {}", login_config.account_host);
    let login_response = curl_post_json_with_headers(&url, &headers, "");
    let account = json_string_value(&login_response, "email")
        .or_else(|| json_string_value(&login_response, "username"))
        .unwrap_or_else(|| "google".to_string());
    save_session_from_login_response(&account, login_config, &login_response, out_path);
}

struct LoginConfig {
    country_code: String,
    account_host: String,
    client_id: String,
    iot_center_url: String,
}

impl LoginConfig {
    fn from_env() -> Self {
        let country_code =
            env::var("TCL_COUNTRY_CODE").unwrap_or_else(|_| DEFAULT_TCL_COUNTRY_CODE.to_string());
        let account_host = env::var("TCL_ACCOUNT_HOST")
            .unwrap_or_else(|_| account_host_for_country(&country_code));
        let client_id =
            env::var("TCL_CLIENT_ID").unwrap_or_else(|_| DEFAULT_TCL_CLIENT_ID.to_string());
        let iot_center_url =
            env::var("TCL_IOT_CENTER_URL").unwrap_or_else(|_| DEFAULT_TCL_IOT_CENTER.to_string());
        Self {
            country_code,
            account_host,
            client_id,
            iot_center_url,
        }
    }
}

fn save_session_from_login_response(
    account: &str,
    login_config: LoginConfig,
    login_response: &str,
    out_path: &str,
) {
    let access_token = required_json_string(login_response, "token", "login response");
    let refresh_token = json_string_value(&login_response, "refreshtoken");
    let sso_id = json_string_value(login_response, "username")
        .or_else(|| json_string_value(login_response, "email").filter(|value| value == account));
    let sso_id = sso_id.unwrap_or_else(|| account.to_string());

    let host_request = IotHostRequest {
        sso_id: sso_id.clone(),
        sso_token: access_token.clone(),
    };
    let host_body = json_or_exit(&host_request, "IoT host request");
    let iot_host_url = format!(
        "{}/v3/global/cloud_url_get",
        login_config.iot_center_url.trim_end_matches('/')
    );
    let iot_headers = vec![
        ("Content-Type", "application/json".to_string()),
        ("Accept", "application/json".to_string()),
        ("THomeVersion", "6.1.1".to_string()),
    ];

    println!("Fetching TCL IoT cloud host...");
    let host_response = curl_post_json_with_headers(&iot_host_url, &iot_headers, &host_body);
    let iot_cloud_url = json_string_value(&host_response, "cloud_url")
        .unwrap_or_else(|| login_config.iot_center_url.clone());
    let iot_cloud_url_emq = json_string_value(&host_response, "cloud_url_emq");

    let refresh_request = IotRefreshRequest {
        user_id: sso_id.clone(),
        sso_token: access_token.clone(),
        app_id: env::var("TCL_APP_ID").unwrap_or_else(|_| "wx6e1af3fa84fbe523".to_string()),
    };
    let refresh_body = json_or_exit(&refresh_request, "IoT token request");
    let refresh_url = format!(
        "{}/v3/auth/refresh_tokens",
        iot_cloud_url.trim_end_matches('/')
    );
    let headers = vec![
        ("Content-Type", "application/json".to_string()),
        ("Accept", "application/json".to_string()),
    ];

    println!("Fetching TCL IoT access token...");
    let refresh_response = curl_post_json_with_headers(&refresh_url, &headers, &refresh_body);

    let session = TclSession {
        account: account.to_string(),
        country_code: login_config.country_code,
        account_host: login_config.account_host,
        client_id: login_config.client_id,
        iot_center_url: login_config.iot_center_url,
        iot_cloud_url: Some(iot_cloud_url),
        iot_cloud_url_emq,
        sso_id: Some(sso_id),
        access_token,
        refresh_token,
        saas_token: json_string_value(&refresh_response, "saasToken"),
        iot_sso_token: json_string_value(&refresh_response, "ssoToken"),
        mqtt_endpoint: json_string_value(&refresh_response, "mqttEndpoint"),
        mqtt_endpoint_emq: json_string_value(&refresh_response, "mqttEndpointEmq"),
        saved_at_ms: now_millis(),
    };

    let out_path = expand_home(out_path);
    let session_json = json_or_exit(&session, "session");
    if let Err(err) = write_private_file(&out_path, session_json.as_bytes()) {
        eprintln!("Failed to write session file: {}", err);
        process::exit(1);
    }

    println!("Login session saved.");
    println!("  File: {}", out_path.display());
    println!(
        "  Account token: present ({} chars)",
        session.access_token.len()
    );
    match session.saas_token.as_deref() {
        Some(token) if !token.is_empty() => {
            println!("  IoT token: present ({} chars)", token.len())
        }
        _ => println!("  IoT token: not present in response"),
    }
}

fn do_fetch_bind_code(out_path: &str) {
    let session = load_tcl_session();
    let access_token = env_or_session("TCL_ACCESS_TOKEN", &session, |session| {
        session
            .saas_token
            .as_deref()
            .or(Some(&session.access_token))
    });
    let sso_token = env::var("TCL_SSO_TOKEN")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            session.as_ref().and_then(|session| {
                session
                    .iot_sso_token
                    .clone()
                    .or(Some(session.access_token.clone()))
            })
        })
        .unwrap_or_default();
    let product_key = required_env("TCL_PRODUCT_KEY");
    let entrance_id = env::var("TCL_ENTRANCE_ID").unwrap_or_else(|_| product_key.clone());
    let job_id = env::var("TCL_JOB_ID").unwrap_or_else(|_| format!("edgerun-{}", now_millis()));
    let add_source = env::var("TCL_ADD_SOURCE").unwrap_or_else(|_| "manual".to_string());
    let cloud_type = env_i32("TCL_CLOUD_TYPE", 1);
    let channel_type = env_i32("TCL_CHANNEL_TYPE", 0);
    let wifi_md5 = env::var("TCL_WIFI_MD5")
        .ok()
        .filter(|value| !value.is_empty());
    let base_url = env::var("TCL_IOT_BASE_URL")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            session
                .as_ref()
                .and_then(|session| session.iot_cloud_url.clone())
        })
        .unwrap_or_else(|| DEFAULT_TCL_IOT_CENTER.into());
    let country_code = env::var("TCL_COUNTRY_CODE")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| session.as_ref().map(|session| session.country_code.clone()))
        .unwrap_or_else(|| DEFAULT_TCL_COUNTRY_CODE.to_string());
    let time_zone = env::var("TCL_TIME_ZONE").unwrap_or_else(|_| "Asia/Bangkok".to_string());
    let accept_language = env::var("TCL_ACCEPT_LANGUAGE").unwrap_or_else(|_| "en-US".to_string());

    let request = BindCodeRequest {
        job_id,
        add_soure: add_source,
        entrance_id,
        product_key,
        cloud_type,
        channel_type,
        wifi_md5,
    };
    let body = match edgerun_json::to_string(&request) {
        Ok(body) => body,
        Err(err) => {
            eprintln!("Failed to build bind-code request JSON: {}", err);
            process::exit(1);
        }
    };

    let timestamp = now_millis().to_string();
    let nonce =
        read_kernel_uuid().unwrap_or_else(|| format!("edgerun-{}-{}", process::id(), timestamp));
    let sign = md5_hex(&format!("{}{}{}", timestamp, nonce, access_token));
    let url = format!("{}/v1/auth/get_bind_code", base_url.trim_end_matches('/'));

    let headers = vec![
        ("Content-Type", "application/json".to_string()),
        ("ssoToken", sso_token),
        ("appId", "wx6e1af3fa84fbe523".to_string()),
        ("platform", "android".to_string()),
        ("appVersion", "7.4.0".to_string()),
        ("THomeVersion", "6.1.1".to_string()),
        ("accessToken", access_token),
        ("countryCode", country_code),
        ("timeZone", time_zone),
        ("Accept-Language", accept_language),
        ("timestamp", timestamp),
        ("nonce", nonce),
        ("sign", sign),
    ];

    println!("Requesting TCL bind code...");
    println!("  URL: {}", url);
    println!("  Output: {}", expand_home(out_path).display());
    let response = curl_post_json_with_headers(&url, &headers, &body);

    let out_path = expand_home(out_path);
    if let Err(err) = write_private_file(&out_path, response.as_bytes()) {
        eprintln!("Failed to write bind response: {}", err);
        process::exit(1);
    }

    if let Some(bind_code) = json_string_value(&response, "bindCode") {
        println!("Bind response saved.");
        println!("  bindCode: present ({} chars)", bind_code.len());
        if let Some(host) = json_string_value(&response, "deviceMqttEndpoint") {
            println!("  deviceMqttEndpoint: {}", redact_url_secret(&host));
        }
        if let Some(host) = json_string_value(&response, "deviceMqttEndpointV2") {
            println!("  deviceMqttEndpointV2: {}", redact_url_secret(&host));
        }
    } else {
        println!("Response saved, but no bindCode was found in the JSON.");
    }
}

fn do_diagnose_provision(address: &str, wifi_path: &str, bind_path: Option<&str>) {
    if address.is_empty() {
        eprintln!("Usage: tcl-ac diagnose-provision <address> [wifi-file] [bind-response-json]");
        process::exit(2);
    }

    let (ssid, password) = read_wifi_credentials(wifi_path);
    let commissioning = read_commissioning_data(bind_path);
    let client = TclAcClient::new();

    println!("Connecting to TCL AC at {}...", address);
    if let Err(err) = client.connect(address) {
        eprintln!("Connection failed: {}", err);
        process::exit(1);
    }

    println!("Connected.");
    println!("  ATT MTU: {}", client.negotiated_mtu());
    println!(
        "  Transport: {}",
        if client.uses_legacy_provisioning() {
            "legacy f100 JSON"
        } else {
            "encrypted f900 protocol"
        }
    );
    println!("  Wi-Fi SSID length: {}", ssid.len());
    println!(
        "  Bind code: {}",
        if commissioning.bind_code.is_empty() {
            "not supplied"
        } else {
            "supplied"
        }
    );
    if let Some(host) = commissioning.server_host.as_deref() {
        println!("  serverHost: {}", redact_url_secret(host));
    }
    if let Some(host) = commissioning.server_host_v2.as_deref() {
        println!("  serverHostV2: {}", redact_url_secret(host));
    }

    println!("Sending provisioning payload and waiting for BLE indications...");
    match client.provision_wifi_with_commission_responses(
        &ssid,
        &password,
        &commissioning.bind_code,
        commissioning.server_host.as_deref(),
        commissioning.server_host_v2.as_deref(),
        None,
        None,
        60_000,
    ) {
        Ok(responses) if responses.is_empty() => {
            println!("  BLE: write accepted, no indication received in 60s");
        }
        Ok(responses) => {
            println!("  BLE: {} indication response(s)", responses.len());
            for response in responses {
                println!("    {}", response);
            }
        }
        Err(err) => {
            eprintln!("Provisioning failed: {}", err);
            process::exit(1);
        }
    }

    println!("Searching LAN for post-provision UDP discovery responses...");
    search_lan_for(Duration::from_secs(75));

    println!("Checking current bluetoothctl device state...");
    print_bluetoothctl_info(address);
}

fn read_wifi_credentials(path: &str) -> (String, String) {
    let path = expand_home(path);
    let data = match fs::read_to_string(&path) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to read Wi-Fi credential file: {}", err);
            process::exit(1);
        }
    };
    let mut lines = data.lines().map(str::trim).filter(|line| !line.is_empty());
    let ssid = lines.next().unwrap_or_default();
    let password = lines.next().unwrap_or_default();
    if ssid.is_empty() || password.is_empty() {
        eprintln!("Wi-Fi credential file must contain SSID on line 1 and password on line 2");
        process::exit(1);
    }
    (ssid.to_string(), password.to_string())
}

fn required_env(name: &str) -> String {
    match env::var(name) {
        Ok(value) if !value.is_empty() => value,
        _ => {
            eprintln!("Missing required environment variable: {}", name);
            process::exit(2);
        }
    }
}

fn env_or_session<F>(name: &str, session: &Option<TclSession>, value: F) -> String
where
    F: FnOnce(&TclSession) -> Option<&str>,
{
    if let Ok(env_value) = env::var(name) {
        if !env_value.is_empty() {
            return env_value;
        }
    }
    if let Some(session_value) = session.as_ref().and_then(value) {
        if !session_value.is_empty() {
            return session_value.to_string();
        }
    }
    eprintln!(
        "Missing {}. Run `tcl-ac login <account>` first or set the environment variable.",
        name
    );
    process::exit(2);
}

fn env_i32(name: &str, default: i32) -> i32 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn json_or_exit<T: serde::Serialize>(value: &T, label: &str) -> String {
    match edgerun_json::to_string(value) {
        Ok(body) => body,
        Err(err) => {
            eprintln!("Failed to build {} JSON: {}", label, err);
            process::exit(1);
        }
    }
}

fn required_json_string(data: &str, key: &str, label: &str) -> String {
    match json_string_value(data, key) {
        Some(value) if !value.is_empty() => value,
        _ => {
            eprintln!("{} does not contain required `{}`", label, key);
            let status = json_string_value(data, "status").unwrap_or_default();
            let msg = json_string_value(data, "msg").unwrap_or_default();
            if !status.is_empty() || !msg.is_empty() {
                eprintln!("  status: {}", status);
                eprintln!("  msg: {}", msg);
            }
            process::exit(1);
        }
    }
}

fn default_session_path() -> PathBuf {
    expand_home(DEFAULT_TCL_SESSION_FILE)
}

fn load_tcl_session() -> Option<TclSession> {
    let path = env::var("TCL_SESSION_FILE")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| expand_home(&value))
        .unwrap_or_else(default_session_path);
    let data = fs::read_to_string(path).ok()?;
    edgerun_json::from_str(&data).ok()
}

fn account_host_for_country(country_code: &str) -> String {
    match country_code.to_ascii_uppercase().as_str() {
        "AU" | "NZ" => "https://au.account.tcl.com",
        "AE" | "KW" | "SA" | "BH" | "ZA" => "https://bh.account.tcl.com",
        "VN" | "MM" | "PH" | "TH" | "SG" | "ID" | "PK" | "HK" | "TW" | "NP" | "MY" | "LK"
        | "GE" | "AZ" | "KR" | "PY" => "https://sg.account.tcl.com",
        "US" | "CA" | "CL" | "EC" | "BO" | "DO" | "PR" | "PA" | "UY" | "CR" | "NI" | "JM" => {
            "https://us.account.tcl.com"
        }
        "BR" | "SR" => "https://br.account.tcl.com",
        "RU" => "https://rus.account.tcl.com",
        "IN" => "https://in.account.tcl.com",
        _ => DEFAULT_TCL_ACCOUNT_HOST,
    }
    .to_string()
}

fn read_secret_prompt(prompt: &str) -> String {
    print!("{}", prompt);
    let _ = io::stdout().flush();
    let echo_disabled = ProcessCommand::new("stty").arg("-echo").status().is_ok();
    let mut value = String::new();
    let read_result = io::stdin().read_line(&mut value);
    if echo_disabled {
        let _ = ProcessCommand::new("stty").arg("echo").status();
        println!();
    }
    if let Err(err) = read_result {
        eprintln!("Failed to read password: {}", err);
        process::exit(1);
    }
    let value = value.trim_end_matches(['\r', '\n']).to_string();
    if value.is_empty() {
        eprintln!("Password cannot be empty.");
        process::exit(2);
    }
    value
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn read_kernel_uuid() -> Option<String> {
    fs::read_to_string("/proc/sys/kernel/random/uuid")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn md5_hex(input: &str) -> String {
    let mut child = match ProcessCommand::new("md5sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            eprintln!("Failed to run md5sum: {}", err);
            process::exit(1);
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        if let Err(err) = stdin.write_all(input.as_bytes()) {
            eprintln!("Failed to write md5 input: {}", err);
            process::exit(1);
        }
    }

    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(err) => {
            eprintln!("Failed to read md5sum output: {}", err);
            process::exit(1);
        }
    };
    if !output.status.success() {
        eprintln!("md5sum failed");
        process::exit(1);
    }
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string()
}

fn curl_post_json_with_headers(url: &str, headers: &[(&str, String)], body: &str) -> String {
    let temp_base = format!("/tmp/tcl-bind-{}-{}", process::id(), now_millis());
    let config_path = PathBuf::from(format!("{}.curl", temp_base));
    let body_path = PathBuf::from(format!("{}.json", temp_base));

    if let Err(err) = write_private_file(&body_path, body.as_bytes()) {
        eprintln!("Failed to write curl request body: {}", err);
        process::exit(1);
    }

    let mut config = String::new();
    config.push_str("request = \"POST\"\n");
    config.push_str(&format!("url = \"{}\"\n", curl_escape(url)));
    config.push_str("silent\n");
    config.push_str("show-error\n");
    config.push_str("fail-with-body\n");
    config.push_str("connect-timeout = 15\n");
    config.push_str("max-time = 45\n");
    config.push_str(&format!(
        "data-binary = \"@{}\"\n",
        curl_escape(&body_path.to_string_lossy())
    ));
    for (name, value) in headers {
        config.push_str(&format!(
            "header = \"{}: {}\"\n",
            curl_escape(name),
            curl_escape(value)
        ));
    }

    if let Err(err) = write_private_file(&config_path, config.as_bytes()) {
        eprintln!("Failed to write curl config: {}", err);
        let _ = fs::remove_file(&body_path);
        process::exit(1);
    }

    let output = ProcessCommand::new("curl")
        .arg("--config")
        .arg(&config_path)
        .output();

    let _ = fs::remove_file(&config_path);
    let _ = fs::remove_file(&body_path);

    let output = match output {
        Ok(output) => output,
        Err(err) => {
            eprintln!("Failed to run curl: {}", err);
            process::exit(1);
        }
    };

    if !output.status.success() {
        eprintln!("TCL HTTP request failed with status: {}", output.status);
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            eprintln!("{}", stdout.trim());
        }
        process::exit(1);
    }

    String::from_utf8_lossy(&output.stdout).to_string()
}

fn curl_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn url_query_escape(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn write_private_file(path: &PathBuf, data: &[u8]) -> io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    #[cfg(unix)]
    {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(data)?;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        fs::write(path, data)
    }
}

fn redact_url_secret(value: &str) -> String {
    if let Some((head, _tail)) = value.split_once('?') {
        return head.to_string();
    }
    value.to_string()
}

fn json_string_value(data: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let key_pos = data.find(&pattern)?;
    let after_key = &data[key_pos + pattern.len()..];
    let colon_pos = after_key.find(':')?;
    let mut value = after_key[colon_pos + 1..].trim_start();
    if value.starts_with("null") {
        return None;
    }
    value = value.strip_prefix('"')?;

    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'b' => out.push('\u{0008}'),
                'f' => out.push('\u{000c}'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'u' => return None,
                other => out.push(other),
            },
            other => out.push(other),
        }
    }
    None
}

fn do_search_lan(timeout_secs: u64, target_mac: Option<&str>) {
    search_lan_for_with_target(Duration::from_secs(timeout_secs), target_mac);
}

fn search_lan_for(timeout: Duration) {
    search_lan_for_with_target(timeout, None);
}

fn search_lan_for_with_target(timeout: Duration, target_mac: Option<&str>) {
    let mut listeners = Vec::new();
    for port in [10074, 10075] {
        match UdpSocket::bind(("0.0.0.0", port)) {
            Ok(socket) => {
                if let Err(err) = socket.set_read_timeout(Some(Duration::from_millis(200))) {
                    eprintln!("Failed to set UDP read timeout on port {}: {}", port, err);
                    process::exit(1);
                }
                listeners.push((port, socket));
            }
            Err(err) if port == 10075 => {
                eprintln!(
                    "Warning: could not bind optional UDP listener on port 10075: {}",
                    err
                );
            }
            Err(err) => {
                eprintln!("Failed to bind UDP listener on port {}: {}", port, err);
                process::exit(1);
            }
        }
    }

    let sender = match UdpSocket::bind(("0.0.0.0", 0)) {
        Ok(socket) => socket,
        Err(err) => {
            eprintln!("Failed to bind UDP sender: {}", err);
            process::exit(1);
        }
    };
    if let Err(err) = sender.set_broadcast(true) {
        eprintln!("Failed to enable UDP broadcast: {}", err);
        process::exit(1);
    }

    println!(
        "Searching LAN for TCL devices via UDP ports 10074/10075 for {}s...",
        timeout.as_secs()
    );
    if let Some(target) = target_mac {
        println!("Target MAC fuzzy match: {}", target);
    }

    let broadcast = SocketAddr::from((Ipv4Addr::BROADCAST, 10075));
    let xml_search = b"<searchDevice></searchDevice>";
    let json_search = br#"{"msgId":"123","version":"123","method":"searchReq"}"#;
    let xml_target_search = target_mac.map(|target| {
        let normalized = normalize_mac(target);
        format!(
            "<searchDevice devid=\"{}\" randcode=\"123456\"></searchDevice>",
            normalized
        )
    });
    let start = Instant::now();
    let mut next_send = Instant::now();
    let mut send_json = false;
    let mut seen = Vec::<String>::new();
    let mut buf = [0u8; 2048];

    while start.elapsed() < timeout {
        if Instant::now() >= next_send {
            let payload: &[u8] = if send_json { json_search } else { xml_search };
            if let Err(err) = sender.send_to(payload, broadcast) {
                eprintln!("UDP broadcast failed: {}", err);
            }
            if let Some(target_payload) = xml_target_search.as_ref() {
                if let Err(err) = sender.send_to(target_payload.as_bytes(), broadcast) {
                    eprintln!("UDP target broadcast failed: {}", err);
                }
            }
            send_json = !send_json;
            next_send = Instant::now() + Duration::from_millis(1500);
        }

        for (port, listen) in &listeners {
            match listen.recv_from(&mut buf) {
                Ok((n, from)) => {
                    let text = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                    if text.is_empty()
                        || text == String::from_utf8_lossy(xml_search)
                        || text == String::from_utf8_lossy(json_search)
                        || xml_target_search
                            .as_deref()
                            .is_some_and(|payload| payload == text)
                    {
                        continue;
                    }
                    let key = format!("{from}/{port}/{text}");
                    if seen.iter().any(|entry| entry == &key) {
                        continue;
                    }
                    seen.push(key);
                    print_lan_response(from, *port, &text, target_mac);
                }
                Err(err)
                    if err.kind() == io::ErrorKind::WouldBlock
                        || err.kind() == io::ErrorKind::TimedOut => {}
                Err(err) => {
                    eprintln!("UDP receive failed on port {}: {}", port, err);
                    process::exit(1);
                }
            }
        }
    }

    if seen.is_empty() {
        println!("No TCL LAN discovery responses received.");
    }
}

fn print_lan_response(from: SocketAddr, listen_port: u16, text: &str, target_mac: Option<&str>) {
    println!("{} -> local:{}  {}", from, listen_port, text);

    let fields = lan_response_fields(text);
    if fields.is_empty() {
        return;
    }

    println!("  parsed:");
    for (name, value) in &fields {
        println!("    {}: {}", name, value);
    }

    if let Some(target) = target_mac {
        let candidate = fields
            .iter()
            .find(|(name, _)| matches!(*name, "mac" | "deviceId" | "did" | "tid"))
            .map(|(_, value)| value.as_str());
        if let Some(candidate) = candidate {
            println!(
                "    targetMatch: {}",
                if fuzzy_mac_match(target, candidate) {
                    "yes"
                } else {
                    "no"
                }
            );
        }
    }
}

fn lan_response_fields(text: &str) -> Vec<(&'static str, String)> {
    let mut fields = Vec::new();
    if text.trim_start().starts_with('{') {
        for key in [
            "did",
            "deviceId",
            "productKey",
            "ip",
            "port",
            "mac",
            "ssid",
            "resetFlag",
        ] {
            if let Some(value) = json_string_value(text, key).filter(|value| !value.is_empty()) {
                fields.push((key, value));
            }
        }
    } else {
        for key in [
            "did",
            "deviceId",
            "tid",
            "productKey",
            "ip",
            "port",
            "mac",
            "ssid",
            "resetFlag",
        ] {
            if let Some(value) = xml_tag_value(text, key).filter(|value| !value.is_empty()) {
                fields.push((key, value));
            }
        }
    }
    fields
}

fn xml_tag_value(data: &str, key: &str) -> Option<String> {
    let start_tag = format!("<{}>", key);
    let end_tag = format!("</{}>", key);
    let start = data.find(&start_tag)? + start_tag.len();
    let tail = &data[start..];
    let end = tail.find(&end_tag)?;
    Some(tail[..end].trim().to_string())
}

fn normalize_mac(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn fuzzy_mac_match(target: &str, candidate: &str) -> bool {
    let target = normalize_mac(target);
    let candidate = normalize_mac(candidate);
    if target.is_empty() || candidate.is_empty() {
        return false;
    }
    if target == candidate || target.ends_with(&candidate) || candidate.ends_with(&target) {
        return true;
    }
    target
        .chars()
        .zip(candidate.chars())
        .filter(|(left, right)| left == right)
        .count()
        >= 5
}

fn print_bluetoothctl_info(address: &str) {
    let output = ProcessCommand::new("bluetoothctl")
        .arg("info")
        .arg(address)
        .output();
    match output {
        Ok(output) => {
            print!("{}", String::from_utf8_lossy(&output.stdout));
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
        Err(err) => {
            eprintln!("bluetoothctl info failed: {}", err);
        }
    }
}

fn expand_home(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

fn do_power(on: bool, off: bool) {
    if on == off {
        eprintln!("Specify either --on or --off, not both");
        process::exit(1);
    }

    with_client(|client| client.set_power(on))
}

fn do_temp(temperature: i8) {
    if !(16..=31).contains(&temperature) {
        eprintln!("Temperature must be between 16 and 31");
        process::exit(1);
    }

    with_client(|client| client.set_temperature(temperature))
}

fn do_mode(mode_str: &str) {
    let mode = parse_mode(mode_str);
    with_client(|client| client.set_mode(mode))
}

fn do_fan(speed_str: &str) {
    let speed = parse_fan_speed(speed_str);
    with_client(|client| client.set_fan_speed(speed))
}

fn do_swing(on: bool, off: bool) {
    if on == off {
        eprintln!("Specify either --on or --off, not both");
        process::exit(1);
    }

    with_client(|client| client.set_wind_swing(on))
}

fn do_eco(on: bool, off: bool) {
    if on == off {
        eprintln!("Specify either --on or --off, not both");
        process::exit(1);
    }

    with_client(|client| client.set_eco(on))
}

fn do_full(power: Option<bool>, temp: Option<i8>, mode: Option<&str>, fan: Option<&str>) {
    let mut state = AcState::default();

    if let Some(p) = power {
        state.power = p;
    }
    if let Some(t) = temp {
        if !(16..=31).contains(&t) {
            eprintln!("Temperature must be between 16 and 31");
            process::exit(1);
        }
        state.temperature = t;
    }
    if let Some(m) = mode {
        state.mode = parse_mode(m);
    }
    if let Some(f) = fan {
        state.fan_speed = parse_fan_speed(f);
    }

    with_client(|client| {
        client.full_control(&state)?;
        print_state(&state);
        Ok(())
    })
}

fn do_disconnect() {
    if let Some(saved) = load_saved_state() {
        let client = TclAcClient::new();
        let _ = client.connect(&saved.address);
        let _ = client.disconnect();
        let _ = fs::remove_file(state_file());
        println!("Disconnected");
    } else {
        println!("Not connected");
    }
    process::exit(0);
}

fn parse_mode(s: &str) -> AcMode {
    match s.to_lowercase().as_str() {
        "cool" | "cooling" => AcMode::Cool,
        "heat" | "heating" | "warm" => AcMode::Heat,
        "auto" | "automatic" => AcMode::Auto,
        "dry" | "dehumidify" => AcMode::Dry,
        "fan" | "fan-only" => AcMode::Fan,
        "eco" | "economy" => AcMode::Eco,
        _ => {
            eprintln!("Unknown mode: {}. Use: cool|heat|auto|dry|fan|eco", s);
            process::exit(1);
        }
    }
}

fn parse_fan_speed(s: &str) -> FanSpeed {
    match s.to_lowercase().as_str() {
        "auto" | "automatic" => FanSpeed::Auto,
        "low" | "min" => FanSpeed::Low,
        "medium" | "med" | "mid" => FanSpeed::Medium,
        "high" | "max" | "max-speed" => FanSpeed::High,
        "turbo" | "max+" | "strong" => FanSpeed::Turbo,
        "quiet" | "silent" => FanSpeed::Quiet,
        _ => {
            eprintln!(
                "Unknown fan speed: {}. Use: auto|low|medium|high|turbo|quiet",
                s
            );
            process::exit(1);
        }
    }
}

fn print_state(state: &AcState) {
    println!("┌─────────────────────────────────────┐");
    println!("│         TCL AC Status               │");
    println!("├─────────────────────────────────────┤");
    println!(
        "│ Power:    {:<25} │",
        if state.power { "ON" } else { "OFF" }
    );
    println!(
        "│ Temp:     {}°C                        │",
        state.temperature
    );
    println!("│ Mode:     {:<25} │", format!("{:?}", state.mode));
    println!("│ Fan:      {:<25} │", format!("{:?}", state.fan_speed));
    println!(
        "│ Swing:    {:<25} │",
        format!("{:?}", state.wind_direction)
    );
    println!("│ Eco:      {:<25} │", state.eco_mode);
    println!("│ Turbo:    {:<25} │", state.turbo_mode);
    println!("│ Quiet:    {:<25} │", state.quiet_mode);
    println!("└─────────────────────────────────────┘");
}
