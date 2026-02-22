// SPDX-License-Identifier: Apache-2.0
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha512};

const DOMAIN_TAG: &[u8] = b"edgerun.solana.vanity.v1";
const INPUT_VERSION: u8 = 1;
const OUTPUT_VERSION: u8 = 1;
const INPUT_HEADER_LEN: usize = 50;
const OUTPUT_HEADER_LEN: usize = 43;
const MAX_ERROR_LEN: usize = 255;
const NONE_COUNTER: u64 = u64::MAX;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRequest {
    pub seed: [u8; 32],
    pub start_counter: u64,
    pub max_attempts: u64,
    pub prefix: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedKeypair {
    pub counter: u64,
    pub seed: [u8; 32],
    pub public_key: [u8; 32],
    pub keypair_bytes: [u8; 64],
    pub address: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    pub counter: u64,
    pub public_key: [u8; 32],
    pub address: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchResponse {
    Found(SearchMatch),
    NotFound,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchError {
    BadVersion(u8),
    InputTooShort,
    InvalidInputLen,
    PrefixTooLong(usize),
    PrefixNotUtf8,
    CounterOverflow,
    HostcallError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputStatus {
    Found = 0,
    NotFound = 1,
    Error = 2,
}

pub fn derive_keypair(seed: [u8; 32], counter: u64) -> DerivedKeypair {
    let derived_seed = derive_seed(seed, counter);
    let signing_key = SigningKey::from_bytes(&derived_seed);
    let public_key = signing_key.verifying_key().to_bytes();

    let mut keypair_bytes = [0_u8; 64];
    keypair_bytes[..32].copy_from_slice(&derived_seed);
    keypair_bytes[32..].copy_from_slice(&public_key);

    DerivedKeypair {
        counter,
        seed: derived_seed,
        public_key,
        keypair_bytes,
        address: bs58::encode(public_key).into_string(),
    }
}

pub fn search_prefix(req: &SearchRequest) -> Result<Option<SearchMatch>, SearchError> {
    let end = req
        .start_counter
        .checked_add(req.max_attempts)
        .ok_or(SearchError::CounterOverflow)?;

    for counter in req.start_counter..end {
        let kp = derive_keypair(req.seed, counter);
        if kp.address.as_bytes().starts_with(&req.prefix) {
            return Ok(Some(SearchMatch {
                counter,
                public_key: kp.public_key,
                address: kp.address,
            }));
        }
    }

    Ok(None)
}

pub fn parse_request(input: &[u8]) -> Result<SearchRequest, SearchError> {
    if input.len() < INPUT_HEADER_LEN {
        return Err(SearchError::InputTooShort);
    }
    let version = input[0];
    if version != INPUT_VERSION {
        return Err(SearchError::BadVersion(version));
    }

    let mut seed = [0_u8; 32];
    seed.copy_from_slice(&input[1..33]);

    let mut start_bytes = [0_u8; 8];
    start_bytes.copy_from_slice(&input[33..41]);
    let start_counter = u64::from_le_bytes(start_bytes);

    let mut attempts_bytes = [0_u8; 8];
    attempts_bytes.copy_from_slice(&input[41..49]);
    let max_attempts = u64::from_le_bytes(attempts_bytes);

    let prefix_len = input[49] as usize;
    if input.len() != INPUT_HEADER_LEN + prefix_len {
        return Err(SearchError::InvalidInputLen);
    }
    if prefix_len > 44 {
        return Err(SearchError::PrefixTooLong(prefix_len));
    }

    let prefix = input[50..50 + prefix_len].to_vec();
    if core::str::from_utf8(&prefix).is_err() {
        return Err(SearchError::PrefixNotUtf8);
    }

    Ok(SearchRequest {
        seed,
        start_counter,
        max_attempts,
        prefix,
    })
}

pub fn encode_request(req: &SearchRequest) -> Result<Vec<u8>, SearchError> {
    if req.prefix.len() > 44 {
        return Err(SearchError::PrefixTooLong(req.prefix.len()));
    }
    if core::str::from_utf8(&req.prefix).is_err() {
        return Err(SearchError::PrefixNotUtf8);
    }

    let mut out = Vec::with_capacity(INPUT_HEADER_LEN + req.prefix.len());
    out.push(INPUT_VERSION);
    out.extend_from_slice(&req.seed);
    out.extend_from_slice(&req.start_counter.to_le_bytes());
    out.extend_from_slice(&req.max_attempts.to_le_bytes());
    out.push(req.prefix.len() as u8);
    out.extend_from_slice(&req.prefix);
    Ok(out)
}

pub fn encode_found(m: &SearchMatch) -> Vec<u8> {
    let address = m.address.as_bytes();
    let address_len = address.len().min(u8::MAX as usize);
    let mut out = Vec::with_capacity(OUTPUT_HEADER_LEN + address_len);
    out.push(OUTPUT_VERSION);
    out.push(OutputStatus::Found as u8);
    out.extend_from_slice(&m.counter.to_le_bytes());
    out.extend_from_slice(&m.public_key);
    out.push(address_len as u8);
    out.extend_from_slice(&address[..address_len]);
    out
}

pub fn encode_not_found() -> Vec<u8> {
    let mut out = Vec::with_capacity(OUTPUT_HEADER_LEN);
    out.push(OUTPUT_VERSION);
    out.push(OutputStatus::NotFound as u8);
    out.extend_from_slice(&NONE_COUNTER.to_le_bytes());
    out.extend_from_slice(&[0_u8; 32]);
    out.push(0);
    out
}

pub fn encode_error(err: SearchError) -> Vec<u8> {
    let msg = error_code(err);
    let bytes = msg.as_bytes();
    let len = bytes.len().min(MAX_ERROR_LEN);
    let mut out = Vec::with_capacity(OUTPUT_HEADER_LEN + len);
    out.push(OUTPUT_VERSION);
    out.push(OutputStatus::Error as u8);
    out.extend_from_slice(&NONE_COUNTER.to_le_bytes());
    out.extend_from_slice(&[0_u8; 32]);
    out.push(len as u8);
    out.extend_from_slice(&bytes[..len]);
    out
}

pub fn decode_response(output: &[u8]) -> Result<SearchResponse, SearchError> {
    if output.len() < OUTPUT_HEADER_LEN {
        return Err(SearchError::InputTooShort);
    }
    if output[0] != OUTPUT_VERSION {
        return Err(SearchError::BadVersion(output[0]));
    }
    let status = output[1];

    let mut counter_bytes = [0_u8; 8];
    counter_bytes.copy_from_slice(&output[2..10]);
    let counter = u64::from_le_bytes(counter_bytes);

    let mut pubkey = [0_u8; 32];
    pubkey.copy_from_slice(&output[10..42]);
    let data_len = output[42] as usize;
    if output.len() != OUTPUT_HEADER_LEN + data_len {
        return Err(SearchError::InvalidInputLen);
    }
    let data = output[43..].to_vec();

    match status {
        0 => {
            let address = String::from_utf8(data).map_err(|_| SearchError::PrefixNotUtf8)?;
            Ok(SearchResponse::Found(SearchMatch {
                counter,
                public_key: pubkey,
                address,
            }))
        }
        1 => Ok(SearchResponse::NotFound),
        2 => {
            let error = String::from_utf8(data).map_err(|_| SearchError::PrefixNotUtf8)?;
            Ok(SearchResponse::Error(error))
        }
        _ => Err(SearchError::InvalidInputLen),
    }
}

fn derive_seed(seed: [u8; 32], counter: u64) -> [u8; 32] {
    let mut hasher = Sha512::new();
    hasher.update(DOMAIN_TAG);
    hasher.update(seed);
    hasher.update(counter.to_le_bytes());
    let digest = hasher.finalize();

    let mut out = [0_u8; 32];
    out.copy_from_slice(&digest[..32]);
    out
}

fn error_code(err: SearchError) -> &'static str {
    match err {
        SearchError::BadVersion(_) => "bad_version",
        SearchError::InputTooShort => "input_too_short",
        SearchError::InvalidInputLen => "invalid_input_len",
        SearchError::PrefixTooLong(_) => "prefix_too_long",
        SearchError::PrefixNotUtf8 => "prefix_not_utf8",
        SearchError::CounterOverflow => "counter_overflow",
        SearchError::HostcallError => "hostcall_error",
    }
}

pub fn execute_request(req: &SearchRequest) -> Vec<u8> {
    match search_prefix(req) {
        Ok(Some(found)) => encode_found(&found),
        Ok(None) => encode_not_found(),
        Err(err) => encode_error(err),
    }
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "edgerun")]
extern "C" {
    fn input_len() -> i32;
    fn read_input(dst_ptr: i32, input_offset: i32, len: i32) -> i32;
    fn write_output(src_ptr: i32, len: i32) -> i32;
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn _start() {
    let output = execute_once();
    let ptr = output.as_ptr() as i32;
    let len = output.len() as i32;
    unsafe {
        let _ = write_output(ptr, len);
    }
}

#[cfg(target_arch = "wasm32")]
fn execute_once() -> Vec<u8> {
    match read_request() {
        Ok(req) => execute_request(&req),
        Err(err) => encode_error(err),
    }
}

#[cfg(target_arch = "wasm32")]
fn read_request() -> Result<SearchRequest, SearchError> {
    let len = unsafe { input_len() };
    if len < 0 {
        return Err(SearchError::HostcallError);
    }
    let len = len as usize;
    let mut buf = vec![0_u8; len];
    let rc = unsafe { read_input(buf.as_mut_ptr() as i32, 0, len as i32) };
    if rc < 0 || rc as usize != len {
        return Err(SearchError::HostcallError);
    }
    parse_request(&buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::{Keypair, Signer};

    fn sample_seed() -> [u8; 32] {
        [7_u8; 32]
    }

    #[test]
    fn deterministic_derivation_for_same_counter() {
        let a = derive_keypair(sample_seed(), 42);
        let b = derive_keypair(sample_seed(), 42);
        assert_eq!(a.seed, b.seed);
        assert_eq!(a.public_key, b.public_key);
        assert_eq!(a.keypair_bytes, b.keypair_bytes);
        assert_eq!(a.address, b.address);
    }

    #[test]
    fn derivation_changes_with_counter() {
        let a = derive_keypair(sample_seed(), 42);
        let b = derive_keypair(sample_seed(), 43);
        assert_ne!(a.seed, b.seed);
        assert_ne!(a.public_key, b.public_key);
        assert_ne!(a.address, b.address);
    }

    #[test]
    fn derived_keypair_is_valid_for_solana() {
        let kp = derive_keypair(sample_seed(), 123456);
        let solana_kp =
            Keypair::try_from(&kp.keypair_bytes[..]).expect("valid solana keypair bytes");
        assert_eq!(solana_kp.pubkey().to_bytes(), kp.public_key);
    }

    #[test]
    fn search_finds_known_prefix() {
        let seed = sample_seed();
        let known = derive_keypair(seed, 11);
        let prefix = known.address.as_bytes()[..5].to_vec();
        let req = SearchRequest {
            seed,
            start_counter: 0,
            max_attempts: 100,
            prefix,
        };

        let found = search_prefix(&req)
            .expect("search ok")
            .expect("must find known counter");

        assert!(found.address.starts_with(&known.address[..5]));
    }

    #[test]
    fn parse_request_roundtrip_and_parse() {
        let seed = sample_seed();
        let prefix = b"So".to_vec();
        let mut input = Vec::new();
        input.push(INPUT_VERSION);
        input.extend_from_slice(&seed);
        input.extend_from_slice(&15_u64.to_le_bytes());
        input.extend_from_slice(&99_u64.to_le_bytes());
        input.push(prefix.len() as u8);
        input.extend_from_slice(&prefix);

        let parsed = parse_request(&input).expect("valid request");
        assert_eq!(parsed.seed, seed);
        assert_eq!(parsed.start_counter, 15);
        assert_eq!(parsed.max_attempts, 99);
        assert_eq!(parsed.prefix, prefix);
    }

    #[test]
    fn request_encode_decode_roundtrip() {
        let req = SearchRequest {
            seed: sample_seed(),
            start_counter: 101,
            max_attempts: 5_000,
            prefix: b"So1".to_vec(),
        };
        let bytes = encode_request(&req).expect("encode");
        let parsed = parse_request(&bytes).expect("parse");
        assert_eq!(parsed, req);
    }

    #[test]
    fn response_decode_found_roundtrip() {
        let req = SearchRequest {
            seed: sample_seed(),
            start_counter: 0,
            max_attempts: 100,
            prefix: b"1".to_vec(),
        };
        let out = execute_request(&req);
        let decoded = decode_response(&out).expect("decode");
        match decoded {
            SearchResponse::Found(found) => {
                assert!(found.address.starts_with('1'));
            }
            other => panic!("expected found response, got {other:?}"),
        }
    }

    #[test]
    fn encode_not_found_has_expected_layout() {
        let out = encode_not_found();
        assert_eq!(out[0], OUTPUT_VERSION);
        assert_eq!(out[1], OutputStatus::NotFound as u8);
        assert_eq!(out.len(), OUTPUT_HEADER_LEN);
        assert_eq!(out[42], 0);
    }
}
