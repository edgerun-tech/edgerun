;; ── CLI Memory: bump allocator and string utilities ──

  (global $cli_heap_ptr (mut i32) (i32.const 0))
  ;; $CLI_HEAP_START and $HEAP_END come from config.wat

  ;; Allocate bytes from bump heap. Returns pointer, or 0 if OOM.
  (func $alloc (export "alloc") (param $size i32) (result i32)
    (local $ptr i32)
    (if (i32.eqz (global.get $cli_heap_ptr))
      (then (global.set $cli_heap_ptr (global.get $CLI_HEAP_START))))
    global.get $cli_heap_ptr
    local.set $ptr
    global.get $cli_heap_ptr
    local.get $size
    i32.add
    global.set $cli_heap_ptr
    global.get $cli_heap_ptr
    global.get $HEAP_END
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $ptr)

  ;; Copy string from src to dst, up to max bytes. Returns bytes written.
  (func $str_copy (export "str_copy") (param $dst i32) (param $src i32) (param $max i32) (result i32)
    (local $i i32)
    block $done
      loop $copy
        local.get $i
        local.get $max
        i32.ge_s
        br_if $done
        local.get $src
        local.get $i
        i32.add
        i32.load8_u
        local.tee $dst
        i32.eqz
        br_if $done
        local.get $dst
        local.get $i
        i32.add
        local.get $dst
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $copy
      end
    end
    local.get $i)

  ;; Get length of null-terminated string.
  (func $str_len (export "str_len") (param $s i32) (result i32)
    (local $len i32)
    block $done
      loop $scan
        local.get $s
        local.get $len
        i32.add
        i32.load8_u
        i32.eqz
        br_if $done
        local.get $len
        i32.const 1
        i32.add
        local.set $len
        br $scan
      end
    end
    local.get $len)

  ;; Compare two strings. Returns 0 if equal, non-zero otherwise.
  (func $str_cmp (export "str_cmp") (param $a i32) (param $b i32) (result i32)
    (local $ca i32)
    (local $cb i32)
    block $done
      loop $compare
        local.get $a
        i32.load8_u
        local.tee $ca
        local.get $b
        i32.load8_u
        local.tee $cb
        i32.ne
        br_if $done
        local.get $ca
        i32.eqz
        br_if $done
        local.get $a
        i32.const 1
        i32.add
        local.set $a
        local.get $b
        i32.const 1
        i32.add
        local.set $b
        br $compare
      end
    end
    local.get $ca
    local.get $cb
    i32.sub)
