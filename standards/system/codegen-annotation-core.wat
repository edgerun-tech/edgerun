(module
  (import "edgerun-core" "memory" (memory 1))
;; Compile-time derive/macro semantics captured from local quote,
  ;; synstructure, thiserror, rkyv-derive, bytecheck-derive, and error-derive.
  ;;
  ;; Codes are compact reducers for agent/runtime verification:
  ;;   data kind: struct=0 enum=1 union=2
  ;;   field shape: unit=0 named=1 unnamed=2
  ;;   repr: rust=0 transparent=1 primitive=2 c=3
  ;;   quote span: call_site=1 interpolated=2 explicit=3
  ;;   generated impl bits: archive=1 archive_with=2 serialize=4 serialize_with=8
  ;;                        display=16 error=32 from=64 source=128

  (func $m73max0 (param $n i32) (result i32)
    local.get $n
    i32.const 0
    i32.gt_s
    if (result i32)
      local.get $n
    else
      i32.const 0
    end)

  (func $m73bool (param $x i32) (result i32)
    local.get $x
    i32.const 0
    i32.ne)

  (func $m73ok (param $bad i32) (result i32)
    local.get $bad
    i32.eqz)

  (export "quote_repetition_tokens" (func $quote_repetition_tokens))
  (func $quote_repetition_tokens
    (param $items i32)
    (param $separator_present i32)
    (result i32)
    ;; quote repeats each interpolated item and emits separators only between
    ;; items. Return packed counts: high 16 bits item count, low 16 separator count.
    (local $n i32)
    local.get $items
    call $m73max0
    local.tee $n
    i32.const 16
    i32.shl
    local.get $separator_present
    call $m73bool
    if (result i32)
      local.get $n
      i32.const 1
      i32.sub
      call $m73max0
    else
      i32.const 0
    end
    i32.or)

  (export "quote_span_source" (func $quote_span_source))
  (func $quote_span_source
    (param $interpolated i32)
    (param $quote_spanned i32)
    (result i32)
    local.get $quote_spanned
    call $m73bool
    if (result i32)
      i32.const 3
    else
      local.get $interpolated
      call $m73bool
      if (result i32)
        i32.const 2
      else
        i32.const 1
      end
    end)

  (export "synstructure_add_bounds_scope" (func $synstructure_add_bounds_scope))
  (func $synstructure_add_bounds_scope
    (param $mode i32)
    (result i32)
    ;; None=0 Fields=1 Generics=2 Both=3.
    local.get $mode
    i32.const 3
    i32.and)

  (export "synstructure_bind_tokens" (func $synstructure_bind_tokens))
  (func $synstructure_bind_tokens
    (param $style i32)
    (result i32)
    ;; Move emits no prefix. MoveMut/ref/ref mut are encoded as 1/2/3.
    local.get $style
    i32.const 3
    i32.and)

  (export "synstructure_fuse_generics" (func $synstructure_fuse_generics))
  (func $synstructure_fuse_generics
    (param $seen i32)
    (param $next i32)
    (result i32)
    local.get $seen
    local.get $next
    i32.or)

  (export "synstructure_bound_fields" (func $synstructure_bound_fields))
  (func $synstructure_bound_fields
    (param $field_count i32)
    (param $omitted_count i32)
    (result i32)
    local.get $field_count
    local.get $omitted_count
    i32.sub
    call $m73max0)

  (export "thiserror_non_field_attrs_valid" (func $thiserror_non_field_attrs_valid))
  (func $thiserror_non_field_attrs_valid
    (param $has_from i32)
    (param $has_source i32)
    (param $has_backtrace i32)
    (param $transparent i32)
    (param $display i32)
    (param $fmt i32)
    (result i32)
    ;; from/source/backtrace are field-only. transparent conflicts with display/fmt.
    ;; display format args and fmt= are mutually exclusive.
    local.get $has_from
    local.get $has_source
    i32.or
    local.get $has_backtrace
    i32.or
    local.get $transparent
    local.get $display
    local.get $fmt
    i32.or
    i32.and
    i32.or
    local.get $transparent
    i32.eqz
    local.get $display
    local.get $fmt
    i32.and
    i32.and
    i32.or
    call $m73ok)

  (export "thiserror_field_attrs_valid" (func $thiserror_field_attrs_valid))
  (func $thiserror_field_attrs_valid
    (param $field_count i32)
    (param $from_count i32)
    (param $source_count i32)
    (param $backtrace_count i32)
    (param $from_is_source i32)
    (param $has_extra_backtrace i32)
    (result i32)
    (local $allowed i32)
    i32.const 1
    local.get $backtrace_count
    i32.const 0
    i32.gt_s
    if (result i32)
      local.get $from_is_source
      i32.eqz
    else
      local.get $has_extra_backtrace
      call $m73bool
    end
    i32.add
    local.set $allowed
    local.get $from_count
    i32.const 1
    i32.gt_s
    local.get $source_count
    i32.const 1
    i32.gt_s
    i32.or
    local.get $backtrace_count
    i32.const 1
    i32.gt_s
    i32.or
    local.get $from_count
    call $m73bool
    local.get $source_count
    call $m73bool
    i32.and
    local.get $from_is_source
    i32.eqz
    i32.and
    i32.or
    local.get $from_count
    call $m73bool
    local.get $field_count
    local.get $allowed
    i32.gt_s
    i32.and
    i32.or
    call $m73ok)

  (export "thiserror_impl_bits" (func $thiserror_impl_bits))
  (func $thiserror_impl_bits
    (param $transparent i32)
    (param $display i32)
    (param $source_or_from i32)
    (param $from i32)
    (result i32)
    ;; Error impl always emits; Display emits for transparent or display attr.
    i32.const 32
    local.get $transparent
    local.get $display
    i32.or
    call $m73bool
    i32.const 16
    i32.mul
    i32.or
    local.get $from
    call $m73bool
    i32.const 64
    i32.mul
    i32.or
    local.get $source_or_from
    call $m73bool
    i32.const 128
    i32.mul
    i32.or)

  (export "rkyv_type_attrs_valid" (func $rkyv_type_attrs_valid))
  (func $rkyv_type_attrs_valid
    (param $as_type i32)
    (param $archived i32)
    (param $metas i32)
    (param $bytecheck i32)
    (result i32)
    ;; `as = ...` means no archived type is generated, so archived name,
    ;; archived attrs, and bytecheck impl requests are invalid.
    local.get $as_type
    call $m73bool
    local.get $archived
    local.get $metas
    i32.or
    local.get $bytecheck
    i32.or
    call $m73bool
    i32.and
    call $m73ok)

  (export "rkyv_field_bound_kind" (func $rkyv_field_bound_kind))
  (func $rkyv_field_bound_kind
    (param $omit_bounds i32)
    (param $with_attr i32)
    (param $phase i32)
    (result i32)
    ;; phase archive/serialize/deserialize = 0/1/2.
    ;; no bound=0, native bound=1..3, with bound=4..6.
    local.get $omit_bounds
    call $m73bool
    if (result i32)
      i32.const 0
    else
      local.get $with_attr
      call $m73bool
      if (result i32)
        local.get $phase
        i32.const 4
        i32.add
      else
        local.get $phase
        i32.const 1
        i32.add
      end
    end)

  (export "rkyv_archive_impl_bits" (func $rkyv_archive_impl_bits))
  (func $rkyv_archive_impl_bits
    (param $remote i32)
    (param $serialize i32)
    (result i32)
    (local $bits i32)
    local.get $remote
    call $m73bool
    if (result i32)
      i32.const 2
    else
      i32.const 1
    end
    local.set $bits
    local.get $serialize
    call $m73bool
    if
      local.get $bits
      local.get $remote
      call $m73bool
      if (result i32)
        i32.const 8
      else
        i32.const 4
      end
      i32.or
      local.set $bits
    end
    local.get $bits)

  (export "rkyv_enum_valid" (func $rkyv_enum_valid))
  (func $rkyv_enum_valid
    (param $variant_count i32)
    (param $compare_partial_ord i32)
    (result i32)
    ;; Archived enum tags are u8, and PartialOrd compare generation is rejected.
    local.get $variant_count
    i32.const 256
    i32.gt_s
    local.get $compare_partial_ord
    call $m73bool
    i32.or
    call $m73ok)

  (export "rkyv_other_variant_valid" (func $rkyv_other_variant_valid))
  (func $rkyv_other_variant_valid
    (param $remote i32)
    (param $unit_variant i32)
    (param $is_last i32)
    (result i32)
    local.get $remote
    local.get $unit_variant
    i32.and
    local.get $is_last
    i32.and
    call $m73bool)

  (export "bytecheck_data_valid" (func $bytecheck_data_valid))
  (func $bytecheck_data_valid
    (param $data_kind i32)
    (param $repr_kind i32)
    (result i32)
    ;; Unions unsupported. Enums require explicit primitive repr only.
    local.get $data_kind
    i32.const 2
    i32.eq
    local.get $data_kind
    i32.const 1
    i32.eq
    local.get $repr_kind
    i32.const 2
    i32.ne
    i32.and
    i32.or
    call $m73ok)

  (export "bytecheck_error_bound_kind" (func $bytecheck_error_bound_kind))
  (func $bytecheck_error_bound_kind
    (param $data_kind i32)
    (param $verify i32)
    (result i32)
    ;; Struct/union need Trace, enum needs Source; verify adds Verify<Self>.
    local.get $data_kind
    i32.const 1
    i32.eq
    if (result i32)
      i32.const 2
    else
      i32.const 1
    end
    local.get $verify
    call $m73bool
    i32.const 4
    i32.mul
    i32.or)

  (export "small_error_derive_bits" (func $small_error_derive_bits))
  (func $small_error_derive_bits
    (param $is_enum i32)
    (param $transparent i32)
    (param $field_count i32)
    (param $source_or_from i32)
    (result i32)
    ;; local zero-dependency derive always emits Display+Error, optionally source/from.
    local.get $transparent
    call $m73bool
    local.get $field_count
    i32.const 1
    i32.ne
    i32.and
    if (result i32)
      i32.const 0
    else
      i32.const 16
      i32.const 32
      i32.or
      local.get $source_or_from
      call $m73bool
      i32.const 128
      i32.mul
      i32.or
      local.get $is_enum
      local.get $source_or_from
      i32.and
      call $m73bool
      i32.const 64
      i32.mul
      i32.or
    end)
)