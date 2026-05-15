use super::Ia5StringRef;
use crate::der::Decode;
use crate::hex;

#[test]
fn parse_bytes() {
    let example_bytes = hex!("16 0d 74 65 73 74 31 40 72 73 61 2e 63 6f 6d");
    let internationalized_string = Ia5StringRef::from_der(&example_bytes).unwrap();
    assert_eq!(internationalized_string.as_str(), "test1@rsa.com");
}
