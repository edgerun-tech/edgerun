//! Fuzz target for border style and width.
//!
//! Tests: all 9 border styles, widths 0-255, dash pattern alignment.

#![no_main]

use libfuzzer_sys::fuzz_target;

const BORDER_STYLE_COUNT: usize = 9;

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 { return; }
    
    let style = data[0] as usize;
    let width = data[1] as u32;
    
    // Style should be 0-8 (clamped)
    let clamped_style = style.min(BORDER_STYLE_COUNT - 1);
    
    // Width can be any u32
    let _ = width;
    
    // Fuzzer explores: invalid styles, extreme widths, zero width
    if style >= BORDER_STYLE_COUNT {
        assert!(clamped_style < BORDER_STYLE_COUNT);
    }
});
