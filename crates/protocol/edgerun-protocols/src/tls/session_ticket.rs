//! TLS 1.3 NewSessionTicket parsing (RFC 8446 §4.6.1).

use alloc::vec::Vec;
use edgerun_encoding::byteorder::{read_u16_be, read_u32_be};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewSessionTicket {
    pub ticket: Vec<u8>,
    pub lifetime: u32,
    pub age_add: u32,
    pub nonce: Vec<u8>,
    pub extensions: Vec<u8>,
}

pub fn parse_new_session_ticket(data: &[u8]) -> Option<NewSessionTicket> {
    if data.len() < 10 {
        return None;
    }

    let lifetime = read_u32_be(data, 0);
    let age_add = read_u32_be(data, 4);

    let nonce_len = data[8] as usize;
    let nonce_start = 9;
    let nonce_end = nonce_start + nonce_len;
    if nonce_end + 2 > data.len() {
        return None;
    }

    let ticket_len = read_u16_be(data, nonce_end) as usize;
    let ticket_start = nonce_end + 2;
    let ticket_end = ticket_start + ticket_len;
    if ticket_end + 2 > data.len() {
        return None;
    }

    let extensions_len = read_u16_be(data, ticket_end) as usize;
    let extensions_start = ticket_end + 2;
    let extensions_end = extensions_start + extensions_len;
    if extensions_end > data.len() {
        return None;
    }

    Some(NewSessionTicket {
        ticket: data[ticket_start..ticket_end].to_vec(),
        lifetime,
        age_add,
        nonce: data[nonce_start..nonce_end].to_vec(),
        extensions: data[extensions_start..extensions_end].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn parses_new_session_ticket() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&7200u32.to_be_bytes());
        bytes.extend_from_slice(&0x11223344u32.to_be_bytes());
        bytes.push(2);
        bytes.extend_from_slice(&[0xaa, 0xbb]);
        bytes.extend_from_slice(&3u16.to_be_bytes());
        bytes.extend_from_slice(&[1, 2, 3]);
        bytes.extend_from_slice(&4u16.to_be_bytes());
        bytes.extend_from_slice(&[4, 5, 6, 7]);

        let ticket = parse_new_session_ticket(&bytes).unwrap();
        assert_eq!(ticket.lifetime, 7200);
        assert_eq!(ticket.age_add, 0x11223344);
        assert_eq!(ticket.nonce, vec![0xaa, 0xbb]);
        assert_eq!(ticket.ticket, vec![1, 2, 3]);
        assert_eq!(ticket.extensions, vec![4, 5, 6, 7]);
    }

    #[test]
    fn rejects_truncated_ticket() {
        assert_eq!(parse_new_session_ticket(&[0; 9]), None);
        assert_eq!(
            parse_new_session_ticket(&[
                0, 0, 0, 1, // lifetime
                0, 0, 0, 2,  // age_add
                10, // nonce len
                1, 2,
            ]),
            None
        );
    }
}
