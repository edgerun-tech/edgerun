use std::ops::Range;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

static STATE: AtomicU64 = AtomicU64::new(0x9e37_79b9_7f4a_7c15);

/// Return a non-cryptographic random `u64`.
///
/// This is intended for jitter, temporary identifiers, and display choices. Do
/// not use it for secrets, authentication, or protocol nonces.
pub fn u64() -> u64 {
    let tick = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    let local = &tick as *const u64 as usize as u64;
    let pid = std::process::id() as u64;
    let state = STATE.fetch_add(0x9e37_79b9_7f4a_7c15, Ordering::Relaxed);

    splitmix64(state ^ tick.rotate_left(17) ^ local.rotate_left(31) ^ pid.rotate_left(7))
}

pub fn f64_range(range: Range<f64>) -> f64 {
    assert!(range.start < range.end, "empty random f64 range");
    let unit = (u64() >> 11) as f64 / ((1u64 << 53) as f64);
    range.start + ((range.end - range.start) * unit)
}

pub fn i32_range(range: Range<i32>) -> i32 {
    assert!(range.start < range.end, "empty random i32 range");
    let span = (range.end as i64 - range.start as i64) as u64;
    range.start + bounded_u64(span) as i32
}

pub fn usize_range(range: Range<usize>) -> usize {
    assert!(range.start < range.end, "empty random usize range");
    let span = range.end - range.start;
    range.start + bounded_u64(span as u64) as usize
}

pub fn choose<T>(items: &[T]) -> Option<&T> {
    if items.is_empty() {
        None
    } else {
        Some(&items[usize_range(0..items.len())])
    }
}

fn bounded_u64(bound: u64) -> u64 {
    debug_assert!(bound > 0);
    let zone = u64::MAX - (u64::MAX % bound);
    loop {
        let value = u64();
        if value < zone {
            return value % bound;
        }
    }
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_stay_inside_ranges() {
        for _ in 0..1000 {
            let f = f64_range(0.9..1.1);
            assert!((0.9..1.1).contains(&f));
            assert!((1_000..100_000).contains(&i32_range(1_000..100_000)));
            assert!((0..16).contains(&usize_range(0..16)));
        }
    }

    #[test]
    fn choose_handles_empty_and_non_empty_slices() {
        let values = ["a", "b", "c"];
        assert!(choose::<&str>(&[]).is_none());
        assert!(values.contains(choose(&values).expect("non-empty slice")));
    }
}
