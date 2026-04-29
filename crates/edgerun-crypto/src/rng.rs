//! Random number generation owned by edgerun-crypto.
//!
//! Source order:
//! 1. registered platform source, intended for TPM-backed entropy
//! 2. CPU hardware random instructions
//! 3. no_std SHA-256 counter DRBG seeded from mixed entropy when available

use core::num::NonZeroU32;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rand_core::{CryptoRng, Error as RandError, RngCore};
use sha2::Digest;

use crate::error::{CryptoError, Result};

pub type RandomSource = fn(&mut [u8]) -> Result<()>;

const RNG_ERROR_CODE: u32 = RandError::CUSTOM_START + 1;
const EXTERNAL_SOURCE_NONE: usize = 0;

static EXTERNAL_SOURCE: AtomicUsize = AtomicUsize::new(EXTERNAL_SOURCE_NONE);
static DRBG_INITIALIZED: AtomicBool = AtomicBool::new(false);
static DRBG_COUNTER: AtomicUsize = AtomicUsize::new(1);
static DRBG_STATE: [AtomicUsize; 8] = [
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
];

#[derive(Clone, Copy, Debug, Default)]
pub struct OsRng;

impl RngCore for OsRng {
    fn next_u32(&mut self) -> u32 {
        random_u32().unwrap_or(0)
    }

