use edgerun_bluetooth_gatt::{format_gatt_uuid, AttProtocol, L2capSocket};
use edgerun_mgmt_bluetooth::{MgmtBluetoothBackend, MgmtDiscoveryTransport};
use edgerun_tcl_ac::{AcMode, AcState, FanSpeed, TclAcClient};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{self, Command as ProcessCommand};

const STATE_FILE: &str = "/tmp/tcl-ac-state.json";

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
    do_provision(ssid, password, bind_code);
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
