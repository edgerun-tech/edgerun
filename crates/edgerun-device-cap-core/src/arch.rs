// SPDX-License-Identifier: Apache-2.0

use crate::{CapabilitySignal, CpuCapabilities, ProbeSource};

pub fn probe_cpu_capabilities() -> CpuCapabilities {
    probe_cpu_capabilities_impl()
}

#[cfg(target_arch = "x86_64")]
fn probe_cpu_capabilities_impl() -> CpuCapabilities {
    // SAFETY: CPUID is available on x86_64 targets.
    let leaf1 = unsafe { core::arch::x86_64::__cpuid(1) };
    // SAFETY: Leaf 7/subleaf 0 is a valid CPUID query.
    let leaf7 = unsafe { core::arch::x86_64::__cpuid_count(7, 0) };

    let sse2 = (leaf1.edx & (1 << 26)) != 0;
    let avx = (leaf1.ecx & (1 << 28)) != 0;
    let avx2 = (leaf7.ebx & (1 << 5)) != 0;
    let avx512f = (leaf7.ebx & (1 << 16)) != 0;
    let aes = (leaf1.ecx & (1 << 25)) != 0;
    let sha = (leaf7.ebx & (1 << 29)) != 0;
    let sse42 = (leaf1.ecx & (1 << 20)) != 0;
    let rdrand = (leaf1.ecx & (1 << 30)) != 0;
    let vmx = (leaf1.ecx & (1 << 5)) != 0;

    CpuCapabilities {
        simd128: bool_signal(sse2, ProbeSource::Runtime),
        simd256: bool_signal(avx && avx2, ProbeSource::Runtime),
        simd512: bool_signal(avx512f, ProbeSource::Runtime),
        aes: bool_signal(aes, ProbeSource::Runtime),
        sha2: bool_signal(sha, ProbeSource::Runtime),
        crc32c: bool_signal(sse42, ProbeSource::Runtime),
        random: bool_signal(rdrand, ProbeSource::Runtime),
        virtualization: bool_signal(vmx, ProbeSource::Runtime),
        atomics_64: CapabilitySignal::supported(ProbeSource::CompileTime),
    }
}

#[cfg(target_arch = "aarch64")]
fn probe_cpu_capabilities_impl() -> CpuCapabilities {
    let neon = cfg!(target_feature = "neon");
    let sve = cfg!(target_feature = "sve");
    let sve2 = cfg!(target_feature = "sve2");
    let aes = cfg!(target_feature = "aes");
    let sha2 = cfg!(target_feature = "sha2");
    let crc = cfg!(target_feature = "crc");
    let rand = cfg!(target_feature = "rand");

    CpuCapabilities {
        simd128: bool_signal(neon, ProbeSource::CompileTime),
        simd256: bool_signal(sve || sve2, ProbeSource::CompileTime),
        simd512: bool_signal(sve2, ProbeSource::CompileTime),
        aes: bool_signal(aes, ProbeSource::CompileTime),
        sha2: bool_signal(sha2, ProbeSource::CompileTime),
        crc32c: bool_signal(crc, ProbeSource::CompileTime),
        random: bool_signal(rand, ProbeSource::CompileTime),
        virtualization: CapabilitySignal::unknown(),
        atomics_64: CapabilitySignal::supported(ProbeSource::CompileTime),
    }
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn probe_cpu_capabilities_impl() -> CpuCapabilities {
    CpuCapabilities {
        simd128: CapabilitySignal::unknown(),
        simd256: CapabilitySignal::unknown(),
        simd512: CapabilitySignal::unknown(),
        aes: CapabilitySignal::unknown(),
        sha2: CapabilitySignal::unknown(),
        crc32c: CapabilitySignal::unknown(),
        random: CapabilitySignal::unknown(),
        virtualization: CapabilitySignal::unknown(),
        atomics_64: if cfg!(target_has_atomic = "64") {
            CapabilitySignal::supported(ProbeSource::CompileTime)
        } else {
            CapabilitySignal::unsupported(ProbeSource::CompileTime)
        },
    }
}

const fn bool_signal(value: bool, source: ProbeSource) -> CapabilitySignal {
    if value {
        CapabilitySignal::supported(source)
    } else {
        CapabilitySignal::unsupported(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_returns_deterministic_shape() {
        let report = probe_cpu_capabilities();

        let _ = report.simd128;
        let _ = report.simd256;
        let _ = report.simd512;
        let _ = report.aes;
        let _ = report.sha2;
        let _ = report.crc32c;
        let _ = report.random;
        let _ = report.virtualization;
        let _ = report.atomics_64;
    }
}
