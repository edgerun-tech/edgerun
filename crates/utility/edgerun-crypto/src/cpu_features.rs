#[macro_export]
macro_rules! cpu_feature_new {
    ($mod_name:ident, $($feature:literal),+ $(,)?) => {
        mod $mod_name {
            use core::sync::atomic::{AtomicU8, Ordering::Relaxed};

            const UNINIT: u8 = u8::MAX;
            static STORAGE: AtomicU8 = AtomicU8::new(UNINIT);

            #[derive(Copy, Clone, Debug)]
            pub struct InitToken(());

            impl InitToken {
                #[inline(always)]
                pub fn get(&self) -> bool {
                    get()
                }
            }

            #[inline]
            pub fn init_get() -> (InitToken, bool) {
                let value = STORAGE.load(Relaxed);
                let supported = if value == UNINIT {
                    let detected = $crate::cpu_features::detect_all(&[$($feature),+]);
                    STORAGE.store(detected as u8, Relaxed);
                    detected
                } else {
                    value == 1
                };
                (InitToken(()), supported)
            }

            #[inline]
            pub fn init() -> InitToken {
                init_get().0
            }

            #[inline]
            pub fn get() -> bool {
                init_get().1
            }
        }
    };
}

pub fn detect_all(features: &[&str]) -> bool {
    features.iter().all(|feature| detect(feature))
}

#[cfg(all(
    any(target_arch = "x86", target_arch = "x86_64"),
    any(target_env = "sgx", target_os = "none", target_os = "uefi")
))]
fn detect(_feature: &str) -> bool {
    false
}

#[cfg(all(
    any(target_arch = "x86", target_arch = "x86_64"),
    not(any(target_env = "sgx", target_os = "none", target_os = "uefi"))
))]
fn detect(feature: &str) -> bool {
    #[cfg(target_arch = "x86")]
    use core::arch::x86::{__cpuid, __cpuid_count};
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::{__cpuid, __cpuid_count};

    #[cfg(target_arch = "x86")]
    use core::arch::x86 as arch;
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64 as arch;

    let leaf1 = __cpuid(1);
    let leaf7 = __cpuid_count(7, 0);

    let xmm = || {
        let mask = 0b11 << 26;
        if leaf1.ecx & mask != mask {
            return false;
        }
        unsafe { arch::_xgetbv(arch::_XCR_XFEATURE_ENABLED_MASK) & 0b10 == 0b10 }
    };
    let ymm = || {
        let mask = 0b11 << 26;
        if leaf1.ecx & mask != mask {
            return false;
        }
        unsafe { arch::_xgetbv(arch::_XCR_XFEATURE_ENABLED_MASK) & 0b110 == 0b110 }
    };
    let zmm = || {
        let mask = 0b11 << 26;
        if leaf1.ecx & mask != mask {
            return false;
        }
        unsafe { arch::_xgetbv(arch::_XCR_XFEATURE_ENABLED_MASK) & 0b1110_0110 == 0b1110_0110 }
    };

    match feature {
        "sha" => leaf7.ebx & (1 << 29) != 0,
        "sse2" => leaf1.edx & (1 << 26) != 0,
        "ssse3" => leaf1.ecx & (1 << 9) != 0,
        "sse4.1" => leaf1.ecx & (1 << 19) != 0,
        "avx2" => ymm() && leaf7.ebx & (1 << 5) != 0 && leaf1.ecx & (1 << 28) != 0,
        "avx512ifma" => zmm() && leaf7.ebx & (1 << 21) != 0,
        "avx512vl" => zmm() && leaf7.ebx & (1 << 31) != 0,
        "avx" => xmm() && leaf1.ecx & (1 << 28) != 0,
        _ => false,
    }
}

#[cfg(target_arch = "aarch64")]
fn detect(feature: &str) -> bool {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        const AT_HWCAP: usize = 16;
        const HWCAP_SHA2: usize = 1 << 6;
        const HWCAP_SHA3: usize = 1 << 17;
        const HWCAP_SHA512: usize = 1 << 21;

        unsafe extern "C" {
            fn getauxval(ty: usize) -> usize;
        }

        let hwcap = unsafe { getauxval(AT_HWCAP) };
        return match feature {
            "sha2" => hwcap & HWCAP_SHA2 != 0,
            "sha3" => hwcap & (HWCAP_SHA3 | HWCAP_SHA512) == (HWCAP_SHA3 | HWCAP_SHA512),
            _ => false,
        };
    }

    #[cfg(target_vendor = "apple")]
    {
        unsafe extern "C" {
            fn sysctlbyname(
                name: *const i8,
                oldp: *mut core::ffi::c_void,
                oldlenp: *mut usize,
                newp: *mut core::ffi::c_void,
                newlen: usize,
            ) -> i32;
        }

        unsafe fn sysctl_bool(name: &[u8]) -> bool {
            let mut value: u32 = 0;
            let mut size = core::mem::size_of::<u32>();
            sysctlbyname(
                name.as_ptr().cast(),
                (&mut value as *mut u32).cast(),
                &mut size,
                core::ptr::null_mut(),
                0,
            ) == 0
                && value != 0
        }

        return match feature {
            "sha2" => true,
            "sha3" => unsafe {
                sysctl_bool(b"hw.optional.armv8_2_sha512\0")
                    && sysctl_bool(b"hw.optional.armv8_2_sha3\0")
            },
            _ => false,
        };
    }

    #[cfg(not(any(target_vendor = "apple", target_os = "linux", target_os = "android")))]
    {
        false
    }
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86", target_arch = "x86_64")))]
fn detect(_: &str) -> bool {
    false
}
