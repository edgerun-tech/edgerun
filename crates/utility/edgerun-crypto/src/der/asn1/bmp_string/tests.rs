use super::BmpString;
use crate::der::{Decode, Encode};
use crate::hex;
use alloc::string::ToString;

const EXAMPLE_BYTES: &[u8] = &hex!(
    "1e 26 00 43 00 65 00 72 00 74"
    "      00 69 00 66 00 69 00 63"
    "      00 61 00 74 00 65 00 54"
    "      00 65 00 6d 00 70 00 6c"
    "      00 61 00 74 00 65"
);

const EXAMPLE_UTF8: &str = "CertificateTemplate";

#[test]
fn decode() {
    let bmp_string = BmpString::from_der(EXAMPLE_BYTES).unwrap();
    assert_eq!(bmp_string.to_string(), EXAMPLE_UTF8);
}

#[test]
fn encode() {
    let bmp_string = BmpString::from_utf8(EXAMPLE_UTF8).unwrap();
    let encoded = bmp_string.to_der().unwrap();
    assert_eq!(encoded, EXAMPLE_BYTES);
}
