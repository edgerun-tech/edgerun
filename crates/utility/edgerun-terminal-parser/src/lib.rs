//! Edgerun terminal control-sequence parser.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::{mem, str};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Params {
    values: Vec<Vec<u16>>,
    len: usize,
}

impl Params {
    fn from_values(values: Vec<Vec<u16>>) -> Self {
        let len = values.iter().map(Vec::len).sum();
        Self { values, len }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &[u16]> {
        self.values.iter().map(Vec::as_slice)
    }
}

pub trait Perform {
    fn print(&mut self, _c: char) {}
    fn execute(&mut self, _byte: u8) {}
    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn csi_dispatch(
        &mut self,
        _params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: char,
    ) {
    }
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

#[derive(Default)]
pub struct Parser {
    state: State,
    utf8: Vec<u8>,
    intermediates: Vec<u8>,
    params: ParamBuilder,
    osc: Vec<u8>,
    dcs_active: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum State {
    #[default]
    Ground,
    Escape,
    Csi,
    Osc,
    OscEscape,
    DcsEntry,
    Dcs,
    DcsEscape,
}

impl Parser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance<P: Perform>(&mut self, performer: &mut P, byte: u8) {
        match self.state {
            State::Ground => self.advance_ground(performer, byte),
            State::Escape => self.advance_escape(performer, byte),
            State::Csi => self.advance_csi(performer, byte),
            State::Osc => self.advance_osc(performer, byte),
            State::OscEscape => self.advance_osc_escape(performer, byte),
            State::DcsEntry => self.advance_dcs_entry(performer, byte),
            State::Dcs => self.advance_dcs(performer, byte),
            State::DcsEscape => self.advance_dcs_escape(performer, byte),
        }
    }

    fn advance_ground<P: Perform>(&mut self, performer: &mut P, byte: u8) {
        match byte {
            0x1b => {
                self.utf8.clear();
                self.state = State::Escape;
            }
            0x00..=0x1f | 0x7f => performer.execute(byte),
            0x20..=0x7e => performer.print(byte as char),
            0x80..=0xff => self.advance_utf8(performer, byte),
        }
    }

    fn advance_utf8<P: Perform>(&mut self, performer: &mut P, byte: u8) {
        if self.utf8.is_empty() && !is_utf8_start(byte) {
            return;
        }
        self.utf8.push(byte);
        let expected = utf8_len(self.utf8[0]);
        if expected == 0 {
            self.utf8.clear();
            return;
        }
        if self.utf8.len() < expected {
            return;
        }
        if let Ok(text) = str::from_utf8(&self.utf8) {
            if let Some(ch) = text.chars().next() {
                performer.print(ch);
            }
        }
        self.utf8.clear();
    }

    fn advance_escape<P: Perform>(&mut self, performer: &mut P, byte: u8) {
        match byte {
            b'[' => self.start_params(State::Csi),
            b']' => {
                self.osc.clear();
                self.state = State::Osc;
            }
            b'P' => self.start_params(State::DcsEntry),
            0x20..=0x2f => {
                self.intermediates.clear();
                self.intermediates.push(byte);
            }
            0x30..=0x7e => {
                performer.esc_dispatch(&self.intermediates, false, byte);
                self.intermediates.clear();
                self.state = State::Ground;
            }
            0x00..=0x1f | 0x7f => performer.execute(byte),
            _ => self.state = State::Ground,
        }
    }

    fn advance_csi<P: Perform>(&mut self, performer: &mut P, byte: u8) {
        match byte {
            b'0'..=b'9' => self.params.digit(byte - b'0'),
            b';' => self.params.next_param(),
            b':' => self.params.next_subparam(),
            b'?' | b'>' | b'=' | b'<' => self.intermediates.push(byte),
            0x20..=0x2f => self.intermediates.push(byte),
            0x40..=0x7e => {
                let params = self.params.finish();
                performer.csi_dispatch(&params, &self.intermediates, false, byte as char);
                self.intermediates.clear();
                self.state = State::Ground;
            }
            0x1b => self.state = State::Escape,
            0x00..=0x1f | 0x7f => performer.execute(byte),
            _ => {}
        }
    }

