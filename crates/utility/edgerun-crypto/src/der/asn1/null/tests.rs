use super::Null;
use crate::der::{Decode, Encode};

#[test]
fn decode() {
    Null::from_der(&[0x05, 0x00]).unwrap();
}

#[test]
fn encode() {
    let mut buffer = [0u8; 2];
    assert_eq!(&[0x05, 0x00], Null.encode_to_slice(&mut buffer).unwrap());
    assert_eq!(&[0x05, 0x00], ().encode_to_slice(&mut buffer).unwrap());
}

#[test]
fn reject_non_canonical() {
    assert!(Null::from_der(&[0x05, 0x81, 0x00]).is_err());
}
