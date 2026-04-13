//! HPACK conformance tests using the official http2jp/hpack-test-case suite.
//!
//! Source: https://github.com/http2jp/hpack-test-case
//!
//! The `raw-data/` stories define header sequences; the `nghttp2/` directory
//! contains the authoritative wire encoding produced by the reference
//! nghttp2 implementation.  These tests decode every wire byte sequence and
//! verify the resulting headers match the expected values — exercising the
//! full HPACK state machine including dynamic table management across all 32
//! stories (each with multiple header sets).

use std::fs;

use edgerun_json::from_str;

// Path to the cloned test-data repo (relative to workspace root).
const SPEC_DIR: &str = env!("CARGO_MANIFEST_DIR");

fn spec_path() -> std::path::PathBuf {
    // Walk up from crates/edgerun-http to workspace root, then into specs/
    let mut p = std::path::PathBuf::from(SPEC_DIR);
    p.push("../../specs/hpack-test-case");
    p
}

/// Decode hex wire data into bytes.
fn hex_decode(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

/// A single case within a story.
#[derive(serde::Deserialize, Debug)]
struct Case {
    seqno: usize,
    wire: String,
    headers: Vec<std::collections::HashMap<String, String>>,
}

/// A story file from nghttp2/.
#[derive(serde::Deserialize, Debug)]
struct Story {
    cases: Vec<Case>,
    description: String,
}

/// Run decoder conformance: feed nghttp2 wire data into our decoder and
/// verify the resulting headers match.
fn run_decoder_conformance() {
    let base = spec_path();
    let raw_base = base.join("raw-data");
    let nghttp2_base = base.join("nghttp2");

    if !nghttp2_base.is_dir() {
        eprintln!(
            "SKIP: HPACK conformance tests — specs/hpack-test-case not found at {:?}",
            nghttp2_base
        );
        return;
    }

    // Collect all story files.
    let mut stories: Vec<_> = fs::read_dir(&nghttp2_base)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext == "json")
        })
        .map(|e| e.path())
        .collect();
    stories.sort();

    let mut total_cases = 0usize;
    let mut total_stories = 0usize;
    let mut passed_cases = 0usize;
    let mut failed_cases = 0usize;

    for story_path in &stories {
        let raw = fs::read_to_string(story_path).unwrap();
        let story: Story = from_str(&raw).unwrap();
        total_stories += 1;

        // Create a fresh decoder for each story.
        let mut decoder = hpack_patched::Decoder::new();

        for case in &story.cases {
            total_cases += 1;
            let wire = hex_decode(&case.wire);
            let decoded = match decoder.decode(&wire) {
                Ok(d) => d,
                Err(e) => {
                    failed_cases += 1;
                    eprintln!(
                        "FAIL Story {:?} case {}: decode error: {:?}",
                        story_path.file_name().unwrap(),
                        case.seqno,
                        e
                    );
                    continue;
                }
            };

            // Build a lookup from the expected headers.
            let expected: std::collections::HashMap<_, _> =
                case.headers.iter().map(|m| {
                    let (k, v) = m.iter().next().unwrap();
                    (k.to_lowercase(), v.to_string())
                }).collect();

            let actual: std::collections::HashMap<_, _> =
                decoded.iter().map(|(k, v)| {
                    (String::from_utf8_lossy(k).to_lowercase(), String::from_utf8_lossy(v).to_string())
                }).collect();

            let mut case_passed = true;
            if actual.len() != expected.len() {
                eprintln!(
                    "FAIL Story {:?} case {}: expected {} headers, got {}",
                    story_path.file_name().unwrap(),
                    case.seqno,
                    expected.len(),
                    actual.len()
                );
                case_passed = false;
            }
            for (key, exp_val) in &expected {
                if actual.get(key) != Some(exp_val) {
                    eprintln!(
                        "FAIL Story {:?} case {}: header {:?} expected {:?}, got {:?}",
                        story_path.file_name().unwrap(),
                        case.seqno,
                        key,
                        exp_val,
                        actual.get(key)
                    );
                    case_passed = false;
                }
            }
            
            if case_passed {
                passed_cases += 1;
            } else {
                failed_cases += 1;
            }
        }
    }

    eprintln!(
        "HPACK conformance: {} stories, {} cases — {} passed, {} failed",
        total_stories, total_cases, passed_cases, failed_cases
    );
    
    assert_eq!(failed_cases, 0, "HPACK conformance test had {} failures", failed_cases);
}

