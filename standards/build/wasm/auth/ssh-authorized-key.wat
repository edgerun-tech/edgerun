(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "is_digit" (func $is_digit (param i32) (result i32)))


  (func (export "proto_standard_id") (result i32)
    i32.const 300045)

  (func $m169is_space (param $b i32) (result i32)
    local.get $b
    i32.const 32
    i32.eq
    local.get $b
    i32.const 9
    i32.eq
    i32.or)

  (func $is_eol (param $b i32) (result i32)
    local.get $b
    i32.const 10
    i32.eq
    local.get $b
    i32.const 13
    i32.eq
    i32.or)

  (func $is_token_byte (param $b i32) (result i32)
    local.get $b
    call $m169is_space
    i32.eqz
    local.get $b
    call $is_eol
    i32.eqz
    i32.and)

  (func $m169is_upper (param $b i32) (result i32)
    local.get $b
    i32.const 65
    i32.ge_u
    local.get $b
    i32.const 90
    i32.le_u
    i32.and)

  (func $m169is_lower (param $b i32) (result i32)
    local.get $b
    i32.const 97
    i32.ge_u
    local.get $b
    i32.const 122
    i32.le_u
    i32.and)


  (func $is_b64_data (param $b i32) (result i32)
    local.get $b
    call $m169is_upper
    local.get $b
    call $m169is_lower
    i32.or
    local.get $b
    call $is_digit
    i32.or
    local.get $b
    i32.const 43
    i32.eq
    i32.or
    local.get $b
    i32.const 47
    i32.eq
    i32.or)

  (func (export "ssh_authorized_key_scan")
    (param $ptr i32) (param $len i32) (param $out i32)
    (result i32)
    (local $end i32)
    (local $i i32)
    (local $b i32)
    (local $type_start i32)
    (local $type_end i32)
    (local $b64_start i32)
    (local $b64_end i32)
    (local $comment_start i32)
    (local $pad_count i32)
    (local $saw_data i32)
    (local $in_padding i32)

    local.get $ptr
    local.get $len
    i32.add
    local.set $end
    local.get $ptr
    local.set $i

    ;; Leading indentation is allowed.
    (block $leading_done
      (loop $leading
        local.get $i
        local.get $end
        i32.ge_u
        br_if $leading_done
        local.get $i
        i32.load8_u
        local.set $b
        local.get $b
        call $m169is_space
        i32.eqz
        br_if $leading_done
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $leading))

    ;; Blank lines and comment lines are not authorized-key records.
    local.get $i
    local.get $end
    i32.ge_u
    if
      i32.const 3
      return
    end
    local.get $i
    i32.load8_u
    local.tee $b
    call $is_eol
    if
      i32.const 3
      return
    end
    local.get $b
    i32.const 35
    i32.eq
    if
      i32.const 3
      return
    end

    local.get $i
    local.set $type_start
    (block $type_done
      (loop $type_loop
        local.get $i
        local.get $end
        i32.ge_u
        br_if $type_done
        local.get $i
        i32.load8_u
        call $is_token_byte
        i32.eqz
        br_if $type_done
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $type_loop))
    local.get $i
    local.set $type_end
    local.get $type_end
    local.get $type_start
    i32.eq
    if
      i32.const 3
      return
    end

    ;; The key type must be followed by at least one space or tab.
    local.get $i
    local.get $end
    i32.ge_u
    if
      i32.const 3
      return
    end
    local.get $i
    i32.load8_u
    call $m169is_space
    i32.eqz
    if
      i32.const 3
      return
    end
    (block $after_type_ws
      (loop $type_ws
        local.get $i
        local.get $end
        i32.ge_u
        br_if $after_type_ws
        local.get $i
        i32.load8_u
        call $m169is_space
        i32.eqz
        br_if $after_type_ws
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $type_ws))

    local.get $i
    local.set $b64_start
    (block $b64_done
      (loop $b64_loop
        local.get $i
        local.get $end
        i32.ge_u
        br_if $b64_done
        local.get $i
        i32.load8_u
        local.set $b

        local.get $b
        call $m169is_space
        local.get $b
        call $is_eol
        i32.or
        br_if $b64_done

        local.get $b
        i32.const 61
        i32.eq
        if
          local.get $saw_data
          i32.eqz
          if
            i32.const 3
            return
          end
          local.get $pad_count
          i32.const 2
          i32.ge_u
          if
            i32.const 3
            return
          end
          local.get $pad_count
          i32.const 1
          i32.add
          local.set $pad_count
          i32.const 1
          local.set $in_padding
        else
          local.get $b
          call $is_b64_data
          i32.eqz
          if
            i32.const 3
            return
          end
          local.get $in_padding
          if
            i32.const 3
            return
          end
          i32.const 1
          local.set $saw_data
        end

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $b64_loop))
    local.get $i
    local.set $b64_end
    local.get $saw_data
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $b64_end
    local.get $b64_start
    i32.eq
    if
      i32.const 3
      return
    end

    ;; Optional comment begins after separating spaces and runs to CR/LF or end.
    (block $after_b64_ws
      (loop $b64_ws
        local.get $i
        local.get $end
        i32.ge_u
        br_if $after_b64_ws
        local.get $i
        i32.load8_u
        call $m169is_space
        i32.eqz
        br_if $after_b64_ws
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $b64_ws))
    local.get $i
    local.set $comment_start
    (block $comment_done
      (loop $comment_loop
        local.get $i
        local.get $end
        i32.ge_u
        br_if $comment_done
        local.get $i
        i32.load8_u
        call $is_eol
        br_if $comment_done
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $comment_loop))

    local.get $out
    local.get $type_start
    local.get $ptr
    i32.sub
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $type_end
    local.get $type_start
    i32.sub
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $b64_start
    local.get $ptr
    i32.sub
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $b64_end
    local.get $b64_start
    i32.sub
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $comment_start
    local.get $ptr
    i32.sub
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $i
    local.get $comment_start
    i32.sub
    i32.store

    i32.const 0)

)