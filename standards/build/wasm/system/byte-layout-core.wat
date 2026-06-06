(module
  (import "edgerun-core" "memory" (memory 1))
;; Byte validation, endian layout, pointer metadata, fallible error strategy,
  ;; and destructuring semantics captured from local bytecheck/rancor/rend/
  ;; ptr-meta/munge compatibility crates.
  ;;
  ;; Type classes:
  ;;   primitive-any=0 bool=1 char=2 nonzero=3 utf8=4 cstr=5
  ;;   array=6 slice=7 tuple=8 range=9 zst=10 wrapper=11 enum-tag=12
  ;; Metadata classes: sized=0 sequence-len=1 dyn-vtable=2 trailing-dst=3
  ;; Destructure modes: borrow=0 move=1
  ;; Result states: ok=0 err=1 err+trace=2 panic=3 unreachable=4

  (func $m35bool (param $x i32) (result i32)
    local.get $x
    i32.const 0
    i32.ne)

  (func $m35ok (param $bad i32) (result i32)
    local.get $bad
    i32.eqz)

  (export "bytecheck_type_needs_source" (func $bytecheck_type_needs_source))
  (func $bytecheck_type_needs_source
    (param $type_class i32)
    (result i32)
    ;; bool, char, nonzero, utf8, cstr, and enum discriminants create source
    ;; errors. Recursive containers only add trace around child errors.
    local.get $type_class
    i32.const 1
    i32.ge_s
    local.get $type_class
    i32.const 5
    i32.le_s
    i32.and
    local.get $type_class
    i32.const 12
    i32.eq
    i32.or)

  (export "bytecheck_bool_valid" (func $bytecheck_bool_valid))
  (func $bytecheck_bool_valid
    (param $byte i32)
    (result i32)
    local.get $byte
    i32.const 0
    i32.eq
    local.get $byte
    i32.const 1
    i32.eq
    i32.or)

  (export "bytecheck_char_valid" (func $bytecheck_char_valid))
  (func $bytecheck_char_valid
    (param $scalar i32)
    (result i32)
    local.get $scalar
    i32.const 0
    i32.ge_u
    local.get $scalar
    i32.const 0x10ffff
    i32.le_u
    i32.and
    local.get $scalar
    i32.const 0xd800
    i32.ge_u
    local.get $scalar
    i32.const 0xdfff
    i32.le_u
    i32.and
    i32.eqz
    i32.and)

  (export "bytecheck_nonzero_valid_i64" (func $bytecheck_nonzero_valid_i64))
  (func $bytecheck_nonzero_valid_i64
    (param $value i64)
    (result i32)
    local.get $value
    i64.const 0
    i64.ne)

  (export "bytecheck_utf8_lead_len" (func $bytecheck_utf8_lead_len))
  (func $bytecheck_utf8_lead_len
    (param $lead i32)
    (result i32)
    ;; Return expected sequence length for a valid UTF-8 leading byte, else 0.
    local.get $lead
    i32.const 0x80
    i32.lt_u
    if (result i32)
      i32.const 1
    else
      local.get $lead
      i32.const 0xc2
      i32.ge_u
      local.get $lead
      i32.const 0xdf
      i32.le_u
      i32.and
      if (result i32)
        i32.const 2
      else
        local.get $lead
        i32.const 0xe0
        i32.ge_u
        local.get $lead
        i32.const 0xef
        i32.le_u
        i32.and
        if (result i32)
          i32.const 3
        else
          local.get $lead
          i32.const 0xf0
          i32.ge_u
          local.get $lead
          i32.const 0xf4
          i32.le_u
          i32.and
          if (result i32)
            i32.const 4
          else
            i32.const 0
          end
        end
      end
    end)

  (export "bytecheck_utf8_cont_valid" (func $bytecheck_utf8_cont_valid))
  (func $bytecheck_utf8_cont_valid
    (param $byte i32)
    (result i32)
    local.get $byte
    i32.const 0x80
    i32.ge_u
    local.get $byte
    i32.const 0xbf
    i32.le_u
    i32.and)

  (export "bytecheck_cstr_shape_valid" (func $bytecheck_cstr_shape_valid))
  (func $bytecheck_cstr_shape_valid
    (param $len i32)
    (param $last_is_nul i32)
    (param $interior_nul_count i32)
    (result i32)
    local.get $len
    i32.const 1
    i32.ge_s
    local.get $last_is_nul
    call $m35bool
    i32.and
    local.get $interior_nul_count
    i32.const 0
    i32.eq
    i32.and)

  (export "bytecheck_container_checks" (func $bytecheck_container_checks))
  (func $bytecheck_container_checks
    (param $type_class i32)
    (param $field_count i32)
    (param $len_metadata i32)
    (result i32)
    ;; Arrays/slices validate every element. Tuples/ranges validate fields.
    local.get $type_class
    i32.const 6
    i32.eq
    local.get $type_class
    i32.const 7
    i32.eq
    i32.or
    if (result i32)
      local.get $len_metadata
    else
      local.get $type_class
      i32.const 8
      i32.eq
      local.get $type_class
      i32.const 9
      i32.eq
      i32.or
      if (result i32)
        local.get $field_count
      else
        i32.const 0
      end
    end)

  (export "bytecheck_range_field_count" (func $bytecheck_range_field_count))
  (func $bytecheck_range_field_count
    (param $range_kind i32)
    (result i32)
    ;; Range=0 has start/end, RangeFrom=1 start, RangeFull=2 none,
    ;; RangeTo=3 end, RangeToInclusive=4 end.
    local.get $range_kind
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 2
    else
      local.get $range_kind
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 0
      else
        i32.const 1
      end
    end)

  (export "rancor_result_transition" (func $rancor_result_transition))
  (func $rancor_result_transition
    (param $is_ok i32)
    (param $convert_source i32)
    (param $add_trace i32)
    (param $panic_strategy i32)
    (result i32)
    local.get $is_ok
    call $m35bool
    if (result i32)
      i32.const 0
    else
      local.get $panic_strategy
      call $m35bool
      if (result i32)
        i32.const 3
      else
        local.get $add_trace
        call $m35bool
        if (result i32)
          i32.const 2
        else
          i32.const 1
        end
      end
    end)

  (export "rancor_option_transition" (func $rancor_option_transition))
  (func $rancor_option_transition
    (param $is_some i32)
    (param $add_trace i32)
    (result i32)
    local.get $is_some
    i32.const 1
    local.get $add_trace
    i32.const 0
    call $rancor_result_transition)

  (export "rancor_always_ok_state" (func $rancor_always_ok_state))
  (func $rancor_always_ok_state
    (param $is_ok i32)
    (param $error_is_never i32)
    (result i32)
    local.get $is_ok
    call $m35bool
    if (result i32)
      i32.const 0
    else
      local.get $error_is_never
      call $m35bool
      if (result i32)
        i32.const 4
      else
        i32.const 1
      end
    end)

  (export "rend_swap_needed" (func $rend_swap_needed))
  (func $rend_swap_needed
    (param $is_big_endian_type i32)
    (param $host_is_big_endian i32)
    (result i32)
    local.get $is_big_endian_type
    call $m35bool
    local.get $host_is_big_endian
    call $m35bool
    i32.xor)

  (export "rend_byte_at_u32" (func $rend_byte_at_u32))
  (func $rend_byte_at_u32
    (param $value i32)
    (param $index i32)
    (param $is_big_endian_type i32)
    (result i32)
    local.get $value
    local.get $is_big_endian_type
    call $m35bool
    if (result i32)
      i32.const 3
      local.get $index
      i32.sub
    else
      local.get $index
    end
    i32.const 8
    i32.mul
    i32.shr_u
    i32.const 0xff
    i32.and)

  (export "rend_fetch_ordering" (func $rend_fetch_ordering))
  (func $rend_fetch_ordering
    (param $ordering i32)
    (result i32)
    ;; Relaxed=0 Release=1 Acquire=2 AcqRel=3 SeqCst=4.
    local.get $ordering
    i32.const 1
    i32.eq
    if (result i32)
      i32.const 0
    else
      local.get $ordering
      i32.const 3
      i32.eq
      if (result i32)
        i32.const 2
      else
        local.get $ordering
      end
    end)

  (export "rend_layout_align" (func $rend_layout_align))
  (func $rend_layout_align
    (param $byte_width i32)
    (param $unaligned i32)
    (result i32)
    local.get $unaligned
    call $m35bool
    if (result i32)
      i32.const 1
    else
      local.get $byte_width
    end)

  (export "ptr_meta_class" (func $ptr_meta_class))
  (func $ptr_meta_class
    (param $is_sized i32)
    (param $is_sequence_dst i32)
    (param $is_dyn_trait i32)
    (param $has_trailing_dst i32)
    (result i32)
    local.get $is_sized
    call $m35bool
    if (result i32)
      i32.const 0
    else
      local.get $is_sequence_dst
      call $m35bool
      if (result i32)
        i32.const 1
      else
        local.get $is_dyn_trait
        call $m35bool
        if (result i32)
          i32.const 2
        else
          local.get $has_trailing_dst
          call $m35bool
          if (result i32)
            i32.const 3
          else
            i32.const 0
          end
        end
      end
    end)

  (export "ptr_meta_roundtrip_valid" (func $ptr_meta_roundtrip_valid))
  (func $ptr_meta_roundtrip_valid
    (param $data_addr i64)
    (param $metadata_a i64)
    (param $metadata_b i64)
    (result i32)
    local.get $data_addr
    i64.const 0
    i64.ne
    local.get $metadata_a
    local.get $metadata_b
    i64.eq
    i32.and)

  (export "dyn_metadata_equal" (func $dyn_metadata_equal))
  (func $dyn_metadata_equal
    (param $vtable_a i64)
    (param $vtable_b i64)
    (result i32)
    local.get $vtable_a
    local.get $vtable_b
    i64.eq)

  (export "munge_destructure_valid" (func $munge_destructure_valid))
  (func $munge_destructure_valid
    (param $mode i32)
    (param $underlying_non_null i32)
    (param $underlying_aligned i32)
    (param $rest_pattern i32)
    (param $all_fields_restructured i32)
    (result i32)
    local.get $underlying_non_null
    call $m35bool
    local.get $underlying_aligned
    call $m35bool
    i32.and
    local.get $mode
    i32.const 1
    i32.eq
    if (result i32)
      local.get $rest_pattern
      call $m35bool
      i32.eqz
      local.get $all_fields_restructured
      call $m35bool
      i32.and
    else
      i32.const 1
    end
    i32.and)

  (export "munge_restructured_action" (func $munge_restructured_action))
  (func $munge_restructured_action
    (param $mode i32)
    (param $wildcard_field i32)
    (result i32)
    ;; borrow=disjoint borrow(1); move=move field(2), wildcard move=drop(3).
    local.get $mode
    i32.const 1
    i32.eq
    if (result i32)
      local.get $wildcard_field
      call $m35bool
      if (result i32)
        i32.const 3
      else
        i32.const 2
      end
    else
      i32.const 1
    end)
)