//! Quectel EC200A AT command helpers.
//!
//! This module owns command strings and response parsing only. Serial ports,
//! timeouts, retries, and device discovery belong to the runtime.

use crate::prelude::*;
use alloc::format;

pub const CMD_NETWORK_REGISTRATION: &str = "AT+CREG?";
pub const CMD_SIGNAL_QUALITY: &str = "AT+CSQ";
pub const CMD_OPERATOR: &str = "AT+COPS?";
pub const CMD_MODEM_INFO: &str = "ATI";
pub const CMD_IMSI: &str = "AT+CIMI";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ec200aParseError {
    MissingField,
    BadNumber,
}

pub fn configure_dta_commands(apn: &str) -> Vec<String> {
    let mut commands = Vec::new();
    commands.push(format!("AT+CGDCONT=1,\"IP\",\"{}\"", apn));
    commands.push("AT+CNMP=0".to_string());
    commands.push("AT+CNSMOD=0".to_string());
    commands.push("AT+CEREG=1".to_string());
    commands
}

pub fn set_radio_function_command(mode: u8) -> String {
    format!("AT+CFUN={mode}")
}

pub fn parse_network_registered(response: &str) -> Result<bool, Ec200aParseError> {
    let line = response
        .lines()
        .find(|line| line.contains("+CREG:"))
        .ok_or(Ec200aParseError::MissingField)?;
    let (_, fields) = line.split_once(':').ok_or(Ec200aParseError::MissingField)?;
    let status = fields
        .split(',')
        .nth(1)
        .ok_or(Ec200aParseError::MissingField)?
        .trim();
    Ok(status == "1" || status == "5")
}

pub fn parse_signal_quality(response: &str) -> Result<(u8, u8), Ec200aParseError> {
    let line = response
        .lines()
        .find(|line| line.contains("+CSQ:"))
        .ok_or(Ec200aParseError::MissingField)?;
    let (_, fields) = line.split_once(':').ok_or(Ec200aParseError::MissingField)?;
    let mut parts = fields.split(',');
    let rssi = parts
        .next()
        .ok_or(Ec200aParseError::MissingField)?
        .trim()
        .parse::<u8>()
        .map_err(|_| Ec200aParseError::BadNumber)?;
    let ber = parts
        .next()
        .ok_or(Ec200aParseError::MissingField)?
        .trim()
        .parse::<u8>()
        .map_err(|_| Ec200aParseError::BadNumber)?;
    Ok((rssi, ber))
}

pub fn parse_network_operator(response: &str) -> Result<String, Ec200aParseError> {
    let line = response
        .lines()
        .find(|line| line.contains("+COPS:"))
        .ok_or(Ec200aParseError::MissingField)?;
    let (_, fields) = line.split_once(':').ok_or(Ec200aParseError::MissingField)?;
    let operator = fields
        .split(',')
        .nth(2)
        .ok_or(Ec200aParseError::MissingField)?
        .trim()
        .trim_matches('"');
    Ok(operator.to_string())
}

pub fn parse_modem_info(response: &str) -> String {
    response
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && *line != "OK")
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn parse_imsi(response: &str) -> Result<String, Ec200aParseError> {
    response
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && *line != "OK" && line.chars().all(|c| c.is_ascii_digit()))
        .map(str::to_string)
        .ok_or(Ec200aParseError::MissingField)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn builds_configure_commands() {
        assert_eq!(
            configure_dta_commands("internet"),
            vec![
                "AT+CGDCONT=1,\"IP\",\"internet\"".to_string(),
                "AT+CNMP=0".to_string(),
                "AT+CNSMOD=0".to_string(),
                "AT+CEREG=1".to_string(),
            ]
        );
    }

    #[test]
    fn parses_registration() {
        assert_eq!(parse_network_registered("\r\n+CREG: 0,5\r\nOK"), Ok(true));
        assert_eq!(parse_network_registered("\r\n+CREG: 0,0\r\nOK"), Ok(false));
    }

    #[test]
    fn parses_signal_quality() {
        assert_eq!(parse_signal_quality("\r\n+CSQ: 18,99\r\nOK"), Ok((18, 99)));
    }

    #[test]
    fn parses_operator() {
        assert_eq!(
            parse_network_operator("\r\n+COPS: 0,0,\"DTA_NET\",2\r\nOK"),
            Ok("DTA_NET".to_string())
        );
    }

    #[test]
    fn parses_identity_fields() {
        assert_eq!(
            parse_modem_info("\r\nQuectel EC200A-EU Rev1.0\r\nOK"),
            "Quectel EC200A-EU Rev1.0"
        );
        assert_eq!(
            parse_imsi("\r\n123456789012345\r\nOK"),
            Ok("123456789012345".to_string())
        );
    }
}
