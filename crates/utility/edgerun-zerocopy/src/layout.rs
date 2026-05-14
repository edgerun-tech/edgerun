// Copyright 2024 The Fuchsia Authors
//
// Licensed under the 2-Clause BSD License <LICENSE-BSD or
// https://opensource.org/license/bsd-2-clause>, Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0>, or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed except according to
// those terms.

use core::{mem, num::NonZeroUsize};

use crate::util;

/// The target pointer width, counted in bits.
const POINTER_WIDTH_BITS: usize = mem::size_of::<usize>() * 8;

/// The layout of a type which might be dynamically-sized.
///
/// `DstLayout` describes the layout of sized types, slice types, and "slice
/// DSTs" - ie, those that are known by the type system to have a trailing slice
/// (as distinguished from `dyn Trait` types - such types *might* have a
/// trailing slice type, but the type system isn't aware of it).
///
/// Note that `DstLayout` does not have any internal invariants, so no guarantee
/// is made that a `DstLayout` conforms to any of Rust's requirements regarding
/// the layout of real Rust types or instances of types.
#[doc(hidden)]
#[allow(missing_debug_implementations, missing_copy_implementations)]
#[cfg_attr(any(kani, test), derive(Debug, PartialEq, Eq))]
#[derive(Copy, Clone)]
pub struct DstLayout {
    pub(crate) align: NonZeroUsize,
    pub(crate) size_info: SizeInfo,
    // Is it guaranteed statically (without knowing a value's runtime metadata)
    // that the top-level type contains no padding? This does *not* apply
    // recursively - for example, `[(u8, u16)]` has `statically_shallow_unpadded
    // = true` even though this type likely has padding inside each `(u8, u16)`.
    pub(crate) statically_shallow_unpadded: bool,
}

#[cfg_attr(any(kani, test), derive(Debug, PartialEq, Eq))]
#[derive(Copy, Clone)]
pub(crate) enum SizeInfo<E = usize> {
    Sized { size: usize },
    SliceDst(TrailingSliceLayout<E>),
}

#[cfg_attr(any(kani, test), derive(Debug, PartialEq, Eq))]
#[derive(Copy, Clone)]
pub(crate) struct TrailingSliceLayout<E = usize> {
    // The offset of the first byte of the trailing slice field. Note that this
    // is NOT the same as the minimum size of the type. For example, consider
    // the following type:
    //
    //   struct Foo {
    //       a: u16,
    //       b: u8,
    //       c: [u8],
    //   }
    //
    // In `Foo`, `c` is at byte offset 3. When `c.len() == 0`, `c` is followed
    // by a padding byte.
    pub(crate) offset: usize,
    // The size of the element type of the trailing slice field.
    pub(crate) elem_size: E,
}

impl SizeInfo {
    /// Attempts to create a `SizeInfo` from `Self` in which `elem_size` is a
    /// `NonZeroUsize`. If `elem_size` is 0, returns `None`.
    #[allow(unused)]
    const fn try_to_nonzero_elem_size(&self) -> Option<SizeInfo<NonZeroUsize>> {
        Some(match *self {
            SizeInfo::Sized { size } => SizeInfo::Sized { size },
            SizeInfo::SliceDst(TrailingSliceLayout { offset, elem_size }) => {
                if let Some(elem_size) = NonZeroUsize::new(elem_size) {
                    SizeInfo::SliceDst(TrailingSliceLayout { offset, elem_size })
                } else {
                    return None;
                }
            }
        })
    }
}

#[doc(hidden)]
#[derive(Copy, Clone)]
#[cfg_attr(test, derive(Debug))]
#[allow(missing_debug_implementations)]
pub enum CastType {
    Prefix,
    Suffix,
}

#[cfg_attr(test, derive(Debug))]
pub(crate) enum MetadataCastError {
    Alignment,
    Size,
}

impl DstLayout {
    /// The minimum possible alignment of a type.
    const MIN_ALIGN: NonZeroUsize = match NonZeroUsize::new(1) {
        Some(min_align) => min_align,
        None => const_unreachable!(),
    };

    /// The maximum theoretic possible alignment of a type.
    ///
    /// For compatibility with future Rust versions, this is defined as the
    /// maximum power-of-two that fits into a `usize`. See also
    /// [`DstLayout::CURRENT_MAX_ALIGN`].
    pub(crate) const THEORETICAL_MAX_ALIGN: NonZeroUsize =
        match NonZeroUsize::new(1 << (POINTER_WIDTH_BITS - 1)) {
            Some(max_align) => max_align,
            None => const_unreachable!(),
        };

    /// The current, documented max alignment of a type \[1\].
    ///
    /// \[1\] Per <https://doc.rust-lang.org/reference/type-layout.html#the-alignment-modifiers>:
    ///
    ///   The alignment value must be a power of two from 1 up to
    ///   2<sup>29</sup>.
    #[cfg(not(kani))]
    #[cfg(not(target_pointer_width = "16"))]
    pub(crate) const CURRENT_MAX_ALIGN: NonZeroUsize = match NonZeroUsize::new(1 << 28) {
        Some(max_align) => max_align,
        None => const_unreachable!(),
    };

    #[cfg(not(kani))]
    #[cfg(target_pointer_width = "16")]
    pub(crate) const CURRENT_MAX_ALIGN: NonZeroUsize = match NonZeroUsize::new(1 << 15) {
        Some(max_align) => max_align,
        None => const_unreachable!(),
    };

