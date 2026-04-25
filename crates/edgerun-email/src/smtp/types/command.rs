use std::io;

use crate::smtp::types::dsn::{DsnNotify, DsnRet};

/// Parsed SMTP command from a client.
#[derive(Debug, Clone)]
pub enum SmtpCommand {
    Ehlo(String),
    Helo(String),
    MailFrom {
        address: String,
        /// ESMTP parameters from MAIL FROM (SIZE, RET, ENVID, …).
        parameters: Vec<(String, Option<String>)>,
    },
    RcptTo {
        address: String,
        /// ESMTP parameters from RCPT TO (NOTIFY, ORCPT, …).
        parameters: Vec<(String, Option<String>)>,
    },
    Data,
    Rset,
    Noop,
    Quit,
    Vrfy(String),
    Expn(String),
    Help(Option<String>),
    Starttls,
    Auth {
        mechanism: String,
        initial_response: Option<String>,
    },
    /// Base64-encoded response during multi-step SASL exchange.
    AuthResponse(String),
    /// BDAT chunked data transfer (RFC 3030).
    /// `size` is the exact number of data bytes in this chunk.
    /// `last` marks this as the final chunk (equivalent to DATA's `.`).
    Bdat {
        size: usize,
        last: bool,
    },
    /// Role reversal (RFC 5321 §3.3.6).
    /// Server becomes client, client becomes server.
    Turn,
    /// Remote mail queue processing (RFC 2476).
    /// Tells the server to start processing its mail queue for the given domain.
    Etrn(String),
}

impl SmtpCommand {
    /// Parse an SMTP command from a raw line.
    pub fn parse(line: &str) -> io::Result<Self> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "empty command"));
        }

        let parts: Vec<&str> = trimmed.splitn(2, |c: char| c.is_whitespace()).collect();
        let cmd = parts[0].to_uppercase();
        let args = if parts.len() > 1 { parts[1].trim() } else { "" };

        match cmd.as_str() {
            "EHLO" => {
                if args.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "EHLO requires domain",
                    ));
                }
                Ok(Self::Ehlo(args.to_string()))
            }
            "HELO" => {
                if args.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "HELO requires domain",
                    ));
                }
                Ok(Self::Helo(args.to_string()))
            }
            "MAIL" => {
                let from_pos = args.to_uppercase().find("FROM:");
                if from_pos.is_none() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "MAIL requires FROM",
                    ));
                }
                let from_start = from_pos.unwrap() + 5;
                let rest = &args[from_start..];

                let (address, params_str) = if rest.starts_with('<') {
                    if let Some(end) = rest.find('>') {
                        (rest[1..end].to_string(), rest[end + 1..].trim())
                    } else {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "MAIL FROM: missing closing >",
                        ));
                    }
                } else {
                    let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
                    (rest[..end].to_string(), rest[end..].trim())
                };

                let parameters = parse_esmtp_parameters(params_str);
                Ok(Self::MailFrom {
                    address,
                    parameters,
                })
            }
            "RCPT" => {
                let to_pos = args.to_uppercase().find("TO:");
                if to_pos.is_none() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "RCPT requires TO",
                    ));
                }
                let to_start = to_pos.unwrap() + 3;
                let rest = &args[to_start..];

                let (address, params_str) = if rest.starts_with('<') {
                    if let Some(end) = rest.find('>') {
                        (rest[1..end].to_string(), rest[end + 1..].trim())
                    } else {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "RCPT TO: missing closing >",
                        ));
                    }
                } else {
                    let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
                    (rest[..end].to_string(), rest[end..].trim())
                };

                let parameters = parse_esmtp_parameters(params_str);
                Ok(Self::RcptTo {
                    address,
                    parameters,
                })
            }
            "DATA" => Ok(Self::Data),
            "BDAT" => {
                // BDAT <size> [LAST]
                let parts: Vec<&str> = args.splitn(2, |c: char| c.is_whitespace()).collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "BDAT requires a size",
                    ));
                }
                let size: usize = parts[0].parse().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "BDAT size must be a number")
                })?;
                let last = if parts.len() > 1 {
                    parts[1].trim().eq_ignore_ascii_case("LAST")
                } else {
                    false
                };
                Ok(Self::Bdat { size, last })
            }
            "RSET" => Ok(Self::Rset),
            "NOOP" => Ok(Self::Noop),
            "QUIT" => Ok(Self::Quit),
            "VRFY" => Ok(Self::Vrfy(args.to_string())),
            "EXPN" => Ok(Self::Expn(args.to_string())),
            "HELP" => {
                if args.is_empty() {
                    Ok(Self::Help(None))
                } else {
                    Ok(Self::Help(Some(args.to_string())))
                }
            }
            "STARTTLS" => Ok(Self::Starttls),
            "TURN" => Ok(Self::Turn),
            "ETRN" => {
                if args.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "ETRN requires a domain",
                    ));
                }
                Ok(Self::Etrn(args.to_string()))
            }
            "AUTH" => {
                let auth_parts: Vec<&str> = args.splitn(2, |c: char| c.is_whitespace()).collect();
                if auth_parts.is_empty() || auth_parts[0].is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "AUTH requires mechanism",
                    ));
                }
                let mechanism = auth_parts[0].to_string();
                let initial_response = if auth_parts.len() > 1 {
                    Some(auth_parts[1].to_string())
                } else {
                    None
                };
                Ok(Self::Auth {
                    mechanism,
                    initial_response,
                })
            }
            // Any unrecognized command — return a syntax error, NOT an AuthResponse.
            // Auth responses during AUTH exchanges are handled by the state machine
            // in session.rs (handle_auth_response), NOT by command parsing.
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unknown command: {}", trimmed),
            )),
        }
    }
}

