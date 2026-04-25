//! BIND-style zone file parser (RFC 1035 §5).
//!
//! Parses standard zone file format as used by BIND, PowerDNS, etc.

use std::net::Ipv4Addr;
use std::str::FromStr;

use super::message::DnsRecord;
use super::record::{DnsRecordData, DnsRecordType};
use super::zone::DnsZone;

/// Parse a BIND-style zone file string and return a [`DnsZone`].
pub fn parse_zone_file(input: &str) -> Result<DnsZone, ZoneFileError> {
    let mut parser = ZoneFileParser::new();
    parser.parse(input)
}

#[derive(Debug, Clone)]
pub struct ZoneFileError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ZoneFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "zone file error at line {}: {}", self.line, self.message)
    }
}
impl std::error::Error for ZoneFileError {}

fn parse_err(line: usize, msg: &str) -> ZoneFileError {
    ZoneFileError {
        line,
        message: msg.to_string(),
    }
}

fn strip_comment_line(line: &str) -> &str {
    let mut in_quotes = false;
    for (i, ch) in line.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ';' if !in_quotes => return &line[..i],
            _ => {}
        }
    }
    line
}

struct ZoneFileParser {
    origin: String,
    default_ttl: u32,
    last_owner: String,
    records: Vec<DnsRecord>,
    line_num: usize,
}

impl ZoneFileParser {
    fn new() -> Self {
        Self {
            origin: String::new(),
            default_ttl: 3600,
            last_owner: String::new(),
            records: Vec::new(),
            line_num: 0,
        }
    }

    fn parse(&mut self, input: &str) -> Result<DnsZone, ZoneFileError> {
        let joined = self.join_continuations(input);
        for raw_line in joined.lines() {
            self.line_num += 1;
            let line = strip_comment_line(raw_line).trim().to_string();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('$') {
                self.parse_directive(&line)?;
                continue;
            }
            self.parse_record_line(&line)?;
        }
        let mut zone = DnsZone::new(&self.origin);
        zone.set_default_ttl(self.default_ttl);
        let records = std::mem::take(&mut self.records);
        for rr in records {
            zone.add_record(rr);
        }
        Ok(zone)
    }

    fn join_continuations(&self, input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let mut in_parens = false;
        for line in input.lines() {
            let trimmed = line.trim();
            if in_parens {
                result.push_str(trimmed);
                if trimmed.contains(')') {
                    in_parens = false;
                } else {
                    result.push(' ');
                }
            } else {
                if trimmed.contains('(') && !trimmed.contains(')') {
                    in_parens = true;
                }
                result.push_str(line);
                result.push('\n');
            }
        }
        result
    }

    fn parse_directive(&mut self, line: &str) -> Result<(), ZoneFileError> {
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        let directive = parts[0].to_uppercase();
        let value = parts.get(1).map(|s| s.trim()).unwrap_or("");
        match directive.as_str() {
            "$ORIGIN" => {
                self.origin = value.trim_end_matches('.').to_lowercase();
                if self.origin.is_empty() {
                    return Err(parse_err(self.line_num, "$ORIGIN value is empty"));
                }
            }
            "$TTL" => {
                self.default_ttl = value
                    .parse()
                    .map_err(|_| parse_err(self.line_num, &format!("invalid $TTL: {}", value)))?;
            }
            "$INCLUDE" => {
                edgerun_log::warn!(
                    "edgerun-dns: $INCLUDE not supported, line {}",
                    self.line_num
                );
            }
            _ => {
                return Err(parse_err(
                    self.line_num,
                    &format!("unknown directive: {}", parts[0]),
                ));
            }
        }
        Ok(())
    }