    fn advance_osc<P: Perform>(&mut self, performer: &mut P, byte: u8) {
        match byte {
            0x07 => {
                self.dispatch_osc(performer, true);
                self.state = State::Ground;
            }
            0x1b => self.state = State::OscEscape,
            _ => self.osc.push(byte),
        }
    }

    fn advance_osc_escape<P: Perform>(&mut self, performer: &mut P, byte: u8) {
        if byte == b'\\' {
            self.dispatch_osc(performer, false);
            self.state = State::Ground;
        } else {
            self.osc.push(0x1b);
            self.osc.push(byte);
            self.state = State::Osc;
        }
    }

    fn advance_dcs_entry<P: Perform>(&mut self, performer: &mut P, byte: u8) {
        match byte {
            b'0'..=b'9' => self.params.digit(byte - b'0'),
            b';' => self.params.next_param(),
            b':' => self.params.next_subparam(),
            b'?' | b'>' | b'=' | b'<' => self.intermediates.push(byte),
            0x20..=0x2f => self.intermediates.push(byte),
            0x40..=0x7e => {
                let params = self.params.finish();
                performer.hook(&params, &self.intermediates, false, byte as char);
                self.intermediates.clear();
                self.dcs_active = true;
                self.state = State::Dcs;
            }
            0x1b => self.state = State::Escape,
            0x00..=0x1f | 0x7f => performer.execute(byte),
            _ => {}
        }
    }

    fn advance_dcs<P: Perform>(&mut self, performer: &mut P, byte: u8) {
        match byte {
            0x1b => self.state = State::DcsEscape,
            _ => performer.put(byte),
        }
    }

    fn advance_dcs_escape<P: Perform>(&mut self, performer: &mut P, byte: u8) {
        if byte == b'\\' {
            if self.dcs_active {
                performer.unhook();
            }
            self.dcs_active = false;
            self.state = State::Ground;
        } else {
            performer.put(0x1b);
            performer.put(byte);
            self.state = State::Dcs;
        }
    }

    fn start_params(&mut self, state: State) {
        self.params.clear();
        self.intermediates.clear();
        self.state = state;
    }

    fn dispatch_osc<P: Perform>(&mut self, performer: &mut P, bell_terminated: bool) {
        let params: Vec<&[u8]> = self.osc.split(|byte| *byte == b';').collect();
        performer.osc_dispatch(&params, bell_terminated);
        self.osc.clear();
    }
}

#[derive(Default)]
struct ParamBuilder {
    values: Vec<Vec<u16>>,
    current: Vec<u16>,
    value: u16,
    has_value: bool,
    saw_separator: bool,
}

impl ParamBuilder {
    fn clear(&mut self) {
        self.values.clear();
        self.current.clear();
        self.value = 0;
        self.has_value = false;
        self.saw_separator = false;
    }

    fn digit(&mut self, digit: u8) {
        self.value = self.value.saturating_mul(10).saturating_add(digit as u16);
        self.has_value = true;
    }

    fn next_param(&mut self) {
        self.push_value();
        self.values.push(mem::take(&mut self.current));
        self.saw_separator = true;
    }

    fn next_subparam(&mut self) {
        self.push_value();
        self.saw_separator = true;
    }

    fn finish(&mut self) -> Params {
        if self.has_value || self.saw_separator || !self.current.is_empty() {
            self.push_value();
            self.values.push(mem::take(&mut self.current));
        }
        let values = mem::take(&mut self.values);
        self.clear();
        Params::from_values(values)
    }

    fn push_value(&mut self) {
        self.current
            .push(if self.has_value { self.value } else { 0 });
        self.value = 0;
        self.has_value = false;
    }
}

fn is_utf8_start(byte: u8) -> bool {
    matches!(byte, 0xc2..=0xf4)
}

fn utf8_len(byte: u8) -> usize {
    match byte {
        0x00..=0x7f => 1,
        0xc2..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf4 => 4,
        _ => 0,
    }
}
