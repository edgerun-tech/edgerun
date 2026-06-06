(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "pack" (func $pack (param i32 i32) (result i64)))
  (import "crypto-sha1" "sha1" (func $sha1 (param i32 i32 i32) (result i64)))

  (data (i32.const 62024) "258EAFA5-E914-47DA-95CA-C5AB0DC85B11")

  (global $MSG_ADDR i32 (i32.const 62000))
  (global $DIGEST_ADDR i32 (i32.const 62100))

  (func (export "proto_standard_id") (result i32)
    i32.const 300048)

  (func $b64val (param $ch i32) (result i32)
    local.get $ch
    i32.const 65
    i32.ge_u
    local.get $ch
    i32.const 90
    i32.le_u
    i32.and
    if
      local.get $ch
      i32.const 65
      i32.sub
      return
    end
    local.get $ch
    i32.const 97
    i32.ge_u
    local.get $ch
    i32.const 122
    i32.le_u
    i32.and
    if
      local.get $ch
      i32.const 71
      i32.sub
      return
    end
    local.get $ch
    i32.const 48
    i32.ge_u
    local.get $ch
    i32.const 57
    i32.le_u
    i32.and
    if
      local.get $ch
      i32.const 4
      i32.add
      return
    end
    local.get $ch
    i32.const 43
    i32.eq
    if
      i32.const 62
      return
    end
    local.get $ch
    i32.const 47
    i32.eq
    if
      i32.const 63
      return
    end
    i32.const -1)

  (func $b64char (param $value i32) (result i32)
    local.get $value
    i32.const 26
    i32.lt_u
    if
      local.get $value
      i32.const 65
      i32.add
      return
    end
    local.get $value
    i32.const 52
    i32.lt_u
    if
      local.get $value
      i32.const 71
      i32.add
      return
    end
    local.get $value
    i32.const 62
    i32.lt_u
    if
      local.get $value
      i32.const 4
      i32.sub
      return
    end
    local.get $value
    i32.const 62
    i32.eq
    if
      i32.const 43
      return
    end
    i32.const 47)

  (func $emit_b64_3 (param $b0 i32) (param $b1 i32) (param $b2 i32) (param $out_ptr i32)
    local.get $out_ptr
    local.get $b0
    i32.const 2
    i32.shr_u
    call $b64char
    i32.store8
    local.get $out_ptr
    i32.const 1
    i32.add
    local.get $b0
    i32.const 3
    i32.and
    i32.const 4
    i32.shl
    local.get $b1
    i32.const 4
    i32.shr_u
    i32.or
    call $b64char
    i32.store8
    local.get $out_ptr
    i32.const 2
    i32.add
    local.get $b1
    i32.const 15
    i32.and
    i32.const 2
    i32.shl
    local.get $b2
    i32.const 6
    i32.shr_u
    i32.or
    call $b64char
    i32.store8
    local.get $out_ptr
    i32.const 3
    i32.add
    local.get $b2
    i32.const 63
    i32.and
    call $b64char
    i32.store8)

  (func (export "ws_accept_key") (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32) (result i64)
    (local $i i32)
    (local $v i32)
    (local $out_i i32)
    (local $result i64)
    (local $digest i32)
    local.get $in_len
    i32.const 24
    i32.ne
    if
      (return (call $pack (i32.const 4) (i32.const 0)))
    end
    local.get $in_ptr
    i32.const 22
    i32.add
    i32.load8_u
    i32.const 61
    i32.ne
    local.get $in_ptr
    i32.const 23
    i32.add
    i32.load8_u
    i32.const 61
    i32.ne
    i32.or
    if
      (return (call $pack (i32.const 3) (i32.const 0)))
    end
    (loop $validate
      local.get $in_ptr
      local.get $i
      i32.add
      i32.load8_u
      call $b64val
      local.set $v
      local.get $v
      i32.const 0
      i32.lt_s
      if
        (return (call $pack (i32.const 3) (i32.const 0)))
      end
      local.get $i
      i32.const 21
      i32.eq
      local.get $v
      i32.const 15
      i32.and
      i32.const 0
      i32.ne
      i32.and
      if
        (return (call $pack (i32.const 3) (i32.const 0)))
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      local.get $i
      i32.const 22
      i32.lt_u
      br_if $validate)
    local.get $out_cap
    i32.const 28
    i32.lt_u
    if
      (return (call $pack (i32.const 2) (i32.const 0)))
    end
    ;; Assemble 60-byte message: key + GUID
    (local.set $i (i32.const 0))
    (loop $copy_key
      (i32.store8
        (i32.add (global.get $MSG_ADDR) (local.get $i))
        (i32.load8_u (i32.add (local.get $in_ptr) (local.get $i))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $copy_key (i32.lt_u (local.get $i) (i32.const 24))))
    ;; Compute SHA-1 of message
    (local.set $result
      (call $sha1 (global.get $MSG_ADDR) (i32.const 60) (global.get $DIGEST_ADDR)))
    (local.set $v (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $v)
      (then (return (call $pack (local.get $v) (i32.const 0)))))
    (local.set $digest (global.get $DIGEST_ADDR))
    ;; Base64-encode the 20-byte digest into 28-byte output
    (local.set $i (i32.const 0))
    (local.set $out_i (i32.const 0))
    (loop $emit_full
      (call $emit_b64_3
        (i32.load8_u (i32.add (local.get $digest) (local.get $i)))
        (i32.load8_u (i32.add (local.get $digest) (i32.add (local.get $i) (i32.const 1))))
        (i32.load8_u (i32.add (local.get $digest) (i32.add (local.get $i) (i32.const 2))))
        (i32.add (local.get $out_ptr) (local.get $out_i)))
      local.get $i
      i32.const 3
      i32.add
      local.set $i
      local.get $out_i
      i32.const 4
      i32.add
      local.set $out_i
      local.get $i
      i32.const 18
      i32.lt_u
      br_if $emit_full)
    local.get $out_ptr
    local.get $out_i
    i32.add
    (i32.load8_u (i32.add (local.get $digest) (i32.const 18)))
    i32.const 2
    i32.shr_u
    call $b64char
    i32.store8
    local.get $out_ptr
    local.get $out_i
    i32.add
    i32.const 1
    i32.add
    (i32.load8_u (i32.add (local.get $digest) (i32.const 18)))
    i32.const 3
    i32.and
    i32.const 4
    i32.shl
    (i32.load8_u (i32.add (local.get $digest) (i32.const 19)))
    i32.const 4
    i32.shr_u
    i32.or
    call $b64char
    i32.store8
    local.get $out_ptr
    local.get $out_i
    i32.add
    i32.const 2
    i32.add
    (i32.load8_u (i32.add (local.get $digest) (i32.const 19)))
    i32.const 15
    i32.and
    i32.const 2
    i32.shl
    call $b64char
    i32.store8
    local.get $out_ptr
    local.get $out_i
    i32.add
    i32.const 3
    i32.add
    i32.const 61
    i32.store8
    (call $pack
      (i32.const 0)
      (i32.const 28)))
)
