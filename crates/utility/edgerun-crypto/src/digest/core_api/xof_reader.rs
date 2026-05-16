use super::{AlgorithmName, XofReaderCore};
use crate::digest::XofReader;
use crate::digest::block_buffer::EagerBuffer;
use crate::digest::crypto_common::typenum::{IsLess, Le, NonZero, U256};
use core::fmt;

/// Wrapper around [`XofReaderCore`] implementations.
///
/// It handles data buffering and implements the mid-level traits.
#[derive(Clone, Default)]
pub struct XofReaderCoreWrapper<T>
where
    T: XofReaderCore,
    T::BlockSize: IsLess<U256>,
    Le<T::BlockSize, U256>: NonZero,
{
    pub(super) core: T,
    pub(super) buffer: EagerBuffer<T::BlockSize>,
}

impl<T> fmt::Debug for XofReaderCoreWrapper<T>
where
    T: XofReaderCore + AlgorithmName,
    T::BlockSize: IsLess<U256>,
    Le<T::BlockSize, U256>: NonZero,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        T::write_alg_name(f)?;
        f.write_str(" { .. }")
    }
}

impl<T> XofReader for XofReaderCoreWrapper<T>
where
    T: XofReaderCore,
    T::BlockSize: IsLess<U256>,
    Le<T::BlockSize, U256>: NonZero,
{
    #[inline]
    fn read(&mut self, buffer: &mut [u8]) {
        let Self { core, buffer: buf } = self;
        buf.set_data(buffer, |blocks| {
            for block in blocks {
                *block = core.read_block();
            }
        });
    }
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl<T> std::io::Read for XofReaderCoreWrapper<T>
where
    T: XofReaderCore,
    T::BlockSize: IsLess<U256>,
    Le<T::BlockSize, U256>: NonZero,
{
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        XofReader::read(self, buf);
        Ok(buf.len())
    }
}
