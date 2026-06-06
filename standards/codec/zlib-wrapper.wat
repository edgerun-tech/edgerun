
(func (export "proto_standard_id") (result i32)
    i32.const 300065)

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid_data, 7 unsupported.
  ;; Packed i64 return: low u32 status, high u32 bytes_written.

  (func $m203read_u32_be (param $ptr i32) (result i32)
    local.get $ptr
    i32.load8_u
    i32.const 24
    i32.shl
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 16
    i32.shl
    i32.or
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 8
    i32.shl
    i32.or
    local.get $ptr
    i32.const 3
    i32.add
    i32.load8_u
    i32.or)

  (func $m203write_record
    (param $out i32)
    (param $deflate_off i32)
    (param $deflate_len i32)
    (param $cmf i32)
    (param $flg i32)
    (param $window_log2 i32)
    (param $flevel i32)
    (param $fdict i32)
    (param $expected_adler32 i32)
    local.get $out
    local.get $deflate_off
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $deflate_len
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $cmf
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $flg
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $window_log2
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $flevel
    i32.store
    local.get $out
    i32.const 24
    i32.add
    local.get $fdict
    i32.store
    local.get $out
    i32.const 28
    i32.add
    local.get $expected_adler32
    i32.store)

  (func (export "zlib_member_scan")
    (param $ptr i32)
    (param $len i32)
    (param $out i32)
    (result i32)
    (local $cmf i32)
    (local $flg i32)
    (local $cinfo i32)
    (local $fdict i32)
    (local $flevel i32)
    (local $adler i32)

    local.get $len
    i32.const 6
    i32.lt_u
    if
      i32.const 1
      return
    end

    local.get $ptr
    i32.load8_u
    local.set $cmf
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    local.set $flg

    local.get $cmf
    i32.const 15
    i32.and
    i32.const 8
    i32.ne
    if
      i32.const 3
      return
    end

    local.get $cmf
    i32.const 4
    i32.shr_u
    local.tee $cinfo
    i32.const 7
    i32.gt_u
    if
      i32.const 3
      return
    end

    local.get $cmf
    i32.const 256
    i32.mul
    local.get $flg
    i32.add
    i32.const 31
    i32.rem_u
    if
      i32.const 3
      return
    end

    local.get $flg
    i32.const 32
    i32.and
    i32.const 0
    i32.ne
    local.tee $fdict
    if
      i32.const 7
      return
    end

    local.get $flg
    i32.const 6
    i32.shr_u
    local.set $flevel

    local.get $ptr
    local.get $len
    i32.add
    i32.const 4
    i32.sub
    call $m203read_u32_be
    local.set $adler

    local.get $out
    i32.const 2
    local.get $len
    i32.const 6
    i32.sub
    local.get $cmf
    local.get $flg
    local.get $cinfo
    i32.const 8
    i32.add
    local.get $flevel
    local.get $fdict
    local.get $adler
    call $m203write_record

    i32.const 0)

  (func (export "zlib_write_header")
    (param $level i32)
    (param $out_ptr i32)
    (param $out_cap i32)
    (result i64)
    (local $flg i32)

    local.get $out_cap
    i32.const 2
    i32.lt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end

    local.get $level
    i32.eqz
    if
      i32.const 1
      local.set $flg
    else
      local.get $level
      i32.const 1
      i32.eq
      if
        i32.const 94
        local.set $flg
      else
        local.get $level
        i32.const 6
        i32.eq
        if
          i32.const 156
          local.set $flg
        else
          local.get $level
          i32.const 9
          i32.eq
          if
            i32.const 218
            local.set $flg
          else
            i32.const 3
            i32.const 0
            call $pack
            return
          end
        end
      end
    end

    local.get $out_ptr
    i32.const 120
    i32.store8
    local.get $out_ptr
    i32.const 1
    i32.add
    local.get $flg
    i32.store8

    i32.const 0
    i32.const 2
    call $pack)

  (func (export "zlib_write_trailer")
    (param $adler32 i32)
    (param $out_ptr i32)
    (param $out_cap i32)
    (result i64)
    local.get $out_cap
    i32.const 4
    i32.lt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end

    local.get $out_ptr
    local.get $adler32
    i32.const 24
    i32.shr_u
    i32.store8
    local.get $out_ptr
    i32.const 1
    i32.add
    local.get $adler32
    i32.const 16
    i32.shr_u
    i32.store8
    local.get $out_ptr
    i32.const 2
    i32.add
    local.get $adler32
    i32.const 8
    i32.shr_u
    i32.store8
    local.get $out_ptr
    i32.const 3
    i32.add
    local.get $adler32
    i32.store8

    i32.const 0
    i32.const 4
    call $pack)
