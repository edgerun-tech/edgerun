use super::SliceReader;
use crate::der::{Decode, ErrorKind, Length, Reader, Tag};
use crate::hex;

// INTEGER: 42
const EXAMPLE_MSG: &[u8] = &hex!("02012A00");

#[test]
fn empty_message() {
    let mut reader = SliceReader::new(&[]).unwrap();
    let err = bool::decode(&mut reader).err().unwrap();
    assert_eq!(Some(Length::ZERO), err.position());

    match err.kind() {
        ErrorKind::Incomplete {
            expected_len,
            actual_len,
        } => {
            assert_eq!(actual_len, 0u8.into());
            assert_eq!(expected_len, 1u8.into());
        }
        other => panic!("unexpected error kind: {:?}", other),
    }
}

#[test]
fn invalid_field_length() {
    const MSG_LEN: usize = 2;

    let mut reader = SliceReader::new(&EXAMPLE_MSG[..MSG_LEN]).unwrap();
    let err = i8::decode(&mut reader).err().unwrap();
    assert_eq!(Some(Length::from(2u8)), err.position());

    match err.kind() {
        ErrorKind::Incomplete {
            expected_len,
            actual_len,
        } => {
            assert_eq!(actual_len, MSG_LEN.try_into().unwrap());
            assert_eq!(expected_len, (MSG_LEN + 1).try_into().unwrap());
        }
        other => panic!("unexpected error kind: {:?}", other),
    }
}

#[test]
fn trailing_data() {
    let mut reader = SliceReader::new(EXAMPLE_MSG).unwrap();
    let x = i8::decode(&mut reader).unwrap();
    assert_eq!(42i8, x);

    let err = reader.finish(x).err().unwrap();
    assert_eq!(Some(Length::from(3u8)), err.position());

    assert_eq!(
        ErrorKind::TrailingData {
            decoded: 3u8.into(),
            remaining: 1u8.into()
        },
        err.kind()
    );
}

#[test]
fn peek_tag() {
    let reader = SliceReader::new(EXAMPLE_MSG).unwrap();
    assert_eq!(reader.position(), Length::ZERO);
    assert_eq!(reader.peek_tag().unwrap(), Tag::Integer);
    assert_eq!(reader.position(), Length::ZERO);
}

#[test]
fn peek_header() {
    let reader = SliceReader::new(EXAMPLE_MSG).unwrap();
    assert_eq!(reader.position(), Length::ZERO);

    let header = reader.peek_header().unwrap();
    assert_eq!(header.tag, Tag::Integer);
    assert_eq!(header.length, Length::ONE);
    assert_eq!(reader.position(), Length::ZERO);
}