    fn parse_record_line(&mut self, line: &str) -> Result<(), ZoneFileError> {
        let tokens = tokenize(line);
        if tokens.is_empty() {
            return Ok(());
        }
        let mut pos = 0;

        let owner = if is_class_or_type(&tokens[0]) {
            self.last_owner.clone()
        } else {
            let owner = normalize_owner(&tokens[0], &self.origin);
            pos += 1;
            self.last_owner = owner.clone();
            owner
        };
        if pos >= tokens.len() {
            return Err(parse_err(self.line_num, "expected record type"));
        }

        let mut ttl = self.default_ttl;
        if let Ok(parsed_ttl) = tokens[pos].parse::<u32>() {
            ttl = parsed_ttl;
            pos += 1;
            if pos >= tokens.len() {
                return Err(parse_err(self.line_num, "expected record type"));
            }
        }
        let upper = tokens[pos].to_uppercase();
        if upper == "IN" || upper == "CH" || upper == "CS" || upper == "HS" {
            pos += 1;
            if pos >= tokens.len() {
                return Err(parse_err(self.line_num, "expected record type"));
            }
        }

        let rtype = rtype_from_str(&tokens[pos].to_uppercase()).ok_or_else(|| {
            parse_err(
                self.line_num,
                &format!("unknown record type: {}", tokens[pos]),
            )
        })?;
        pos += 1;
        let rdata: Vec<String> = tokens[pos..].to_vec();
        let rr = self.build_record(&owner, rtype, ttl, &rdata)?;
        self.records.push(rr);
        Ok(())
    }

    fn build_record(
        &self,
        owner: &str,
        rtype: DnsRecordType,
        ttl: u32,
        rdata: &[String],
    ) -> Result<DnsRecord, ZoneFileError> {
        let ln = self.line_num;
        let expect = |i: usize, msg: &str| -> Result<&String, ZoneFileError> {
            rdata.get(i).ok_or_else(|| parse_err(ln, msg))
        };
        let parse_u16 = |s: &String, field: &str| -> Result<u16, ZoneFileError> {
            s.parse()
                .map_err(|_| parse_err(ln, &format!("invalid {}: {}", field, s)))
        };
        let parse_u32 = |s: &String, field: &str| -> Result<u32, ZoneFileError> {
            s.parse()
                .map_err(|_| parse_err(ln, &format!("invalid {}: {}", field, s)))
        };
        match rtype {
            DnsRecordType::A => {
                let ip = Ipv4Addr::from_str(expect(0, "A: expected IPv4")?)
                    .map_err(|e| parse_err(ln, &e.to_string()))?;
                Ok(DnsRecord::a(owner.to_string(), ip, ttl))
            }
            DnsRecordType::AAAA => {
                let ip = std::net::Ipv6Addr::from_str(expect(0, "AAAA: expected IPv6")?)
                    .map_err(|e| parse_err(ln, &e.to_string()))?;
                Ok(DnsRecord::aaaa(owner.to_string(), ip, ttl))
            }
            DnsRecordType::CNAME => Ok(DnsRecord::cname(
                owner.to_string(),
                normalize_target(expect(0, "CNAME: expected target")?, &self.origin),
                ttl,
            )),
            DnsRecordType::NS => Ok(DnsRecord::ns(
                owner.to_string(),
                normalize_target(expect(0, "NS: expected nameserver")?, &self.origin),
                ttl,
            )),
            DnsRecordType::PTR => Ok(DnsRecord::ptr(
                owner.to_string(),
                normalize_target(expect(0, "PTR: expected target")?, &self.origin),
                ttl,
            )),
            DnsRecordType::MX => {
                let priority = parse_u16(expect(0, "MX: expected priority")?, "priority")?;
                Ok(DnsRecord::mx(
                    owner.to_string(),
                    priority,
                    normalize_target(expect(1, "MX: expected exchange")?, &self.origin),
                    ttl,
                ))
            }
            DnsRecordType::TXT => {
                let text = rdata
                    .iter()
                    .map(|s| s.replace('"', ""))
                    .collect::<Vec<_>>()
                    .join(" ");
                Ok(DnsRecord::txt(
                    owner.to_string(),
                    text.trim().to_string(),
                    ttl,
                ))
            }
            DnsRecordType::SRV => Ok(DnsRecord {
                name: owner.to_string(),
                rtype,
                rclass: 1,
                ttl,
                data: DnsRecordData::SRV {
                    priority: parse_u16(expect(0, "SRV: expected priority")?, "priority")?,
                    weight: parse_u16(expect(1, "SRV: expected weight")?, "weight")?,
                    port: parse_u16(expect(2, "SRV: expected port")?, "port")?,
                    target: normalize_target(expect(3, "SRV: expected target")?, &self.origin),
                },
            }),
            DnsRecordType::SOA => Ok(DnsRecord {
                name: owner.to_string(),
                rtype,
                rclass: 1,
                ttl,
                data: DnsRecordData::SOA {
                    mname: normalize_target(expect(0, "SOA: expected mname")?, &self.origin),
                    rname: normalize_target(expect(1, "SOA: expected rname")?, &self.origin),
                    serial: parse_u32(expect(2, "SOA: expected serial")?, "serial")?,
                    refresh: parse_u32(expect(3, "SOA: expected refresh")?, "refresh")?,
                    retry: parse_u32(expect(4, "SOA: expected retry")?, "retry")?,
                    expire: parse_u32(expect(5, "SOA: expected expire")?, "expire")?,
                    minimum: parse_u32(expect(6, "SOA: expected minimum")?, "minimum")?,
                },
            }),
            DnsRecordType::HINFO => {
                let cpu = expect(0, "HINFO: expected cpu")?.to_string();
                let os = expect(1, "HINFO: expected os")?.to_string();
                Ok(DnsRecord::hinfo(owner.to_string(), cpu, os, ttl))
            }
            DnsRecordType::RP => Ok(DnsRecord::rp(
                owner.to_string(),
                normalize_target(expect(0, "RP: expected mbox")?, &self.origin),
                normalize_target(expect(1, "RP: expected txt")?, &self.origin),
                ttl,
            )),
            DnsRecordType::AFSDB => {
                let subtype = parse_u16(expect(0, "AFSDB: expected subtype")?, "subtype")?;
                Ok(DnsRecord::afsdb(
                    owner.to_string(),
                    subtype,
                    normalize_target(expect(1, "AFSDB: expected hostname")?, &self.origin),
                    ttl,
                ))
            }
            DnsRecordType::URI => {
                let priority = parse_u16(expect(0, "URI: expected priority")?, "priority")?;
                let weight = parse_u16(expect(1, "URI: expected weight")?, "weight")?;
                let target = rdata[2..].join(" ").trim_matches('"').to_string();
                Ok(DnsRecord::uri(
                    owner.to_string(),
                    priority,
                    weight,
                    target,
                    ttl,
                ))
            }
            _ => Ok(DnsRecord {
                name: owner.to_string(),
                rtype,
                rclass: 1,
                ttl,
                data: DnsRecordData::Raw(rdata.join(" ").into_bytes()),
            }),
        }
    }
}

fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in line.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                current.push(ch);
            }
            '(' | ')' if !in_quotes => {}
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn is_class_or_type(token: &str) -> bool {
    matches!(
        token.to_uppercase().as_str(),
        "IN" | "CS"
            | "CH"
            | "HS"
            | "A"
            | "AAAA"
            | "CNAME"
            | "NS"
            | "MX"
            | "TXT"
            | "PTR"
            | "SOA"
            | "SRV"
            | "NAPTR"
            | "CAA"
            | "TLSA"
            | "HTTPS"
            | "SVCB"
            | "DS"
            | "DNSKEY"
            | "RRSIG"
            | "NSEC"
            | "NSEC3"
            | "ANY"
            | "OPT"
            | "SPF"
            | "TSIG"
            | "HINFO"
            | "RP"
            | "LOC"
            | "AFSDB"
            | "URI"
    )
}

fn normalize_owner(token: &str, origin: &str) -> String {
    let t = token.trim_end_matches('.');
    if t == "@" || t.is_empty() {
        origin.to_string()
    } else if t == origin || t.ends_with(&format!(".{}", origin)) {
        t.to_lowercase()
    } else {
        format!("{}.{}", t.to_lowercase(), origin)
    }
}

fn normalize_target(token: &str, origin: &str) -> String {
    let t = token.trim_end_matches('.').trim_matches('"');
    if t == "@" || t.is_empty() {
        origin.to_string()
    } else if t.ends_with(&format!(".{}", origin)) {
        t.to_lowercase()
    } else if t.contains('.') {
        t.to_lowercase()
    } else {
        format!("{}.{}", t.to_lowercase(), origin)
    }
}

