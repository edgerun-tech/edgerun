use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use term_core::terminal::{GridPerformer, Terminal};
use vte::Parser;

#[derive(Clone, Default)]
struct LockedBuf(Arc<Mutex<Vec<u8>>>);

impl Write for LockedBuf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn capture_writer() -> (Arc<Mutex<Vec<u8>>>, Arc<Mutex<Box<dyn Write + Send>>>) {
    let buf = Arc::new(Mutex::new(Vec::new()));
    let writer: Box<dyn Write + Send> = Box::new(LockedBuf(buf.clone()));
    (buf, Arc::new(Mutex::new(writer)))
}

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