/// Parse ESMTP parameters (e.g., `SIZE=12345 RET=FULL ENVID=abc`).
pub fn parse_esmtp_parameters(s: &str) -> Vec<(String, Option<String>)> {
    if s.is_empty() {
        return Vec::new();
    }
    let mut params = Vec::new();
    for token in s.split_whitespace() {
        if let Some(eq_pos) = token.find('=') {
            let key = token[..eq_pos].to_string();
            let value = token[eq_pos + 1..].to_string();
            params.push((key, Some(value)));
        } else {
            params.push((token.to_string(), None));
        }
    }
    params
}

// ===========================================================================
// DSN helpers
// ===========================================================================

/// Extract DSN RET value from MAIL FROM parameters.
pub fn extract_dsn_ret(params: &[(String, Option<String>)]) -> Option<DsnRet> {
    params
        .iter()
        .find(|(k, _)| k == "RET")
        .and_then(|(_, v)| v.as_ref())
        .and_then(|v| DsnRet::parse(v))
}

/// Extract DSN ENVID from MAIL FROM parameters.
pub fn extract_dsn_envid(params: &[(String, Option<String>)]) -> Option<String> {
    params
        .iter()
        .find(|(k, _)| k == "ENVID")
        .and_then(|(_, v)| v.clone())
}

/// Extract DSN NOTIFY from RCPT TO parameters.
pub fn extract_dsn_notify(params: &[(String, Option<String>)]) -> Option<DsnNotify> {
    params
        .iter()
        .find(|(k, _)| k == "NOTIFY")
        .and_then(|(_, v)| v.as_ref())
        .map(|v| DsnNotify::parse(v))
}

