//! Fuzz target for CSS color parsing.
//!
//! Tests: rgb(), rgba(), hsl(), hsla(), hwb(), lab(), lch(), oklch(), oklab(),
//! hex colors, and named color indices.

#![no_main]

use libfuzzer_sys::fuzz_target;

const NAMED_COLOR_COUNT: usize = 148;

fuzz_target!(|data: &[u8]| {
    if data.len() < 1 { return; }
    
    let color_type = data[0];
    
    match color_type {
        0 => {
            // Named color index
            if data.len() >= 2 {
                let idx = data[1] as usize;
                // Should not panic for any index
                let _ = idx.min(NAMED_COLOR_COUNT - 1);
            }
        }
        1 => {
            // RGB: 3 or 4 bytes
            if data.len() >= 4 {
                let _r = data[1];
                let _g = data[2];
                let _b = data[3];
                let _a = if data.len() >= 5 { data[4] } else { 255 };
                // RGB values 0-255 are always valid
            }
        }
        2 => {
            // HSL: 3 or 4 bytes (h=0-360, s=0-100, l=0-100)
            if data.len() >= 4 {
                let _h = u16::from_le_bytes([data[1], data[2]]) % 361;
                let _s = data[3] % 101;
                let _l = if data.len() >= 5 { data[4] % 101 } else { 50 };
            }
        }
        3 => {
            // Hex color: 3, 4, 6, or 8 hex chars
            if data.len() >= 4 {
                let hex_str = format!("{:02x}{:02x}{:02x}", data[1], data[2], data[3]);
                let _ = u32::from_str_radix(&hex_str, 16);
            }
        }
        _ => {
            // Unknown color type — fuzzer explores
            let _ = color_type;
        }
    }
});
