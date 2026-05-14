use super::TeletexStringRef;
use crate::der::Decode;
use crate::der::SliceWriter;

#[test]
fn parse_bytes() {
    let example_bytes = &[
        0x14, 0x0b, 0x54, 0x65, 0x73, 0x74, 0x20, 0x55, 0x73, 0x65, 0x72, 0x20, 0x31,
    ];

    let teletex_string = TeletexStringRef::from_der(example_bytes).unwrap();
    assert_eq!(teletex_string.as_str(), "Test User 1");
    let mut out = [0_u8; 30];
    let mut writer = SliceWriter::new(&mut out);
    writer.encode(&teletex_string).unwrap();
    let encoded = writer.finish().unwrap();
    assert_eq!(encoded, example_bytes);
}
