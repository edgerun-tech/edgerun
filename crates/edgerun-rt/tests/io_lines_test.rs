use core::future::Future;
use edgerun_rt::{block_on, io::lines, BufReader, Cursor};

#[test]
fn lines_handles_crlf_without_newline_tail() {
    let value = block_on(lines(BufReader::new(Cursor::new(
        b"alpha\r\nbeta".to_vec(),
    ))))
    .unwrap();
    assert_eq!(value, Some("alpha".to_string()));
}

#[test]
fn lines_returns_empty_on_blank_line() {
    let value = block_on(lines(BufReader::new(Cursor::new(b"\nrest".to_vec())))).unwrap();
    assert_eq!(value, Some(String::new()));
}

#[test]
fn lines_reads_last_fragment_without_newline() {
    let value = block_on(lines(BufReader::new(Cursor::new(b"final".to_vec())))).unwrap();
    assert_eq!(value, Some("final".to_string()));
}

#[test]
fn lines_returns_none_on_empty_input() {
    let value = block_on(lines(BufReader::new(Cursor::new(Vec::<u8>::new())))).unwrap();
    assert_eq!(value, None);
}
