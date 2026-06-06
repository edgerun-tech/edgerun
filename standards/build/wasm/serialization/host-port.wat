  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300060)

  (func $m101write_record
    (param $out i32)
    (param $host_off i32)
    (param $host_len i32)
    (param $port i32)
    (param $has_explicit_port i32)
    local.get $out
    local.get $host_off
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $host_len
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $port
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $has_explicit_port
    i32.store)


  ;; Scan host[:port] using the last colon as the separator, matching the Rust
  ;; parse_host_port_with_default helper. Output offsets are relative to ptr.
  (func (export "host_port_scan")
    (param $ptr i32)
    (param $len i32)
    (param $default_port i32)
    (param $out i32)
    (result i32)
    (local $i i32)
    (local $colon i32)
    (local $b i32)
    (local $digit i32)
    (local $port i32)

    local.get $len
    i32.eqz
    if
      i32.const 3
      return
    end

    i32.const -1
    local.set $colon
    i32.const 0
    local.set $i

    (block $scan_done
      (loop $scan
        local.get $i
        local.get $len
        i32.ge_u
        br_if $scan_done

        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 58
        i32.eq
        if
          local.get $i
          local.set $colon
        end

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan))

    local.get $colon
    i32.const -1
    i32.eq
    if
      local.get $out
      i32.const 0
      local.get $len
      local.get $default_port
      i32.const 0
      call $m101write_record
      i32.const 0
      return
    end

    local.get $colon
    i32.eqz
    if
      i32.const 3
      return
    end

    local.get $colon
    i32.const 1
    i32.add
    local.get $len
    i32.ge_u
    if
      i32.const 3
      return
    end

    local.get $colon
    i32.const 1
    i32.add
    local.set $i
    i32.const 0
    local.set $port

    (block $port_done
      (loop $port_loop
        local.get $i
        local.get $len
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

    local.get $out
    i32.const 0
    local.get $colon
    local.get $port
    i32.const 1
    call $m101write_record
    i32.const 0)