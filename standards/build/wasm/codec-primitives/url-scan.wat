(module
  (memory (export "memory") 1)

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300104)

  ;; Status values: 0 ok, 3 invalid.
  ;; Record layout, all little-endian u32:
  ;;  0 scheme_off,  4 scheme_len
  ;;  8 authority_off, 12 authority_len
  ;; 16 host_off, 20 host_len
  ;; 24 path_off, 28 path_len
  ;; 32 query_off, 36 query_len
  ;; 40 fragment_off, 44 fragment_len
  ;; 48 port, 52 has_port
  (func $write_record
    (param $out i32)
    (param $scheme_off i32) (param $scheme_len i32)
    (param $authority_off i32) (param $authority_len i32)
    (param $host_off i32) (param $host_len i32)
    (param $path_off i32) (param $path_len i32)
    (param $query_off i32) (param $query_len i32)
    (param $fragment_off i32) (param $fragment_len i32)
    (param $port i32) (param $has_port i32)
    local.get $out
    local.get $scheme_off
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $scheme_len
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $authority_off
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $authority_len
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $host_off
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $host_len
    i32.store
    local.get $out
    i32.const 24
    i32.add
    local.get $path_off
    i32.store
    local.get $out
    i32.const 28
    i32.add
    local.get $path_len
    i32.store
    local.get $out
    i32.const 32
    i32.add
    local.get $query_off
    i32.store
    local.get $out
    i32.const 36
    i32.add
    local.get $query_len
    i32.store
    local.get $out
    i32.const 40
    i32.add
    local.get $fragment_off
    i32.store
    local.get $out
    i32.const 44
    i32.add
    local.get $fragment_len
    i32.store
    local.get $out
    i32.const 48
    i32.add
    local.get $port
    i32.store
    local.get $out
    i32.const 52
    i32.add
    local.get $has_port
    i32.store)

  (func $is_scheme_byte (param $b i32) (result i32)
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
    i32.or
    local.get $b
    i32.const 43
    i32.eq
    i32.or
    local.get $b
    i32.const 45
    i32.eq
    i32.or
    local.get $b
    i32.const 46
    i32.eq
    i32.or)

  (func $is_digit (param $b i32) (result i32)
    local.get $b
    i32.const 48
    i32.ge_u
    local.get $b
    i32.const 57
    i32.le_u
    i32.and)

  (func $is_ascii_ws (param $b i32) (result i32)
    local.get $b
    i32.const 32
    i32.eq
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

  (func $url_scheme_validate (export "url_scheme_validate") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    local.get $len
    i32.eqz
    if
      i32.const 3
      return
    end
    i32.const 0
    local.set $i
    (block $done
      (loop $loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        call $is_scheme_byte
        i32.eqz
        if
          i32.const 3
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop))
    i32.const 0)

  (func (export "url_scan") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $start i32)
    (local $end i32)
    (local $i i32)
    (local $b i32)
    (local $colon i32)
    (local $auth_start i32)
    (local $auth_end i32)
    (local $last_colon i32)
    (local $host_len i32)
    (local $port i32)
    (local $digit i32)
    (local $has_port i32)
    (local $path_off i32)
    (local $path_len i32)
    (local $query_off i32)
    (local $query_len i32)
    (local $fragment_off i32)
    (local $fragment_len i32)

    i32.const 0
    local.set $start
    local.get $len
    local.set $end

    (block $trim_start_done
      (loop $trim_start
        local.get $start
        local.get $end
        i32.ge_u
        br_if $trim_start_done
        local.get $ptr
        local.get $start
        i32.add
        i32.load8_u
        call $is_ascii_ws
        i32.eqz
        br_if $trim_start_done
        local.get $start
        i32.const 1
        i32.add
        local.set $start
        br $trim_start))

    (block $trim_end_done
      (loop $trim_end
        local.get $end
        local.get $start
        i32.le_u
        br_if $trim_end_done
        local.get $ptr
        local.get $end
        i32.const 1
        i32.sub
        i32.add
        i32.load8_u
        call $is_ascii_ws
        i32.eqz
        br_if $trim_end_done
        local.get $end
        i32.const 1
        i32.sub
        local.set $end
        br $trim_end))

    local.get $start
    local.get $end
    i32.ge_u
    if
      i32.const 3
      return
    end

    i32.const -1
    local.set $colon
    local.get $start
    local.set $i
    (block $scheme_done
      (loop $scheme_scan
        local.get $i
        local.get $end
        i32.ge_u
        br_if $scheme_done
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.tee $b
        i32.const 58
        i32.eq
        if
          local.get $i
          local.set $colon
          br $scheme_done
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scheme_scan))

    local.get $colon
    i32.const -1
    i32.eq
    if
      i32.const 3
      return
    end

    local.get $ptr
    local.get $start
    i32.add
    local.get $colon
    local.get $start
    i32.sub
    call $url_scheme_validate
    if
      i32.const 3
      return
    end

    local.get $colon
    i32.const 3
    i32.add
    local.get $end
    i32.gt_u
    if
      i32.const 3
      return
    end
    local.get $ptr
    local.get $colon
    i32.const 1
    i32.add
    i32.add
    i32.load8_u
    i32.const 47
    i32.ne
    if
      i32.const 3
      return
    end
    local.get $ptr
    local.get $colon
    i32.const 2
    i32.add
    i32.add
    i32.load8_u
    i32.const 47
    i32.ne
    if
      i32.const 3
      return
    end

    local.get $colon
    i32.const 3
    i32.add
    local.set $auth_start
    local.get $auth_start
    local.set $auth_end
    i32.const -1
    local.set $last_colon

    (block $auth_done
      (loop $auth_scan
        local.get $auth_end
        local.get $end
        i32.ge_u
        br_if $auth_done
        local.get $ptr
        local.get $auth_end
        i32.add
        i32.load8_u
        local.tee $b
        i32.const 47
        i32.eq
        local.get $b
        i32.const 63
        i32.eq
        i32.or
        local.get $b
        i32.const 35
        i32.eq
        i32.or
        br_if $auth_done
        local.get $b
        i32.const 64
        i32.eq
        local.get $b
        i32.const 91
        i32.eq
        i32.or
        local.get $b
        i32.const 93
        i32.eq
        i32.or
        if
          i32.const 3
          return
        end
        local.get $b
        i32.const 58
        i32.eq
        if
          local.get $auth_end
          local.set $last_colon
        end
        local.get $auth_end
        i32.const 1
        i32.add
        local.set $auth_end
        br $auth_scan))

    local.get $auth_start
    local.get $auth_end
    i32.ge_u
    if
      i32.const 3
      return
    end

    local.get $auth_end
    local.get $auth_start
    i32.sub
    local.set $host_len
    i32.const 0
    local.set $port
    i32.const 0
    local.set $has_port

    local.get $last_colon
    i32.const -1
    i32.ne
    if
      local.get $last_colon
      local.get $auth_start
      i32.eq
      if
        i32.const 3
        return
      end
      local.get $last_colon
      i32.const 1
      i32.add
      local.get $auth_end
      i32.ge_u
      if
        i32.const 3
        return
      end
      local.get $last_colon
      local.get $auth_start
      i32.sub
      local.set $host_len
      i32.const 1
      local.set $has_port
      i32.const 0
      local.set $port
      local.get $last_colon
      i32.const 1
      i32.add
      local.set $i
      (block $port_done
        (loop $port_loop
          local.get $i
          local.get $auth_end
          i32.ge_u
          br_if $port_done
          local.get $ptr
          local.get $i
          i32.add
          i32.load8_u
          local.tee $b
          call $is_digit
          i32.eqz
          if
            i32.const 3
            return
          end
          local.get $b
          i32.const 48
          i32.sub
          local.set $digit
          local.get $port
          i32.const 6553
          i32.gt_u
          if
            i32.const 3
            return
          end
          local.get $port
          i32.const 6553
          i32.eq
          local.get $digit
          i32.const 5
          i32.gt_u
          i32.and
          if
            i32.const 3
            return
          end
          local.get $port
          i32.const 10
          i32.mul
          local.get $digit
          i32.add
          local.set $port
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $port_loop))
    end

    local.get $auth_end
    local.set $i
    local.get $i
    local.set $path_off
    i32.const 0
    local.set $path_len
    local.get $i
    local.set $query_off
    i32.const 0
    local.set $query_len
    local.get $i
    local.set $fragment_off
    i32.const 0
    local.set $fragment_len

    local.get $i
    local.get $end
    i32.lt_u
    if
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      i32.const 47
      i32.eq
      if
        local.get $i
        local.set $path_off
        (block $path_done
          (loop $path_scan
            local.get $i
            local.get $end
            i32.ge_u
            br_if $path_done
            local.get $ptr
            local.get $i
            i32.add
            i32.load8_u
            local.tee $b
            i32.const 63
            i32.eq
            local.get $b
            i32.const 35
            i32.eq
            i32.or
            br_if $path_done
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $path_scan))
        local.get $i
        local.get $path_off
        i32.sub
        local.set $path_len
      end
    end

    local.get $i
    local.get $end
    i32.lt_u
    if
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      i32.const 63
      i32.eq
      if
        local.get $i
        i32.const 1
        i32.add
        local.set $query_off
        local.get $query_off
        local.set $i
        (block $query_done
          (loop $query_scan
            local.get $i
            local.get $end
            i32.ge_u
            br_if $query_done
            local.get $ptr
            local.get $i
            i32.add
            i32.load8_u
            i32.const 35
            i32.eq
            br_if $query_done
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $query_scan))
        local.get $i
        local.get $query_off
        i32.sub
        local.set $query_len
      end
    end

    local.get $i
    local.get $end
    i32.lt_u
    if
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      i32.const 35
      i32.eq
      if
        local.get $i
        i32.const 1
        i32.add
        local.set $fragment_off
        local.get $end
        local.get $fragment_off
        i32.sub
        local.set $fragment_len
      else
        i32.const 3
        return
      end
    end

    local.get $out
    local.get $start
    local.get $colon
    local.get $start
    i32.sub
    local.get $auth_start
    local.get $auth_end
    local.get $auth_start
    i32.sub
    local.get $auth_start
    local.get $host_len
    local.get $path_off
    local.get $path_len
    local.get $query_off
    local.get $query_len
    local.get $fragment_off
    local.get $fragment_len
    local.get $port
    local.get $has_port
    call $write_record
    i32.const 0)
)
