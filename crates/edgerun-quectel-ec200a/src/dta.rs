//! DTA (Direct Terminal Access) network support for Quectel EC200A

use crate::{DtaNetwork, Model};
use std::process::Command;
use std::time::Duration;

/// DTA network configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub enabled: DtaNetwork,
    pub apn: &'static str,
    pub pin: Option<&'static str>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: DtaNetwork::Disabled,
            apn: "internet",
            pin: None,
        }
    }
}

/// Configure DTA network on the modem
pub fn configure_dta(
    modem: &crate::Ec200a<impl crate::ModelVariant>,
    config: Config,
) -> Result<String, String> {
    // Validate configuration
    if let Some(pin) = config.pin {
        if pin.len() != 4 || !pin.chars().all(|c| c.is_ascii_digit()) {
            return Err("PIN must be exactly 4 digits".to_string());
        }
    }

    // Set APN for DTA network
    let apn_cmd = format!("AT+CGDCONT=1,\"IP\",\"{}\"", config.apn);
    send_at_command(&apn_cmd)?;

    // Set network mode to automatic (allows DTA network selection)
    send_at_command("AT+CNMP=0")?; // Auto mode
    send_at_command("AT+CNSMOD=0")?; // Auto network selection

    // Enable DTA network registration
    send_at_command("AT+CEREG=1")?; // Enable network registration unsolicited result code

    Ok(format!("DTA network configured with APN: {}", config.apn))
}

/// Check DTA network status
pub fn check_dta_status(modem: &crate::Ec200a<impl crate::ModelVariant>) -> Result<bool, String> {
    let response = send_at_command("AT+CREG?")?;

    // Parse +CREG response: +CREG: <mode>,<status>
    if let Some(creg_line) = response.lines().find(|line| line.contains("+CREG:")) {
        let parts: Vec<&str> = creg_line.split(',').collect();
        if parts.len() >= 2 {
            let status = parts[1].trim();
            // Status 1-5 indicates registration (1: not registered, 2: searching, 3: denied, 4: unknown, 5: registered)
            return Ok(status == "5" || status == "1");
        }
    }

    Err("Could not parse network registration status".to_string())
}

/// Get signal quality from modem
pub fn get_signal_quality() -> Result<(u8, u8), String> {
    let response = send_at_command("AT+CSQ")?;

    // Parse +CSQ response: +CSQ: <rssi>,<ber>
    if let Some(csq_line) = response.lines().find(|line| line.contains("+CSQ:")) {
        let parts: Vec<&str> = csq_line.split(',').collect();
        if parts.len() >= 2 {
            let rssi = parts[1].trim().parse().unwrap_or(99);
            let ber = if parts.len() >= 3 {
                parts[2].trim().parse().unwrap_or(99)
            } else {
                99
            };
            return Ok((rssi, ber));
        }
    }

    Err("Could not parse signal quality".to_string())
}

/// Get network operator information
pub fn get_network_operator() -> Result<String, String> {
    let response = send_at_command("AT+COPS?")?;

    // Parse +COPS response: +COPS: <mode>,<format>,<operator>,<stat>
    if let Some(cops_line) = response.lines().find(|line| line.contains("+COPS:")) {
        let parts: Vec<&str> = cops_line.split(',').collect();
        if parts.len() >= 3 {
            let op_name = parts[2].trim_matches('"');
            return Ok(op_name.to_string());
        }
    }

    Err("Could not parse network operator".to_string())
}

/// Get modem information
pub fn get_modem_info() -> Result<String, String> {
    let response = send_at_command("ATI")?;

    // Remove OK and newlines, return the modem info
    Ok(response
        .lines()
        .filter(|line| !line.contains("OK") && !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n"))
}

/// Get IMSI (International Mobile Subscriber Identity)
pub fn get_imsi() -> Result<String, String> {
    let response = send_at_command("AT+CIMI")?;

    // Extract IMSI from response (first non-empty, non-OK line)
    for line in response.lines() {
        let line = line.trim();
        if !line.is_empty() && line != "OK" && line.chars().all(|c| c.is_ascii_digit()) {
            return Ok(line.to_string());
        }
    }

    Err("Could not retrieve IMSI".to_string())
}

/// Set radio function (0=off, 1=full, 4=airplane)
pub fn set_radio_function(mode: u8) -> Result<String, String> {
    let cmd = format!("AT+CFUN={}", mode);
    send_at_command(&cmd)
}

/// Send AT command to modem via serial port
fn send_at_command(cmd: &str) -> Result<String, String> {
    let port = "/dev/ttyUSB1"; // Primary AT command port
    let baud = 115200;
    
    let python_script = format!(
        r#"import serial, time
try:
    ser = serial.Serial('{}', {}, timeout=3.0, xonxoff=False, rtscts=False, dsrdtr=False)
    ser.flushInput()
    ser.flushOutput()
    ser.write(b'{}\r\n')
    time.sleep(1.0)
    response = ""
    timeout = time.time() + 5.0  # 5 second timeout
    while time.time() < timeout:
        line = ser.readline().decode('utf-8', errors='replace').strip()
        if line:
            response += line + "\n"
            if line == "OK" or line == "ERROR":
                break
    ser.close()
    print(response.strip())
except Exception as e:
    print(f"ERROR: {{e}}")"#,
        port, baud, cmd
    );

    let output = Command::new("python3")
        .args(["-c", &python_script])
        .output()
        .map_err(|e| format!("Failed to execute Python script: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    
    if !output.status.success() || stdout.starts_with("ERROR:") {
        return Err(stdout);
    }

    Ok(stdout)
}

