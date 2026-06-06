(module
  (import "edgerun-core" "memory" (memory 1))
  ;; XTEA block cipher — 64-bit block, 128-bit key, 32 Feistel rounds.
  ;; Big-endian I/O, operates in-place on buffer.
  ;; Remaining bytes (< 8) at end are copied through unchanged.
  ;;
  ;; Exports:
  ;;   xtea_encrypt(data_ptr, data_len, key_ptr) -> 0
  ;;   xtea_decrypt(data_ptr, data_len, key_ptr) -> 0
  ;;
  ;; key_ptr points to 16 bytes of key material (4 × 32-bit big-endian words).
  (func (export "proto_standard_id") (result i32) i32.const 300520)

  (func $m65load_be32 (param $p i32) (result i32)
    local.get $p i32.load8_u i32.const 24 i32.shl
    local.get $p i32.const 1 i32.add i32.load8_u i32.const 16 i32.shl i32.or
    local.get $p i32.const 2 i32.add i32.load8_u i32.const 8 i32.shl i32.or
    local.get $p i32.const 3 i32.add i32.load8_u i32.or
  )

  (func $m65store_be32 (param $p i32) (param $v i32)
    local.get $p local.get $v i32.const 24 i32.shr_u i32.store8
    local.get $p i32.const 1 i32.add local.get $v i32.const 16 i32.shr_u i32.const 0xff i32.and i32.store8
    local.get $p i32.const 2 i32.add local.get $v i32.const 8 i32.shr_u i32.const 0xff i32.and i32.store8
    local.get $p i32.const 3 i32.add local.get $v i32.const 0xff i32.and i32.store8
  )

  (func $encrypt_block (param $block i32) (param $key i32)
    (local $v0 i32) (local $v1 i32) (local $sum i32) (local $i i32)
    local.get $block call $m65load_be32 local.set $v0
    local.get $block i32.const 4 i32.add call $m65load_be32 local.set $v1
    i32.const 0 local.set $sum
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i i32.const 32 i32.eq br_if $done
      local.get $v0
      local.get $v1 i32.const 4 i32.shl
      local.get $v1 i32.const 5 i32.shr_u i32.xor
      local.get $v1 i32.add
      local.get $sum
      local.get $key local.get $sum i32.const 3 i32.and i32.const 2 i32.shl i32.add call $m65load_be32
      i32.add i32.xor
      i32.add local.set $v0
      local.get $sum i32.const 0x9E3779B9 i32.add local.set $sum
      local.get $v1
      local.get $v0 i32.const 4 i32.shl
      local.get $v0 i32.const 5 i32.shr_u i32.xor
      local.get $v0 i32.add
      local.get $sum
      local.get $key local.get $sum i32.const 11 i32.shr_u i32.const 3 i32.and i32.const 2 i32.shl i32.add call $m65load_be32
      i32.add i32.xor
      i32.add local.set $v1
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $block local.get $v0 call $m65store_be32
    local.get $block i32.const 4 i32.add local.get $v1 call $m65store_be32
  )

  (func $decrypt_block (param $block i32) (param $key i32)
    (local $v0 i32) (local $v1 i32) (local $sum i32) (local $i i32)
    local.get $block call $m65load_be32 local.set $v0
    local.get $block i32.const 4 i32.add call $m65load_be32 local.set $v1
    i32.const 0x9E3779B9 i32.const 32 i32.mul local.set $sum
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i i32.const 32 i32.eq br_if $done
      local.get $v1
      local.get $v0 i32.const 4 i32.shl
      local.get $v0 i32.const 5 i32.shr_u i32.xor
      local.get $v0 i32.add
      local.get $sum
      local.get $key local.get $sum i32.const 11 i32.shr_u i32.const 3 i32.and i32.const 2 i32.shl i32.add call $m65load_be32
      i32.add i32.xor
      i32.sub local.set $v1
      local.get $sum i32.const 0x9E3779B9 i32.sub local.set $sum
      local.get $v0
      local.get $v1 i32.const 4 i32.shl
      local.get $v1 i32.const 5 i32.shr_u i32.xor
      local.get $v1 i32.add
      local.get $sum
      local.get $key local.get $sum i32.const 3 i32.and i32.const 2 i32.shl i32.add call $m65load_be32
      i32.add i32.xor
      i32.sub local.set $v0
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $block local.get $v0 call $m65store_be32
    local.get $block i32.const 4 i32.add local.get $v1 call $m65store_be32
  )

  (func (export "xtea_encrypt") (param $data i32) (param $len i32) (param $key i32) (result i32)
    (local $i i32) (local $num i32)
    local.get $len i32.const 3 i32.shr_u local.set $num
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i local.get $num i32.ge_u br_if $done
      local.get $data local.get $i i32.const 3 i32.shl i32.add local.get $key call $encrypt_block
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    i32.const 0
  )

  (func (export "xtea_decrypt") (param $data i32) (param $len i32) (param $key i32) (result i32)
    (local $i i32) (local $num i32)
    local.get $len i32.const 3 i32.shr_u local.set $num
    local.get $num i32.const 1 i32.sub local.set $i
    block $done
    loop $loop
      local.get $i i32.const 0 i32.lt_s br_if $done
      local.get $data local.get $i i32.const 3 i32.shl i32.add local.get $key call $decrypt_block
      local.get $i i32.const 1 i32.sub local.set $i
      br $loop
    end
    end
    i32.const 0
  )
)