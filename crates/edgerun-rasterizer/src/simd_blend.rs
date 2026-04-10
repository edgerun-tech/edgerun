//! SIMD alpha blending — auto-generated from CSS compositing spec.
//! DO NOT EDIT. Regenerate with: scripts/generate_rasterizer.py
#![cfg(target_arch = "x86_64")]

use core::arch::x86_64::*;

/// Fill 8 pixels with solid opaque color using AVX2.
#[target_feature(enable = "avx2")]
pub unsafe fn fill_solid_8_avx2(dst: *mut u32, color: u32) {
    let c = _mm256_set1_epi32(color as i32);
    _mm256_storeu_si256(dst as *mut __m256i, c);
}

/// Fill rectangle with solid color, 8 pixels per iteration.
#[target_feature(enable = "avx2")]
pub unsafe fn fill_rect_avx2(
    fb: *mut u32,
    stride: usize,
    x: usize, y: usize,
    w: usize, h: usize,
    color: u32,
) {
    let c = _mm256_set1_epi32(color as i32);
    let vec_count = w / 8;
    let rem = w % 8;

    for row in 0..h {
        let mut ptr = fb.add(y * stride + x + row * stride);
        for _ in 0..vec_count {
            _mm256_storeu_si256(ptr as *mut __m256i, c);
            ptr = ptr.add(8);
        }
        for _ in 0..rem {
            *ptr = color;
            ptr = ptr.add(1);
        }
    }
}

/// Alpha-blend 8 pixels: src over dst. Both ARGB32.
#[target_feature(enable = "avx2")]
pub unsafe fn blend_8_avx2(src: *const u32, dst: *mut u32) {
    let s = _mm256_loadu_si256(src as *const __m256i);
    let d = _mm256_loadu_si256(dst as *const __m256i);

    // Unpack to 16-bit for multiply
    let s_lo = _mm256_unpacklo_epi8(s, _mm256_setzero_si256());
    let s_hi = _mm256_unpackhi_epi8(s, _mm256_setzero_si256());
    let d_lo = _mm256_unpacklo_epi8(d, _mm256_setzero_si256());
    let d_hi = _mm256_unpackhi_epi8(d, _mm256_setzero_si256());

    // Extract alpha (high byte of each 16-bit lane)
    // alpha is in bits 8-15 of each 16-bit value
    let a_lo = _mm256_srli_epi16(s_lo, 8);
    let a_hi = _mm256_srli_epi16(s_hi, 8);

    // inv_alpha = 256 - alpha
    let inv_lo = _mm256_subs_epu16(_mm256_set1_epi16(256), a_lo);
    let inv_hi = _mm256_subs_epu16(_mm256_set1_epi16(256), a_hi);

    // result = (src * alpha + dst * inv_alpha) >> 8
    let sa_lo = _mm256_mullo_epi16(s_lo, a_lo);
    let sa_hi = _mm256_mullo_epi16(s_hi, a_hi);
    let da_lo = _mm256_mullo_epi16(d_lo, inv_lo);
    let da_hi = _mm256_mullo_epi16(d_hi, inv_hi);

    let sum_lo = _mm256_srli_epi16(_mm256_add_epi16(sa_lo, da_lo), 8);
    let sum_hi = _mm256_srli_epi16(_mm256_add_epi16(sa_hi, da_hi), 8);

    let result = _mm256_packus_epi16(sum_lo, sum_hi);
    _mm256_storeu_si256(dst as *mut __m256i, result);
}

/// Blend rectangle: src over dst.
#[target_feature(enable = "avx2")]
pub unsafe fn blend_rect_avx2(
    src: *const u32,
    dst: *mut u32,
    count: usize,
) {
    let vec_count = count / 8;
    let rem = count % 8;
    let mut s = src;
    let mut d = dst;

    for _ in 0..vec_count {
        blend_8_avx2(s, d);
        s = s.add(8);
        d = d.add(8);
    }

    // Scalar remainder
    for _ in 0..rem {
        let sp = *s;
        let dp = *d;
        let sa = (sp >> 24) as u32;
        if sa == 0 { /* transparent, skip */ }
        else if sa == 255 { *d = sp; }
        else {
            let inv = 256 - sa;
            let sr = sp & 0xFF; let sg = (sp >> 8) & 0xFF; let sb = (sp >> 16) & 0xFF;
            let dr = dp & 0xFF; let dg = (dp >> 8) & 0xFF; let db = (dp >> 16) & 0xFF;
            let r = (sr * sa + dr * inv) >> 8;
            let g = (sg * sa + dg * inv) >> 8;
            let b = (sb * sa + db * inv) >> 8;
            *d = r as u32 | (g as u32) << 8 | (b as u32) << 16;
        }
        s = s.add(1);
        d = d.add(1);
    }
}
