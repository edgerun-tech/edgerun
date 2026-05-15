//! `expand_message_xof` for the `ExpandMsg` trait

use super::{Domain, ExpandMsg, Expander};
use crate::digest::{ExtendableOutput, Update, XofReader};
use crate::elliptic_curve::{Error, Result};
use generic_array::typenum::U32;

/// Placeholder type for implementing `expand_message_xof` based on an extendable output function
///
/// # Errors
/// - `dst.is_empty()`
/// - `len_in_bytes == 0`
/// - `len_in_bytes > u16::MAX`
pub struct ExpandMsgXof<HashT>
where
    HashT: Default + ExtendableOutput + Update,
{
    reader: <HashT as ExtendableOutput>::Reader,
}

/// ExpandMsgXof implements `expand_message_xof` for the [`ExpandMsg`] trait
impl<'a, HashT> ExpandMsg<'a> for ExpandMsgXof<HashT>
where
    HashT: Default + ExtendableOutput + Update,
{
    type Expander = Self;

    fn expand_message(
        msgs: &[&[u8]],
        dsts: &'a [&'a [u8]],
        len_in_bytes: usize,
    ) -> Result<Self::Expander> {
        if len_in_bytes == 0 {
            return Err(Error);
        }

        let len_in_bytes = u16::try_from(len_in_bytes).map_err(|_| Error)?;

        let domain = Domain::<U32>::xof::<HashT>(dsts)?;
        let mut reader = HashT::default();

        for msg in msgs {
            reader = reader.chain(msg);
        }

        reader.update(&len_in_bytes.to_be_bytes());
        domain.update_hash(&mut reader);
        reader.update(&[domain.len()]);
        let reader = reader.finalize_xof();
        Ok(Self { reader })
    }
}

impl<HashT> Expander for ExpandMsgXof<HashT>
where
    HashT: Default + ExtendableOutput + Update,
{
    fn fill_bytes(&mut self, okm: &mut [u8]) {
        self.reader.read(okm);
    }
}
