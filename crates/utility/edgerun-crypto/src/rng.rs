//! Random number generation owned by edgerun-crypto.
//!
//! Source order:
//! 1. registered platform source, intended for TPM-backed entropy
//! 2. CPU hardware random instructions
//! 3. Linux kernel randomness

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::error::{CryptoError, Result};
use crate::sha::Digest;

pub type RandomSource = fn(&mut [u8]) -> Result<()>;

const EXTERNAL_SOURCE_NONE: usize = 0;

static EXTERNAL_SOURCE: AtomicUsize = AtomicUsize::new(EXTERNAL_SOURCE_NONE);
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

impl OsRng {
    pub fn next_u32(&mut self) -> u32 {
        random_u32().unwrap_or(0)
    }

    pub fn next_u64(&mut self) -> u64 {
        random_u64().unwrap_or(0)
    }

    pub fn fill_bytes(&mut self, dest: &mut [u8]) {
        fill_random(dest).expect("edgerun secure random source unavailable");
    }

    pub fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<()> {
        fill_random(dest)
    }
}

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
    Digest::update(&mut hasher, b"edgerun-crypto rng mix v1");
    Digest::update(&mut hasher, state_bytes().as_slice());
    Digest::update(&mut hasher, entropy);
    Digest::update(
        &mut hasher,
        &DRBG_COUNTER.fetch_add(1, Ordering::AcqRel).to_le_bytes(),
    );
    store_state(hasher.finalize().as_slice());
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

pub fn random_u128() -> Result<u128> {
    let mut bytes = [0u8; 16];
    fill_random(&mut bytes)?;
    Ok(u128::from_le_bytes(bytes))
}

pub fn random_f64() -> Result<f64> {
    const SCALE: f64 = (1u64 << 53) as f64;
    Ok(((random_u64()? >> 11) as f64) / SCALE)
}

pub fn random_below_u64(upper: u64) -> Result<u64> {
    if upper == 0 {
        return Ok(0);
    }
    let zone = u64::MAX - (u64::MAX % upper);
    loop {
        let value = random_u64()?;
        if value < zone {
            return Ok(value % upper);
        }
    }
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
    if linux_random::fill_bytes(buf) {
        mix_entropy(buf);
        return Ok(());
    }
    Err(CryptoError::RandomGenerationFailed)
}

fn fill_from_external(buf: &mut [u8]) -> Result<()> {
    let source = EXTERNAL_SOURCE.load(Ordering::Acquire);
    if source == EXTERNAL_SOURCE_NONE {
        return Err(CryptoError::TpmUnavailable);
    }
    let source: RandomSource = unsafe { core::mem::transmute(source) };
    source(buf)
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
}

#[cfg(target_os = "linux")]
mod linux_random {
    const EINTR: isize = 4;

    pub fn fill_bytes(buf: &mut [u8]) -> bool {
        let mut filled = 0;
        while filled < buf.len() {
            let ret = unsafe { getrandom(buf[filled..].as_mut_ptr(), buf.len() - filled) };
            if ret > 0 {
                filled += ret as usize;
                continue;
            }
            if ret == -EINTR {
                continue;
            }
            return false;
        }
        true
    }

    #[cfg(target_arch = "x86_64")]
    unsafe fn getrandom(buf: *mut u8, len: usize) -> isize {
        const SYS_GETRANDOM: usize = 318;
        let ret: isize;
        unsafe {
            core::arch::asm!(
                "syscall",
                inlateout("rax") SYS_GETRANDOM => ret,
                in("rdi") buf,
                in("rsi") len,
                in("rdx") 0usize,
                lateout("rcx") _,
                lateout("r11") _,
                options(nostack, preserves_flags),
            );
        }
        ret
    }

    #[cfg(target_arch = "aarch64")]
    unsafe fn getrandom(buf: *mut u8, len: usize) -> isize {
        const SYS_GETRANDOM: usize = 278;
        let ret: isize;
        unsafe {
            core::arch::asm!(
                "svc 0",
                inlateout("x8") SYS_GETRANDOM => _,
                inlateout("x0") buf => ret,
                in("x1") len,
                in("x2") 0usize,
                options(nostack),
            );
        }
        ret
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    unsafe fn getrandom(_buf: *mut u8, _len: usize) -> isize {
        -1
    }
}

#[cfg(not(target_os = "linux"))]
mod linux_random {
    pub fn fill_bytes(_buf: &mut [u8]) -> bool {
        false
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
    fn os_rng_fills_bytes() {
        let mut rng = OsRng;
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        assert!(bytes.iter().any(|b| *b != 0));
    }
}
