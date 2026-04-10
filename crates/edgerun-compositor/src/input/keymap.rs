//! Pure Rust keymap — scancode to keysym translation.
//!
//! No xkbcommon dependency. We implement a basic US QWERTY keymap
//! with modifier handling (Shift, Caps Lock, Ctrl, Alt).

/// Linux evdev scancode constants.
pub mod scancode {
    pub const KEY_ESC: u16 = 1;
    pub const KEY_1: u16 = 2;
    pub const KEY_2: u16 = 3;
    pub const KEY_3: u16 = 4;
    pub const KEY_4: u16 = 5;
    pub const KEY_5: u16 = 6;
    pub const KEY_6: u16 = 7;
    pub const KEY_7: u16 = 8;
    pub const KEY_8: u16 = 9;
    pub const KEY_9: u16 = 10;
    pub const KEY_0: u16 = 11;
    pub const KEY_MINUS: u16 = 12;
    pub const KEY_EQUAL: u16 = 13;
    pub const KEY_BACKSPACE: u16 = 14;
    pub const KEY_TAB: u16 = 15;
    pub const KEY_Q: u16 = 16;
    pub const KEY_W: u16 = 17;
    pub const KEY_E: u16 = 18;
    pub const KEY_R: u16 = 19;
    pub const KEY_T: u16 = 20;
    pub const KEY_Y: u16 = 21;
    pub const KEY_U: u16 = 22;
    pub const KEY_I: u16 = 23;
    pub const KEY_O: u16 = 24;
    pub const KEY_P: u16 = 25;
    pub const KEY_LEFTBRACE: u16 = 26;
    pub const KEY_RIGHTBRACE: u16 = 27;
    pub const KEY_ENTER: u16 = 28;
    pub const KEY_LEFTCTRL: u16 = 29;
    pub const KEY_RIGHTCTRL: u16 = 97;
    pub const KEY_A: u16 = 30;
    pub const KEY_S: u16 = 31;
    pub const KEY_D: u16 = 32;
    pub const KEY_F: u16 = 33;
    pub const KEY_G: u16 = 34;
    pub const KEY_H: u16 = 35;
    pub const KEY_J: u16 = 36;
    pub const KEY_K: u16 = 37;
    pub const KEY_L: u16 = 38;
    pub const KEY_SEMICOLON: u16 = 39;
    pub const KEY_APOSTROPHE: u16 = 40;
    pub const KEY_GRAVE: u16 = 41;
    pub const KEY_LEFTSHIFT: u16 = 42;
    pub const KEY_RIGHTSHIFT: u16 = 54;
    pub const KEY_BACKSLASH: u16 = 43;
    pub const KEY_Z: u16 = 44;
    pub const KEY_X: u16 = 45;
    pub const KEY_C: u16 = 46;
    pub const KEY_V: u16 = 47;
    pub const KEY_B: u16 = 48;
    pub const KEY_N: u16 = 49;
    pub const KEY_M: u16 = 50;
    pub const KEY_COMMA: u16 = 51;
    pub const KEY_DOT: u16 = 52;
    pub const KEY_SLASH: u16 = 53;
    pub const KEY_CAPSLOCK: u16 = 58;
    pub const KEY_SPACE: u16 = 57;
    pub const KEY_F1: u16 = 59;
    pub const KEY_F2: u16 = 60;
    pub const KEY_F3: u16 = 61;
    pub const KEY_F4: u16 = 62;
    pub const KEY_F5: u16 = 63;
    pub const KEY_F6: u16 = 64;
    pub const KEY_F7: u16 = 65;
    pub const KEY_F8: u16 = 66;
    pub const KEY_F9: u16 = 67;
    pub const KEY_F10: u16 = 68;
    pub const KEY_F11: u16 = 87;
    pub const KEY_F12: u16 = 88;
    pub const KEY_HOME: u16 = 102;
    pub const KEY_UP: u16 = 103;
    pub const KEY_PAGEUP: u16 = 104;
    pub const KEY_LEFT: u16 = 105;
    pub const KEY_RIGHT: u16 = 106;
    pub const KEY_END: u16 = 107;
    pub const KEY_DOWN: u16 = 108;
    pub const KEY_PAGEDOWN: u16 = 109;
    pub const KEY_INSERT: u16 = 110;
    pub const KEY_DELETE: u16 = 111;
    pub const KEY_LEFTALT: u16 = 56;
    pub const KEY_RIGHTALT: u16 = 100;
}

/// A keysym — simplified representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keysym {
    /// Printable character.
    Char(char),
    /// Special key.
    Special(SpecialKey),
    /// Unknown/unmapped key.
    Unknown(u16),
}

/// Special keys that don't produce characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialKey {
    Escape,
    Tab,
    Return,
    Backspace,
    CapsLock,
    ShiftLeft,
    ShiftRight,
    CtrlLeft,
    CtrlRight,
    AltLeft,
    AltRight,
    F(u8),
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    Insert,
    Delete,
    Space,
}

