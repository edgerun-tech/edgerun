use crate::render::{
    FONT_DATA, FONT_SIZE, GlyphCache, draw_background, draw_cursor_overlay, draw_grid,
    draw_text_line, draw_text_line_clipped,
};
use crate::terminal::{
    DEFAULT_BG, DEFAULT_FG, GridPerformer, Rgba, Terminal, ansi_color, brightened, selection_text,
    xterm_color,
};
use std::io;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use vte::Parser as VteParser;

#[derive(Clone, Default)]
struct LockedBuffer(Arc<Mutex<Vec<u8>>>);

impl io::Write for LockedBuffer {
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
    let writer: Box<dyn Write + Send> = Box::new(LockedBuffer(buf.clone()));
    (buf, Arc::new(Mutex::new(writer)))
}

#[test]
fn background_fill_matches_default_bg() {
    let width = 8;
    let height = 6;
    let mut frame = vec![0u8; (width * height * 4) as usize];

    draw_background(&mut frame, width, height, Instant::now(), DEFAULT_BG);

    for chunk in frame.chunks_exact(4) {
        assert_eq!(chunk[0], DEFAULT_BG.r);
        assert_eq!(chunk[1], DEFAULT_BG.g);
        assert_eq!(chunk[2], DEFAULT_BG.b);
        assert_eq!(chunk[3], DEFAULT_BG.a);
    }
}

#[test]
fn cell_background_pixels_are_rendered() {
    let primary = Arc::new(FONT_DATA.to_vec());
    let mut glyphs = GlyphCache::new(primary, FONT_SIZE);
    let (cell_w, cell_h) = glyphs.cell_size();

    let mut term = Terminal::new(1, 1);
    let bg = Rgba {
        r: 50,
        g: 100,
        b: 150,
        a: 255,
    };
    term.pen_bg = bg;
    term.put_char(' ');
    term.set_view_offset(1); // hide cursor overlay so background is testable

    let width = cell_w.max(1);
    let height = cell_h.max(1);
    let mut frame = vec![0u8; (width * height * 4) as usize];

    draw_background(&mut frame, width, height, Instant::now(), DEFAULT_BG);
    draw_grid(
        &term,
        &mut glyphs,
        &mut frame,
        width,
        height,
        cell_w,
        cell_h,
        0,
        0,
        None,
        None,
        true,
        None,
        None,
    );
    draw_cursor_overlay(
        &term,
        &mut glyphs,
        &mut frame,
        width,
        height,
        cell_w,
        cell_h,
        0,
        0,
        None,
        true,
        true,
    );
    draw_cursor_overlay(
        &term,
        &mut glyphs,
        &mut frame,
        width,
        height,
        cell_w,
        cell_h,
        0,
        0,
        None,
        true,
        true,
    );
    let sample_x = (cell_w / 2).min(width.saturating_sub(1));
    let sample_y = (cell_h / 2).min(height.saturating_sub(1));
    let idx = ((sample_y * width + sample_x) * 4) as usize;
    assert_eq!(frame[idx], bg.r);
    assert_eq!(frame[idx + 1], bg.g);
    assert_eq!(frame[idx + 2], bg.b);
    assert_eq!(frame[idx + 3], 255);
}

#[test]
fn dsr_reports_cursor_position() {
    let mut term = Terminal::new(10, 5);
    term.set_cursor(3, 2); // 0-based -> expect 3;4 in response
    let (buf, writer) = capture_writer();
    let mut app_cursor_keys = false;
    let mut performer = GridPerformer {
        grid: &mut term,
        writer: writer.clone(),
        app_cursor_keys: &mut app_cursor_keys,
        dcs_state: None,
    };
    let mut parser = VteParser::new();
    for byte in b"\x1b[6n" {
        parser.advance(&mut performer, *byte);
    }
    let data = buf.lock().unwrap().clone();
    assert_eq!(data, b"\x1b[3;4R");
}

#[test]
fn alt_screen_round_trip_restores_primary() {
    let mut term = Terminal::new(3, 2);
    term.put_char('A');
    term.put_char('B');
    term.enter_alt_screen();
    term.put_char('X');
    assert!(term.in_alt_screen());
    term.leave_alt_screen();
    assert!(!term.in_alt_screen());
    // Primary grid restores after leaving alt screen.
    assert_eq!(term.display_cell(0, 0).text, "A");
    assert_eq!(term.display_cell(1, 0).text, "B");
}

#[test]
fn alt_screen_exit_restores_visible_grid() {
    let mut term = Terminal::new(3, 2);
    term.put_char('h');
    term.put_char('i');

    term.enter_alt_screen();
    term.put_char('x');
    term.put_char('y');

    term.leave_alt_screen();

    // Exiting alternate screen restores the prior contents.
    assert_eq!(term.display_cell(0, 0).text, "h");
    assert_eq!(term.display_cell(1, 0).text, "i");
    assert_eq!(term.display_cell(0, 1).text, " ");
}

