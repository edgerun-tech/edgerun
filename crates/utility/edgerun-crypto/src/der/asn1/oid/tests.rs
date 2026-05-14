use super::ObjectIdentifier;
use crate::der::{Decode, Encode, Length};

const EXAMPLE_OID: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549");
const EXAMPLE_OID_BYTES: &[u8; 8] = &[0x06, 0x06, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d];

#[test]
fn decode() {
    let oid = ObjectIdentifier::from_der(EXAMPLE_OID_BYTES).unwrap();
    assert_eq!(EXAMPLE_OID, oid);
}

#[test]
fn encode() {
    let mut buffer = [0u8; 8];
    assert_eq!(
        EXAMPLE_OID_BYTES,
        EXAMPLE_OID.encode_to_slice(&mut buffer).unwrap()
    );
}

#[test]
fn length() {
    assert!(ObjectIdentifier::MAX_SIZE <= Length::MAX.try_into().unwrap());
}
