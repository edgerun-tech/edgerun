//! DNS protocol types and codecs.
//!
//! This module owns DNS wire-format parsing, TCP framing, zone records, and
//! deterministic zone-transfer/update helpers. It does not own sockets,
//! resolver transports, timers, filesystems, or host policy.

pub mod axfr;
#[cfg(feature = "dnssec")]
pub mod dnssec;
pub mod doh;
pub mod io;
pub mod limits;
pub mod message;
pub mod name;
pub mod query;
pub mod record;
pub mod resolv_conf;
pub mod root_hints;
pub mod tcp_frame;
pub mod tsig;
pub mod zone;
pub mod zone_file;

pub use axfr::{handle_axfr, handle_notify, handle_update};
#[cfg(feature = "dnssec")]
pub use dnssec::{
    compute_key_tag, find_nsec3_covering, generate_dnskey_ecdsap256, generate_dnskey_ed25519,
    nsec3_base32hex, nsec3_hash_owner, nsec3_type_bitmap, sign_rrset_ecdsap256, sign_rrsig_ed25519,
    sign_zone_ecdsap256, sign_zone_ed25519, synthesize_nsec3_chain, validate_response,
    verify_chain_of_trust, verify_rrsig, DnssecResult,
};
pub use doh::{
    decode_doh_get_query, decode_doh_post_query, doh_error_http_response,
    doh_formerr_http_response, DohRequestError, DOH_DNS_MESSAGE_CONTENT_TYPE,
    DOH_TEXT_CONTENT_TYPE,
};
pub use limits::{
    dns_section_counts, parse_dns_message_bounded, validate_dns_wire_bounds, DnsSectionCounts,
    MAX_DNS_MESSAGE_LEN, MAX_DNS_QUESTIONS, MAX_DNS_SECTION_RECORDS,
};
pub use message::{DnsHeader, DnsMessage, DnsOpcode, DnsQuestion, DnsRecord, DnsResponseCode};
pub use name::{normalize_name, validate_name, NameError};
pub use query::{
    handle_query_without_forwarding, resolve, udp_response_wire, ParseError, MAX_UDP_RESPONSE,
};
pub use record::{DnsRecordData, DnsRecordType};
pub use resolv_conf::{Nameserver, ResolvConf};
pub use root_hints::{default_root_hints, RootHint};
pub use tcp_frame::{
    decode_single_dns_tcp_frame, dns_tcp_frame_len, encode_dns_tcp_frame, DnsTcpFrameDecoder,
    DnsTcpFrameError,
};
pub use tsig::TsigRdata;
pub use zone::DnsZone;
pub use zone_file::{parse_zone_file, ZoneFileError};