#[test]
fn scrollback_collects_wrapped_lines() {
    let mut term = Terminal::new(2, 2);
    for _ in 0..6 {
        term.put_char('x');
    }
    assert!(!term.scrollback.is_empty());
    assert_eq!(term.scrollback[0][0].text, "x");
}

#[test]
fn carriage_return_clears_wrap_pending() {
    let mut term = Terminal::new(3, 2);
    term.put_char('a');
    term.put_char('b');
    term.put_char('c'); // hits last column and sets wrap_next
    term.carriage_return(); // should cancel pending wrap
    term.put_char('1');
    term.put_char('2');
    term.put_char('3');

    assert!(
        term.scrollback.is_empty(),
        "carriage return must not scroll"
    );
    assert_eq!(term.cursor_row, 0);
    assert_eq!(term.display_cell(0, 0).text, "1");
    assert_eq!(term.display_cell(2, 0).text, "3");
}

#[test]
fn vte_carriage_return_replaces_line() {
    let mut term = Terminal::new(12, 1);
    let writer: Arc<Mutex<Box<dyn Write + Send>>> = Arc::new(Mutex::new(Box::new(std::io::sink())));
    let mut app_cursor_keys = false;
    let mut performer = GridPerformer {
        grid: &mut term,
        writer: writer.clone(),
        app_cursor_keys: &mut app_cursor_keys,
        dcs_state: None,
    };
    let mut parser = VteParser::new();

    for byte in b"first\rsecond\rthird" {
        parser.advance(&mut performer, *byte);
    }

    let mut line = String::new();
    for c in 0..term.cols {
        line.push_str(&term.display_cell(c, 0).text);
    }
    assert!(line.starts_with("third"));
    assert!(line[5..].chars().all(|c| c == ' '));
}

#[test]
fn bold_brightens_foreground() {
    let mut term = Terminal::new(2, 1);
    let (_, writer) = capture_writer();
    let mut app_cursor_keys = false;
    let mut parser = VteParser::new();
    {
        let mut performer = GridPerformer {
            grid: &mut term,
            writer: writer.clone(),
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };
        for byte in b"\x1b[31m\x1b[1mX" {
            parser.advance(&mut performer, *byte);
        }
    }
    let cell = term.display_cell(0, 0);
    let expected = brightened(ansi_color(1, false));
    assert_eq!(cell.text, "X");
    assert_eq!(cell.fg, expected);
    assert!(cell.bold);
}

#[test]
fn selection_text_trims_trailing_spaces() {
    let mut term = Terminal::new(3, 1);
    term.put_char('a');
    term.put_char(' ');
    term.put_char(' ');
    let text = selection_text(&term, (0, 0), (2, 0));
    assert_eq!(text, "a");
}

#[test]
fn selection_text_trims_and_spans_bounds() {
    let mut term = Terminal::new(4, 2);
    term.put_char('h');
    term.put_char('i');
    term.new_line();
    term.put_char('x');
    let text = selection_text(&term, (0, 0), (3, 1));
    assert_eq!(text, "hi\nx");
}

#[test]
fn double_width_glyph_copies_correctly() {
    let mut term = Terminal::new(4, 1);
    term.put_char('界');
    let lead = term.display_cell(0, 0);
    let trail = term.display_cell(1, 0);
    assert_eq!(lead.text, "界");
    assert!(lead.wide);
    assert!(trail.wide_continuation);

    let text = selection_text(&term, (0, 0), (3, 0));
    assert_eq!(text, "界");
}

#[test]
fn colon_sgr_background_is_respected() {
    let mut term = Terminal::new(2, 1);
    let mut parser = VteParser::new();
    let writer: Arc<Mutex<Box<dyn Write + Send>>> = Arc::new(Mutex::new(Box::new(std::io::sink())));
    let mut app_cursor_keys = false;

    {
        let mut performer = GridPerformer {
            grid: &mut term,
            writer: writer.clone(),
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };
        for byte in b"\x1b[48:2:10:20:30m \x1b[m" {
            parser.advance(&mut performer, *byte);
        }
    }

    let cell = term.display_cell(0, 0);
    assert_eq!(cell.bg.r, 10);
    assert_eq!(cell.bg.g, 20);
    assert_eq!(cell.bg.b, 30);
}

