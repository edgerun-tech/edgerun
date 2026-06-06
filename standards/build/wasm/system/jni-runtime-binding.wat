(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "is_digit" (func $is_digit (param i32) (result i32)))

(func (export "proto_standard_id") (result i32)
    i32.const 300135)

  (func $is_upper_ascii (param $b i32) (result i32)
    local.get $b
    i32.const 65
    i32.ge_u
    local.get $b
    i32.const 90
    i32.le_u
    i32.and)

  (func $is_ascii_alnum (param $b i32) (result i32)
    local.get $b
    i32.const 48
    i32.ge_u
    local.get $b
    i32.const 57
    i32.le_u
    i32.and
    local.get $b
    i32.const 65
    i32.ge_u
    local.get $b
    i32.const 90
    i32.le_u
    i32.and
    i32.or
    local.get $b
    i32.const 97
    i32.ge_u
    local.get $b
    i32.const 122
    i32.le_u
    i32.and
    i32.or)

  (func $forbidden_namespace_byte (param $b i32) (result i32)
    local.get $b
    i32.const 32
    i32.eq
    local.get $b
    i32.const 44
    i32.eq
    i32.or
    local.get $b
    i32.const 58
    i32.eq
    i32.or
    local.get $b
    i32.const 59
    i32.eq
    i32.or
    local.get $b
    i32.const 124
    i32.eq
    i32.or
    local.get $b
    i32.const 92
    i32.eq
    i32.or
    local.get $b
    i32.const 47
    i32.eq
    i32.or
    local.get $b
    i32.const 33
    i32.eq
    i32.or
    local.get $b
    i32.const 64
    i32.eq
    i32.or
    local.get $b
    i32.const 35
    i32.eq
    i32.or
    local.get $b
    i32.const 37
    i32.eq
    i32.or
    local.get $b
    i32.const 94
    i32.eq
    i32.or
    local.get $b
    i32.const 38
    i32.eq
    i32.or
    local.get $b
    i32.const 42
    i32.eq
    i32.or
    local.get $b
    i32.const 40
    i32.eq
    i32.or
    local.get $b
    i32.const 41
    i32.eq
    i32.or
    local.get $b
    i32.const 123
    i32.eq
    i32.or
    local.get $b
    i32.const 125
    i32.eq
    i32.or
    local.get $b
    i32.const 91
    i32.eq
    i32.or
    local.get $b
    i32.const 93
    i32.eq
    i32.or
    local.get $b
    i32.const 45
    i32.eq
    i32.or
    local.get $b
    i32.const 96
    i32.eq
    i32.or
    local.get $b
    i32.const 126
    i32.eq
    i32.or
    local.get $b
    i32.const 9
    i32.eq
    i32.or
    local.get $b
    i32.const 10
    i32.eq
    i32.or
    local.get $b
    i32.const 13
    i32.eq
    i32.or)

  ;; Returns 0 ok, 1 empty, 2 forbidden byte, 3 leading/trailing dot,
  ;; 4 consecutive dot, 5 segment starts with digit.
  (func (export "jni_namespace_status") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $b i32)
    (local $seg_start i32)
    local.get $len
    i32.eqz
    if
      i32.const 1
      return
    end
    local.get $ptr
    i32.load8_u
    i32.const 46
    i32.eq
    local.get $ptr
    local.get $len
    i32.add
    i32.const 1
    i32.sub
    i32.load8_u
    i32.const 46
    i32.eq
    i32.or
    if
      i32.const 3
      return
    end
    i32.const 1
    local.set $seg_start
    (loop $scan
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.set $b
        local.get $b
        call $forbidden_namespace_byte
        if
          i32.const 2
          return
        end
        local.get $seg_start
        local.get $b
        call $is_digit
        i32.and
        if
          i32.const 5
          return
        end
        local.get $b
        i32.const 46
        i32.eq
        if
          local.get $seg_start
          if
            i32.const 4
            return
          end
          i32.const 1
          local.set $seg_start
        else
          i32.const 0
          local.set $seg_start
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end)
    i32.const 0)

  ;; Identifier mangling category: 1 keep ASCII alnum, 2 '_' => _1,
  ;; 3 '.' => '_', 4 encode as _0xxxx.
  (func (export "jni_identifier_mangle_kind") (param $b i32) (result i32)
    local.get $b
    call $is_ascii_alnum
    if
      i32.const 1
      return
    end
    local.get $b
    i32.const 95
    i32.eq
    if
      i32.const 2
      return
    end
    local.get $b
    i32.const 46
    i32.eq
    if
      i32.const 3
      return
    end
    i32.const 4)

  ;; Signature arg mangling category: 1 keep ASCII alnum, 2 '_' => _1,
  ;; 3 '/' => '_', 4 encode as _0xxxx, 5 ';' => _2, 6 '[' => _3.
  (func (export "jni_signature_arg_mangle_kind") (param $b i32) (result i32)
    local.get $b
    call $is_ascii_alnum
    if
      i32.const 1
      return
    end
    local.get $b
    i32.const 95
    i32.eq
    if
      i32.const 2
      return
    end
    local.get $b
    i32.const 47
    i32.eq
    if
      i32.const 3
      return
    end
    local.get $b
    i32.const 59
    i32.eq
    if
      i32.const 5
      return
    end
    local.get $b
    i32.const 91
    i32.eq
    if
      i32.const 6
      return
    end
    i32.const 4)

  ;; lowerCamel conversion action: 0 unchanged, 1 preserve because uppercase,
  ;; 2 all underscores, 3 remove one leading underscore, 4 convert underscores.
  (func (export "jni_snake_camel_action") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $b i32)
    (local $seen_non_us i32)
    (local $seen_us i32)
    (local $leading_us i32)
    (loop $scan
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.set $b
        local.get $b
        call $is_upper_ascii
        if
          i32.const 1
          return
        end
        local.get $b
        i32.const 95
        i32.eq
        if
          i32.const 1
          local.set $seen_us
          local.get $seen_non_us
          i32.eqz
          if
            local.get $leading_us
            i32.const 1
            i32.add
            local.set $leading_us
          end
        else
          i32.const 1
          local.set $seen_non_us
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end)
    local.get $seen_non_us
    i32.eqz
    if
      i32.const 2
      return
    end
    local.get $leading_us
    i32.const 0
    i32.gt_u
    if
      i32.const 3
      return
    end
    local.get $seen_us
    if
      i32.const 4
      return
    end
    i32.const 0)

  ;; Native macro validation status: 0 ok, 1 missing name, 2 missing signature,
  ;; 3 missing function path, 4 export needs java_type, 5 raw cannot use policy,
  ;; 6 raw cannot catch_unwind. export_mode: 0 no, 1 auto, 2 explicit.
  (func (export "jni_native_method_status")
    (param $has_name i32) (param $has_sig i32) (param $has_fn i32)
    (param $is_raw i32) (param $has_error_policy i32) (param $has_catch_unwind i32)
    (param $export_mode i32) (param $has_java_type i32)
    (result i32)
    local.get $has_name
    i32.eqz
    if
      i32.const 1
      return
    end
    local.get $has_sig
    i32.eqz
    if
      i32.const 2
      return
    end
    local.get $has_fn
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $export_mode
    i32.const 1
    i32.eq
    local.get $has_java_type
    i32.eqz
    i32.and
    if
      i32.const 4
      return
    end
    local.get $is_raw
    local.get $has_error_policy
    i32.and
    if
      i32.const 5
      return
    end
    local.get $is_raw
    local.get $has_catch_unwind
    i32.and
    if
      i32.const 6
      return
    end
    i32.const 0)

  ;; Wrapper shape: low bits 1 raw, 2 wrapped with_env, 3 wrapped no_catch.
  ;; Add 16 when exported and 32 when ABI check is required.
  (func (export "jni_native_wrapper_shape")
    (param $is_raw i32) (param $catch_unwind i32) (param $export_mode i32) (param $abi_check i32)
    (result i32)
    (local $shape i32)
    local.get $is_raw
    if
      i32.const 1
      local.set $shape
    else
      local.get $catch_unwind
      if
        i32.const 2
        local.set $shape
      else
        i32.const 3
        local.set $shape
      end
    end
    local.get $export_mode
    i32.const 0
    i32.gt_u
    if
      local.get $shape
      i32.const 16
      i32.or
      local.set $shape
    end
    local.get $abi_check
    if
      local.get $shape
      i32.const 32
      i32.or
      local.set $shape
    end
    local.get $shape)

  ;; JNI call macro postcondition: 0 ok, 1 pending exception before call,
  ;; 2 exception after call, 3 null return, 4 caught exception mapped.
  (func (export "jni_call_guard_result")
    (param $pre_exception i32) (param $post_exception i32) (param $ret_is_null i32) (param $catch_mode i32)
    (result i32)
    local.get $pre_exception
    if
      i32.const 1
      return
    end
    local.get $post_exception
    if
      local.get $catch_mode
      if
        i32.const 4
        return
      end
      i32.const 2
      return
    end
    local.get $ret_is_null
    if
      i32.const 3
      return
    end
    i32.const 0)

  ;; Reference categories: 1 local, 2 global, 3 weak global, 4 auto-local,
  ;; 5 checked cast, 6 class-loader context.
  (func (export "jni_reference_category") (param $kind i32) (result i32)
    local.get $kind
    i32.const 1
    i32.eq
    if (result i32) i32.const 1 else
      local.get $kind
      i32.const 2
      i32.eq
      if (result i32) i32.const 2 else
        local.get $kind
        i32.const 3
        i32.eq
        if (result i32) i32.const 3 else
          local.get $kind
          i32.const 4
          i32.eq
          if (result i32) i32.const 4 else
            local.get $kind
            i32.const 5
            i32.eq
            if (result i32) i32.const 5 else
              local.get $kind
              i32.const 6
              i32.eq
              if (result i32) i32.const 6 else i32.const 0 end
            end
          end
        end
      end
    end)

  ;; LoaderContext lookup plan bitset: 1 TCCL, 2 direct loader,
  ;; 4 candidate object's loader, 8 FindClass fallback.
  (func (export "jni_loader_plan") (param $context i32) (result i32)
    local.get $context
    i32.const 1
    i32.eq
    if
      i32.const 2
      return
    end
    local.get $context
    i32.const 2
    i32.eq
    if
      i32.const 13
      return
    end
    i32.const 9)

  ;; JValue descriptor category: 1 object, 2 array, 3 primitive, 4 void, 0 bad.
  (func (export "jni_jvalue_kind_from_descriptor") (param $b i32) (result i32)
    local.get $b
    i32.const 76
    i32.eq
    if
      i32.const 1
      return
    end
    local.get $b
    i32.const 91
    i32.eq
    if
      i32.const 2
      return
    end
    local.get $b
    i32.const 86
    i32.eq
    if
      i32.const 4
      return
    end
    local.get $b
    i32.const 90
    i32.eq
    local.get $b
    i32.const 66
    i32.eq
    i32.or
    local.get $b
    i32.const 67
    i32.eq
    i32.or
    local.get $b
    i32.const 68
    i32.eq
    i32.or
    local.get $b
    i32.const 70
    i32.eq
    i32.or
    local.get $b
    i32.const 73
    i32.eq
    i32.or
    local.get $b
    i32.const 74
    i32.eq
    i32.or
    local.get $b
    i32.const 83
    i32.eq
    i32.or
    if
      i32.const 3
      return
    end
    i32.const 0)

  ;; JNI status code mapping from jni_error_code_to_result.
  ;; 0 ok, 1 unknown, 2 detached, 3 version, 4 no-memory, 5 exists, 6 invalid, 7 other.
  (func (export "jni_error_code_category") (param $code i32) (result i32)
    local.get $code
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $code
    i32.const -1
    i32.eq
    if
      i32.const 1
      return
    end
    local.get $code
    i32.const -2
    i32.eq
    if
      i32.const 2
      return
    end
    local.get $code
    i32.const -3
    i32.eq
    if
      i32.const 3
      return
    end
    local.get $code
    i32.const -4
    i32.eq
    if
      i32.const 4
      return
    end
    local.get $code
    i32.const -5
    i32.eq
    if
      i32.const 5
      return
    end
    local.get $code
    i32.const -6
    i32.eq
    if
      i32.const 6
      return
    end
    i32.const 7)

  ;; ReleaseMode: 0 copy back, 2 JNI_ABORT/no-copy-back, else invalid.
  (func (export "jni_release_mode_category") (param $mode i32) (result i32)
    local.get $mode
    i32.eqz
    if
      i32.const 1
      return
    end
    local.get $mode
    i32.const 2
    i32.eq
    if
      i32.const 2
      return
    end
    i32.const 0)

  ;; Raw JNI version constants expose major in bits 16..23 and minor in bits 0..7.
  (func (export "jni_version_major") (param $ver i32) (result i32)
    local.get $ver
    i32.const 16
    i32.shr_u
    i32.const 255
    i32.and)

  (func (export "jni_version_minor") (param $ver i32) (result i32)
    local.get $ver
    i32.const 255
    i32.and)
)
