// SPDX-License-Identifier: Apache-2.0
use term_core::terminal::{GridPerformer, Terminal};
use term_core::test_support::capture_writer;
use vte::Parser;

#[test]
fn vte_print_and_newline_renders_expected_cells() {
    let mut term = Terminal::new(2, 2);
    let (buf, writer) = capture_writer();
    let mut app_cursor_keys = false;
    let mut performer = GridPerformer {
        grid: &mut term,
        writer,
        app_cursor_keys: &mut app_cursor_keys,
        dcs_state: None,
    };
    let mut parser = Parser::new();
    for b in b"A\nB" {
        parser.advance(&mut performer, *b);
    }

    let first = term.display_cell(0, 0).text;
    let second_row_first = term.display_cell(0, 1).text;
    assert_eq!(first, "A");
    assert_eq!(second_row_first, "B");
    assert!(buf.lock().unwrap().is_empty());
}