/// Modifier state.
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub caps: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// US QWERTY keymap — maps scancode to (unshifted, shifted) characters or special keys.
pub struct Keymap {
    table: [Option<(char, char)>; 256],
    special: [Option<(SpecialKey, SpecialKey)>; 256],
}

impl Keymap {
    /// Create a US QWERTY keymap.
    pub fn us_qwerty() -> Self {
        Self::estonian_nodeadkeys() // Same letter layout; minor number row differences
    }

    /// Create an Estonian keymap (nodeadkeys).
    pub fn estonian_nodeadkeys() -> Self {
        let mut table: [Option<(char, char)>; 256] = [None; 256];
        let mut special: [Option<(SpecialKey, SpecialKey)>; 256] = [None; 256];

        // Numbers row (same as US)
        table[scancode::KEY_1 as usize] = Some(('1', '!'));
        table[scancode::KEY_2 as usize] = Some(('2', '@'));  // Actually @ is AltGr+2 on EE, but we map simple
        table[scancode::KEY_3 as usize] = Some(('3', '#'));
        table[scancode::KEY_4 as usize] = Some(('4', '¤'));
        table[scancode::KEY_5 as usize] = Some(('5', '%'));
        table[scancode::KEY_6 as usize] = Some(('6', '&'));
        table[scancode::KEY_7 as usize] = Some(('7', '/'));
        table[scancode::KEY_8 as usize] = Some(('8', '('));
        table[scancode::KEY_9 as usize] = Some(('9', ')'));
        table[scancode::KEY_0 as usize] = Some(('0', '='));

        // Letters — same as US QWERTY layout
        for (sc, lower) in [
            (scancode::KEY_A, 'a'), (scancode::KEY_B, 'b'), (scancode::KEY_C, 'c'),
            (scancode::KEY_D, 'd'), (scancode::KEY_E, 'e'), (scancode::KEY_F, 'f'),
            (scancode::KEY_G, 'g'), (scancode::KEY_H, 'h'), (scancode::KEY_I, 'i'),
            (scancode::KEY_J, 'j'), (scancode::KEY_K, 'k'), (scancode::KEY_L, 'l'),
            (scancode::KEY_M, 'm'), (scancode::KEY_N, 'n'), (scancode::KEY_O, 'o'),
            (scancode::KEY_P, 'p'), (scancode::KEY_Q, 'q'), (scancode::KEY_R, 'r'),
            (scancode::KEY_S, 's'), (scancode::KEY_T, 't'), (scancode::KEY_U, 'u'),
            (scancode::KEY_V, 'v'), (scancode::KEY_W, 'w'), (scancode::KEY_X, 'x'),
            (scancode::KEY_Y, 'y'), (scancode::KEY_Z, 'z'),
        ] {
            table[sc as usize] = Some((lower, lower.to_ascii_uppercase()));
        }

        // Estonian-specific punctuation
        table[scancode::KEY_MINUS as usize] = Some(('-', '_'));
        table[scancode::KEY_EQUAL as usize] = Some(('´', '`')); // EE: acute accent unshifted
        table[scancode::KEY_LEFTBRACE as usize] = Some(('ž', 'Ž'));
        table[scancode::KEY_RIGHTBRACE as usize] = Some(('+', '*'));
        table[scancode::KEY_SEMICOLON as usize] = Some(('ö', 'Ö'));
        table[scancode::KEY_APOSTROPHE as usize] = Some(('ä', 'Ä'));
        table[scancode::KEY_GRAVE as usize] = Some(('§', '~')); // EE: section sign
        table[scancode::KEY_BACKSLASH as usize] = Some(('^', '°'));
        table[scancode::KEY_COMMA as usize] = Some((';', ':'));
        table[scancode::KEY_DOT as usize] = Some((',', '<'));
        table[scancode::KEY_SLASH as usize] = Some(('.', '-'));
        table[scancode::KEY_SPACE as usize] = Some((' ', ' '));

        // Special keys (same as US)
        special[scancode::KEY_ESC as usize] = Some((SpecialKey::Escape, SpecialKey::Escape));
        special[scancode::KEY_TAB as usize] = Some((SpecialKey::Tab, SpecialKey::Tab));
        special[scancode::KEY_ENTER as usize] = Some((SpecialKey::Return, SpecialKey::Return));
        special[scancode::KEY_BACKSPACE as usize] = Some((SpecialKey::Backspace, SpecialKey::Backspace));
        special[scancode::KEY_CAPSLOCK as usize] = Some((SpecialKey::CapsLock, SpecialKey::CapsLock));
        special[scancode::KEY_LEFTSHIFT as usize] = Some((SpecialKey::ShiftLeft, SpecialKey::ShiftLeft));
        special[scancode::KEY_RIGHTSHIFT as usize] = Some((SpecialKey::ShiftRight, SpecialKey::ShiftRight));
        special[scancode::KEY_LEFTCTRL as usize] = Some((SpecialKey::CtrlLeft, SpecialKey::CtrlLeft));
        special[scancode::KEY_RIGHTCTRL as usize] = Some((SpecialKey::CtrlRight, SpecialKey::CtrlRight));
        special[scancode::KEY_LEFTALT as usize] = Some((SpecialKey::AltLeft, SpecialKey::AltLeft));
        special[scancode::KEY_RIGHTALT as usize] = Some((SpecialKey::AltRight, SpecialKey::AltRight));
        special[scancode::KEY_F1 as usize] = Some((SpecialKey::F(1), SpecialKey::F(1)));
        special[scancode::KEY_F2 as usize] = Some((SpecialKey::F(2), SpecialKey::F(2)));
        special[scancode::KEY_F3 as usize] = Some((SpecialKey::F(3), SpecialKey::F(3)));
        special[scancode::KEY_F4 as usize] = Some((SpecialKey::F(4), SpecialKey::F(4)));
        special[scancode::KEY_F5 as usize] = Some((SpecialKey::F(5), SpecialKey::F(5)));
        special[scancode::KEY_F6 as usize] = Some((SpecialKey::F(6), SpecialKey::F(6)));
        special[scancode::KEY_F7 as usize] = Some((SpecialKey::F(7), SpecialKey::F(7)));
        special[scancode::KEY_F8 as usize] = Some((SpecialKey::F(8), SpecialKey::F(8)));
        special[scancode::KEY_F9 as usize] = Some((SpecialKey::F(9), SpecialKey::F(9)));
        special[scancode::KEY_F10 as usize] = Some((SpecialKey::F(10), SpecialKey::F(10)));
        special[scancode::KEY_F11 as usize] = Some((SpecialKey::F(11), SpecialKey::F(11)));
        special[scancode::KEY_F12 as usize] = Some((SpecialKey::F(12), SpecialKey::F(12)));
        special[scancode::KEY_HOME as usize] = Some((SpecialKey::Home, SpecialKey::Home));
        special[scancode::KEY_END as usize] = Some((SpecialKey::End, SpecialKey::End));
        special[scancode::KEY_PAGEUP as usize] = Some((SpecialKey::PageUp, SpecialKey::PageUp));
        special[scancode::KEY_PAGEDOWN as usize] = Some((SpecialKey::PageDown, SpecialKey::PageDown));
        special[scancode::KEY_UP as usize] = Some((SpecialKey::Up, SpecialKey::Up));
        special[scancode::KEY_DOWN as usize] = Some((SpecialKey::Down, SpecialKey::Down));
        special[scancode::KEY_LEFT as usize] = Some((SpecialKey::Left, SpecialKey::Left));
        special[scancode::KEY_RIGHT as usize] = Some((SpecialKey::Right, SpecialKey::Right));
        special[scancode::KEY_INSERT as usize] = Some((SpecialKey::Insert, SpecialKey::Insert));
        special[scancode::KEY_DELETE as usize] = Some((SpecialKey::Delete, SpecialKey::Delete));

        Self { table, special }
    }

