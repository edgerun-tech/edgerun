#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT_DIR="$ROOT/build/wasm/app-primitives/tor-hs-pow-v1"
WORK_DIR="${TMPDIR:-/tmp}/edgerun-tor-hs-pow-v1.$$"
mkdir -p "$OUT_DIR" "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT

cat > "$WORK_DIR/Cargo.toml" <<'TOML'
[package]
name = "tor-hs-pow-v1"
version = "0.1.0"
edition = "2021"

[lib]
name = "tor_hs_pow_v1"
crate-type = ["cdylib"]

[dependencies]
blake2 = { version = "0.10", default-features = false }
equix = { version = "0.6.2", default-features = false }
TOML

mkdir -p "$WORK_DIR/src"
cat > "$WORK_DIR/src/lib.rs" <<'RS'
use blake2::digest::{Update, VariableOutput};
use blake2::Blake2bVar;
use equix::{EquiXBuilder, RuntimeOption};

const STATUS_OK: i32 = 0;
const STATUS_INVALID: i32 = -1;
const STATUS_BOUNDS: i32 = -2;
const STATUS_AUTH: i32 = -3;
const STATUS_UNSUPPORTED: i32 = -4;
const STATUS_REPLAY: i32 = -5;
const STATUS_NO_SOLUTION: i32 = -6;

const PERSONALIZATION: &[u8; 16] = b"Tor hs intro v1\0";
const CHALLENGE_LEN: usize = 16 + 32 + 32 + 16 + 4;
const SOLUTION_LEN: usize = 16;
const REPLAY_SLOTS: usize = 256;

static mut REPLAY_NONCES: [[u8; 16]; REPLAY_SLOTS] = [[0; 16]; REPLAY_SLOTS];
static mut REPLAY_SEEDS: [[u8; 4]; REPLAY_SLOTS] = [[0; 4]; REPLAY_SLOTS];
static mut REPLAY_USED: [u8; REPLAY_SLOTS] = [0; REPLAY_SLOTS];
static mut REPLAY_CURSOR: usize = 0;
static mut REPLAY_COUNT: usize = 0;

fn put32be(out: &mut [u8], v: u32) {
    out[0] = ((v >> 24) & 0xff) as u8;
    out[1] = ((v >> 16) & 0xff) as u8;
    out[2] = ((v >> 8) & 0xff) as u8;
    out[3] = (v & 0xff) as u8;
}

fn get32be(s: &[u8]) -> u32 {
    ((s[0] as u32) << 24) | ((s[1] as u32) << 16) | ((s[2] as u32) << 8) | (s[3] as u32)
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut acc = 0u8;
    for i in 0..a.len() {
        acc |= a[i] ^ b[i];
    }
    acc == 0
}

fn challenge(identity: &[u8], seed: &[u8], nonce: &[u8], effort: u32, out: &mut [u8]) {
    out[0..16].copy_from_slice(PERSONALIZATION);
    out[16..48].copy_from_slice(&identity[0..32]);
    out[48..80].copy_from_slice(&seed[0..32]);
    out[80..96].copy_from_slice(&nonce[0..16]);
    put32be(&mut out[96..100], effort);
}

fn blake2b32_be(input: &[u8]) -> Result<u32, i32> {
    let mut h = Blake2bVar::new(4).map_err(|_| STATUS_INVALID)?;
    h.update(input);
    let mut out = [0u8; 4];
    h.finalize_variable(&mut out).map_err(|_| STATUS_INVALID)?;
    Ok(get32be(&out))
}

fn equix_verify(challenge: &[u8], solution: &[u8]) -> i32 {
    if solution.len() != SOLUTION_LEN {
        return STATUS_INVALID;
    }
    let mut sol = [0u8; SOLUTION_LEN];
    sol.copy_from_slice(solution);
    let mut builder = EquiXBuilder::new();
    builder.runtime(RuntimeOption::InterpretOnly);
    match builder.verify_bytes(challenge, &sol) {
        Ok(()) => STATUS_OK,
        Err(_) => STATUS_AUTH,
    }
}

fn replay_seen(seed_prefix: &[u8], nonce: &[u8]) -> bool {
    unsafe {
        let mut i = 0;
        while i < REPLAY_SLOTS {
            if REPLAY_USED[i] != 0 && ct_eq(&REPLAY_SEEDS[i], seed_prefix) && ct_eq(&REPLAY_NONCES[i], nonce) {
                return true;
            }
            i += 1;
        }
        false
    }
}

