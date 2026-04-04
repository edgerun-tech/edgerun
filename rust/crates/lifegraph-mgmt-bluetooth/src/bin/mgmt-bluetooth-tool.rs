use lifegraph_mgmt_bluetooth::{
    MgmtBluetoothBackend, MgmtControllerEvent, MgmtDiscoveryTransport, discover_controllers,
    read_controller_indices, read_management_version, set_controller_connectable,
    set_controller_discoverable, set_controller_local_name, set_controller_pairable,
    set_controller_powered,
};

fn parse_bool_arg(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn usage() {
    eprintln!(
        "usage: mgmt-bluetooth-tool version | indices | list | power <index> <on|off|1|0|true|false|yes|no> | connectable <index> <on|off|1|0|true|false|yes|no> | discoverable <index> <on|off|1|0|true|false|yes|no> [timeout_secs] | pairable <index> <on|off|1|0|true|false|yes|no> | local-name <index> <name> [short_name] | connections <index> | known <index> | monitor <index> [timeout_ms] | discover <index> [classic|le|all] [timeout_ms]"
    );
}

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("version") => match read_management_version() {
            Ok(v) => println!("mgmt version={} revision={}", v.version, v.revision),
            Err(err) => {
                eprintln!("version error: {err}");
                std::process::exit(1);
            }
        },
        Some("indices") => match read_controller_indices() {
            Ok(indices) => {
                for index in indices {
                    println!("index={index}");
                }
            }
            Err(err) => {
                eprintln!("indices error: {err}");
                std::process::exit(1);
            }
        },
        Some("list") => match discover_controllers() {
            Ok(controllers) => {
                for c in controllers {
                    println!(
                        "index={} addr={} ver={} mfg={} powered={} le={} bredr={} discoverable={} class=0x{:06x} name={:?} short={:?}",
                        c.index,
                        c.address,
                        c.bluetooth_version,
                        c.manufacturer,
                        c.current_settings.powered,
                        c.current_settings.low_energy,
                        c.current_settings.bredr,
                        c.current_settings.discoverable,
                        c.class_of_device,
                        c.name,
                        c.short_name,
                    );
                }
            }
            Err(err) => {
                eprintln!("list error: {err}");
                std::process::exit(1);
            }
        },
        Some("power") => {
            let Some(index) = args.next().and_then(|s| s.parse::<u16>().ok()) else {
                eprintln!("missing index");
                std::process::exit(2);
            };
            let Some(mode) = args.next() else {
                eprintln!("missing on/off");
                std::process::exit(2);
            };
            let Some(powered) = parse_bool_arg(&mode) else {
                eprintln!("expected on|off|1|0|true|false|yes|no");
                std::process::exit(2);
            };
            match set_controller_powered(index, powered) {
                Ok(info) => println!(
                    "index={} powered={} current_settings=0x{:08x}",
                    info.index, info.current_settings.powered, info.current_settings_raw
                ),
                Err(err) => {
                    eprintln!("power error: {err}");
                    std::process::exit(1);
                }
            }
        }
        Some("connectable") => {
            let Some(index) = args.next().and_then(|s| s.parse::<u16>().ok()) else {
                eprintln!("missing index");
                std::process::exit(2);
            };
            let Some(mode) = args.next() else {
                eprintln!("missing on/off");
                std::process::exit(2);
            };
            let Some(enabled) = parse_bool_arg(&mode) else {
                eprintln!("expected on|off|1|0|true|false|yes|no");
                std::process::exit(2);
            };
            match set_controller_connectable(index, enabled) {
                Ok(info) => println!(
                    "index={} connectable={} current_settings=0x{:08x}",
                    info.index, info.current_settings.connectable, info.current_settings_raw
                ),
                Err(err) => {
                    eprintln!("connectable error: {err}");
                    std::process::exit(1);
                }
            }
        }
        Some("discoverable") => {
            let Some(index) = args.next().and_then(|s| s.parse::<u16>().ok()) else {
                eprintln!("missing index");
                std::process::exit(2);
            };
            let Some(mode) = args.next() else {
                eprintln!("missing on/off");
                std::process::exit(2);
            };
            let Some(enabled) = parse_bool_arg(&mode) else {
                eprintln!("expected on|off|1|0|true|false|yes|no");
                std::process::exit(2);
            };
            let timeout_secs = args
                .next()
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(60);
            match set_controller_discoverable(
                index,
                enabled,
                if enabled { timeout_secs } else { 0 },
            ) {
                Ok(info) => println!(
                    "index={} discoverable={} current_settings=0x{:08x}",
                    info.index, info.current_settings.discoverable, info.current_settings_raw
                ),
                Err(err) => {
                    eprintln!("discoverable error: {err}");
                    std::process::exit(1);
                }
            }
        }
        Some("pairable") => {
            let Some(index) = args.next().and_then(|s| s.parse::<u16>().ok()) else {
                eprintln!("missing index");
                std::process::exit(2);
            };
            let Some(mode) = args.next() else {
                eprintln!("missing on/off");
                std::process::exit(2);
            };
            let Some(enabled) = parse_bool_arg(&mode) else {
                eprintln!("expected on|off|1|0|true|false|yes|no");
                std::process::exit(2);
            };
            match set_controller_pairable(index, enabled) {
                Ok(info) => println!(
                    "index={} pairable={} current_settings=0x{:08x}",
                    info.index, info.current_settings.pairable, info.current_settings_raw
                ),
                Err(err) => {
                    eprintln!("pairable error: {err}");
                    std::process::exit(1);
                }
            }
        }
        Some("local-name") => {
            let Some(index) = args.next().and_then(|s| s.parse::<u16>().ok()) else {
                eprintln!("missing index");
                std::process::exit(2);
            };
            let Some(name) = args.next() else {
                eprintln!("missing name");
                std::process::exit(2);
            };
            let short_name = args.next().unwrap_or_default();
            match set_controller_local_name(index, &name, &short_name) {
                Ok(info) => println!(
                    "index={} name={:?} short={:?}",
                    info.index, info.name, info.short_name
                ),
                Err(err) => {
                    eprintln!("local-name error: {err}");
                    std::process::exit(1);
                }
            }
        }
        Some("known") => {
            let Some(index) = args.next().and_then(|s| s.parse::<u16>().ok()) else {
                eprintln!("missing index");
                std::process::exit(2);
            };
            let controller = discover_controllers()
                .ok()
                .and_then(|controllers| controllers.into_iter().find(|c| c.index == index));
            let Some(controller) = controller else {
                eprintln!("controller {index} not found");
                std::process::exit(1);
            };
            let backend = MgmtBluetoothBackend { controller };
            match backend.known_devices() {
                Ok(devices) => {
                    for dev in devices {
                        println!(
                            "device={} kind={:?} transport={:?} name={:?} trusted={:?} paired={:?} services={:?} profiles={:?}",
                            dev.device_address,
                            dev.address_kind,
                            dev.transport_kind,
                            dev.local_name,
                            dev.trusted,
                            dev.paired,
                            dev.service_uuids,
                            dev.profiles
                        );
                    }
                }
                Err(err) => {
                    eprintln!("known error: {err}");
                    std::process::exit(1);
                }
            }
        }
        Some("monitor") => {
            let Some(index) = args.next().and_then(|s| s.parse::<u16>().ok()) else {
                eprintln!("missing index");
                std::process::exit(2);
            };
            let timeout_ms = args
                .next()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(1_500);
            let controller = discover_controllers()
                .ok()
                .and_then(|controllers| controllers.into_iter().find(|c| c.index == index));
            let Some(controller) = controller else {
                eprintln!("controller {index} not found");
                std::process::exit(1);
            };
            let backend = MgmtBluetoothBackend { controller };
            match backend.monitor_events(timeout_ms) {
                Ok(events) => {
                    for event in events {
                        match event {
                            MgmtControllerEvent::SettingsChanged {
                                index,
                                settings_raw,
                                settings,
                            } => println!(
                                "settings index={} raw=0x{:08x} powered={} le={} bredr={} discoverable={}",
                                index,
                                settings_raw,
                                settings.powered,
                                settings.low_energy,
                                settings.bredr,
                                settings.discoverable
                            ),
                            MgmtControllerEvent::DeviceConnected {
                                index,
                                device_id,
                                transport_kind,
                                address_kind,
                                flags,
                                local_name,
                                profiles,
                                ..
                            } => println!(
                                "connected index={} device={} transport={:?} kind={:?} flags=0x{:08x} name={:?} profiles={:?}",
                                index,
                                device_id,
                                transport_kind,
                                address_kind,
                                flags,
                                local_name,
                                profiles
                            ),
                            MgmtControllerEvent::DeviceDisconnected {
                                index,
                                device_id,
                                transport_kind,
                                address_kind,
                                reason,
                            } => println!(
                                "disconnected index={} device={} transport={:?} kind={:?} reason=0x{:02x}",
                                index, device_id, transport_kind, address_kind, reason
                            ),
                            MgmtControllerEvent::ConnectFailed {
                                index,
                                device_id,
                                transport_kind,
                                address_kind,
                                status,
                            } => println!(
                                "connect-failed index={} device={} transport={:?} kind={:?} status=0x{:02x}",
                                index, device_id, transport_kind, address_kind, status
                            ),
                            MgmtControllerEvent::DiscoveringChanged {
                                index,
                                address_mask,
                                discovering,
                            } => println!(
                                "discovering index={} mask=0x{:02x} discovering={}",
                                index, address_mask, discovering
                            ),
                            MgmtControllerEvent::ClassOfDeviceChanged {
                                index,
                                class_of_device,
                            } => println!(
                                "class-of-device index={} class=0x{:06x}",
                                index, class_of_device
                            ),
                            MgmtControllerEvent::LocalNameChanged {
                                index,
                                name,
                                short_name,
                            } => println!(
                                "local-name index={} name={:?} short={:?}",
                                index, name, short_name
                            ),
                        }
                    }
                }
                Err(err) => {
                    eprintln!("monitor error: {err}");
                    std::process::exit(1);
                }
            }
        }
        Some("connections") => {
            let Some(index) = args.next().and_then(|s| s.parse::<u16>().ok()) else {
                eprintln!("missing index");
                std::process::exit(2);
            };
            let controller = discover_controllers()
                .ok()
                .and_then(|controllers| controllers.into_iter().find(|c| c.index == index));
            let Some(controller) = controller else {
                eprintln!("controller {index} not found");
                std::process::exit(1);
            };
            let backend = MgmtBluetoothBackend { controller };
            match backend.list_connections() {
                Ok(connections) => {
                    for conn in connections {
                        println!(
                            "device={} transport={:?} kind={:?} link={:?} outbound={} state={} name={:?} trusted={:?} paired={:?} profiles={:?}",
                            conn.device_id,
                            conn.transport_kind,
                            conn.address_kind,
                            conn.link_kind,
                            conn.outbound,
                            conn.state,
                            conn.local_name,
                            conn.trusted,
                            conn.paired,
                            conn.profiles
                        );
                    }
                }
                Err(err) => {
                    eprintln!("connections error: {err}");
                    std::process::exit(1);
                }
            }
        }
        Some("discover") => {
            let Some(index) = args.next().and_then(|s| s.parse::<u16>().ok()) else {
                eprintln!("missing index");
                std::process::exit(2);
            };
            let transport = match args.next().as_deref() {
                Some("classic") => MgmtDiscoveryTransport::Classic,
                Some("le") => MgmtDiscoveryTransport::LowEnergy,
                Some("all") | None => MgmtDiscoveryTransport::Interleaved,
                Some(_) => {
                    eprintln!("expected classic|le|all");
                    std::process::exit(2);
                }
            };
            let timeout_ms = args
                .next()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(1_500);
            let controller = discover_controllers()
                .ok()
                .and_then(|controllers| controllers.into_iter().find(|c| c.index == index));
            let Some(controller) = controller else {
                eprintln!("controller {index} not found");
                std::process::exit(1);
            };
            let backend = MgmtBluetoothBackend { controller };
            match backend.discover_nearby(transport, timeout_ms) {
                Ok(result) => {
                    for obs in result.observations {
                        println!(
                            "device={} transport={:?} kind={:?} name={:?} services={:?} profiles={:?} rssi={} adv_len={} at={}",
                            obs.device_id,
                            obs.transport_kind,
                            obs.address_kind,
                            obs.local_name,
                            obs.service_uuids,
                            obs.profiles,
                            obs.rssi_dbm,
                            obs.advertisement_data.len(),
                            obs.captured_at_unix_ms
                        );
                    }
                }
                Err(err) => {
                    eprintln!("discover error: {err}");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            usage();
            std::process::exit(2);
        }
    }
}
