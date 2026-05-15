use crate::der::{Decode, Encode};

#[test]
fn decode() {
    assert_eq!(true, bool::from_der(&[0x01, 0x01, 0xFF]).unwrap());
    assert_eq!(false, bool::from_der(&[0x01, 0x01, 0x00]).unwrap());
}

#[test]
fn encode() {
    let mut buffer = [0u8; 3];
    assert_eq!(
        &[0x01, 0x01, 0xFF],
        true.encode_to_slice(&mut buffer).unwrap()
    );
    assert_eq!(
        &[0x01, 0x01, 0x00],
        false.encode_to_slice(&mut buffer).unwrap()
    );
}

#[test]
fn reject_non_canonical() {
    assert!(bool::from_der(&[0x01, 0x01, 0x01]).is_err());
}
