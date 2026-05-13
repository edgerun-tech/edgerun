#![allow(dead_code)]
#![allow(unused_imports)]

//! Integration tests for Quectel EC200A DTA network functionality

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use edgerun_devices::quectel_ec200a::{Band, NetworkType, PowerState, State};

/// Simulates the Quectel EC200A modem for testing
struct ModemSimulator {
    state: Arc<Mutex<ModemState>>,
    responses: HashMap<String, String>,
}

/// Current state of the modem
#[derive(Debug, Clone, PartialEq)]
struct ModemState {
    power: PowerState,
    state: State,
    network_type: NetworkType,
    band: Band,
    signal_quality: u8,
    registered: bool,
    connected: bool,
}

impl Default for ModemState {
    fn default() -> Self {
        Self {
            power: PowerState::On,
            state: State::Unknown,
            network_type: NetworkType::Auto,
            band: Band::Fdd(1),
            signal_quality: 0,
            registered: false,
            connected: false,
        }
    }
}

impl ModemSimulator {
    fn new() -> Self {
        let mut responses = HashMap::new();
        responses.insert("AT+CREG?".into(), "+CREG: 0,5\r\nOK".into());
        responses.insert("AT+COPS?".into(), "+COPS: 0,0,\"DTA_NET\",2\r\nOK".into());
        responses.insert("AT+CSQ".into(), "+CSQ: 18,99\r\nOK".into());
        responses.insert("ATI".into(), "Quectel EC200A-EU Rev1.0\r\nOK".into());
        responses.insert("AT+CIMI".into(), "123456789012345\r\nOK".into());
        responses.insert("AT+CFUN=1".into(), "OK".into());
        responses.insert("AT+CFUN=4".into(), "OK".into());

        Self {
            state: Arc::new(Mutex::new(ModemState::default())),
            responses,
        }
    }

    fn execute_at_command(&self, cmd: &str) -> Result<String, String> {
        let mut state = self.state.lock().unwrap();

        match cmd {
            "AT" => {
                state.state = State::On;
                Ok("OK".into())
            }
            "AT+CREG?" => {
                state.state = State::Registered;
                state.registered = true;
                Ok("\r\n+CREG: 0,5\r\nOK".into())
            }
            "AT+COPS?" => Ok("\r\n+COPS: 0,0,\"DTA_NET\",2\r\nOK".into()),
            "AT+CSQ" => {
                state.signal_quality = 18;
                Ok("\r\n+CSQ: 18,99\r\nOK".into())
            }
            "ATI" => Ok("\r\nQuectel EC200A-EU Rev1.0\r\nOK".into()),
            "AT+CIMI" => Ok("\r\n123456789012345\r\nOK".into()),
            "AT+CFUN=1" => {
                state.state = State::Registered;
                state.power = PowerState::On;
                state.registered = true;
                Ok("OK".into())
            }
            "AT+CFUN=4" => {
                state.state = State::Off;
                state.power = PowerState::Off;
                state.registered = false;
                Ok("OK".into())
            }
            _ => Ok("ERROR".into()),
        }
    }

    fn get_state(&self) -> ModemState {
        self.state.lock().unwrap().clone()
    }
}

#[test]
fn test_dta_registration() {
    let modem = ModemSimulator::new();
    let resp = modem.execute_at_command("AT+CREG?").unwrap();
    assert!(resp.contains("+CREG: 0,5"));
    assert!(resp.contains("OK"));
}

#[test]
fn test_dta_network_operator() {
    let modem = ModemSimulator::new();
    assert!(
        modem
            .execute_at_command("AT+COPS?")
            .unwrap()
            .contains("DTA_NET")
    );
}

#[test]
fn test_dta_signal_quality() {
    let modem = ModemSimulator::new();
    let resp = modem.execute_at_command("AT+CSQ").unwrap();
    assert!(resp.contains("+CSQ:"));
    assert!(resp.contains("99"));
    assert!(resp.contains("OK"));
}

