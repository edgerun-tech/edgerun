use super::Encoder;
use crate::hex;

/// OID `1.2.840.10045.2.1` encoded as ASN.1 BER/DER
const EXAMPLE_OID_BER: &[u8] = &hex!("2A8648CE3D0201");

#[test]
fn encode() {
    let encoder = Encoder::new();
    let encoder = encoder.arc(1).unwrap();
    let encoder = encoder.arc(2).unwrap();
    let encoder = encoder.arc(840).unwrap();
    let encoder = encoder.arc(10045).unwrap();
    let encoder = encoder.arc(2).unwrap();
    let encoder = encoder.arc(1).unwrap();
    assert_eq!(&encoder.bytes[..encoder.cursor], EXAMPLE_OID_BER);
}