/// Extract ORCPT (original recipient) from RCPT TO parameters.
pub fn extract_dsn_orcpt(params: &[(String, Option<String>)]) -> Option<String> {
    params
        .iter()
        .find(|(k, _)| k == "ORCPT")
        .and_then(|(_, v)| v.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ehlo() {
        let cmd = SmtpCommand::parse("EHLO mail.example.com").unwrap();
        assert!(matches!(cmd, SmtpCommand::Ehlo(ref d) if d == "mail.example.com"));
    }

    #[test]
    fn test_parse_mail_from_with_dsn() {
        let cmd =
            SmtpCommand::parse("MAIL FROM:<sender@example.com> SIZE=1024 RET=FULL ENVID=abc123")
                .unwrap();
        match cmd {
            SmtpCommand::MailFrom {
                address,
                parameters,
            } => {
                assert_eq!(address, "sender@example.com");
                assert_eq!(parameters.len(), 3);
            }
            _ => panic!("Expected MailFrom"),
        }
    }

    #[test]
    fn test_parse_rcpt_to_with_dsn() {
        let cmd = SmtpCommand::parse(
            "RCPT TO:<recipient@example.com> NOTIFY=SUCCESS,FAILURE ORCPT=rfc822;orig@example.com",
        )
        .unwrap();
        match cmd {
            SmtpCommand::RcptTo {
                address,
                parameters,
            } => {
                assert_eq!(address, "recipient@example.com");
                assert_eq!(parameters.len(), 2);
            }
            _ => panic!("Expected RcptTo"),
        }
    }

    #[test]
    fn test_parse_all_commands() {
        assert!(matches!(
            SmtpCommand::parse("DATA").unwrap(),
            SmtpCommand::Data
        ));
        assert!(matches!(
            SmtpCommand::parse("RSET").unwrap(),
            SmtpCommand::Rset
        ));
        assert!(matches!(
            SmtpCommand::parse("NOOP").unwrap(),
            SmtpCommand::Noop
        ));
        assert!(matches!(
            SmtpCommand::parse("QUIT").unwrap(),
            SmtpCommand::Quit
        ));
        assert!(matches!(
            SmtpCommand::parse("STARTTLS").unwrap(),
            SmtpCommand::Starttls
        ));
        assert!(matches!(
            SmtpCommand::parse("VRFY user").unwrap(),
            SmtpCommand::Vrfy(_)
        ));
        assert!(matches!(
            SmtpCommand::parse("EXPN list").unwrap(),
            SmtpCommand::Expn(_)
        ));
        assert!(matches!(
            SmtpCommand::parse("HELP").unwrap(),
            SmtpCommand::Help(None)
        ));
        assert!(matches!(
            SmtpCommand::parse("HELP EHLO").unwrap(),
            SmtpCommand::Help(Some(_))
        ));
    }

    #[test]
    fn test_parse_case_insensitive() {
        assert!(matches!(
            SmtpCommand::parse("ehlo localhost").unwrap(),
            SmtpCommand::Ehlo(_)
        ));
        assert!(matches!(
            SmtpCommand::parse("mail FROM:<test@test.com>").unwrap(),
            SmtpCommand::MailFrom { .. }
        ));
    }

    #[test]
    fn test_parse_empty_command() {
        assert!(SmtpCommand::parse("").is_err());
    }

    #[test]
    fn test_parse_unknown_command() {
        assert!(SmtpCommand::parse("FOOBAR").is_err());
    }

    #[test]
    fn test_parse_turn_etrn() {
        match SmtpCommand::parse("TURN").unwrap() {
            SmtpCommand::Turn => {}
            other => panic!("expected Turn, got {:?}", other),
        }

        match SmtpCommand::parse("ETRN example.com").unwrap() {
            SmtpCommand::Etrn(domain) => assert_eq!(domain, "example.com"),
            other => panic!("expected Etrn, got {:?}", other),
        }

        assert!(SmtpCommand::parse("ETRN").is_err());
    }

    #[test]
    fn test_esmtp_parameters() {
        let params = parse_esmtp_parameters("SIZE=1024 RET=FULL ENVID=abc");
        assert_eq!(params.len(), 3);
        assert_eq!(params[0], ("SIZE".to_string(), Some("1024".to_string())));
        assert_eq!(params[1], ("RET".to_string(), Some("FULL".to_string())));
        assert!(parse_esmtp_parameters("").is_empty());
    }

    #[test]
    fn test_dsn_extractors() {
        let mail_params = parse_esmtp_parameters("SIZE=1024 RET=FULL ENVID=abc");
        assert_eq!(extract_dsn_ret(&mail_params), Some(DsnRet::Full));
        assert_eq!(extract_dsn_envid(&mail_params), Some("abc".to_string()));

        let rcpt_params =
            parse_esmtp_parameters("NOTIFY=SUCCESS,FAILURE ORCPT=rfc822;orig@example.com");
        let notify = extract_dsn_notify(&rcpt_params).unwrap();
        assert!(notify.success);
        assert!(notify.failure);
        assert!(!notify.never);
        assert_eq!(
            extract_dsn_orcpt(&rcpt_params),
            Some("rfc822;orig@example.com".to_string())
        );
    }
}