    /// Assumes that this layout lacks static shallow padding.
    ///
    /// # Panics
    ///
    /// This method does not panic.
    ///
    /// # Safety
    ///
    /// If `self` describes the size and alignment of type that lacks static
    /// shallow padding, unsafe code may assume that the result of this method
    /// accurately reflects the size, alignment, and lack of static shallow
    /// padding of that type.
    const fn assume_shallow_unpadded(self) -> Self {
        Self { statically_shallow_unpadded: true, ..self }
    }

    /// Constructs a `DstLayout` for a zero-sized type with `repr_align`
    /// alignment (or 1). If `repr_align` is provided, then it must be a power
    /// of two.
    ///
    /// # Panics
    ///
    /// This function panics if the supplied `repr_align` is not a power of two.
    ///
    /// # Safety
    ///
    /// Unsafe code may assume that the contract of this function is satisfied.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn new_zst(repr_align: Option<NonZeroUsize>) -> DstLayout {
        let align = match repr_align {
            Some(align) => align,
            None => Self::MIN_ALIGN,
        };

        const_assert!(align.get().is_power_of_two());

        DstLayout {
            align,
            size_info: SizeInfo::Sized { size: 0 },
            statically_shallow_unpadded: true,
        }
    }

    /// Constructs a `DstLayout` which describes `T` and assumes `T` may contain
    /// padding.
    ///
    /// # Safety
    ///
    /// Unsafe code may assume that `DstLayout` is the correct layout for `T`.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn for_type<T>() -> DstLayout {
        // SAFETY: `align` is correct by construction. `T: Sized`, and so it is
        // sound to initialize `size_info` to `SizeInfo::Sized { size }`; the
        // `size` field is also correct by construction. `unpadded` can safely
        // default to `false`.
        DstLayout {
            align: match NonZeroUsize::new(mem::align_of::<T>()) {
                Some(align) => align,
                None => const_unreachable!(),
            },
            size_info: SizeInfo::Sized { size: mem::size_of::<T>() },
            statically_shallow_unpadded: false,
        }
    }

    /// Constructs a `DstLayout` which describes a `T` that does not contain
    /// padding.
    ///
    /// # Safety
    ///
    /// Unsafe code may assume that `DstLayout` is the correct layout for `T`.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn for_unpadded_type<T>() -> DstLayout {
        Self::for_type::<T>().assume_shallow_unpadded()
    }

    /// Constructs a `DstLayout` which describes `[T]`.
    ///
    /// # Safety
    ///
    /// Unsafe code may assume that `DstLayout` is the correct layout for `[T]`.
    pub(crate) const fn for_slice<T>() -> DstLayout {
        // SAFETY: The alignment of a slice is equal to the alignment of its
        // element type, and so `align` is initialized correctly.
        //
        // Since this is just a slice type, there is no offset between the
        // beginning of the type and the beginning of the slice, so it is
        // correct to set `offset: 0`. The `elem_size` is correct by
        // construction. Since `[T]` is a (degenerate case of a) slice DST, it
        // is correct to initialize `size_info` to `SizeInfo::SliceDst`.
        DstLayout {
            align: match NonZeroUsize::new(mem::align_of::<T>()) {
                Some(align) => align,
                None => const_unreachable!(),
            },
            size_info: SizeInfo::SliceDst(TrailingSliceLayout {
                offset: 0,
                elem_size: mem::size_of::<T>(),
            }),
            statically_shallow_unpadded: true,
        }
    }

    /// Constructs a complete `DstLayout` reflecting a `repr(C)` struct with the
    /// given alignment modifiers and fields.
    ///
    /// This method cannot be used to match the layout of a record with the
    /// default representation, as that representation is mostly unspecified.
    ///
    /// # Safety
    ///
    /// For any definition of a `repr(C)` struct, if this method is invoked with
    /// alignment modifiers and fields corresponding to that definition, the
    /// resulting `DstLayout` will correctly encode the layout of that struct.
    ///
    /// We make no guarantees to the behavior of this method when it is invoked
    /// with arguments that cannot correspond to a valid `repr(C)` struct.
    #[must_use]
    #[inline]
    pub const fn for_repr_c_struct(
        repr_align: Option<NonZeroUsize>,
        repr_packed: Option<NonZeroUsize>,
        fields: &[DstLayout],
    ) -> DstLayout {
        let mut layout = DstLayout::new_zst(repr_align);

        let mut i = 0;
        #[allow(clippy::arithmetic_side_effects)]
        while i < fields.len() {
            #[allow(clippy::indexing_slicing)]
            let field = fields[i];
            layout = layout.extend(field, repr_packed);
            i += 1;
        }

        layout = layout.pad_to_align();

        // SAFETY: `layout` accurately describes the layout of a `repr(C)`
        // struct with `repr_align` or `repr_packed` alignment modifications and
        // the given `fields`. The `layout` is constructed using a sequence of
        // invocations of `DstLayout::{new_zst,extend,pad_to_align}`. The
        // documentation of these items vows that invocations in this manner
        // will accurately describe a type, so long as:
        //
        //  - that type is `repr(C)`,
        //  - its fields are enumerated in the order they appear,
        //  - the presence of `repr_align` and `repr_packed` are correctly accounted for.
        //
        // We respect all three of these preconditions above.
        layout
    }

    /// Like `Layout::extend`, this creates a layout that describes a record
    /// whose layout consists of `self` followed by `next` that includes the
    /// necessary inter-field padding, but not any trailing padding.
    ///
    /// In order to match the layout of a `#[repr(C)]` struct, this method
    /// should be invoked for each field in declaration order. To add trailing
    /// padding, call `DstLayout::pad_to_align` after extending the layout for
    /// all fields. If `self` corresponds to a type marked with
    /// `repr(packed(N))`, then `repr_packed` should be set to `Some(N)`,
    /// otherwise `None`.
    ///
    /// This method cannot be used to match the layout of a record with the
    /// default representation, as that representation is mostly unspecified.
    ///
    /// # Safety
    ///
    /// If a (potentially hypothetical) valid `repr(C)` Rust type begins with
    /// fields whose layout are `self`, and those fields are immediately
    /// followed by a field whose layout is `field`, then unsafe code may rely
    /// on `self.extend(field, repr_packed)` producing a layout that correctly
    /// encompasses those two components.
    ///
    /// We make no guarantees to the behavior of this method if these fragments
    /// cannot appear in a valid Rust type (e.g., the concatenation of the
    /// layouts would lead to a size larger than `isize::MAX`).
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn extend(self, field: DstLayout, repr_packed: Option<NonZeroUsize>) -> Self {
        use util::{max, min, padding_needed_for};

        // If `repr_packed` is `None`, there are no alignment constraints, and
        // the value can be defaulted to `THEORETICAL_MAX_ALIGN`.
        let max_align = match repr_packed {
            Some(max_align) => max_align,
            None => Self::THEORETICAL_MAX_ALIGN,
        };

        const_assert!(max_align.get().is_power_of_two());

        // We use Kani to prove that this method is robust to future increases
        // in Rust's maximum allowed alignment. However, if such a change ever
        // actually occurs, we'd like to be notified via assertion failures.
        #[cfg(not(kani))]
        {
            const_debug_assert!(self.align.get() <= DstLayout::CURRENT_MAX_ALIGN.get());
            const_debug_assert!(field.align.get() <= DstLayout::CURRENT_MAX_ALIGN.get());
            if let Some(repr_packed) = repr_packed {
                const_debug_assert!(repr_packed.get() <= DstLayout::CURRENT_MAX_ALIGN.get());
            }
        }

        // The field's alignment is clamped by `repr_packed` (i.e., the
        // `repr(packed(N))` attribute, if any) [1].
        //
        // [1] Per https://doc.rust-lang.org/reference/type-layout.html#the-alignment-modifiers:
        //
        //   The alignments of each field, for the purpose of positioning
        //   fields, is the smaller of the specified alignment and the alignment
        //   of the field's type.
        let field_align = min(field.align, max_align);

        // The struct's alignment is the maximum of its previous alignment and
        // `field_align`.
        let align = max(self.align, field_align);

        let (interfield_padding, size_info) = match self.size_info {
            // If the layout is already a DST, we panic; DSTs cannot be extended
            // with additional fields.
            SizeInfo::SliceDst(..) => const_panic!("Cannot extend a DST with additional fields."),

            SizeInfo::Sized { size: preceding_size } => {
                // Compute the minimum amount of inter-field padding needed to
                // satisfy the field's alignment, and offset of the trailing
                // field. [1]
                //
                // [1] Per https://doc.rust-lang.org/reference/type-layout.html#the-alignment-modifiers:
                //
                //   Inter-field padding is guaranteed to be the minimum
                //   required in order to satisfy each field's (possibly
                //   altered) alignment.
                let padding = padding_needed_for(preceding_size, field_align);

                // This will not panic (and is proven to not panic, with Kani)
                // if the layout components can correspond to a leading layout
                // fragment of a valid Rust type, but may panic otherwise (e.g.,
                // combining or aligning the components would create a size
                // exceeding `isize::MAX`).
                let offset = match preceding_size.checked_add(padding) {
                    Some(offset) => offset,
                    None => const_panic!("Adding padding to `self`'s size overflows `usize`."),
                };

                (
                    padding,
                    match field.size_info {
                        SizeInfo::Sized { size: field_size } => {
                            // If the trailing field is sized, the resulting layout
                            // will be sized. Its size will be the sum of the
                            // preceding layout, the size of the new field, and the
                            // size of inter-field padding between the two.
                            //
                            // This will not panic (and is proven with Kani to not
                            // panic) if the layout components can correspond to a
                            // leading layout fragment of a valid Rust type, but may
                            // panic otherwise (e.g., combining or aligning the
                            // components would create a size exceeding
                            // `usize::MAX`).
                            let size = match offset.checked_add(field_size) {
                                Some(size) => size,
                                None => const_panic!("`field` cannot be appended without the total size overflowing `usize`"),
                            };
                            SizeInfo::Sized { size }
                        }
                        SizeInfo::SliceDst(TrailingSliceLayout {
                            offset: trailing_offset,
                            elem_size,
                        }) => {
                            // If the trailing field is dynamically sized, so too
                            // will the resulting layout. The offset of the trailing
                            // slice component is the sum of the offset of the
                            // trailing field and the trailing slice offset within
                            // that field.
                            //
                            // This will not panic (and is proven with Kani to not
                            // panic) if the layout components can correspond to a
                            // leading layout fragment of a valid Rust type, but may
                            // panic otherwise (e.g., combining or aligning the
                            // components would create a size exceeding
                            // `usize::MAX`).
                            let offset = match offset.checked_add(trailing_offset) {
                                Some(offset) => offset,
                                None => const_panic!("`field` cannot be appended without the total size overflowing `usize`"),
                            };
                            SizeInfo::SliceDst(TrailingSliceLayout { offset, elem_size })
                        }
                    },
                )
            }
        };

        let statically_shallow_unpadded = self.statically_shallow_unpadded
            && field.statically_shallow_unpadded
            && interfield_padding == 0;

        DstLayout { align, size_info, statically_shallow_unpadded }
    }

    /// Like `Layout::pad_to_align`, this routine rounds the size of this layout
    /// up to the nearest multiple of this type's alignment or `repr_packed`
    /// (whichever is less). This method leaves DST layouts unchanged, since the
    /// trailing padding of DSTs is computed at runtime.
    ///
    /// The accompanying boolean is `true` if the resulting composition of
    /// fields necessitated static (as opposed to dynamic) padding; otherwise
    /// `false`.
    ///
    /// In order to match the layout of a `#[repr(C)]` struct, this method
    /// should be invoked after the invocations of [`DstLayout::extend`]. If
    /// `self` corresponds to a type marked with `repr(packed(N))`, then
    /// `repr_packed` should be set to `Some(N)`, otherwise `None`.
    ///
    /// This method cannot be used to match the layout of a record with the
    /// default representation, as that representation is mostly unspecified.
    ///
    /// # Safety
    ///
    /// If a (potentially hypothetical) valid `repr(C)` type begins with fields
    /// whose layout are `self` followed only by zero or more bytes of trailing
    /// padding (not included in `self`), then unsafe code may rely on
    /// `self.pad_to_align(repr_packed)` producing a layout that correctly
    /// encapsulates the layout of that type.
    ///
    /// We make no guarantees to the behavior of this method if `self` cannot
    /// appear in a valid Rust type (e.g., because the addition of trailing
    /// padding would lead to a size larger than `isize::MAX`).
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn pad_to_align(self) -> Self {
        use util::padding_needed_for;

        let (static_padding, size_info) = match self.size_info {
            // For sized layouts, we add the minimum amount of trailing padding
            // needed to satisfy alignment.
            SizeInfo::Sized { size: unpadded_size } => {
                let padding = padding_needed_for(unpadded_size, self.align);
                let size = match unpadded_size.checked_add(padding) {
                    Some(size) => size,
                    None => const_panic!("Adding padding caused size to overflow `usize`."),
                };
                (padding, SizeInfo::Sized { size })
            }
            // For DST layouts, trailing padding depends on the length of the
            // trailing DST and is computed at runtime. This does not alter the
            // offset or element size of the layout, so we leave `size_info`
            // unchanged.
            size_info @ SizeInfo::SliceDst(_) => (0, size_info),
        };

        let statically_shallow_unpadded = self.statically_shallow_unpadded && static_padding == 0;

        DstLayout { align: self.align, size_info, statically_shallow_unpadded }
    }

    /// Produces `true` if `self` requires static padding; otherwise `false`.
    #[must_use]
    #[inline(always)]
    pub const fn requires_static_padding(self) -> bool {
        !self.statically_shallow_unpadded
    }

    /// Produces `true` if there exists any metadata for which a type of layout
    /// `self` would require dynamic trailing padding; otherwise `false`.
    #[must_use]
    #[inline(always)]
    pub const fn requires_dynamic_padding(self) -> bool {
        // A `% self.align.get()` cannot panic, since `align` is non-zero.
        #[allow(clippy::arithmetic_side_effects)]
        match self.size_info {
            SizeInfo::Sized { .. } => false,
            SizeInfo::SliceDst(trailing_slice_layout) => {
                // SAFETY: This predicate is formally proved sound by
                // `proofs::prove_requires_dynamic_padding`.
                trailing_slice_layout.offset % self.align.get() != 0
                    || trailing_slice_layout.elem_size % self.align.get() != 0
            }
        }
    }

    /// Validates that a cast is sound from a layout perspective.
    ///
    /// Validates that the size and alignment requirements of a type with the
    /// layout described in `self` would not be violated by performing a
    /// `cast_type` cast from a pointer with address `addr` which refers to a
    /// memory region of size `bytes_len`.
    ///
    /// If the cast is valid, `validate_cast_and_convert_metadata` returns
    /// `(elems, split_at)`. If `self` describes a dynamically-sized type, then
    /// `elems` is the maximum number of trailing slice elements for which a
    /// cast would be valid (for sized types, `elem` is meaningless and should
    /// be ignored). `split_at` is the index at which to split the memory region
    /// in order for the prefix (suffix) to contain the result of the cast, and
    /// in order for the remaining suffix (prefix) to contain the leftover
    /// bytes.
    ///
    /// There are three conditions under which a cast can fail:
    /// - The smallest possible value for the type is larger than the provided
    ///   memory region
    /// - A prefix cast is requested, and `addr` does not satisfy `self`'s
    ///   alignment requirement
    /// - A suffix cast is requested, and `addr + bytes_len` does not satisfy
    ///   `self`'s alignment requirement (as a consequence, since all instances
    ///   of the type are a multiple of its alignment, no size for the type will
    ///   result in a starting address which is properly aligned)
    ///
    /// # Safety
    ///
    /// The caller may assume that this implementation is correct, and may rely
    /// on that assumption for the soundness of their code. In particular, the
    /// caller may assume that, if `validate_cast_and_convert_metadata` returns
    /// `Some((elems, split_at))`, then:
    /// - A pointer to the type (for dynamically sized types, this includes
    ///   `elems` as its pointer metadata) describes an object of size `size <=
    ///   bytes_len`
    /// - If this is a prefix cast:
    ///   - `addr` satisfies `self`'s alignment
    ///   - `size == split_at`
    /// - If this is a suffix cast:
    ///   - `split_at == bytes_len - size`
    ///   - `addr + split_at` satisfies `self`'s alignment
    ///
    /// Note that this method does *not* ensure that a pointer constructed from
    /// its return values will be a valid pointer. In particular, this method
    /// does not reason about `isize` overflow, which is a requirement of many
    /// Rust pointer APIs, and may at some point be determined to be a validity
    /// invariant of pointer types themselves. This should never be a problem so
    /// long as the arguments to this method are derived from a known-valid
    /// pointer (e.g., one derived from a safe Rust reference), but it is
    /// nonetheless the caller's responsibility to justify that pointer
    /// arithmetic will not overflow based on a safety argument *other than* the
    /// mere fact that this method returned successfully.
    ///
    /// # Panics
    ///
    /// `validate_cast_and_convert_metadata` will panic if `self` describes a
    /// DST whose trailing slice element is zero-sized.
    ///
    /// If `addr + bytes_len` overflows `usize`,
    /// `validate_cast_and_convert_metadata` may panic, or it may return
    /// incorrect results. No guarantees are made about when
    /// `validate_cast_and_convert_metadata` will panic. The caller should not
    /// rely on `validate_cast_and_convert_metadata` panicking in any particular
    /// condition, even if `debug_assertions` are enabled.
    #[allow(unused)]
    #[inline(always)]
    pub(crate) const fn validate_cast_and_convert_metadata(
        &self,
        addr: usize,
        bytes_len: usize,
        cast_type: CastType,
    ) -> Result<(usize, usize), MetadataCastError> {
        // `debug_assert!`, but with `#[allow(clippy::arithmetic_side_effects)]`.
        macro_rules! __const_debug_assert {
            ($e:expr $(, $msg:expr)?) => {
                const_debug_assert!({
                    #[allow(clippy::arithmetic_side_effects)]
                    let e = $e;
                    e
                } $(, $msg)?);
            };
        }

        // Note that, in practice, `self` is always a compile-time constant. We
        // do this check earlier than needed to ensure that we always panic as a
        // result of bugs in the program (such as calling this function on an
        // invalid type) instead of allowing this panic to be hidden if the cast
        // would have failed anyway for runtime reasons (such as a too-small
        // memory region).
        //
        // FIXME(#67): Once our MSRV is 1.65, use let-else:
        // https://blog.rust-lang.org/2022/11/03/Rust-1.65.0.html#let-else-statements
        let size_info = match self.size_info.try_to_nonzero_elem_size() {
            Some(size_info) => size_info,
            None => const_panic!("attempted to cast to slice type with zero-sized element"),
        };

        // Precondition
        __const_debug_assert!(
            addr.checked_add(bytes_len).is_some(),
            "`addr` + `bytes_len` > usize::MAX"
        );

        // Alignment checks go in their own block to avoid introducing variables
        // into the top-level scope.
        {
            // We check alignment for `addr` (for prefix casts) or `addr +
            // bytes_len` (for suffix casts). For a prefix cast, the correctness
            // of this check is trivial - `addr` is the address the object will
            // live at.
            //
            // For a suffix cast, we know that all valid sizes for the type are
            // a multiple of the alignment (and by safety precondition, we know
            // `DstLayout` may only describe valid Rust types). Thus, a
            // validly-sized instance which lives at a validly-aligned address
            // must also end at a validly-aligned address. Thus, if the end
            // address for a suffix cast (`addr + bytes_len`) is not aligned,
            // then no valid start address will be aligned either.
            let offset = match cast_type {
                CastType::Prefix => 0,
                CastType::Suffix => bytes_len,
            };

            // Addition is guaranteed not to overflow because `offset <=
            // bytes_len`, and `addr + bytes_len <= usize::MAX` is a
            // precondition of this method. Modulus is guaranteed not to divide
            // by 0 because `align` is non-zero.
            #[allow(clippy::arithmetic_side_effects)]
            if (addr + offset) % self.align.get() != 0 {
                return Err(MetadataCastError::Alignment);
            }
        }

        let (elems, self_bytes) = match size_info {
            SizeInfo::Sized { size } => {
                if size > bytes_len {
                    return Err(MetadataCastError::Size);
                }
                (0, size)
            }
            SizeInfo::SliceDst(TrailingSliceLayout { offset, elem_size }) => {
                // Calculate the maximum number of bytes that could be consumed
                // - any number of bytes larger than this will either not be a
                // multiple of the alignment, or will be larger than
                // `bytes_len`.
                let max_total_bytes =
                    util::round_down_to_next_multiple_of_alignment(bytes_len, self.align);
                // Calculate the maximum number of bytes that could be consumed
                // by the trailing slice.
                //
                // FIXME(#67): Once our MSRV is 1.65, use let-else:
                // https://blog.rust-lang.org/2022/11/03/Rust-1.65.0.html#let-else-statements
                let max_slice_and_padding_bytes = match max_total_bytes.checked_sub(offset) {
                    Some(max) => max,
                    // `bytes_len` too small even for 0 trailing slice elements.
                    None => return Err(MetadataCastError::Size),
                };

                // Calculate the number of elements that fit in
                // `max_slice_and_padding_bytes`; any remaining bytes will be
                // considered padding.
                //
                // Guaranteed not to divide by zero: `elem_size` is non-zero.
                #[allow(clippy::arithmetic_side_effects)]
                let elems = max_slice_and_padding_bytes / elem_size.get();
                // Guaranteed not to overflow on multiplication: `usize::MAX >=
                // max_slice_and_padding_bytes >= (max_slice_and_padding_bytes /
                // elem_size) * elem_size`.
                //
                // Guaranteed not to overflow on addition:
                // - max_slice_and_padding_bytes == max_total_bytes - offset
                // - elems * elem_size <= max_slice_and_padding_bytes == max_total_bytes - offset
                // - elems * elem_size + offset <= max_total_bytes <= usize::MAX
                #[allow(clippy::arithmetic_side_effects)]
                let without_padding = offset + elems * elem_size.get();
                // `self_bytes` is equal to the offset bytes plus the bytes
                // consumed by the trailing slice plus any padding bytes
                // required to satisfy the alignment. Note that we have computed
                // the maximum number of trailing slice elements that could fit
                // in `self_bytes`, so any padding is guaranteed to be less than
                // the size of an extra element.
                //
                // Guaranteed not to overflow:
                // - By previous comment: without_padding == elems * elem_size +
                //   offset <= max_total_bytes
                // - By construction, `max_total_bytes` is a multiple of
                //   `self.align`.
                // - At most, adding padding needed to round `without_padding`
                //   up to the next multiple of the alignment will bring
                //   `self_bytes` up to `max_total_bytes`.
                #[allow(clippy::arithmetic_side_effects)]
                let self_bytes =
                    without_padding + util::padding_needed_for(without_padding, self.align);
                (elems, self_bytes)
            }
        };

        __const_debug_assert!(self_bytes <= bytes_len);

        let split_at = match cast_type {
            CastType::Prefix => self_bytes,
            // Guaranteed not to underflow:
            // - In the `Sized` branch, only returns `size` if `size <=
            //   bytes_len`.
            // - In the `SliceDst` branch, calculates `self_bytes <=
            //   max_toatl_bytes`, which is upper-bounded by `bytes_len`.
            #[allow(clippy::arithmetic_side_effects)]
            CastType::Suffix => bytes_len - self_bytes,
        };

        Ok((elems, split_at))
    }
}