fn rtype_from_str(s: &str) -> Option<DnsRecordType> {
    match s {
        "A" => Some(DnsRecordType::A),
        "AAAA" => Some(DnsRecordType::AAAA),
        "CNAME" => Some(DnsRecordType::CNAME),
        "NS" => Some(DnsRecordType::NS),
        "MX" => Some(DnsRecordType::MX),
        "TXT" => Some(DnsRecordType::TXT),
        "PTR" => Some(DnsRecordType::PTR),
        "SOA" => Some(DnsRecordType::SOA),
        "SRV" => Some(DnsRecordType::SRV),
        "NAPTR" => Some(DnsRecordType::NAPTR),
        "CAA" => Some(DnsRecordType::CAA),
        "TLSA" => Some(DnsRecordType::TLSA),
        "HTTPS" => Some(DnsRecordType::HTTPS),
        "SVCB" => Some(DnsRecordType::SVCB),
        "DS" => Some(DnsRecordType::DS),
        "DNSKEY" => Some(DnsRecordType::DNSKEY),
        "RRSIG" => Some(DnsRecordType::RRSIG),
        "NSEC" => Some(DnsRecordType::NSEC),
        "NSEC3" => Some(DnsRecordType::NSEC3),
        "SPF" => Some(DnsRecordType::TXT),
        "ANY" => Some(DnsRecordType::ANY),
        "HINFO" => Some(DnsRecordType::HINFO),
        "RP" => Some(DnsRecordType::RP),
        "LOC" => Some(DnsRecordType::LOC),
        "AFSDB" => Some(DnsRecordType::AFSDB),
        "URI" => Some(DnsRecordType::URI),
        "TSIG" => Some(DnsRecordType::TSIG),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_simple_zone() {
        let z = "$TTL 3600\n$ORIGIN example.com.\n@ IN SOA ns1.example.com. admin.example.com. 2024010101 3600 900 604800 86400\n@ IN NS ns1.example.com.\n@ IN A 192.168.1.1\nwww IN A 192.168.1.2\nmail IN MX 10 mail.example.com.\n";
        let zone = parse_zone_file(z).unwrap();
        assert_eq!(zone.origin, "example.com");
        assert_eq!(zone.total_records(), 5);
    }
    #[test]
    fn test_parse_multiline_soa() {
        let z = "$TTL 3600\n$ORIGIN example.com.\n@ IN SOA ns1.example.com. admin.example.com. ( 2024010101 3600 900 604800 86400 )\n";
        let zone = parse_zone_file(z).unwrap();
        assert!(zone.resolve("@", DnsRecordType::SOA).is_some());
    }
    #[test]
    fn test_parse_cname() {
        let z =
            "$TTL 300\n$ORIGIN example.com.\nwww IN A 1.2.3.4\nblog IN CNAME www.example.com.\n";
        let zone = parse_zone_file(z).unwrap();
        assert_eq!(
            zone.resolve("blog.example.com", DnsRecordType::CNAME)
                .unwrap()
                .len(),
            1
        );
    }
    #[test]
    fn test_parse_error() {
        assert!(parse_zone_file("$TTL 3600\n$ORIGIN example.com.\n@ IN BOGUS 1.2.3.4\n").is_err());
    }
    #[test]
    fn test_relative_owner() {
        let z = "$TTL 3600\n$ORIGIN example.com.\nwww IN A 1.2.3.4\n    IN A 1.2.3.5\n";
        let zone = parse_zone_file(z).unwrap();
        assert_eq!(
            zone.resolve("www.example.com", DnsRecordType::A)
                .unwrap()
                .len(),
            2
        );
    }
}