fn replay_remember(seed_prefix: &[u8], nonce: &[u8]) {
    unsafe {
        let slot = REPLAY_CURSOR % REPLAY_SLOTS;
        REPLAY_SEEDS[slot].copy_from_slice(&seed_prefix[0..4]);
        REPLAY_NONCES[slot].copy_from_slice(&nonce[0..16]);
        REPLAY_USED[slot] = 1;
        REPLAY_CURSOR = (REPLAY_CURSOR + 1) % REPLAY_SLOTS;
        if REPLAY_COUNT < REPLAY_SLOTS {
            REPLAY_COUNT += 1;
        }
    }
}

#[no_mangle]
pub extern "C" fn proto_standard_id() -> u32 {
    300219
}

#[no_mangle]
pub extern "C" fn proto_abi_version() -> u32 {
    1
}

#[no_mangle]
pub extern "C" fn tor_hs_pow_v1_challenge_len() -> u32 {
    CHALLENGE_LEN as u32
}

#[no_mangle]
pub extern "C" fn tor_hs_pow_v1_solution_len() -> u32 {
    SOLUTION_LEN as u32
}

#[no_mangle]
pub extern "C" fn tor_hs_pow_v1_replay_capacity() -> u32 {
    REPLAY_SLOTS as u32
}

#[no_mangle]
pub extern "C" fn tor_hs_pow_v1_replay_reset() -> i32 {
    unsafe {
        REPLAY_USED = [0; REPLAY_SLOTS];
        REPLAY_CURSOR = 0;
        REPLAY_COUNT = 0;
    }
    STATUS_OK
}

#[no_mangle]
pub extern "C" fn tor_hs_pow_v1_replay_count() -> u32 {
    unsafe { REPLAY_COUNT as u32 }
}

#[no_mangle]
pub unsafe extern "C" fn tor_hs_pow_v1_challenge(
    identity32: *const u8,
    seed32: *const u8,
    nonce16: *const u8,
    effort: u32,
    out_challenge: *mut u8,
) -> i32 {
    if identity32.is_null() || seed32.is_null() || nonce16.is_null() || out_challenge.is_null() {
        return STATUS_INVALID;
    }
    let identity = core::slice::from_raw_parts(identity32, 32);
    let seed = core::slice::from_raw_parts(seed32, 32);
    let nonce = core::slice::from_raw_parts(nonce16, 16);
    let out = core::slice::from_raw_parts_mut(out_challenge, CHALLENGE_LEN);
    challenge(identity, seed, nonce, effort, out);
    CHALLENGE_LEN as i32
}

#[no_mangle]
pub unsafe extern "C" fn tor_hs_pow_v1_equix_verify(
    challenge_ptr: *const u8,
    challenge_len: u32,
    solution16: *const u8,
) -> i32 {
    if challenge_ptr.is_null() || solution16.is_null() {
        return STATUS_INVALID;
    }
    if challenge_len as usize != CHALLENGE_LEN {
        return STATUS_BOUNDS;
    }
    let c = core::slice::from_raw_parts(challenge_ptr, CHALLENGE_LEN);
    let s = core::slice::from_raw_parts(solution16, SOLUTION_LEN);
    equix_verify(c, s)
}

#[no_mangle]
pub unsafe extern "C" fn tor_hs_pow_v1_blake2b_result(
    challenge_ptr: *const u8,
    challenge_len: u32,
    solution16: *const u8,
    out_r: *mut u32,
) -> i32 {
    if challenge_ptr.is_null() || solution16.is_null() || out_r.is_null() {
        return STATUS_INVALID;
    }
    if challenge_len as usize != CHALLENGE_LEN {
        return STATUS_BOUNDS;
    }
    let c = core::slice::from_raw_parts(challenge_ptr, CHALLENGE_LEN);
    let s = core::slice::from_raw_parts(solution16, SOLUTION_LEN);
    let mut input = [0u8; CHALLENGE_LEN + SOLUTION_LEN];
    input[0..CHALLENGE_LEN].copy_from_slice(c);
    input[CHALLENGE_LEN..].copy_from_slice(s);
    match blake2b32_be(&input) {
        Ok(r) => {
            *out_r = r;
            STATUS_OK
        }
        Err(rc) => rc,
    }
}

