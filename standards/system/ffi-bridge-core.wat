  (import "edgerun" "to_lower" (func $m195lower (param i32) (result i32)))

;; Captures the portable meaning from edgerun-wasm-bindgen and its macro-support:
  ;; ABI primitive packing, descriptor wrappers, closure ownership, attribute parsing,
  ;; AST import/export classification, and custom-section encoding rules.
  (func $m195fnv1a_lower (param $ptr i32) (param $len i32) (result i32)
    (local $end i32) (local $h i32)
    (local.set $end (i32.add (local.get $ptr) (local.get $len)))
    (local.set $h (i32.const 0x811c9dc5))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $ptr) (local.get $end)))
        (local.set $h
          (i32.mul
            (i32.xor (local.get $h) (call $m195lower (i32.load8_u (local.get $ptr))))
            (i32.const 0x01000193)))
        (local.set $ptr (i32.add (local.get $ptr) (i32.const 1)))
        (br $loop)))
    local.get $h)

  (func $m195is (param $h i32) (param $want i32) (result i32)
    (i32.eq (local.get $h) (local.get $want)))

  ;; Type-kind input codes: 0 unit, 1 i8, 2 u8, 3 i16, 4 u16, 5 i32, 6 u32,
  ;; 7 i64, 8 u64, 9 i128, 10 u128, 11 f32, 12 f64, 13 bool, 14 char,
  ;; 15 JsValue/externref, 16 isize, 17 usize, 18 string/str, 19 raw pointer.
  ;; Descriptor result codes keep wasm-bindgen's important distinctions:
  ;; 1..15 primitive/object, 16 raw pointer, 17 nonnull, 18 i64-as-f64,
  ;; 19 u64-as-f64, 20 string, 21 cached string.
  (func (export "wasm_bindgen_descriptor_code")
    (param $kind i32) (param $wasm64 i32) (param $interning i32) (result i32)
    (if (i32.eq (local.get $kind) (i32.const 0)) (then (return (i32.const 0))))
    (if (i32.le_u (local.get $kind) (i32.const 15)) (then (return (local.get $kind))))
    (if (i32.eq (local.get $kind) (i32.const 16))
      (then
        (if (local.get $wasm64) (then (return (i32.const 18))))
        (return (i32.const 5))))
    (if (i32.eq (local.get $kind) (i32.const 17))
      (then
        (if (local.get $wasm64) (then (return (i32.const 19))))
        (return (i32.const 6))))
    (if (i32.eq (local.get $kind) (i32.const 18))
      (then
        (if (local.get $interning) (then (return (i32.const 21))))
        (return (i32.const 20))))
    (if (i32.eq (local.get $kind) (i32.const 19)) (then (return (i32.const 16))))
    i32.const -1)

  ;; Descriptor wrapper token order: optional wraps first, then ref/refmut,
  ;; then slice/vector/clamped. 0 means no wrapper token is emitted.
  (func (export "wasm_bindgen_descriptor_wrapper")
    (param $optional i32) (param $ref_kind i32) (param $slice i32) (param $vector i32) (param $clamped i32) (result i32)
    (if (local.get $optional) (then (return (i32.const 1))))
    (if (i32.eq (local.get $ref_kind) (i32.const 2)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $ref_kind) (i32.const 1)) (then (return (i32.const 2))))
    (if (local.get $slice) (then (return (i32.const 4))))
    (if (local.get $vector) (then (return (i32.const 5))))
    (if (local.get $clamped) (then (return (i32.const 6))))
    i32.const 0)

  ;; ABI primitive count for single values. WasmAbi supports four primitive slots.
  (func (export "wasm_bindgen_abi_primitive_count") (param $kind i32) (result i32)
    (if (i32.eq (local.get $kind) (i32.const 0)) (then (return (i32.const 0))))
    (if (i32.or (i32.eq (local.get $kind) (i32.const 9)) (i32.eq (local.get $kind) (i32.const 10)))
      (then (return (i32.const 2))))
    (if (i32.eq (local.get $kind) (i32.const 20)) (then (return (i32.const 2)))) ;; WasmSlice ptr,len
    (if (i32.eq (local.get $kind) (i32.const 21)) (then (return (i32.const 3)))) ;; WasmMutSlice ptr,len,idx
    (if (i32.gt_u (local.get $kind) (i32.const 0)) (then (return (i32.const 1))))
    i32.const -1)

  (func (export "wasm_bindgen_option_abi_count") (param $inner_count i32) (result i32)
    (if (i32.gt_u (local.get $inner_count) (i32.const 3)) (then (return (i32.const -1))))
    (i32.add (local.get $inner_count) (i32.const 1)))

  (func (export "wasm_bindgen_result_abi_count") (param $ok_count i32) (result i32)
    (if (i32.gt_u (local.get $ok_count) (i32.const 2)) (then (return (i32.const -1))))
    (i32.add (local.get $ok_count) (i32.const 2)))

  (func (export "wasm_bindgen_option_tag") (param $m195is_some i32) (result i32)
    (if (local.get $m195is_some) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "wasm_bindgen_result_is_err") (param $m195is_err i32) (result i32)
    (if (local.get $m195is_err) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "wasm_bindgen_f64_option_sentinel") (result f64)
    f64.const 9007199254740991)

  (func (export "wasm_bindgen_null_slice_ptr") (result i32) i32.const 0)
  (func (export "wasm_bindgen_slice_abi_count") (param $mutable i32) (result i32)
    (if (local.get $mutable) (then (return (i32.const 3))))
    i32.const 2)

  ;; Closure descriptor packs CLOSURE, destructor-present, mutability, and unwind-catching.
  (func (export "wasm_bindgen_closure_descriptor")
    (param $owned i32) (param $m195is_mut i32) (param $catch_unwind i32) (result i32)
    (i32.add
      (i32.const 1000)
      (i32.add
        (i32.mul (local.get $owned) (i32.const 100))
        (i32.add (i32.mul (local.get $m195is_mut) (i32.const 10)) (local.get $catch_unwind)))))

  ;; Drop/transfer actions: 1 owned unref/dtor, 2 borrowed invalidate, 3 forgotten,
  ;; 4 transferred to JS GC with weakrefs, 5 transferred/leaked without weakrefs.
  (func (export "wasm_bindgen_closure_lifetime_action")
    (param $owned i32) (param $forgotten i32) (param $into_js_value i32) (param $weakrefs i32) (result i32)
    (if (local.get $forgotten) (then (return (i32.const 3))))
    (if (local.get $into_js_value)
      (then
        (if (local.get $weakrefs) (then (return (i32.const 4))))
        (return (i32.const 5))))
    (if (local.get $owned) (then (return (i32.const 1))))
    i32.const 2)

  ;; FnOnce closure call state: first call consumes, repeated call throws.
  (func (export "wasm_bindgen_once_call_state") (param $already_called i32) (result i32)
    (if (local.get $already_called) (then (return (i32.const 2))))
    i32.const 1)

  ;; Macro attribute codes follow parser attrgen order for high-value attrs.
  (func (export "wasm_bindgen_attr_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $m195fnv1a_lower (local.get $ptr) (local.get $len)))
    (if (call $m195is (local.get $h) (i32.const 0x4288e94c)) (then (return (i32.const 1))))  ;; catch
    (if (call $m195is (local.get $h) (i32.const 0xf25d9f4f)) (then (return (i32.const 2))))  ;; constructor
    (if (call $m195is (local.get $h) (i32.const 0xab45f730)) (then (return (i32.const 3))))  ;; method
    (if (call $m195is (local.get $h) (i32.const 0xda2bd281)) (then (return (i32.const 4))))  ;; this
    (if (call $m195is (local.get $h) (i32.const 0xc1a9642f)) (then (return (i32.const 5))))  ;; static_method_of
    (if (call $m195is (local.get $h) (i32.const 0x2875dbd2)) (then (return (i32.const 6))))  ;; js_namespace
    (if (call $m195is (local.get $h) (i32.const 0xd79f909d)) (then (return (i32.const 7))))  ;; module
    (if (call $m195is (local.get $h) (i32.const 0x5d2b4cd6)) (then (return (i32.const 8))))  ;; raw_module
    (if (call $m195is (local.get $h) (i32.const 0x1118f2f6)) (then (return (i32.const 9))))  ;; inline_js
    (if (call $m195is (local.get $h) (i32.const 0xe0f9e332)) (then (return (i32.const 10)))) ;; getter
    (if (call $m195is (local.get $h) (i32.const 0xb23634c6)) (then (return (i32.const 11)))) ;; setter
    (if (call $m195is (local.get $h) (i32.const 0x0d150a5a)) (then (return (i32.const 15)))) ;; structural
    (if (call $m195is (local.get $h) (i32.const 0xce0beff7)) (then (return (i32.const 17)))) ;; readonly
    (if (call $m195is (local.get $h) (i32.const 0x73ef3fcc)) (then (return (i32.const 18)))) ;; js_name
    (if (call $m195is (local.get $h) (i32.const 0x9d85d64e)) (then (return (i32.const 23)))) ;; extends
    (if (call $m195is (local.get $h) (i32.const 0xa52a0930)) (then (return (i32.const 29)))) ;; variadic
    (if (call $m195is (local.get $h) (i32.const 0xd54c0328)) (then (return (i32.const 31)))) ;; skip_typescript
    (if (call $m195is (local.get $h) (i32.const 0x62cb0d0c)) (then (return (i32.const 33)))) ;; private
    (if (call $m195is (local.get $h) (i32.const 0xe27c17bb)) (then (return (i32.const 34)))) ;; fallback
    (if (call $m195is (local.get $h) (i32.const 0x652b04df)) (then (return (i32.const 36)))) ;; start
    (if (call $m195is (local.get $h) (i32.const 0x829706cd)) (then (return (i32.const 41)))) ;; slice_to_array
    (if (call $m195is (local.get $h) (i32.const 0xfb2db0fd)) (then (return (i32.const 43)))) ;; getter_with_clone
    (if (call $m195is (local.get $h) (i32.const 0xcd3c1aa1)) (then (return (i32.const 45)))) ;; thread_local
    (if (call $m195is (local.get $h) (i32.const 0x493dd242)) (then (return (i32.const 46)))) ;; thread_local_v2
    (if (call $m195is (local.get $h) (i32.const 0xa148242d)) (then (return (i32.const 52)))) ;; assert_no_shim
    i32.const 0)

  ;; Attribute value shape: 1 flag, 2 ident/path/expression, 3 string-or-ident,
  ;; 4 namespace list/string/ident, 5 static string-only metadata.
  (func (export "wasm_bindgen_attr_value_shape") (param $attr_code i32) (result i32)
    (if (i32.or (i32.eq (local.get $attr_code) (i32.const 5)) (i32.eq (local.get $attr_code) (i32.const 23)))
      (then (return (i32.const 2))))
    (if (i32.eq (local.get $attr_code) (i32.const 6)) (then (return (i32.const 4))))
    (if (i32.or (i32.or (i32.eq (local.get $attr_code) (i32.const 7)) (i32.eq (local.get $attr_code) (i32.const 8)))
                (i32.or (i32.eq (local.get $attr_code) (i32.const 9)) (i32.eq (local.get $attr_code) (i32.const 18))))
      (then (return (i32.const 3))))
    (if (i32.gt_u (local.get $attr_code) (i32.const 0)) (then (return (i32.const 1))))
    i32.const 0)

  ;; Computed js_name validation: 0 plain, 1 valid [Symbol.ident], 2 malformed.
  (func (export "wasm_bindgen_computed_key_status") (param $ptr i32) (param $len i32) (result i32)
    (local $end i32) (local $i i32) (local $c i32)
    (if (i32.lt_u (local.get $len) (i32.const 2)) (then (return (i32.const 0))))
    (if (i32.ne (i32.load8_u (local.get $ptr)) (i32.const 91)) (then (return (i32.const 0))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $len) (i32.const 1)))) (i32.const 93))
      (then (return (i32.const 2))))
    (if (i32.lt_u (local.get $len) (i32.const 10)) (then (return (i32.const 2))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 83)) (then (return (i32.const 2)))) ;; S
    (if (i32.ne (call $m195lower (i32.load8_u (i32.add (local.get $ptr) (i32.const 2)))) (i32.const 121)) (then (return (i32.const 2))))
    (if (i32.ne (call $m195lower (i32.load8_u (i32.add (local.get $ptr) (i32.const 3)))) (i32.const 109)) (then (return (i32.const 2))))
    (if (i32.ne (call $m195lower (i32.load8_u (i32.add (local.get $ptr) (i32.const 4)))) (i32.const 98)) (then (return (i32.const 2))))
    (if (i32.ne (call $m195lower (i32.load8_u (i32.add (local.get $ptr) (i32.const 5)))) (i32.const 111)) (then (return (i32.const 2))))
    (if (i32.ne (call $m195lower (i32.load8_u (i32.add (local.get $ptr) (i32.const 6)))) (i32.const 108)) (then (return (i32.const 2))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 7))) (i32.const 46)) (then (return (i32.const 2))))
    (local.set $i (i32.add (local.get $ptr) (i32.const 8)))
    (local.set $end (i32.add (local.get $ptr) (i32.sub (local.get $len) (i32.const 1))))
    (if (i32.ge_u (local.get $i) (local.get $end)) (then (return (i32.const 2))))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $end)))
        (local.set $c (i32.load8_u (local.get $i)))
        (if (i32.eqz
              (i32.or
                (i32.or (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))
                        (i32.and (i32.ge_u (call $m195lower (local.get $c)) (i32.const 97)) (i32.le_u (call $m195lower (local.get $c)) (i32.const 122))))
                (i32.eq (local.get $c) (i32.const 95))))
          (then (return (i32.const 2))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    i32.const 1)

  ;; Namespace parse status: empty list invalid; first non-value JS keyword invalid unless default.
  (func (export "wasm_bindgen_namespace_status") (param $count i32) (param $first_is_non_value_keyword i32) (param $first_is_default i32) (result i32)
    (if (i32.eqz (local.get $count)) (then (return (i32.const 2))))
    (if (i32.and (local.get $first_is_non_value_keyword) (i32.eqz (local.get $first_is_default)))
      (then (return (i32.const 3))))
    i32.const 1)

  ;; Import module resolution: absolute local paths are uniqued/included, relative
  ;; paths are rejected, bare names are raw module names, inline stays inline.
  (func (export "wasm_bindgen_import_module_kind")
    (param $inline i32) (param $absolute i32) (param $relative i32) (result i32)
    (if (local.get $inline) (then (return (i32.const 3))))
    (if (local.get $relative) (then (return (i32.const 4))))
    (if (local.get $absolute) (then (return (i32.const 1))))
    i32.const 2)

  ;; Import kind codes: 1 function, 2 static, 3 static string, 4 type, 5 string enum, 6 dynamic union.
  (func (export "wasm_bindgen_import_kind_code") (param $kind i32) (result i32)
    (if (i32.and (i32.ge_u (local.get $kind) (i32.const 1)) (i32.le_u (local.get $kind) (i32.const 6)))
      (then (return (local.get $kind))))
    i32.const 0)

  ;; Method operation codes: 1 constructor, 2 regular, 3 regular-this, 4 getter,
  ;; 5 setter, 6 indexing-getter, 7 indexing-setter, 8 indexing-deleter.
  (func (export "wasm_bindgen_method_operation_code")
    (param $constructor i32) (param $operation_kind i32) (result i32)
    (if (local.get $constructor) (then (return (i32.const 1))))
    (if (i32.and (i32.ge_u (local.get $operation_kind) (i32.const 1)) (i32.le_u (local.get $operation_kind) (i32.const 7)))
      (then (return (i32.add (local.get $operation_kind) (i32.const 1)))))
    i32.const 0)

  ;; Struct export status: parent fields are filtered, extends may appear once,
  ;; self-extends is rejected, exported structs cannot have generics.
  (func (export "wasm_bindgen_struct_export_status")
    (param $generic_param_count i32) (param $extends_count i32) (param $self_extends i32) (result i32)
    (if (local.get $generic_param_count) (then (return (i32.const 2))))
    (if (i32.gt_u (local.get $extends_count) (i32.const 1)) (then (return (i32.const 3))))
    (if (local.get $self_extends) (then (return (i32.const 4))))
    i32.const 1)

  ;; Dynamic union summary: empty variant field => string literal branch,
  ;; one-field tuple variant => typed branch; fallback may consume the final typed branch.
  (func (export "wasm_bindgen_dynamic_union_variant_kind")
    (param $field_count i32) (param $m195is_last i32) (param $fallback_enabled i32) (result i32)
    (if (i32.eqz (local.get $field_count)) (then (return (i32.const 1))))
    (if (i32.and (local.get $m195is_last) (local.get $fallback_enabled)) (then (return (i32.const 3))))
    i32.const 2)

  ;; Custom-section encoding helpers.
  (func $wasm_bindgen_leb128_u32_len (export "wasm_bindgen_leb128_u32_len") (param $value i32) (result i32)
    (local $n i32)
    (local.set $n (i32.const 1))
    (block $done
      (loop $loop
        (br_if $done (i32.eqz (i32.shr_u (local.get $value) (i32.const 7))))
        (local.set $value (i32.shr_u (local.get $value) (i32.const 7)))
        (local.set $n (i32.add (local.get $n) (i32.const 1)))
        (br $loop)))
    local.get $n)

  (func (export "wasm_bindgen_option_encode_len") (param $m195is_some i32) (param $payload_len i32) (result i32)
    (if (local.get $m195is_some) (then (return (i32.add (i32.const 1) (local.get $payload_len)))))
    i32.const 1)

  (func (export "wasm_bindgen_slice_encode_len") (param $byte_len i32) (result i32)
    (i32.add (call $wasm_bindgen_leb128_u32_len (local.get $byte_len)) (local.get $byte_len)))

  (func (export "wasm_bindgen_vec_encode_len") (param $item_count i32) (param $items_encoded_len i32) (result i32)
    (i32.add (call $wasm_bindgen_leb128_u32_len (local.get $item_count)) (local.get $items_encoded_len)))

  (func (export "wasm_bindgen_enum_encode_len") (param $payload_len i32) (result i32)
    (i32.add (i32.const 1) (local.get $payload_len)))