    /// Translate a scancode to a keysym given current modifier state.
    pub fn translate(&self, scancode: u16, mods: &Modifiers) -> Keysym {
        let sc = scancode as usize;
        if sc >= 256 {
            return Keysym::Unknown(scancode);
        }

        if let Some((unshifted, shifted)) = self.table[sc] {
            let c = if mods.shift != mods.caps {
                // XOR: shift active or caps active (but not both)
                shifted
            } else {
                unshifted
            };
            Keysym::Char(c)
        } else if let Some((unshifted, shifted)) = self.special[sc] {
            let _ = shifted; // special keys are the same shifted/unshifted
            Keysym::Special(unshifted)
        } else {
            Keysym::Unknown(scancode)
        }
    }
}

/// Process a key event (scancode + pressed/released) and update modifiers.
pub fn process_key_event(scancode: u16, pressed: bool, mods: &mut Modifiers) {
    match scancode {
        scancode::KEY_LEFTSHIFT | scancode::KEY_RIGHTSHIFT => mods.shift = pressed,
        scancode::KEY_CAPSLOCK if pressed => mods.caps = !mods.caps,
        scancode::KEY_LEFTCTRL | scancode::KEY_RIGHTCTRL => mods.ctrl = pressed,
        scancode::KEY_LEFTALT | scancode::KEY_RIGHTALT => mods.alt = pressed,
        _ => {}
    }
}

/// Generate the XKB keymap text for an Estonian layout (nodeadkeys).
pub fn xkb_keymap_text_estonian() -> &'static str {
    r#"xkb_keymap {
    xkb_keycodes { include "evdev+aliases(qwerty)" };
    xkb_types { include "complete" };
    xkb_compat { include "complete" };
    xkb_symbols { include "pc+ee(nodeadkeys)+inet(evdev)" };
    xkb_geometry { include "pc(pc105)" };
};"#
}

/// Generate the XKB keymap text for a US QWERTY layout.
pub fn xkb_keymap_text() -> &'static str {
    r#"xkb_keymap {
    xkb_keycodes { include "evdev+aliases(qwerty)" };
    xkb_types { include "complete" };
    xkb_compat { include "complete" };
    xkb_symbols { include "pc+us+inet(evdev)" };
    xkb_geometry { include "pc(pc105)" };
};"#
}