    fn next_u64(&mut self) -> u64 {
        random_u64().unwrap_or(0)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        let _ = fill_random(dest);
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> core::result::Result<(), RandError> {
        fill_random(dest).map_err(|_| rand_error())
    }
}

impl CryptoRng for OsRng {}

pub fn register_random_source(source: RandomSource) {
    EXTERNAL_SOURCE.store(source as usize, Ordering::Release);
}

pub fn unregister_random_source() {
    EXTERNAL_SOURCE.store(EXTERNAL_SOURCE_NONE, Ordering::Release);
}

pub fn mix_entropy(entropy: &[u8]) {
    if entropy.is_empty() {
        return;
    }
    let mut hasher = crate::sha::Sha256::new();
    sha2::Digest::update(&mut hasher, b"edgerun-crypto rng mix v1");
    sha2::Digest::update(&mut hasher, state_bytes().as_slice());
    sha2::Digest::update(&mut hasher, entropy);
    sha2::Digest::update(
        &mut hasher,
        &DRBG_COUNTER.fetch_add(1, Ordering::AcqRel).to_le_bytes(),
    );
    store_state(hasher.finalize().as_slice());
    DRBG_INITIALIZED.store(true, Ordering::Release);
}

pub fn random_bytes(len: usize) -> Result<alloc::vec::Vec<u8>> {
    let mut buf = alloc::vec::Vec::with_capacity(len);
    buf.resize(len, 0);
    fill_random(&mut buf)?;
    Ok(buf)
}

pub fn random_u32() -> Result<u32> {
    let mut bytes = [0u8; 4];
    fill_random(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

pub fn random_u64() -> Result<u64> {
    let mut bytes = [0u8; 8];
    fill_random(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

#[inline]
pub fn fill_random(buf: &mut [u8]) -> Result<()> {
    if buf.is_empty() {
        return Ok(());
    }
    if fill_from_external(buf).is_ok() {
        mix_entropy(buf);
        return Ok(());
    }
    if cpu_random::fill_bytes(buf) {
        mix_entropy(buf);
        return Ok(());
    }
    software_fill(buf);
    Ok(())
}

fn fill_from_external(buf: &mut [u8]) -> Result<()> {
    let source = EXTERNAL_SOURCE.load(Ordering::Acquire);
    if source == EXTERNAL_SOURCE_NONE {
        return Err(CryptoError::TpmUnavailable);
    }
    let source: RandomSource = unsafe { core::mem::transmute(source) };
    source(buf)
}

fn software_fill(out: &mut [u8]) {
    ensure_drbg_initialized();
    let mut offset = 0;
    while offset < out.len() {
        let block = next_drbg_block();
        let n = core::cmp::min(block.len(), out.len() - offset);
        out[offset..offset + n].copy_from_slice(&block[..n]);
        offset += n;
    }
}

fn ensure_drbg_initialized() {
    if DRBG_INITIALIZED.load(Ordering::Acquire) {
        return;
    }

    let mut seed = [0u8; 32];
    if !cpu_random::fill_bytes(&mut seed) {
        let counter = DRBG_COUNTER.fetch_add(1, Ordering::AcqRel);
        let stack_addr = (&seed as *const [u8; 32] as usize as u64).to_le_bytes();
        let mut hasher = crate::sha::Sha256::new();
        sha2::Digest::update(&mut hasher, b"edgerun-crypto software rng bootstrap v1");
        sha2::Digest::update(&mut hasher, &counter.to_le_bytes());
        sha2::Digest::update(&mut hasher, &stack_addr);
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        sha2::Digest::update(&mut hasher, &cpu_random::timestamp_counter().to_le_bytes());
        seed.copy_from_slice(hasher.finalize().as_slice());
    }
    store_state(&seed);
    DRBG_INITIALIZED.store(true, Ordering::Release);
}

fn next_drbg_block() -> [u8; 32] {
    let counter = DRBG_COUNTER.fetch_add(1, Ordering::AcqRel);
    let mut hasher = crate::sha::Sha256::new();
    sha2::Digest::update(&mut hasher, b"edgerun-crypto software rng generate v1");
    sha2::Digest::update(&mut hasher, state_bytes().as_slice());
    sha2::Digest::update(&mut hasher, &counter.to_le_bytes());
    let block: [u8; 32] = hasher.finalize().into();

    let mut update = crate::sha::Sha256::new();
    sha2::Digest::update(&mut update, b"edgerun-crypto software rng update v1");
    sha2::Digest::update(&mut update, state_bytes().as_slice());
    sha2::Digest::update(&mut update, &block);
    sha2::Digest::update(&mut update, &counter.to_le_bytes());
    store_state(update.finalize().as_slice());
    block
}

fn state_bytes() -> [u8; 32] {
    let mut out = [0u8; 32];
    for (chunk, word) in out
        .chunks_mut(core::mem::size_of::<usize>())
        .zip(DRBG_STATE.iter())
    {
        chunk.copy_from_slice(&word.load(Ordering::Acquire).to_le_bytes());
    }
    out
}

fn store_state(bytes: &[u8]) {
    debug_assert!(bytes.len() >= 32);
    for (chunk, word) in bytes[..32]
        .chunks(core::mem::size_of::<usize>())
        .zip(DRBG_STATE.iter())
    {
        let mut word_bytes = [0u8; core::mem::size_of::<usize>()];
        word_bytes.copy_from_slice(chunk);
        word.store(usize::from_le_bytes(word_bytes), Ordering::Release);
    }
}

fn rand_error() -> RandError {
    RandError::from(NonZeroU32::new(RNG_ERROR_CODE).unwrap())
}

#[cfg(target_arch = "x86_64")]
mod cpu_random {
    use core::arch::x86_64::{__cpuid, __cpuid_count, _rdrand64_step, _rdseed64_step, _rdtsc};

    pub fn fill_bytes(buf: &mut [u8]) -> bool {
        if fill_with_rdseed(buf) {
            return true;
        }
        fill_with_rdrand(buf)
    }

    pub fn timestamp_counter() -> u64 {
        unsafe { _rdtsc() }
    }

    fn fill_with_rdseed(buf: &mut [u8]) -> bool {
        if !has_rdseed() {
            return false;
        }
        fill_words(buf, rdseed_u64)
    }

    fn fill_with_rdrand(buf: &mut [u8]) -> bool {
        if !has_rdrand() {
            return false;
        }
        fill_words(buf, rdrand_u64)
    }

    fn fill_words(buf: &mut [u8], mut next: impl FnMut() -> Option<u64>) -> bool {
        let mut offset = 0;
        while offset < buf.len() {
            let Some(word) = next() else {
                return false;
            };
            let bytes = word.to_le_bytes();
            let n = core::cmp::min(bytes.len(), buf.len() - offset);
            buf[offset..offset + n].copy_from_slice(&bytes[..n]);
            offset += n;
        }
        true
    }

    fn rdseed_u64() -> Option<u64> {
        for _ in 0..32 {
            let mut value = 0u64;
            if unsafe { _rdseed64_step(&mut value) } == 1 {
                return Some(value);
            }
        }
        None
    }

    fn rdrand_u64() -> Option<u64> {
        for _ in 0..16 {
            let mut value = 0u64;
            if unsafe { _rdrand64_step(&mut value) } == 1 {
                return Some(value);
            }
        }
        None
    }

    fn has_rdrand() -> bool {
        (__cpuid(1).ecx & (1 << 30)) != 0
    }

    fn has_rdseed() -> bool {
        let max_leaf = __cpuid(0).eax;
        max_leaf >= 7 && (__cpuid_count(7, 0).ebx & (1 << 18)) != 0
    }
}

#[cfg(not(target_arch = "x86_64"))]
mod cpu_random {
    pub fn fill_bytes(_buf: &mut [u8]) -> bool {
        false
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    pub fn timestamp_counter() -> u64 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_source(out: &mut [u8]) -> Result<()> {
        out.fill(0xA5);
        Ok(())
    }

    #[test]
    fn registered_source_takes_priority() {
        register_random_source(fixed_source);
        let mut bytes = [0u8; 16];
        fill_random(&mut bytes).unwrap();
        unregister_random_source();
        assert_eq!(bytes, [0xA5; 16]);
    }

    #[test]
    fn os_rng_implements_rand_core() {
        let mut rng = OsRng;
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        assert!(bytes.iter().any(|b| *b != 0));
    }
}
