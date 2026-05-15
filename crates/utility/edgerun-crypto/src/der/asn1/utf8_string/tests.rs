use super::Utf8StringRef;
use crate::der::Decode;

#[test]
fn parse_ascii_bytes() {
    let example_bytes = &[
        0x0c, 0x0b, 0x54, 0x65, 0x73, 0x74, 0x20, 0x55, 0x73, 0x65, 0x72, 0x20, 0x31,
    ];

    let utf8_string = Utf8StringRef::from_der(example_bytes).unwrap();
    assert_eq!(utf8_string.as_str(), "Test User 1");
}

#[test]
fn parse_utf8_bytes() {
    let example_bytes = &[0x0c, 0x06, 0x48, 0x65, 0x6c, 0x6c, 0xc3, 0xb3];
    let utf8_string = Utf8StringRef::from_der(example_bytes).unwrap();
    assert_eq!(utf8_string.as_str(), "Helló");
}
