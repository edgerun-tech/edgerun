;; ── CLI I/O: string and number output, stdin reading ──

  ;; Write null-terminated string to stdout.
  (func $print_str (export "print_str") (param $s i32)
    (local $len i32)
    (local $p i32)
    local.get $s
    local.set $p
    block $done
      loop $scan
        local.get $p
        i32.load8_u
        local.tee $len
        i32.eqz
        br_if $done
        local.get $p
        i32.const 1
        i32.add
        local.set $p
        br $scan
      end
    end
    local.get $p
    local.get $s
    i32.sub
    local.set $len
    i32.const 1
    local.get $s
    local.get $len
    call $write_all
    drop)

  ;; Write buffer to stdout.
  (func $print_chars (export "print_chars") (param $buf i32) (param $len i32)
    i32.const 1
    local.get $buf
    local.get $len
    call $write_all
    drop)

  ;; Write newline to stdout.
  (func $println (export "println")
    i32.const 1
    i32.const 10
    i32.const 1
    call $write_all
    drop)

  ;; Write i32 as decimal to stdout.
  (func $print_i32 (export "print_i32") (param $val i32)
    (local $buf i32)
    (local $pos i32)
    (local $neg i32)
    (local $digit i32)
    i32.const 16
    local.set $buf
    i32.const 16
    i32.const 10
    i32.add
    local.set $pos
    local.get $val
    i32.const 0
    i32.lt_s
    local.set $neg
    local.get $val
    i32.const 0
    i32.lt_s
    if
      i32.const 0
      local.get $val
      i32.sub
      local.set $val
    end
    block $done
      loop $divide
        local.get $pos
        i32.const 1
        i32.sub
        local.tee $pos
        local.get $val
        i32.const 10
        i32.rem_u
        local.tee $digit
        i32.const 48
        i32.add
        i32.store8
        local.get $val
        i32.const 10
        i32.div_u
        local.set $val
        local.get $val
        i32.eqz
        br_if $done
        br $divide
      end
    end
    local.get $neg
    if
      local.get $pos
      i32.const 1
      i32.sub
      local.tee $pos
      i32.const 45
      i32.store8
    end
    i32.const 1
    local.get $pos
    local.get $buf
    i32.const 16
    i32.add
    local.get $pos
    i32.sub
    call $write_all
    drop)

  ;; Write i32 as hex to stdout (with 0x prefix).
  (func $print_hex (export "print_hex") (param $val i32)
    (local $i i32)
    (local $nib i32)
    ;; Write "0x"
    i32.const 1
    i32.const 16
    i32.const 2
    call $write_all
    drop
    ;; Convert nibbles to hex chars at buffer[16..23]
    i32.const 0
    local.set $i
    block $done
      loop $emit
        local.get $i
        i32.const 8
        i32.ge_s
        br_if $done
        local.get $val
        i32.const 28
        local.get $i
        i32.const 2
        i32.shl
        i32.sub
        i32.shr_u
        i32.const 15
        i32.and
        local.tee $nib
        i32.const 10
        i32.lt_s
        if (result i32)
          local.get $nib
          i32.const 48
          i32.add
        else
          local.get $nib
          i32.const 87
          i32.add
        end
        local.set $nib
        i32.const 16
        local.get $i
        i32.add
        local.get $nib
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $emit
      end
    end
    i32.const 1
    i32.const 16
    i32.const 8
    call $write_all
    drop)

  ;; Read from stdin into buffer. Returns bytes read (or error code).
  (func $read_stdin (export "read_stdin") (param $buf i32) (param $maxlen i32) (result i32)
    i32.const 0
    local.get $buf
    i32.store
    i32.const 4
    local.get $maxlen
    i32.store
    i32.const 0
    i32.const 0
    i32.const 1
    i32.const 8
    call $fd_read
    if (result i32)
      i32.const -1
    else
      i32.const 8
      i32.load
    end)
