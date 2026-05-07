//! DNS protocol types and codecs.
//!
//! This module owns DNS wire-format parsing, TCP framing, zone records, and
//! deterministic zone-transfer/update helpers. It does not own sockets,
//! resolver transports, timers, filesystems, or host policy.

pub mod axfr;
pub mod io;
pub mod limits;
pub mod message;
pub mod name;
pub mod record;
pub mod tcp_frame;
pub mod tsig;
pub mod zone;
pub mod zone_file;

pub use axfr::{handle_axfr, handle_notify, handle_update};
pub use limits::{
    dns_section_counts, parse_dns_message_bounded, validate_dns_wire_bounds, DnsSectionCounts,
    MAX_DNS_MESSAGE_LEN, MAX_DNS_QUESTIONS, MAX_DNS_SECTION_RECORDS,
};
pub use message::{DnsHeader, DnsMessage, DnsOpcode, DnsQuestion, DnsRecord, DnsResponseCode};
pub use name::{normalize_name, validate_name, NameError};
pub use record::{DnsRecordData, DnsRecordType};
pub use tcp_frame::{
    decode_single_dns_tcp_frame, dns_tcp_frame_len, encode_dns_tcp_frame, DnsTcpFrameDecoder,
    DnsTcpFrameError,
};
pub use tsig::TsigRdata;
pub use zone::DnsZone;
pub use zone_file::{parse_zone_file, ZoneFileError};
