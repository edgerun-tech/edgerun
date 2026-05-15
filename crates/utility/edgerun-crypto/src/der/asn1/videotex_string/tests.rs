use super::VideotexStringRef;
use crate::der::Decode;

#[test]
fn parse_bytes() {
    let example_bytes = &[
        0x15, 0x0b, 0x54, 0x65, 0x73, 0x74, 0x20, 0x55, 0x73, 0x65, 0x72, 0x20, 0x31,
    ];

    let printable_string = VideotexStringRef::from_der(example_bytes).unwrap();
    assert_eq!(printable_string.as_str(), "Test User 1");
}
