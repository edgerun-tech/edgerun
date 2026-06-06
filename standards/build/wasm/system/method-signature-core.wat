(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "pack" (func $pack (param i32 i32) (result i64)))

(global $parsed_kind (mut i32) (i32.const 0))

  (func (export "proto_standard_id") (result i32)
    i32.const 300157)

  (func $write_summary
    (param $rec i32) (param $kind i32) (param $arg_count i32)
    (param $ret_kind i32) (param $field_kind i32) (param $consumed i32)
    (param $error i32)
    local.get $rec
    local.get $kind
    i32.store
    local.get $rec
    i32.const 4
    i32.add
    local.get $arg_count
    i32.store
    local.get $rec
    i32.const 8
    i32.add
    local.get $ret_kind
    i32.store
    local.get $rec
    i32.const 12
    i32.add
    local.get $field_kind
    i32.store
    local.get $rec
    i32.const 16
    i32.add
    local.get $consumed
    i32.store
    local.get $rec
    i32.const 20
    i32.add
    local.get $error
    i32.store)

  (func $is_primitive (param $b i32) (result i32)
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
    local.get $b
    i32.const 86
    i32.eq
    i32.or)

  (func $is_forbidden_object_byte (param $b i32) (result i32)
    local.get $b
    i32.const 32
    i32.le_u
    local.get $b
    i32.const 46
    i32.eq
    i32.or
    local.get $b
    i32.const 59
    i32.eq
    i32.or
    local.get $b
    i32.const 91
    i32.eq
    i32.or
    local.get $b
    i32.const 47
    i32.eq
    i32.or
    local.get $b
    i32.const 40
    i32.eq
    i32.or
    local.get $b
    i32.const 41
    i32.eq
    i32.or)

  ;; Parse one object type at offset i.
  ;; Status: 0 ok, 1 empty, 3 missing semicolon, 4 bad object byte.
  ;; On success parsed_kind = 9 and high u32 is the next offset.
  (func $m121parse_object (param $ptr i32) (param $len i32) (param $i i32) (result i64)
    (local $b i32)
    (local $segment_len i32)
    local.get $i
    i32.const 1
    i32.add
    local.set $i
    i32.const 0
    local.set $segment_len
    (block $done
      (loop $scan
        local.get $i
        local.get $len
        i32.ge_u
        if
          i32.const 3
          local.get $i
          call $pack
          return
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.set $b

        local.get $b
        i32.const 59
        i32.eq
        if
          local.get $segment_len
          i32.eqz
          if
            i32.const 1
            local.get $i
            call $pack
            return
          end
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $done
        end

        local.get $b
        i32.const 47
        i32.eq
        if
          local.get $segment_len
          i32.eqz
          if
            i32.const 1
            local.get $i
            call $pack
            return
          end
          i32.const 0
          local.set $segment_len
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $scan
        end

        local.get $b
        call $is_forbidden_object_byte
        if
          i32.const 4
          local.get $i
          call $pack
          return
        end
        local.get $segment_len
        i32.const 1
        i32.add
        local.set $segment_len
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan))

    i32.const 9
    global.set $parsed_kind
    i32.const 0
    local.get $i
    call $pack)

  ;; Parse one JNI type at offset i. allow_void controls whether V is legal.
  ;; Kind values: primitive ASCII code, object 9, array 10.
  ;; Status: 0 ok, 1 empty/missing type, 2 void disallowed, 3 truncated,
  ;; 4 bad object byte, 5 unknown type.
  (func $parse_type (param $ptr i32) (param $len i32) (param $i i32) (param $allow_void i32) (result i64)
    (local $b i32)
    (local $step i64)
    local.get $i
    local.get $len
    i32.ge_u
    if
      i32.const 1
      local.get $i
      call $pack
      return
    end

    local.get $ptr
    local.get $i
    i32.add
    i32.load8_u
    local.set $b

    local.get $b
    call $is_primitive
    if
      local.get $b
      i32.const 86
      i32.eq
      local.get $allow_void
      i32.eqz
      i32.and
      if
        i32.const 2
        local.get $i
        call $pack
        return
      end
      local.get $b
      global.set $parsed_kind
      i32.const 0
      local.get $i
      i32.const 1
      i32.add
      call $pack
      return
    end

    local.get $b
    i32.const 76
    i32.eq
    if
      local.get $ptr
      local.get $len
      local.get $i
      call $m121parse_object
      return
    end

    local.get $b
    i32.const 91
    i32.eq
    if
      local.get $ptr
      local.get $len
      local.get $i
      i32.const 1
      i32.add
      i32.const 0
      call $parse_type
      local.set $step
      local.get $step
      i32.wrap_i64
      i32.eqz
      if
        i32.const 10
        global.set $parsed_kind
        i32.const 0
        local.get $step
        i64.const 32
        i64.shr_u
        i32.wrap_i64
        call $pack
        return
      end
      local.get $step
      return
    end

    i32.const 5
    local.get $i
    call $pack)

  ;; Validate and summarize a field or method signature.
  ;;
  ;; Return: packed low u32 status, high u32 consumed length.
  ;; Status: 0 ok, 1 empty/missing type, 2 void disallowed, 3 delimiter missing,
  ;; 4 bad object byte, 5 unknown type, 6 trailing input, 7 bad mode.
  ;;
  ;; mode: 0 auto, 1 field, 2 method.
  ;; Summary record u32[6]: kind (1 field, 2 method), arg_count, ret_kind,
  ;; field_kind, consumed, error. Type kinds are primitive ASCII codes, object 9,
  ;; and array 10.
  (func (export "jni_signature_scan")
    (param $ptr i32) (param $len i32) (param $mode i32) (param $rec i32)
    (result i64)
    (local $i i32)
    (local $b i32)
    (local $step i64)
    (local $status i32)
    (local $next i32)
    (local $arg_count i32)
    (local $ret_kind i32)
    (local $field_kind i32)
    (local $kind i32)

    local.get $len
    i32.eqz
    if
      local.get $rec
      i32.const 0
      i32.const 0
      i32.const 0
      i32.const 0
      i32.const 0
      i32.const 1
      call $write_summary
      i32.const 1
      i32.const 0
      call $pack
      return
    end

    local.get $mode
    i32.eqz
    if
      local.get $ptr
      i32.load8_u
      i32.const 40
      i32.eq
      if
        i32.const 2
        local.set $mode
      else
        i32.const 1
        local.set $mode
      end
    end

    local.get $mode
    i32.const 1
    i32.eq
    if
      local.get $ptr
      local.get $len
      i32.const 0
      i32.const 0
      call $parse_type
      local.set $step
      local.get $step
      i32.wrap_i64
      local.set $status
      local.get $step
      i64.const 32
      i64.shr_u
      i32.wrap_i64
      local.set $next
      local.get $status
      i32.eqz
      if
        local.get $next
        local.get $len
        i32.ne
        if
          i32.const 6
          local.set $status
        end
      end
      global.get $parsed_kind
      local.set $field_kind
      local.get $rec
      i32.const 1
      i32.const 0
      i32.const 0
      local.get $field_kind
      local.get $next
      local.get $status
      call $write_summary
      local.get $status
      local.get $next
      call $pack
      return
    end

    local.get $mode
    i32.const 2
    i32.ne
    if
      local.get $rec
      i32.const 0
      i32.const 0
      i32.const 0
      i32.const 0
      i32.const 0
      i32.const 7
      call $write_summary
      i32.const 7
      i32.const 0
      call $pack
      return
    end

    local.get $ptr
    i32.load8_u
    i32.const 40
    i32.ne
    if
      local.get $rec
      i32.const 2
      i32.const 0
      i32.const 0
      i32.const 0
      i32.const 0
      i32.const 3
      call $write_summary
      i32.const 3
      i32.const 0
      call $pack
      return
    end

    i32.const 1
    local.set $i
    i32.const 0
    local.set $arg_count

    (block $args_done
      (loop $args
        local.get $i
        local.get $len
        i32.ge_u
        if
          local.get $rec
          i32.const 2
          local.get $arg_count
          i32.const 0
          i32.const 0
          local.get $i
          i32.const 3
          call $write_summary
          i32.const 3
          local.get $i
          call $pack
          return
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.set $b
        local.get $b
        i32.const 41
        i32.eq
        if
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $args_done
        end
        local.get $ptr
        local.get $len
        local.get $i
        i32.const 0
        call $parse_type
        local.set $step
        local.get $step
        i32.wrap_i64
        local.set $status
        local.get $step
        i64.const 32
        i64.shr_u
        i32.wrap_i64
        local.set $next
        local.get $status
        i32.eqz
        i32.eqz
        if
          local.get $rec
          i32.const 2
          local.get $arg_count
          i32.const 0
          i32.const 0
          local.get $next
          local.get $status
          call $write_summary
          local.get $status
          local.get $next
          call $pack
          return
        end
        local.get $arg_count
        i32.const 1
        i32.add
        local.set $arg_count
        local.get $next
        local.set $i
        br $args))

    local.get $ptr
    local.get $len
    local.get $i
    i32.const 1
    call $parse_type
    local.set $step
    local.get $step
    i32.wrap_i64
    local.set $status
    local.get $step
    i64.const 32
    i64.shr_u
    i32.wrap_i64
    local.set $next
    global.get $parsed_kind
    local.set $ret_kind

    local.get $status
    i32.eqz
    if
      local.get $next
      local.get $len
      i32.ne
      if
        i32.const 6
        local.set $status
      end
    end

    local.get $rec
    i32.const 2
    local.get $arg_count
    local.get $ret_kind
    i32.const 0
    local.get $next
    local.get $status
    call $write_summary
    local.get $status
    local.get $next
    call $pack)
)
