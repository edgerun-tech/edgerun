use core::borrow::{Borrow, BorrowMut};
use core::cmp::Ordering;
use core::fmt::{self, Debug};
use core::hash::{Hash, Hasher};

use super::{ArrayLength, GenericArray};

use crate::functional::*;
use crate::sequence::*;

impl<T: Default, N> Default for GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    #[inline(always)]
    fn default() -> Self {
        Self::generate(|_| T::default())
    }
}

impl<T: Clone, N> Clone for GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    fn clone(&self) -> GenericArray<T, N> {
        self.map(Clone::clone)
    }
}

impl<T: Copy, N> Copy for GenericArray<T, N>
where
    N: ArrayLength<T>,
    N::ArrayType: Copy,
{
}

impl<T: PartialEq, N> PartialEq for GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
impl<T: Eq, N> Eq for GenericArray<T, N> where N: ArrayLength<T> {}

impl<T: PartialOrd, N> PartialOrd for GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    fn partial_cmp(&self, other: &GenericArray<T, N>) -> Option<Ordering> {
        PartialOrd::partial_cmp(self.as_slice(), other.as_slice())
    }
}

impl<T: Ord, N> Ord for GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    fn cmp(&self, other: &GenericArray<T, N>) -> Ordering {
        Ord::cmp(self.as_slice(), other.as_slice())
    }
}

impl<T: Debug, N> Debug for GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self[..].fmt(fmt)
    }
}

impl<T, N> Borrow<[T]> for GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    #[inline(always)]
    fn borrow(&self) -> &[T] {
        &self[..]
    }
}

impl<T, N> BorrowMut<[T]> for GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    #[inline(always)]
    fn borrow_mut(&mut self) -> &mut [T] {
        &mut self[..]
    }
}

impl<T, N> AsRef<[T]> for GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    #[inline(always)]
    fn as_ref(&self) -> &[T] {
        &self[..]
    }
}

impl<T, N> AsMut<[T]> for GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [T] {
        &mut self[..]
    }
}

impl<T: Hash, N> Hash for GenericArray<T, N>
where
    N: ArrayLength<T>,
{
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        Hash::hash(&self[..], state)
    }
}

macro_rules! impl_from {
    ($($n: expr => $ty: ty),*) => {
        $(
            impl<T> From<[T; $n]> for GenericArray<T, $ty> {
                #[inline(always)]
                fn from(arr: [T; $n]) -> Self {
                    unsafe { $crate::transmute(arr) }
                }
            }

            #[cfg(relaxed_coherence)]
            impl<T> From<GenericArray<T, $ty>> for [T; $n] {
                #[inline(always)]
                fn from(sel: GenericArray<T, $ty>) -> [T; $n] {
                    unsafe { $crate::transmute(sel) }
                }
            }

            impl<'a, T> From<&'a [T; $n]> for &'a GenericArray<T, $ty> {
                #[inline]
                fn from(slice: &[T; $n]) -> &GenericArray<T, $ty> {
                    unsafe { &*(slice.as_ptr() as *const GenericArray<T, $ty>) }
                }
            }

            impl<'a, T> From<&'a mut [T; $n]> for &'a mut GenericArray<T, $ty> {
                #[inline]
                fn from(slice: &mut [T; $n]) -> &mut GenericArray<T, $ty> {
                    unsafe { &mut *(slice.as_mut_ptr() as *mut GenericArray<T, $ty>) }
                }
            }

            #[cfg(not(relaxed_coherence))]
            impl<T> Into<[T; $n]> for GenericArray<T, $ty> {
                #[inline(always)]
                fn into(self) -> [T; $n] {
                    unsafe { $crate::transmute(self) }
                }
            }

            impl<T> AsRef<[T; $n]> for GenericArray<T, $ty> {
                #[inline]
                fn as_ref(&self) -> &[T; $n] {
                    unsafe { $crate::transmute(self) }
                }
            }

            impl<T> AsMut<[T; $n]> for GenericArray<T, $ty> {
                #[inline]
                fn as_mut(&mut self) -> &mut [T; $n] {
                    unsafe { $crate::transmute(self) }
                }
            }
        )*
    }
}

impl_from! {
    1  => crate::typenum::U1,
    2  => crate::typenum::U2,
    3  => crate::typenum::U3,
    4  => crate::typenum::U4,
    5  => crate::typenum::U5,
    6  => crate::typenum::U6,
    7  => crate::typenum::U7,
    8  => crate::typenum::U8,
    9  => crate::typenum::U9,
    10 => crate::typenum::U10,
    11 => crate::typenum::U11,
    12 => crate::typenum::U12,
    13 => crate::typenum::U13,
    14 => crate::typenum::U14,
    15 => crate::typenum::U15,
    16 => crate::typenum::U16,
    17 => crate::typenum::U17,
    18 => crate::typenum::U18,
    19 => crate::typenum::U19,
    20 => crate::typenum::U20,
    21 => crate::typenum::U21,
    22 => crate::typenum::U22,
    23 => crate::typenum::U23,
    24 => crate::typenum::U24,
    25 => crate::typenum::U25,
    26 => crate::typenum::U26,
    27 => crate::typenum::U27,
    28 => crate::typenum::U28,
    29 => crate::typenum::U29,
    30 => crate::typenum::U30,
    31 => crate::typenum::U31,
    32 => crate::typenum::U32
}

#[cfg(feature = "more_lengths")]
impl_from! {
    33 => crate::typenum::U33,
    34 => crate::typenum::U34,
    35 => crate::typenum::U35,
    36 => crate::typenum::U36,
    37 => crate::typenum::U37,
    38 => crate::typenum::U38,
    39 => crate::typenum::U39,
    40 => crate::typenum::U40,
    41 => crate::typenum::U41,
    42 => crate::typenum::U42,
    43 => crate::typenum::U43,
    44 => crate::typenum::U44,
    45 => crate::typenum::U45,
    46 => crate::typenum::U46,
    47 => crate::typenum::U47,
    48 => crate::typenum::U48,
    49 => crate::typenum::U49,
    50 => crate::typenum::U50,
    51 => crate::typenum::U51,
    52 => crate::typenum::U52,
    53 => crate::typenum::U53,
    54 => crate::typenum::U54,
    55 => crate::typenum::U55,
    56 => crate::typenum::U56,
    57 => crate::typenum::U57,
    58 => crate::typenum::U58,
    59 => crate::typenum::U59,
    60 => crate::typenum::U60,
    61 => crate::typenum::U61,
    62 => crate::typenum::U62,
    63 => crate::typenum::U63,
    64 => crate::typenum::U64,

    70 => crate::typenum::U70,
    80 => crate::typenum::U80,
    90 => crate::typenum::U90,

    100 => crate::typenum::U100,
    200 => crate::typenum::U200,
    300 => crate::typenum::U300,
    400 => crate::typenum::U400,
    500 => crate::typenum::U500,

    128 => crate::typenum::U128,
    256 => crate::typenum::U256,
    512 => crate::typenum::U512,

    1000 => crate::typenum::U1000,
    1024 => crate::typenum::U1024
}