#[test]
fn empty_sgr_resets_style() {
    let mut term = Terminal::new(2, 1);
    let mut parser = VteParser::new();
    let writer: Arc<Mutex<Box<dyn Write + Send>>> = Arc::new(Mutex::new(Box::new(std::io::sink())));
    let mut app_cursor_keys = false;

    {
        let mut performer = GridPerformer {
            grid: &mut term,
            writer: writer.clone(),
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };
        for byte in b"\x1b[31;48;5;160mX\x1b[mY" {
            parser.advance(&mut performer, *byte);
        }
    }

    let first = term.display_cell(0, 0);
    assert_eq!(first.text, "X");
    assert_eq!(first.fg, ansi_color(1, false));
    assert_eq!(first.bg, xterm_color(160));

    let second = term.display_cell(1, 0);
    assert_eq!(second.text, "Y");
    assert_eq!(second.fg, DEFAULT_FG);
    assert_eq!(second.bg, DEFAULT_BG);
    assert!(!second.bold);
    assert!(!second.italic);
    assert!(!second.underline);
}

#[test]
fn italic_and_underline_are_recorded() {
    let mut term = Terminal::new(3, 1);
    let mut parser = VteParser::new();
    let writer: Arc<Mutex<Box<dyn Write + Send>>> = Arc::new(Mutex::new(Box::new(std::io::sink())));
    let mut app_cursor_keys = false;

    {
        let mut performer = GridPerformer {
            grid: &mut term,
            writer: writer.clone(),
            app_cursor_keys: &mut app_cursor_keys,
            dcs_state: None,
        };
        for byte in b"\x1b[3;4mI" {
            parser.advance(&mut performer, *byte);
        }
    }

    let cell = term.display_cell(0, 0);
    assert_eq!(cell.text, "I");
    assert!(cell.italic);
    assert!(cell.underline);
}

#[test]
fn draw_text_line_clipped_emits_ellipsis_when_too_narrow() {
    let primary = Arc::new(FONT_DATA.to_vec());
    let mut glyphs = GlyphCache::new(primary, FONT_SIZE);

    let width = 80;
    let height = 32;
    let mut frame_clipped = vec![0u8; width * height * 4];
    let mut frame_expected = vec![0u8; width * height * 4];

    let color = [255, 255, 255, 255];
    let x = 0;
    let y = 0;

    // Force clipping: max_x is too small for the first glyph.
    draw_text_line_clipped(
        &mut glyphs,
        &mut frame_clipped,
        width as u32,
        height as u32,
        x,
        y,
        "Hello",
        color,
        1,
    );
    // Expected output is just ellipsis.
    draw_text_line(
        &mut glyphs,
        &mut frame_expected,
        width as u32,
        height as u32,
        x,
        y,
        "...",
        color,
    );

    assert_eq!(frame_clipped, frame_expected);
}

#[test]
fn draw_text_line_with_emoji_writes_pixels() {
    let primary = Arc::new(FONT_DATA.to_vec());
    let mut glyphs = GlyphCache::new(primary, FONT_SIZE);
    glyphs.add_fonts(GlyphCache::load_fallback_fonts());

    let width = 96;
    let height = 48;
    let mut frame = vec![0u8; width * height * 4];
    draw_text_line(
        &mut glyphs,
        &mut frame,
        width as u32,
        height as u32,
        4,
        4,
        "Hello 😊",
        [255, 255, 255, 255],
    );

    assert!(
        frame.chunks_exact(4).any(|px| px[3] > 0),
        "expected at least one drawn pixel"
    );
}

#[test]
fn draw_text_line_clipped_emoji_uses_ellipsis() {
    let primary = Arc::new(FONT_DATA.to_vec());
    let mut glyphs = GlyphCache::new(primary, FONT_SIZE);
    glyphs.add_fonts(GlyphCache::load_fallback_fonts());

    let width = 64;
    let height = 32;
    let mut frame_clipped = vec![0u8; width * height * 4];
    let mut frame_expected = vec![0u8; width * height * 4];

    let color = [255, 255, 255, 255];
    let x = 0;
    let y = 0;

    // Force clipping: width too small for emoji string.
    draw_text_line_clipped(
        &mut glyphs,
        &mut frame_clipped,
        width as u32,
        height as u32,
        x,
        y,
        "emoji 😊 test",
        color,
        10,
    );
    draw_text_line(
        &mut glyphs,
        &mut frame_expected,
        width as u32,
        height as u32,
        x,
        y,
        "...",
        color,
    );

    assert_eq!(frame_clipped, frame_expected);
}