#[no_mangle]
pub unsafe extern "C" fn tor_hs_pow_v1_verify(
    identity32: *const u8,
    seed32: *const u8,
    nonce16: *const u8,
    effort: u32,
    seed_prefix4: *const u8,
    solution16: *const u8,
    out_r: *mut u32,
) -> i32 {
    if identity32.is_null() || seed32.is_null() || nonce16.is_null() || seed_prefix4.is_null() || solution16.is_null() || out_r.is_null() {
        return STATUS_INVALID;
    }
    let identity = core::slice::from_raw_parts(identity32, 32);
    let seed = core::slice::from_raw_parts(seed32, 32);
    let nonce = core::slice::from_raw_parts(nonce16, 16);
    let prefix = core::slice::from_raw_parts(seed_prefix4, 4);
    let solution = core::slice::from_raw_parts(solution16, SOLUTION_LEN);
    if !ct_eq(&seed[0..4], prefix) {
        return STATUS_AUTH;
    }
    if replay_seen(prefix, nonce) {
        return STATUS_REPLAY;
    }
    let mut c = [0u8; CHALLENGE_LEN];
    challenge(identity, seed, nonce, effort, &mut c);
    let mut input = [0u8; CHALLENGE_LEN + SOLUTION_LEN];
    input[0..CHALLENGE_LEN].copy_from_slice(&c);
    input[CHALLENGE_LEN..].copy_from_slice(solution);
    let r = match blake2b32_be(&input) {
        Ok(r) => r,
        Err(rc) => return rc,
    };
    *out_r = r;
    if (r as u64) * (effort as u64) > 0xffff_ffff {
        return STATUS_AUTH;
    }
    let rc = equix_verify(&c, solution);
    if rc != STATUS_OK {
        return rc;
    }
    replay_remember(prefix, nonce);
    STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn tor_hs_pow_v1_solve_first(
    identity32: *const u8,
    seed32: *const u8,
    nonce16: *const u8,
    effort: u32,
    out_solution16: *mut u8,
    out_r: *mut u32,
) -> i32 {
    if identity32.is_null() || seed32.is_null() || nonce16.is_null() || out_solution16.is_null() || out_r.is_null() {
        return STATUS_INVALID;
    }
    let identity = core::slice::from_raw_parts(identity32, 32);
    let seed = core::slice::from_raw_parts(seed32, 32);
    let nonce = core::slice::from_raw_parts(nonce16, 16);
    let mut c = [0u8; CHALLENGE_LEN];
    challenge(identity, seed, nonce, effort, &mut c);
    let mut builder = EquiXBuilder::new();
    builder.runtime(RuntimeOption::InterpretOnly);
    let instance = match builder.build(&c) {
        Ok(i) => i,
        Err(_) => return STATUS_UNSUPPORTED,
    };
    for solution in instance.solve().iter() {
        let bytes = solution.to_bytes();
        let mut input = [0u8; CHALLENGE_LEN + SOLUTION_LEN];
        input[0..CHALLENGE_LEN].copy_from_slice(&c);
        input[CHALLENGE_LEN..].copy_from_slice(&bytes);
        let r = match blake2b32_be(&input) {
            Ok(r) => r,
            Err(rc) => return rc,
        };
        if (r as u64) * (effort as u64) <= 0xffff_ffff {
            core::slice::from_raw_parts_mut(out_solution16, SOLUTION_LEN).copy_from_slice(&bytes);
            *out_r = r;
            return STATUS_OK;
        }
    }
    STATUS_NO_SOLUTION
}
RS

CARGO_TARGET_DIR="$WORK_DIR/target" cargo build --manifest-path "$WORK_DIR/Cargo.toml" --target wasm32-unknown-unknown --release
wasm2wat "$WORK_DIR/target/wasm32-unknown-unknown/release/tor_hs_pow_v1.wasm" -o "$OUT_DIR/tor-hs-pow-v1.wat"
wat2wasm "$OUT_DIR/tor-hs-pow-v1.wat" -o "$WORK_DIR/roundtrip.wasm"
wasm-validate "$WORK_DIR/roundtrip.wasm"
echo "wrote $OUT_DIR/tor-hs-pow-v1.wat"
