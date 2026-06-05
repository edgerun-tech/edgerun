use crate::prelude::*;
use core::fmt::Write as _;

pub(crate) fn write_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch <= '\u{1f}' => {
                write!(out, "\\u{:04x}", ch as u32).expect("writing to String cannot fail");
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

pub(crate) fn push_field_prefix(out: &mut String, first: &mut bool, name: &str) {
    if !*first {
        out.push(',');
    }
    *first = false;
    write_string(out, name);
    out.push(':');
}

pub(crate) fn push_string_field(out: &mut String, first: &mut bool, name: &str, value: &str) {
    push_field_prefix(out, first, name);
    write_string(out, value);
}

pub(crate) fn push_u64_field(out: &mut String, first: &mut bool, name: &str, value: u64) {
    push_field_prefix(out, first, name);
    write!(out, "{value}").expect("writing to String cannot fail");
}

pub(crate) fn push_i64_field(out: &mut String, first: &mut bool, name: &str, value: i64) {
    push_field_prefix(out, first, name);
    write!(out, "{value}").expect("writing to String cannot fail");
}

pub(crate) fn push_bool_field(out: &mut String, first: &mut bool, name: &str, value: bool) {
    push_field_prefix(out, first, name);
    out.push_str(if value { "true" } else { "false" });
}

pub(crate) fn push_string_array(out: &mut String, values: &[String]) {
    out.push('[');
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        write_string(out, value);
    }
    out.push(']');
}

pub(crate) fn push_str_array(out: &mut String, values: &[&str]) {
    out.push('[');
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        write_string(out, value);
    }
    out.push(']');
}