pub(crate) use cast_from::CastFrom;
mod cast_from {
    use crate::*;

    pub(crate) struct CastFrom<Dst: ?Sized> {
        _never: core::convert::Infallible,
        _marker: PhantomData<Dst>,
    }

    // SAFETY: The implementation of `Project::project` preserves the address
    // of the referent – it only modifies pointer metadata.
    unsafe impl<Src, Dst> crate::pointer::cast::Cast<Src, Dst> for CastFrom<Dst>
    where
        Src: KnownLayout + ?Sized,
        Dst: KnownLayout + ?Sized,
    {
    }

    // SAFETY: The implementation of `Project::project` preserves the size of
    // the referent (see inline comments for a more detailed proof of this).
    unsafe impl<Src, Dst> crate::pointer::cast::CastExact<Src, Dst> for CastFrom<Dst>
    where
        Src: KnownLayout + ?Sized,
        Dst: KnownLayout + ?Sized,
    {
    }

    // SAFETY: `project` produces a pointer which refers to the same referent
    // bytes as its input, or to a subset of them (see inline comments for a
    // more detailed proof of this). It does this using provenance-preserving
    // operations.
    unsafe impl<Src, Dst> crate::pointer::cast::Project<Src, Dst> for CastFrom<Dst>
    where
        Src: KnownLayout + ?Sized,
        Dst: KnownLayout + ?Sized,
    {
        /// # PME
        ///
        /// Generates a post-monomorphization error if it is not possible to
        /// implement soundly.
        //
        // FIXME(#1817): Support Sized->Unsized and Unsized->Sized casts
        fn project(src: PtrInner<'_, Src>) -> *mut Dst {
            /// The parameters required in order to perform a pointer cast from
            /// `Src` to `Dst`.
            ///
            /// These are a compile-time function of the layouts of `Src`
            /// and `Dst`.
            ///
            /// # Safety
            ///
            /// `Src`'s alignment must not be smaller than `Dst`'s alignment.
            struct CastParams<Src: ?Sized, Dst: ?Sized> {
                inner: CastParamsInner,
                _src: PhantomData<Src>,
                _dst: PhantomData<Dst>,
            }

            #[derive(Copy, Clone)]
            enum CastParamsInner {
                // At compile time (specifically, post-monomorphization time),
                // we need to compute two things:
                // - Whether, given *any* `*Src`, it is possible to construct a
                //   `*Dst` which addresses the same number of bytes (ie,
                //   whether, for any `Src` pointer metadata, there exists `Dst`
                //   pointer metadata that addresses the same number of bytes)
                // - If this is possible, any information necessary to perform
                //   the `Src`->`Dst` metadata conversion at runtime.
                //
                // Assume that `Src` and `Dst` are slice DSTs, and define:
                // - `S_OFF = Src::LAYOUT.size_info.offset`
                // - `S_ELEM = Src::LAYOUT.size_info.elem_size`
                // - `D_OFF = Dst::LAYOUT.size_info.offset`
                // - `D_ELEM = Dst::LAYOUT.size_info.elem_size`
                //
                // We are trying to solve the following equation:
                //
                //   D_OFF + d_meta * D_ELEM = S_OFF + s_meta * S_ELEM
                //
                // At runtime, we will be attempting to compute `d_meta`, given
                // `s_meta` (a runtime value) and all other parameters (which
                // are compile-time values). We can solve like so:
                //
                //   D_OFF + d_meta * D_ELEM = S_OFF + s_meta * S_ELEM
                //
                //   d_meta * D_ELEM = S_OFF - D_OFF + s_meta * S_ELEM
                //
                //   d_meta = (S_OFF - D_OFF + s_meta * S_ELEM)/D_ELEM
                //
                // Since `d_meta` will be a `usize`, we need the right-hand side
                // to be an integer, and this needs to hold for *any* value of
                // `s_meta` (in order for our conversion to be infallible - ie,
                // to not have to reject certain values of `s_meta` at runtime).
                // This means that:
                //
                // - `s_meta * S_ELEM` must be a multiple of `D_ELEM`
                // - Since this must hold for any value of `s_meta`, `S_ELEM`
                //   must be a multiple of `D_ELEM`
                // - `S_OFF - D_OFF` must be a multiple of `D_ELEM`
                //
                // Thus, let `OFFSET_DELTA_ELEMS = (S_OFF - D_OFF)/D_ELEM` and
                // `ELEM_MULTIPLE = S_ELEM/D_ELEM`. We can rewrite the above
                // expression as:
                //
                //   d_meta = (S_OFF - D_OFF + s_meta * S_ELEM)/D_ELEM
                //
                //   d_meta = OFFSET_DELTA_ELEMS + s_meta * ELEM_MULTIPLE
                //
                // Thus, we just need to compute the following and confirm that
                // they have integer solutions in order to both a) determine
                // whether infallible `Src` -> `Dst` casts are possible and, b)
                // pre-compute the parameters necessary to perform those casts
                // at runtime. These parameters are encapsulated in
                // `CastParams`, which acts as a witness that such infallible
                // casts are possible.
                /// The parameters required in order to perform an
                /// unsized-to-unsized pointer cast from `Src` to `Dst` as
                /// described above.
                ///
                /// # Safety
                ///
                /// `Src` and `Dst` must both be slice DSTs.
                ///
                /// `offset_delta_elems` and `elem_multiple` must be valid as
                /// described above.
                UnsizedToUnsized { offset_delta_elems: usize, elem_multiple: usize },

                /// The metadata of a `Dst` which has the same size as `Src:
                /// Sized`.
                ///
                /// # Safety
                ///
                /// `Src: Sized` and `Dst` must be a slice DST.
                ///
                /// A raw `Dst` pointer with metadata `dst_meta` must address
                /// `size_of::<Src>()` bytes.
                SizedToUnsized { dst_meta: usize },

                /// The metadata of a `Dst` which has the same size as `Src:
                /// Sized`.
                ///
                /// # Safety
                ///
                /// `Src` and `Dst` must both be `Sized` and `size_of::<Src>()
                /// == size_of::<Dst>()`.
                SizedToSized,
            }

            impl<Src: ?Sized, Dst: ?Sized> Copy for CastParams<Src, Dst> {}
            impl<Src: ?Sized, Dst: ?Sized> Clone for CastParams<Src, Dst> {
                fn clone(&self) -> Self {
                    *self
                }
            }

            impl<Src: ?Sized, Dst: ?Sized> CastParams<Src, Dst> {
                const fn try_compute(
                    src: &DstLayout,
                    dst: &DstLayout,
                ) -> Option<CastParams<Src, Dst>> {
                    if src.align.get() < dst.align.get() {
                        return None;
                    }

                    let inner = match (src.size_info, dst.size_info) {
                        (
                            SizeInfo::Sized { size: src_size },
                            SizeInfo::Sized { size: dst_size },
                        ) => {
                            if src_size != dst_size {
                                return None;
                            }

                            // SAFETY: We checked above that `src_size ==
                            // dst_size`.
                            CastParamsInner::SizedToSized
                        }
                        (SizeInfo::Sized { size: src_size }, SizeInfo::SliceDst(dst)) => {
                            let offset_delta = if let Some(od) = src_size.checked_sub(dst.offset) {
                                od
                            } else {
                                return None;
                            };

                            let dst_elem_size = if let Some(e) = NonZeroUsize::new(dst.elem_size) {
                                e
                            } else {
                                return None;
                            };

                            // PANICS: `dst_elem_size: NonZeroUsize`, so this won't
                            // divide by zero.
                            #[allow(clippy::arithmetic_side_effects)]
                            let delta_mod_other_elem = offset_delta % dst_elem_size.get();

                            if delta_mod_other_elem != 0 {
                                return None;
                            }

                            // PANICS: `dst_elem_size: NonZeroUsize`, so this won't
                            // divide by zero.
                            #[allow(clippy::arithmetic_side_effects)]
                            let dst_meta = offset_delta / dst_elem_size.get();

                            // SAFETY: The preceding math ensures that a `Dst`
                            // with `dst_meta` addresses `src_size` bytes.
                            CastParamsInner::SizedToUnsized { dst_meta }
                        }
                        (SizeInfo::SliceDst(src), SizeInfo::SliceDst(dst)) => {
                            let offset_delta = if let Some(od) = src.offset.checked_sub(dst.offset)
                            {
                                od
                            } else {
                                return None;
                            };

                            let dst_elem_size = if let Some(e) = NonZeroUsize::new(dst.elem_size) {
                                e
                            } else {
                                return None;
                            };

                            // PANICS: `dst_elem_size: NonZeroUsize`, so this won't
                            // divide by zero.
                            #[allow(clippy::arithmetic_side_effects)]
                            let delta_mod_other_elem = offset_delta % dst_elem_size.get();

                            // PANICS: `dst_elem_size: NonZeroUsize`, so this won't
                            // divide by zero.
                            #[allow(clippy::arithmetic_side_effects)]
                            let elem_remainder = src.elem_size % dst_elem_size.get();

                            if delta_mod_other_elem != 0
                                || src.elem_size < dst.elem_size
                                || elem_remainder != 0
                            {
                                return None;
                            }

                            // PANICS: `dst_elem_size: NonZeroUsize`, so this won't
                            // divide by zero.
                            #[allow(clippy::arithmetic_side_effects)]
                            let offset_delta_elems = offset_delta / dst_elem_size.get();

                            // PANICS: `dst_elem_size: NonZeroUsize`, so this won't
                            // divide by zero.
                            #[allow(clippy::arithmetic_side_effects)]
                            let elem_multiple = src.elem_size / dst_elem_size.get();

                            CastParamsInner::UnsizedToUnsized {
                                // SAFETY: We checked above that this is an exact ratio.
                                offset_delta_elems,
                                // SAFETY: We checked above that this is an exact ratio.
                                elem_multiple,
                            }
                        }
                        _ => return None,
                    };

                    // SAFETY: We checked above that `src.align >= dst.align`.
                    Some(CastParams { inner, _src: PhantomData, _dst: PhantomData })
                }
            }

            impl<Src: KnownLayout + ?Sized, Dst: KnownLayout + ?Sized> CastParams<Src, Dst> {
                /// # Safety
                ///
                /// `src_meta` describes a `Src` whose size is no larger than
                /// `isize::MAX`.
                ///
                /// The returned metadata describes a `Dst` of the same size as
                /// the original `Src`.
                #[inline(always)]
                unsafe fn cast_metadata(
                    self,
                    src_meta: Src::PointerMetadata,
                ) -> Dst::PointerMetadata {
                    #[allow(unused)]
                    use crate::util::polyfills::*;

                    let dst_meta = match self.inner {
                        CastParamsInner::UnsizedToUnsized { offset_delta_elems, elem_multiple } => {
                            let src_meta = src_meta.to_elem_count();
                            #[allow(
                                unstable_name_collisions,
                                clippy::multiple_unsafe_ops_per_block
                            )]
                            // SAFETY: `self` is a witness that the following
                            // equation holds:
                            //
                            //   D_OFF + d_meta * D_ELEM = S_OFF + s_meta * S_ELEM
                            //
                            // Since the caller promises that `src_meta` is
                            // valid `Src` metadata, this math will not
                            // overflow, and the returned value will describe a
                            // `Dst` of the same size.
                            unsafe {
                                offset_delta_elems
                                    .unchecked_add(src_meta.unchecked_mul(elem_multiple))
                            }
                        }
                        CastParamsInner::SizedToUnsized { dst_meta } => dst_meta,
                        CastParamsInner::SizedToSized => 0,
                    };
                    Dst::PointerMetadata::from_elem_count(dst_meta)
                }
            }

