// TODO(tarcieri): use `generic_const_exprs` when stable to make generic around bits.
macro_rules! impl_uint_aliases {
    ($(($name:ident, $bits:expr, $doc:expr)),+) => {
        $(
            #[doc = $doc]
            #[doc="unsigned big integer."]
            pub type $name = Uint<{nlimbs!($bits)}>;

            impl Encoding for $name {

                type Repr = [u8; $bits / 8];

                #[inline]
                fn from_be_bytes(bytes: Self::Repr) -> Self {
                    Self::from_be_slice(&bytes)
                }

                #[inline]
                fn from_le_bytes(bytes: Self::Repr) -> Self {
                    Self::from_le_slice(&bytes)
                }

                #[inline]
                fn to_be_bytes(&self) -> Self::Repr {
                    let mut result = [0u8; $bits / 8];
                    self.write_be_bytes(&mut result);
                    result
                }

                #[inline]
                fn to_le_bytes(&self) -> Self::Repr {
                    let mut result = [0u8; $bits / 8];
                    self.write_le_bytes(&mut result);
                    result
                }
            }
        )+
     };
}

macro_rules! impl_uint_concat_split_mixed {
    ($name:ident, $size:literal) => {
        impl $crate::crypto_bigint::traits::ConcatMixed<Uint<{ U64::LIMBS * $size }>> for Uint<{ <$name>::LIMBS - U64::LIMBS * $size }>
        {
            type MixedOutput = $name;

            fn concat_mixed(&self, lo: &Uint<{ U64::LIMBS * $size }>) -> Self::MixedOutput {
                $crate::crypto_bigint::uint::concat::concat_mixed(lo, self)
            }
        }

        impl $crate::crypto_bigint::traits::SplitMixed<Uint<{ U64::LIMBS * $size }>, Uint<{ <$name>::LIMBS - U64::LIMBS * $size }>> for $name
        {
            fn split_mixed(&self) -> (Uint<{ U64::LIMBS * $size }>, Uint<{ <$name>::LIMBS - U64::LIMBS * $size }>) {
                $crate::crypto_bigint::uint::split::split_mixed(self)
            }
        }
    };
    ($name:ident, [ $($size:literal),+ ]) => {
        $(
            impl_uint_concat_split_mixed!($name, $size);
        )+
    };
    ($( ($name:ident, $sizes:tt), )+) => {
        $(
            impl_uint_concat_split_mixed!($name, $sizes);
        )+
    };
}

macro_rules! impl_uint_concat_split_even {
    ($name:ident) => {
        impl $crate::crypto_bigint::traits::ConcatMixed<Uint<{ <$name>::LIMBS / 2 }>> for Uint<{ <$name>::LIMBS / 2 }>
        {
            type MixedOutput = $name;

            fn concat_mixed(&self, lo: &Uint<{ <$name>::LIMBS / 2 }>) -> Self::MixedOutput {
                $crate::crypto_bigint::uint::concat::concat_mixed(lo, self)
            }
        }

        impl Uint<{ <$name>::LIMBS / 2 }> {
            /// Concatenate the two values, with `self` as most significant and `rhs`
            /// as the least significant.
            pub const fn concat(&self, lo: &Uint<{ <$name>::LIMBS / 2 }>) -> $name {
                $crate::crypto_bigint::uint::concat::concat_mixed(lo, self)
            }
        }

        impl $crate::crypto_bigint::traits::SplitMixed<Uint<{ <$name>::LIMBS / 2 }>, Uint<{ <$name>::LIMBS / 2 }>> for $name
        {
            fn split_mixed(&self) -> (Uint<{ <$name>::LIMBS / 2 }>, Uint<{ <$name>::LIMBS / 2 }>) {
                $crate::crypto_bigint::uint::split::split_mixed(self)
            }
        }

        impl $crate::crypto_bigint::traits::Split for $name
        {
            type Output = Uint<{ <$name>::LIMBS / 2 }>;
        }

        impl $name {
            /// Split this number in half, returning its high and low components
            /// respectively.
            pub const fn split(&self) -> (Uint<{ <$name>::LIMBS / 2 }>, Uint<{ <$name>::LIMBS / 2 }>) {
                $crate::crypto_bigint::uint::split::split_mixed(self)
            }
        }
    };
    ($($name:ident,)+) => {
        $(
            impl_uint_concat_split_even!($name);
        )+
    }
}
