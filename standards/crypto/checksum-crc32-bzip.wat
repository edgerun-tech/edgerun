  ;; BZip2 CRC-32 — non-reflected CRC-32 with polynomial 0x04c11db7.
  ;; This is the CRC used by bzip2 block checksums, distinct from the
  ;; reflected IEEE/GZip CRC-32 (polynomial 0xedb88320).
  ;; Exports: bzip_crc32(data_ptr, data_len) -> i32
  ;;          bzip_crc32_update(crc, byte) -> i32  (one-byte update)
  (func $bzip_crc32_byte (param $crc i32) (param $byte i32) (result i32)
    (local $i i32)
    local.get $crc local.get $byte i32.const 24 i32.shl i32.xor local.set $crc
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i i32.const 8 i32.eq br_if $done
      local.get $crc i32.const 0x80000000 i32.and i32.const 0 i32.ne
      if
        local.get $crc i32.const 1 i32.shl i32.const 0x04c11db7 i32.xor local.set $crc
      else
        local.get $crc i32.const 1 i32.shl local.set $crc
      end
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $crc
  )

  (func (export "bzip_crc32_update") (param $crc i32) (param $byte i32) (result i32)
    local.get $crc local.get $byte call $bzip_crc32_byte
  )

  (func $bzip_crc32 (export "bzip_crc32") (param $ptr i32) (param $len i32) (result i32)
    (local $crc i32) (local $i i32)
    i32.const 0 local.set $crc
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $crc local.get $ptr local.get $i i32.add i32.load8_u call $bzip_crc32_byte local.set $crc
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $crc
  )
