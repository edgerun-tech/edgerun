use super::StreamOutput;

#[test]
fn test_utf8_shell_output() {
    // Baseline: UTF-8 output should bypass the detector and remain unchanged.
    assert_eq!(decode_shell_output("пример".as_bytes()), "пример");
}

#[test]
fn test_invalid_bytes_still_fall_back_to_lossy() {
    // If detection fails, we still want the user to see replacement characters.
    let bytes = b"\xFF\xFE\xFD";
    assert_eq!(decode_shell_output(bytes), String::from_utf8_lossy(bytes));
}

fn decode_shell_output(bytes: &[u8]) -> String {
    StreamOutput {
        text: bytes.to_vec(),
        truncated_after_lines: None,
    }
    .from_utf8_lossy()
    .text
}