#[test]
fn hpack_nghttp2_decoder_conformance() {
    run_decoder_conformance();
}

// ---------------------------------------------------------------------------
// HPACK Encoder Conformance
// ---------------------------------------------------------------------------

use crate::http2::hpack::{Decoder, Encoder};

/// A single case within a raw-data story (no wire field).
#[derive(serde::Deserialize, Debug)]
struct RawCase {
    headers: Vec<std::collections::HashMap<String, String>>,
}

/// A story file from raw-data/.
#[derive(serde::Deserialize, Debug)]
struct RawStory {
    cases: Vec<RawCase>,
    description: Option<String>,
}

/// Run encoder conformance: encode headers, decode with our own decoder,
/// and verify the round-trip produces identical headers.
fn run_encoder_conformance() {
    let base = spec_path();
    let raw_base = base.join("raw-data");

    if !raw_base.is_dir() {
        eprintln!(
            "SKIP: HPACK encoder conformance tests — raw-data not found at {:?}",
            raw_base
        );
        return;
    }

    // Collect all story files from raw-data.
    let mut stories: Vec<_> = fs::read_dir(&raw_base)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext == "json")
        })
        .map(|e| e.path())
        .collect();
    stories.sort();

    let mut total_cases = 0usize;
    let mut total_stories = 0usize;
    let mut passed_cases = 0usize;
    let mut failed_cases = 0usize;

    for story_path in &stories {
        let raw = fs::read_to_string(story_path).unwrap();
        let story: RawStory = from_str(&raw).unwrap();
        total_stories += 1;

        // Fresh encoder and decoder for each story.
        let mut encoder = Encoder::new();
        let mut decoder = Decoder::new();

        for (seqno, case) in story.cases.iter().enumerate() {
            total_cases += 1;

            // Build headers from the case data.
            let headers: Vec<(Vec<u8>, Vec<u8>)> = case.headers.iter().map(|m| {
                let (k, v) = m.iter().next().unwrap();
                (k.as_bytes().to_vec(), v.as_bytes().to_vec())
            }).collect();

            // Encode the headers.
            let encoded = encoder.encode(
                headers.iter().map(|(k, v)| (k.as_slice(), v.as_slice()))
            );

            // Decode the encoded bytes.
            let decoded = match decoder.decode(&encoded) {
                Ok(d) => d,
                Err(e) => {
                    failed_cases += 1;
                    eprintln!(
                        "FAIL encoder {:?} case {}: decode after encode error: {:?}",
                        story_path.file_name().unwrap(),
                        seqno,
                        e
                    );
                    continue;
                }
            };

            // Build lookup for comparison.
            let expected: std::collections::HashMap<_, _> =
                case.headers.iter().map(|m| {
                    let (k, v) = m.iter().next().unwrap();
                    (k.to_lowercase(), v.to_string())
                }).collect();

            let actual: std::collections::HashMap<_, _> =
                decoded.iter().map(|(k, v)| {
                    (String::from_utf8_lossy(k).to_lowercase(), String::from_utf8_lossy(v).to_string())
                }).collect();

            let mut case_passed = true;
            if actual.len() != expected.len() {
                eprintln!(
                    "FAIL encoder {:?} case {}: expected {} headers, got {}",
                    story_path.file_name().unwrap(),
                    seqno,
                    expected.len(),
                    actual.len()
                );
                case_passed = false;
            }
            for (key, exp_val) in &expected {
                if actual.get(key) != Some(exp_val) {
                    eprintln!(
                        "FAIL encoder {:?} case {}: header {:?} expected {:?}, got {:?}",
                        story_path.file_name().unwrap(),
                        seqno,
                        key,
                        exp_val,
                        actual.get(key)
                    );
                    case_passed = false;
                }
            }

            if case_passed {
                passed_cases += 1;
            } else {
                failed_cases += 1;
            }
        }
    }

    eprintln!(
        "HPACK encoder conformance: {} stories, {} cases — {} passed, {} failed",
        total_stories, total_cases, passed_cases, failed_cases
    );

    assert_eq!(failed_cases, 0, "HPACK encoder conformance test had {} failures", failed_cases);
}

#[test]
fn hpack_nghttp2_encoder_conformance() {
    run_encoder_conformance();
}
