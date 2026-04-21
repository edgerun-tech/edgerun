use edgerun_rt::Runtime;
use edgerun_tuya::{TuyaController, TuyaDevice, TuyaDeviceState};
use std::env;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();

    let args: Vec<String> = env::args().collect();

    let default_ip = "192.168.1.35";
    let default_key = "e0d652d4843a48a6be9eeb4fe6dd5b9c";

    if args.len() < 3 {
        eprintln!("Usage: {} [device-ip] [local-key] [on|off|temp <N>|mode <M>|fan <F>]", args[0]);
        eprintln!("Using defaults: {} {} [command]", default_ip, default_key);
        eprintln!("Examples:");
        eprintln!("  {} {} on", args[0], default_ip);
        eprintln!("  {} {} temp 24", args[0], default_ip);
        eprintln!("  {} {} mode cool", args[0], default_ip);
        eprintln!("  {} {} fan auto", args[0], default_ip);
        return;
    }

    let ip = if args[1] == "default" { default_ip.to_string() } else { args[1].clone() };
    let key = if args[2] == "default" { default_key.to_string() } else { args[2].clone() };
    let cmd = if args.len() > 3 { args[3].clone() } else { "on".to_string() };

    let device = TuyaDevice {
        id: "".to_string(),
        key: None,
        ip: ip.clone(),
        name: None,
        product_type: None,
        version: None,
        state: TuyaDeviceState::default(),
    };

    let mut controller = TuyaController::new(device, &key);

    let ip_for_output = ip.clone();
    rt.block_on(async move {
        let result = match cmd.as_str() {
            "on" => controller.set_power(true).await,
            "off" => controller.set_power(false).await,
            "temp" => {
                let temp: i32 = args.get(4)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(24);
                controller.set_temperature(temp).await
            }
            "mode" => {
                let mode = args.get(4).map(|s| s.as_str()).unwrap_or("cool");
                controller.set_mode(mode).await
            }
            "fan" => {
                let speed = args.get(4).map(|s| s.as_str()).unwrap_or("auto");
                controller.set_fan_speed(speed).await
            }
            _ => {
                eprintln!("Unknown command: {}", cmd);
                return;
            }
        };

        match result {
            Ok(()) => println!("OK"),
            Err(e) => eprintln!("Failed: {}", e),
        }
    });
}