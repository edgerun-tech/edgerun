use std::sync::Arc;
use std::sync::Mutex;

use winit::keyboard::{Key, ModifiersState, NamedKey};

use term_core::terminal::write_bytes;

pub fn encode_modifiers(mods: ModifiersState) -> u8 {
    1 + (mods.shift_key() as u8) + (mods.alt_key() as u8) * 2 + (mods.control_key() as u8) * 4
}

fn send_cursor_key(
    writer: &Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    app_cursor_keys: bool,
    code: u8,
    mods: ModifiersState,
    kitty_keyboard: bool,
) {
    let modifier = encode_modifiers(mods);

    if modifier == 1 && !kitty_keyboard {
        if app_cursor_keys {
            write_bytes(writer, &[0x1b, b'O', code]);
        } else {
            write_bytes(writer, &[0x1b, b'[', code]);
        }
        return;
    }

    if kitty_keyboard {
        let seq = format!("\x1b[{};{}u", code, modifier);
        write_bytes(writer, seq.as_bytes());
    } else {
        let prefix = if app_cursor_keys { "\x1bO" } else { "\x1b[" };
        let seq = format!("{}1;{}{}", prefix, modifier, code as char);
        write_bytes(writer, seq.as_bytes());
    }
}

pub fn function_key_sequence(key: NamedKey, mods: ModifiersState) -> Option<String> {
    let modifier = encode_modifiers(mods);
    match key {
        NamedKey::F1 => Some(if modifier == 1 {
            "\x1bOP".to_string()
        } else {
            format!("\x1b[1;{}P", modifier)
        }),
        NamedKey::F2 => Some(if modifier == 1 {
            "\x1bOQ".to_string()
        } else {
            format!("\x1b[1;{}Q", modifier)
        }),
        NamedKey::F3 => Some(if modifier == 1 {
            "\x1bOR".to_string()
        } else {
            format!("\x1b[1;{}R", modifier)
        }),
        NamedKey::F4 => Some(if modifier == 1 {
            "\x1bOS".to_string()
        } else {
            format!("\x1b[1;{}S", modifier)
        }),
        NamedKey::F5 => Some(format!("\x1b[15;{}~", modifier)),
        NamedKey::F6 => Some(format!("\x1b[17;{}~", modifier)),
        NamedKey::F7 => Some(format!("\x1b[18;{}~", modifier)),
        NamedKey::F8 => Some(format!("\x1b[19;{}~", modifier)),
        NamedKey::F9 => Some(format!("\x1b[20;{}~", modifier)),
        NamedKey::F10 => Some(format!("\x1b[21;{}~", modifier)),
        NamedKey::F11 => Some(format!("\x1b[23;{}~", modifier)),
        NamedKey::F12 => Some(format!("\x1b[24;{}~", modifier)),
        _ => None,
    }
}

pub fn send_key(
    event: winit::event::KeyEvent,
    modifiers: ModifiersState,
    writer: &Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    app_cursor_keys: bool,
    kitty_keyboard: bool,
) {
    use winit::keyboard::NamedKey::*;

    if event.state != winit::event::ElementState::Pressed {
        return;
    }

    match event.logical_key {
        Key::Named(NamedKey::Enter) => write_bytes(writer, b"\r"),
        Key::Named(NamedKey::Backspace) => write_bytes(writer, b"\x7f"),
        Key::Named(NamedKey::Delete) => write_bytes(writer, b"\x1b[3~"),
        Key::Named(NamedKey::Tab) => write_bytes(writer, b"\t"),
        Key::Named(NamedKey::Escape) => write_bytes(writer, b"\x1b"),
        Key::Named(NamedKey::ArrowUp) => {
            send_cursor_key(writer, app_cursor_keys, b'A', modifiers, kitty_keyboard);
        }
        Key::Named(NamedKey::ArrowDown) => {
            send_cursor_key(writer, app_cursor_keys, b'B', modifiers, kitty_keyboard);
        }
        Key::Named(NamedKey::ArrowRight) => {
            send_cursor_key(writer, app_cursor_keys, b'C', modifiers, kitty_keyboard)
        }
        Key::Named(NamedKey::ArrowLeft) => {
            send_cursor_key(writer, app_cursor_keys, b'D', modifiers, kitty_keyboard)
        }
        Key::Named(nk)
            if matches!(
                nk,
                F1 | F2 | F3 | F4 | F5 | F6 | F7 | F8 | F9 | F10 | F11 | F12
            ) =>
        {
            if let Some(seq) = function_key_sequence(nk, modifiers) {
                write_bytes(writer, seq.as_bytes());
            }
        }
        Key::Named(NamedKey::Space) => write_bytes(writer, b" "),
        Key::Named(_) => {}
        Key::Character(text) => {
            if text.is_empty() {
                return;
            }

            if kitty_keyboard {
                if let Some(ch) = event.text.as_deref().and_then(|t| t.chars().next()) {
                    let codepoint = ch as u32;
                    let mod_bits = encode_modifiers(modifiers);
                    let seq = format!("\x1b[{};{}u", codepoint, mod_bits);
                    write_bytes(writer, seq.as_bytes());
                    return;
                }
            }

            if modifiers.control_key() && !modifiers.shift_key() {
                if let Some(ch) = text.chars().next() {
                    let ctrl = (ch.to_ascii_uppercase() as u8) & 0x1f;
                    write_bytes(writer, &[ctrl]);
                }
            } else if let Some(text) = event.text {
                write_bytes(writer, text.as_bytes());
            } else {
                write_bytes(writer, text.as_bytes());
            }
        }
        Key::Unidentified(_) | Key::Dead(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[derive(Default)]
    struct Buf(Arc<Mutex<Vec<u8>>>);

    impl Write for Buf {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn capture_writer() -> (Arc<Mutex<Vec<u8>>>, Arc<Mutex<Box<dyn std::io::Write + Send>>>) {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let writer: Box<dyn std::io::Write + Send> = Box::new(Buf(buf.clone()));
        (buf, Arc::new(Mutex::new(writer)))
    }

    #[test]
    fn send_cursor_key_respects_modes() {
        let (buf, writer) = capture_writer();
        send_cursor_key(&writer, false, b'A', ModifiersState::empty(), false);
        assert_eq!(buf.lock().unwrap().as_slice(), b"\x1b[A");

        buf.lock().unwrap().clear();
        send_cursor_key(&writer, true, b'A', ModifiersState::empty(), false);
        assert_eq!(buf.lock().unwrap().as_slice(), b"\x1bOA");

        buf.lock().unwrap().clear();
        send_cursor_key(&writer, false, b'A', ModifiersState::empty(), true);
        assert_eq!(buf.lock().unwrap().as_slice(), b"\x1b[65;1u");
    }
}