            trait Params<Src: ?Sized> {
                const CAST_PARAMS: CastParams<Src, Self>;
            }

            impl<Src, Dst> Params<Src> for Dst
            where
                Src: KnownLayout + ?Sized,
                Dst: KnownLayout + ?Sized,
            {
                const CAST_PARAMS: CastParams<Src, Dst> =
                    match CastParams::try_compute(&Src::LAYOUT, &Dst::LAYOUT) {
                        Some(params) => params,
                        None => const_panic!(
                            "cannot `transmute_ref!` or `transmute_mut!` between incompatible types"
                        ),
                    };
            }

            let src_meta = <Src as KnownLayout>::pointer_to_metadata(src.as_ptr());
            let params = <Dst as Params<Src>>::CAST_PARAMS;

            // SAFETY: `src: PtrInner` guarantees that `src`'s referent is zero
            // bytes or lives in a single allocation, which means that it is no
            // larger than `isize::MAX` bytes [1].
            //
            // [1] https://doc.rust-lang.org/1.92.0/std/ptr/index.html#allocation
            let dst_meta = unsafe { params.cast_metadata(src_meta) };

            <Dst as KnownLayout>::raw_from_ptr_len(src.as_non_null().cast(), dst_meta).as_ptr()
        }
    }
}

// FIXME(#67): For some reason, on our MSRV toolchain, this `allow` isn't
// enforced despite having `#![allow(unknown_lints)]` at the crate root, but
// putting it here works. Once our MSRV is high enough that this bug has been
// fixed, remove this `allow`.