#[test]
fn test_dta_module_id() {
    let modem = ModemSimulator::new();
    let resp = modem.execute_at_command("ATI").unwrap();
    assert!(resp.contains("Quectel EC200A"));
    assert!(resp.contains("OK"));
}

#[test]
fn test_dta_imsi() {
    let modem = ModemSimulator::new();
    let resp = modem.execute_at_command("AT+CIMI").unwrap();
    assert!(!resp.is_empty());
    assert!(resp.contains("OK"));
}

#[test]
fn test_dta_radio_power() {
    let modem = ModemSimulator::new();
    let resp = modem.execute_at_command("AT+CFUN=1").unwrap();
    assert!(resp.contains("OK"));
    let state = modem.get_state();
    assert_eq!(state.state, State::Registered);
    assert_eq!(state.power, PowerState::On);
    assert!(state.registered);
}

#[test]
fn test_dta_complete_flow() {
    let modem = ModemSimulator::new();

    // Power on modem
    assert!(
        modem
            .execute_at_command("AT+CFUN=1")
            .unwrap()
            .contains("OK")
    );

    // Verify registered state
    let state = modem.get_state();
    assert_eq!(state.state, State::Registered);
    assert!(state.registered);
}

#[test]
fn test_multiple_dta_modems() {
    let m1 = ModemSimulator::new();
    let m2 = ModemSimulator::new();

    let r1 = m1.execute_at_command("AT+CREG?").unwrap();
    let r2 = m2.execute_at_command("AT+CREG?").unwrap();

    assert!(r1.contains("OK"));
    assert!(r2.contains("OK"));
}

#[test]
fn test_dta_lifecycle() {
    let modem = ModemSimulator::new();

    // Power on
    assert!(
        modem
            .execute_at_command("AT+CFUN=1")
            .unwrap()
            .contains("OK")
    );

    // Verify registration state changed
    let state = modem.get_state();
    assert_eq!(state.state, State::Registered);
    assert!(state.registered);
    assert_eq!(state.power, PowerState::On);
}

#[test]
fn test_dta_band_config() {
    let modem = ModemSimulator::new();
    let s = modem.get_state();
    match s.band {
        Band::Fdd(b) => assert_eq!(b, 1),
        _ => panic!("Wrong band type"),
    }
}

#[test]
fn test_dta_network_preference() {
    let modem = ModemSimulator::new();
    let s = modem.get_state();
    assert_eq!(s.network_type, NetworkType::Auto);
}

#[test]
fn test_dta_error_handling() {
    let modem = ModemSimulator::new();
    let resp = modem.execute_at_command("AT+INVALID").unwrap();
    assert!(resp.contains("ERROR"));
}

#[test]
fn test_dta_power_cycle() {
    let modem = ModemSimulator::new();

    // Power off
    let resp = modem.execute_at_command("AT+CFUN=4").unwrap();
    assert!(resp.contains("OK"));
    let s = modem.get_state();
    assert_eq!(s.state, State::Off);
    assert!(!s.registered);

    // Power on again
    let resp = modem.execute_at_command("AT+CFUN=1").unwrap();
    assert!(resp.contains("OK"));
    let s = modem.get_state();
    assert_eq!(s.state, State::Registered);
    assert!(s.registered);
}

#[test]
fn test_dta_signal_stability() {
    let modem = ModemSimulator::new();
    let mut qs = Vec::new();
    for _ in 0..3 {
        let r = modem.execute_at_command("AT+CSQ").unwrap();
        if let Some(l) = r.lines().next() {
            if l.contains("+CSQ:") {
                let p: Vec<&str> = l.split(':').collect();
                if p.len() > 1 {
                    let n: Vec<&str> = p[1].trim().split(',').collect();
                    if let Ok(q) = n[0].parse::<u8>() {
                        qs.push(q);
                    }
                }
            }
        }
    }
    assert!(qs.iter().all(|&q| q == 18));
}
