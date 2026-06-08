;; ── CLI Args: argument parsing via WASI args_sizes_get / args_get ──
  ;; The argument pointers are stored at 0x20 + idx*4 (up to 64 args).
  ;; The argument strings are stored starting at 0x200.

  (global $ARGV_BUF i32 (i32.const 0x20))
  (global $ARGV_STR_BUF i32 (i32.const 0x200))
  (global $ARGV_MAX i32 (i32.const 64))
  (global $ARGV_STR_MAX i32 (i32.const 0xE00))

  (global $argc (mut i32) (i32.const -1))
  (global $argc_fetched (mut i32) (i32.const 0))

  ;; Fetch argv from runtime. Called once, cached.
  (func $fetch_argv
    (local $ret i32)
    global.get $argc_fetched
    if return end
    global.get $ARGV_BUF
    global.get $ARGV_STR_BUF
    call $args_get
    local.set $ret
    global.get $ret
    i32.eqz
    if
      global.get $ARGV_BUF
      i32.load
      global.set $argc
    end
    i32.const 1
    global.set $argc_fetched)

  ;; Get argument count.
  (func $get_argc (export "get_argc") (result i32)
    global.get $argc_fetched
    if (result i32)
      global.get $argc
    else
      (local $ret i32)
      (local $count i32)
      (local $size i32)
      global.get $ARGV_BUF
      global.get $ARGV_STR_BUF
      call $args_sizes_get
      local.set $ret
      global.get $ret
      if (result i32)
        i32.const 0
      else
        global.get $ARGV_BUF
        i32.load
        local.set $argc
        i32.const 1
        global.set $argc_fetched
        global.get $argc
      end
    end)

  ;; Copy argument at index into buffer. Returns actual string length.
  (func $get_argv (export "get_argv") (param $idx i32) (param $out i32) (param $maxlen i32) (result i32)
    (local $src i32)
    (local $len i32)
    (local $p i32)
    call $fetch_argv
    global.get $ARGV_BUF
    i32.const 1
    i32.add
    local.get $idx
    i32.const 2
    i32.shl
    i32.add
    i32.load
    local.set $src
    local.get $maxlen
    i32.const 0
    i32.gt_s
    if
      block $done
        loop $copy
          local.get $len
          local.get $maxlen
          i32.ge_s
          br_if $done
          local.get $src
          local.get $len
          i32.add
          i32.load8_u
          local.tee $p
          i32.eqz
          br_if $done
          local.get $out
          local.get $len
          i32.add
          local.get $p
          i32.store8
          local.get $len
          i32.const 1
          i32.add
          local.set $len
          br $copy
        end
      end
    end
    local.get $len)
