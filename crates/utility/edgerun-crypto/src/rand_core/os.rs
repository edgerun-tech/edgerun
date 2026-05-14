// Copyright 2019 Developers of the Rand project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Interface to the random number generator of the operating system.

use crate::rand_core::{CryptoRng, Error, RngCore, impls};

/// A random number generator that retrieves randomness from the
/// operating system.
///
/// This is a zero-sized struct. It can be freely constructed with `OsRng`.
///
/// The implementation is provided by the centralized EdgeRun random source:
/// registered secure hardware, CPU hardware random instructions, then Linux
/// kernel randomness.
///
/// This struct is only available when specifying the crate feature `getrandom`
/// or `std`. When using the `rand` lib, it is also available as `rand::rngs::OsRng`.
///
/// # Blocking and error handling
///
/// It is possible that when used during early boot the first call to `OsRng`
/// will block until the system's RNG is initialised. It is also possible
/// (though highly unlikely) for `OsRng` to fail on some platforms, most
/// likely due to system mis-configuration.
///
/// After the first successful call, it is highly unlikely that failures or
/// significant delays will occur (although performance should be expected to
/// be much slower than a user-space PRNG).
///
/// # Usage example
/// ```ignore
/// use rand_core::{RngCore, OsRng};
///
/// let mut key = [0u8; 16];
/// OsRng.fill_bytes(&mut key);
/// let random_u64 = OsRng.next_u64();
/// ```ignore
///
#[derive(Clone, Copy, Debug, Default)]
pub struct OsRng;

impl CryptoRng for OsRng {}

impl RngCore for OsRng {
    fn next_u32(&mut self) -> u32 {
        impls::next_u32_via_fill(self)
    }

    fn next_u64(&mut self) -> u64 {
        impls::next_u64_via_fill(self)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        if let Err(e) = self.try_fill_bytes(dest) {
            panic!("Error: {}", e);
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        crate::rng::fill_random(dest).map_err(|_| Error::from_rng_failure())
    }
}

#[test]
fn test_os_rng() {
    let x = OsRng.next_u64();
    let y = OsRng.next_u64();
    assert!(x != 0);
    assert!(x != y);
}

#[test]
fn test_construction() {
    let mut rng = OsRng::default();
    assert!(rng.next_u64() != 0);
}
