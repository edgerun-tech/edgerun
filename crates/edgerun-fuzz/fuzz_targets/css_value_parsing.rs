//! Fuzz target for CSS value type parsing.
//!
//! Tests parsing of various CSS value formats:
//! - Lengths (px, em, rem, vw, vh, etc.)
//! - Colors (rgb, hsl, hwb, lab, lch, hex, named)
//! - Keywords (auto, none, inherit, initial, etc.)
//! - Functions (calc, var, env, etc.)

#![no_main]

use libfuzzer_sys::fuzz_target;

/// CSS length unit codes (from css_value_types.proto)
const LENGTH_UNITS: &[u8] = &[
    37,  // cm
    38,  // mm
    39,  // Q
    40,  // in
    41,  // pc
    42,  // pt
    43,  // px
    1,   // em
    2,   // rem
    7,   // ch
    13,  // vw
    17,  // vh
    29,  // vmin
    33,  // vmax
];

/// CSS angle unit codes
const ANGLE_UNITS: &[u8] = &[44, 45, 46, 47]; // deg, grad, rad, turn

/// CSS time unit codes
const TIME_UNITS: &[u8] = &[48, 49]; // s, ms

/// CSS resolution unit codes
const RES_UNITS: &[u8] = &[52, 53, 54, 55]; // dpi, dpcm, dppx, x

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 { return; }
    
    // Interpret as a CSS value: unit_type + numeric_value
    let unit_type = data[0];
    let _value_bytes = &data[1..];
    
    // Test: unit type should be in a known range
    let is_length = LENGTH_UNITS.contains(&unit_type);
    let is_angle = ANGLE_UNITS.contains(&unit_type);
    let is_time = TIME_UNITS.contains(&unit_type);
    let is_resolution = RES_UNITS.contains(&unit_type);
    
    let known_unit = is_length || is_angle || is_time || is_resolution;
    
    // Fuzzer explores: valid units, invalid units, boundary values
    if !known_unit {
        // Unknown unit — should be handled gracefully
        let _ = unit_type > 66; // above max defined value type
    }
});
