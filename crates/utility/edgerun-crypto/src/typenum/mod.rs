#![allow(missing_docs)]

// For debugging macros:
// #![feature(trace_macros)]
// trace_macros!(true);

use core::cmp::Ordering;

pub mod bit;
mod r#gen;
pub mod int;
pub mod marker_traits;
pub mod operator_aliases;
pub mod private;
pub mod type_operators;
pub mod uint;

pub mod array;
pub mod tuple;

pub use crate::typenum::{
    array::{ATerm, TArr},
    r#gen::consts,
    int::{NInt, PInt},
    marker_traits::*,
    operator_aliases::*,
    type_operators::*,
    uint::{UInt, UTerm},
};

#[doc(no_inline)]
#[rustfmt::skip]
pub use consts::*;

#[cfg(feature = "const-generics")]
pub use crate::typenum::r#gen::generic_const_mappings;

#[cfg(feature = "const-generics")]
#[doc(no_inline)]
pub use generic_const_mappings::{Const, ToUInt, U};

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, Default)]
#[cfg_attr(feature = "scale_info", derive(scale_info::TypeInfo))]
pub struct Greater;

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, Default)]
#[cfg_attr(feature = "scale_info", derive(scale_info::TypeInfo))]
pub struct Less;

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, Default)]
#[cfg_attr(feature = "scale_info", derive(scale_info::TypeInfo))]
pub struct Equal;

impl Ord for Greater {
    #[inline]
    fn to_ordering() -> Ordering {
        Ordering::Greater
    }
}

impl Ord for Less {
    #[inline]
    fn to_ordering() -> Ordering {
        Ordering::Less
    }
}

impl Ord for Equal {
    #[inline]
    fn to_ordering() -> Ordering {
        Ordering::Equal
    }
}

#[macro_export]
macro_rules! assert_type_eq {
    ($a:ty, $b:ty) => {
        const _: core::marker::PhantomData<<$a as $crate::Same<$b>>::Output> =
            core::marker::PhantomData;
    };
}

#[macro_export]
macro_rules! assert_type {
    ($a:ty) => {
        const _: core::marker::PhantomData<<$a as $crate::Same<True>>::Output> =
            core::marker::PhantomData;
    };
}

mod sealed {
    use crate::typenum::{
        ATerm, B0, B1, Bit, Equal, Greater, Less, NInt, NonZero, PInt, TArr, UInt, UTerm, Unsigned,
        Z0,
    };

    pub trait Sealed {}

    impl Sealed for B0 {}
    impl Sealed for B1 {}

    impl Sealed for UTerm {}
    impl<U: Unsigned, B: Bit> Sealed for UInt<U, B> {}

    impl Sealed for Z0 {}
    impl<U: Unsigned + NonZero> Sealed for PInt<U> {}
    impl<U: Unsigned + NonZero> Sealed for NInt<U> {}

    impl Sealed for Less {}
    impl Sealed for Equal {}
    impl Sealed for Greater {}

    impl Sealed for ATerm {}
    impl<V, A> Sealed for TArr<V, A> {}
}
