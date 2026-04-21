use edgerun_clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::process;
use edgerun_tcl_ac::{TclAcClient, AcState, AcMode, FanSpeed};
use edgerun_mgmt_bluetooth::{MgmtBluetoothBackend, MgmtDiscoveryTransport};

const STATE_FILE: &str = "/tmp/tcl-ac-state.json";

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedState {
    address: String,
    service_handle: Option<u16>,
    char_handle: Option<u16>,
}

#[derive(Parser)]
#[command(name = "tcl-ac", about = "TCL Air Conditioner Control CLI")]
enum Command {
    Scan {
        #[arg(short, long, help = "Timeout in seconds")]
        timeout: Option<u8>,
    },
    Connect {
        #[arg(help = "Device MAC address (e.g. AA:BB:CC:DD:EE:FF)")]
        address: String,
    },
    Pair {
        #[arg(help = "Device MAC address (e.g. AA:BB:CC:DD:EE:FF)")]
        address: String,
    },
    Status,
    Power {
        #[arg(long = "on", help = "Turn on")]
        power_on: bool,
        #[arg(long = "off", help = "Turn off")]
        power_off: bool,
    },
    Temp {
        #[arg(value_name = "TEMP", help = "Temperature (16-31)")]
        temperature: i8,
    },
    Mode {
        #[arg(value_name = "MODE", help = "cool|heat|auto|dry|fan|eco")]
        mode: String,
    },
    Fan {
        #[arg(value_name = "SPEED", help = "auto|low|medium|high|turbo|quiet")]
        speed: String,
    },
    Swing {
        #[arg(long = "on", help = "Enable swing")]
        swing_on: bool,
        #[arg(long = "off", help = "Disable swing")]
        swing_off: bool,
    },
    Eco {
        #[arg(long = "on", help = "Enable eco mode")]
        eco_on: bool,
        #[arg(long = "off", help = "Disable eco mode")]
        eco_off: bool,
    },
    Full {
        #[arg(long, help = "Power state")]
        power: Option<bool>,
        #[arg(long, help = "Temperature (16-31)")]
        temp: Option<i8>,
        #[arg(long, help = "Mode: cool|heat|auto|dry|fan|eco")]
        mode: Option<String>,
        #[arg(long, help = "Fan speed: auto|low|medium|high|turbo|quiet")]
        fan: Option<String>,
    },
    Disconnect,
}

fn main() {
    let cmd = Command::parse();

    match cmd {
        Command::Scan { timeout } => {
            do_scan(timeout.unwrap_or(10) as u64)
        }
        Command::Connect { address } => {
            do_connect(&address)
        }
        Command::Pair { address } => {
            do_pair(&address)
        }
        Command::Status => {
            do_status()
        }
        Command::Power { power_on, power_off } => {
            do_power(power_on, power_off)
        }
        Command::Temp { temperature } => {
            do_temp(temperature)
        }
        Command::Mode { mode } => {
            do_mode(&mode)
        }
        Command::Fan { speed } => {
            do_fan(&speed)
        }
        Command::Swing { swing_on, swing_off } => {
            do_swing(swing_on, swing_off)
        }
        Command::Eco { eco_on, eco_off } => {
            do_eco(eco_on, eco_off)
        }
        Command::Full { power, temp, mode, fan } => {
            do_full(power, temp, mode.as_deref(), fan.as_deref())
        }
        Command::Disconnect => {
            do_disconnect()
        }
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

    let result = match backend.discover_nearby(MgmtDiscoveryTransport::LowEnergy, (timeout_secs * 1000) as u32) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Scan failed: {}", e);
            process::exit(1);
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
            println!("    Services: {:?}", obs.service_uuids.iter().take(3).collect::<Vec<_>>());
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
                        edgerun_mgmt_bluetooth::MgmtControllerEvent::DeviceConnected { device_id, .. } => {
                            if device_id.eq_ignore_ascii_case(address) {
                                println!("Pairing successful!");
                                return;
                            }
                        }
                        edgerun_mgmt_bluetooth::MgmtControllerEvent::ConnectFailed { device_id, status, .. } => {
                            if device_id.eq_ignore_ascii_case(address) {
                                eprintln!("Pairing failed with status: 0x{:02x}", status);
                                process::exit(1);
                            }
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
    F: FnOnce(&TclAcClient) -> Result<(), edgerun_capabilities::CapabilityError>
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
    with_client(|client| {
        match client.read_state() {
            Ok(state) => {
                print_state(&state);
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to read state: {}", e);
                Err(e)
            }
        }
    })
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
            eprintln!("Unknown fan speed: {}. Use: auto|low|medium|high|turbo|quiet", s);
            process::exit(1);
        }
    }
}

fn print_state(state: &AcState) {
    println!("┌─────────────────────────────────────┐");
    println!("│         TCL AC Status               │");
    println!("├─────────────────────────────────────┤");
    println!("│ Power:    {:<25} │", if state.power { "ON" } else { "OFF" });
    println!("│ Temp:     {}°C                        │", state.temperature);
    println!("│ Mode:     {:<25} │", format!("{:?}", state.mode));
    println!("│ Fan:      {:<25} │", format!("{:?}", state.fan_speed));
    println!("│ Swing:    {:<25} │", format!("{:?}", state.wind_direction));
    println!("│ Eco:      {:<25} │", state.eco_mode);
    println!("│ Turbo:    {:<25} │", state.turbo_mode);
    println!("│ Quiet:    {:<25} │", state.quiet_mode);
    println!("└─────────────────────────────────────┘");
}