#[test]
fn shaped_text_offsets_are_applied() {
    let primary = Arc::new(FONT_DATA.to_vec());
    let mut glyphs = GlyphCache::new(primary, FONT_SIZE);
    glyphs.add_fonts(GlyphCache::load_fallback_fonts());

    let width = 200;
    let height = 80;
    let mut frame = vec![0u8; width * height * 4];

    draw_text_line(
        &mut glyphs,
        &mut frame,
        width as u32,
        height as u32,
        10,
        10,
        "fi ligature 😊 café",
        [255, 255, 255, 255],
    );

    // Verify baseline row has non-zero alpha to ensure glyphs respect offsets.
    let baseline_row = 10 + glyphs.baseline() as usize;
    let row_range = (baseline_row * width as usize * 4)
        ..((baseline_row + 1) * width as usize * 4).min(frame.len());
    let has_pixels = frame[row_range].chunks_exact(4).any(|px| px[3] > 0);
    assert!(
        has_pixels,
        "expected pixels on baseline row for shaped text"
    );
}

#[test]
fn underline_aligns_with_cell_bottom() {
    let primary = Arc::new(FONT_DATA.to_vec());
    let mut glyphs = GlyphCache::new(primary, FONT_SIZE);
    let (cell_w, cell_h) = glyphs.cell_size();

    let mut term = Terminal::new(4, 1);
    term.put_char('u');
    term.pen_underline = true;
    term.put_char('n');
    term.pen_underline = false;
    // Force cursor row 0
    term.cursor_row = 0;
    term.cursor_col = 2;

    let width = cell_w * 4;
    let height = cell_h;
    let mut frame = vec![0u8; (width * height * 4) as usize];

    draw_grid(
        &term,
        &mut glyphs,
        &mut frame,
        width,
        height,
        cell_w,
        cell_h,
        0,
        0,
        None,
        None,
        true,
        None,
        None,
    );

    // Check bottom row has underline pixels.
    let last_row_start = ((height - 2) * width * 4) as usize;
    let last_row = &frame[last_row_start..];
    assert!(
        last_row.chunks_exact(4).any(|px| px[3] > 0),
        "expected underline pixels on bottom row"
    );
}

#[test]
fn hover_link_range_draws_underline() {
    let primary = Arc::new(FONT_DATA.to_vec());
    let mut glyphs = GlyphCache::new(primary, FONT_SIZE);
    let (cell_w, cell_h) = glyphs.cell_size();
    assert!(cell_h >= 2, "cell height too small for underline");

    let mut term = Terminal::new(2, 1);
    term.put_char('a');
    term.put_char('b');
    term.set_view_offset(1);

    let width = cell_w * 2;
    let height = cell_h;
    let mut frame = vec![0u8; (width * height * 4) as usize];
    draw_background(&mut frame, width, height, Instant::now(), DEFAULT_BG);
    draw_grid(
        &term,
        &mut glyphs,
        &mut frame,
        width,
        height,
        cell_w,
        cell_h,
        0,
        0,
        None,
        None,
        true,
        Some((0, 0, 1)),
        None,
    );

    let line_y = cell_h.saturating_sub(2);
    let idx = (line_y * width * 4) as usize;
    assert_eq!(frame[idx], 20);
    assert_eq!(frame[idx + 1], 60);
    assert_eq!(frame[idx + 2], 120);
}

#[test]
fn cursor_over_wide_emoji_draws_rect() {
    let primary = Arc::new(FONT_DATA.to_vec());
    let mut glyphs = GlyphCache::new(primary, FONT_SIZE);
    glyphs.add_fonts(GlyphCache::load_fallback_fonts());
    let (cell_w, cell_h) = glyphs.cell_size();

    let mut term = Terminal::new(4, 1);
    term.put_char('😊'); // wide in color font
    term.cursor_col = 0;
    term.cursor_row = 0;

    let width = cell_w * 4;
    let height = cell_h * 2;
    let mut frame = vec![0u8; (width * height * 4) as usize];

    draw_grid(
        &term,
        &mut glyphs,
        &mut frame,
        width,
        height,
        cell_w,
        cell_h,
        0,
        0,
        None,
        None,
        true,
        None,
        None,
    );

    // Cursor rect should fill first cell(s) with alpha.
    let cursor_region = &frame[..(cell_w * cell_h * 2 * 4) as usize];
    assert!(
        cursor_region.chunks_exact(4).any(|px| px[3] > 0),
        "expected cursor overlay over wide emoji"
    );
}

#[test]
fn selection_text_trims_trailing_spaces_across_rows() {
    let mut term = Terminal::new(2, 2);
    term.put_char('a');
    term.put_char(' ');
    term.put_char('b');
    term.put_char(' ');

    let text = selection_text(&term, (0, 0), (1, 1));
    assert_eq!(text, "a\nb");
}

#[test]
fn trims_selection_without_trailing_space() {
    let mut term = Terminal::new(2, 1);
    term.put_char('a');
    term.put_char('b');
    let text = selection_text(&term, (0, 0), (1, 0));
    assert_eq!(text, "ab");
}
