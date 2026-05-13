//! DTA (Direct Terminal Access) network support for Quectel EC200A.
//!
//! This crate no longer owns serial ports or command execution. Runtime code
//! provides an AT transport; this module only sequences EC200A commands and
//! parses EC200A responses.

use super::DtaNetwork;
use super::prelude::v1::*;
use edgerun_protocols::quectel_ec200a::{
    CMD_IMSI, CMD_MODEM_INFO, CMD_NETWORK_REGISTRATION, CMD_OPERATOR, CMD_SIGNAL_QUALITY,
    configure_dta_commands, parse_imsi, parse_modem_info, parse_network_operator,
    parse_network_registered, parse_signal_quality, set_radio_function_command,
};

/// DTA network configuration.
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

pub trait AtTransport {
    fn send_at_command(&mut self, command: &str) -> Result<String, String>;
}

/// Configure DTA network on the modem through a runtime-provided transport.
pub fn configure_dta_with_transport<T: AtTransport>(
    transport: &mut T,
    config: Config,
) -> Result<String, String> {
    if let Some(pin) = config.pin {
        if pin.len() != 4 || !pin.chars().all(|c| c.is_ascii_digit()) {
            return Err("PIN must be exactly 4 digits".to_string());
        }
    }

    for command in configure_dta_commands(config.apn) {
        transport.send_at_command(&command)?;
    }

    Ok(format!("DTA network configured with APN: {}", config.apn))
}

pub fn check_dta_status_with_transport<T: AtTransport>(transport: &mut T) -> Result<bool, String> {
    let response = transport.send_at_command(CMD_NETWORK_REGISTRATION)?;
    parse_network_registered(&response)
        .map_err(|_| "Could not parse network registration status".to_string())
}

pub fn get_signal_quality_with_transport<T: AtTransport>(
    transport: &mut T,
) -> Result<(u8, u8), String> {
    let response = transport.send_at_command(CMD_SIGNAL_QUALITY)?;
    parse_signal_quality(&response).map_err(|_| "Could not parse signal quality".to_string())
}

pub fn get_network_operator_with_transport<T: AtTransport>(
    transport: &mut T,
) -> Result<String, String> {
    let response = transport.send_at_command(CMD_OPERATOR)?;
    parse_network_operator(&response).map_err(|_| "Could not parse network operator".to_string())
}

pub fn get_modem_info_with_transport<T: AtTransport>(transport: &mut T) -> Result<String, String> {
    let response = transport.send_at_command(CMD_MODEM_INFO)?;
    Ok(parse_modem_info(&response))
}

pub fn get_imsi_with_transport<T: AtTransport>(transport: &mut T) -> Result<String, String> {
    let response = transport.send_at_command(CMD_IMSI)?;
    parse_imsi(&response).map_err(|_| "Could not retrieve IMSI".to_string())
}

pub fn set_radio_function_with_transport<T: AtTransport>(
    transport: &mut T,
    mode: u8,
) -> Result<String, String> {
    let command = set_radio_function_command(mode);
    transport.send_at_command(&command)
}

/// Configure DTA network on the modem.
///
/// Direct host serial access is runtime-owned. Use
/// [`configure_dta_with_transport`] with a node-provided transport.
pub fn configure_dta(
    _modem: &super::Ec200a<impl super::ModelVariant>,
    _config: Config,
) -> Result<String, String> {
    Err(runtime_transport_required())
}

pub fn check_dta_status(_modem: &super::Ec200a<impl super::ModelVariant>) -> Result<bool, String> {
    Err(runtime_transport_required())
}

pub fn get_signal_quality() -> Result<(u8, u8), String> {
    Err(runtime_transport_required())
}

pub fn get_network_operator() -> Result<String, String> {
    Err(runtime_transport_required())
}

pub fn get_modem_info() -> Result<String, String> {
    Err(runtime_transport_required())
}

pub fn get_imsi() -> Result<String, String> {
    Err(runtime_transport_required())
}

pub fn set_radio_function(_mode: u8) -> Result<String, String> {
    Err(runtime_transport_required())
}

fn runtime_transport_required() -> String {
    "EC200A AT execution is runtime-owned; use the *_with_transport APIs".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeTransport {
        commands: Vec<String>,
    }

    impl FakeTransport {
        fn new() -> Self {
            Self {
                commands: Vec::new(),
            }
        }
    }

    impl AtTransport for FakeTransport {
        fn send_at_command(&mut self, command: &str) -> Result<String, String> {
            self.commands.push(command.to_string());
            match command {
                CMD_NETWORK_REGISTRATION => Ok("\r\n+CREG: 0,5\r\nOK".to_string()),
                CMD_SIGNAL_QUALITY => Ok("\r\n+CSQ: 18,99\r\nOK".to_string()),
                CMD_OPERATOR => Ok("\r\n+COPS: 0,0,\"DTA_NET\",2\r\nOK".to_string()),
                CMD_MODEM_INFO => Ok("\r\nQuectel EC200A-EU Rev1.0\r\nOK".to_string()),
                CMD_IMSI => Ok("\r\n123456789012345\r\nOK".to_string()),
                _ => Ok("OK".to_string()),
            }
        }
    }

    #[test]
    fn configure_dta_uses_protocol_command_sequence() {
        let mut transport = FakeTransport::new();
        configure_dta_with_transport(&mut transport, Config::default()).unwrap();
        assert_eq!(
            transport.commands,
            vec![
                "AT+CGDCONT=1,\"IP\",\"internet\"".to_string(),
                "AT+CNMP=0".to_string(),
                "AT+CNSMOD=0".to_string(),
                "AT+CEREG=1".to_string(),
            ]
        );
    }

    #[test]
    fn query_helpers_parse_responses() {
        let mut transport = FakeTransport::new();
        assert_eq!(check_dta_status_with_transport(&mut transport), Ok(true));
        assert_eq!(
            get_signal_quality_with_transport(&mut transport),
            Ok((18, 99))
        );
        assert_eq!(
            get_network_operator_with_transport(&mut transport),
            Ok("DTA_NET".to_string())
        );
        assert_eq!(
            get_modem_info_with_transport(&mut transport),
            Ok("Quectel EC200A-EU Rev1.0".to_string())
        );
        assert_eq!(
            get_imsi_with_transport(&mut transport),
            Ok("123456789012345".to_string())
        );
    }
}
