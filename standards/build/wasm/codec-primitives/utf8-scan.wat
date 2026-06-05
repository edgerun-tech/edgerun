(module
  (memory (export "memory") 1)

  ;; Status values: 0 ok, 2 output_short, 3 invalid, 5 incomplete.
  ;; utf8_scan out record: valid_up_to, error_len, suffix_len, expected_len.
  ;; error_len is 0 for incomplete suffixes.
  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300068)

  (func $pack (param $status i32) (param $count i32) (result i64)
    local.get $count
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get $status
    i64.extend_i32_u
    i64.or)

  (func $byte (param $ptr i32) (param $off i32) (result i32)
    local.get $ptr
    local.get $off
    i32.add
    i32.load8_u)

  (func $is_cont (param $c i32) (result i32)
    local.get $c
    i32.const 128
    i32.ge_u
    local.get $c
    i32.const 191
    i32.le_u
    i32.and)

  (func $write_scan (param $out i32) (param $valid i32) (param $err_len i32) (param $suffix i32) (param $expected i32)
    local.get $out
    local.get $valid
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $err_len
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $suffix
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $expected
    i32.store)

  (func $utf8_scan (export "utf8_scan") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32)
    (local $c i32)
    (local $b1 i32)
    (local $b2 i32)
    (local $b3 i32)
    (local $remain i32)
    (loop $again
      local.get $i
      local.get $len
      i32.ge_u
      if
        local.get $out
        local.get $len
        i32.const 0
        i32.const 0
        i32.const 0
        call $write_scan
        i32.const 0
        return
      end
      local.get $ptr
      local.get $i
      call $byte
      local.set $c
      local.get $c
      i32.const 128
      i32.lt_u
      if
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end

      local.get $len
      local.get $i
      i32.sub
      local.set $remain

      ;; Two-byte sequence: C2..DF 80..BF.
      local.get $c
      i32.const 194
      i32.ge_u
      local.get $c
      i32.const 223
      i32.le_u
      i32.and
      if
        local.get $remain
        i32.const 2
        i32.lt_u
        if
          local.get $out
          local.get $i
          i32.const 0
          local.get $remain
          i32.const 2
          call $write_scan
          i32.const 5
          return
        end
        local.get $ptr
        local.get $i
        i32.const 1
        i32.add
        call $byte
        call $is_cont
        i32.eqz
        if
          local.get $out
          local.get $i
          i32.const 1
          i32.const 0
          i32.const 0
          call $write_scan
          i32.const 3
          return
        end
        local.get $i
        i32.const 2
        i32.add
        local.set $i
        br $again
      end

      ;; Three-byte sequence with overlong and surrogate exclusions.
      local.get $c
      i32.const 224
      i32.ge_u
      local.get $c
      i32.const 239
      i32.le_u
      i32.and
      if
        local.get $remain
        i32.const 3
        i32.lt_u
        if
          local.get $out
          local.get $i
          i32.const 0
          local.get $remain
          i32.const 3
          call $write_scan
          i32.const 5
          return
        end
        local.get $ptr
        local.get $i
        i32.const 1
        i32.add
        call $byte
        local.set $b1
        local.get $ptr
        local.get $i
        i32.const 2
        i32.add
        call $byte
        local.set $b2
        local.get $c
        i32.const 224
        i32.eq
        if
          local.get $b1
          i32.const 160
          i32.ge_u
          local.get $b1
          i32.const 191
          i32.le_u
          i32.and
          i32.eqz
          if
            local.get $out
            local.get $i
            i32.const 1
            i32.const 0
            i32.const 0
            call $write_scan
            i32.const 3
            return
          end
        else
          local.get $c
          i32.const 237
          i32.eq
          if
            local.get $b1
            i32.const 128
            i32.ge_u
            local.get $b1
            i32.const 159
            i32.le_u
            i32.and
            i32.eqz
            if
              local.get $out
              local.get $i
              i32.const 1
              i32.const 0
              i32.const 0
              call $write_scan
              i32.const 3
              return
            end
          else
            local.get $b1
            call $is_cont
            i32.eqz
            if
              local.get $out
              local.get $i
              i32.const 1
              i32.const 0
              i32.const 0
              call $write_scan
              i32.const 3
              return
            end
          end
        end
        local.get $b2
        call $is_cont
        i32.eqz
        if
          local.get $out
          local.get $i
          i32.const 1
          i32.const 0
          i32.const 0
          call $write_scan
          i32.const 3
          return
        end
        local.get $i
        i32.const 3
        i32.add
        local.set $i
        br $again
      end

      ;; Four-byte sequence for U+10000..U+10FFFF.
      local.get $c
      i32.const 240
      i32.ge_u
      local.get $c
      i32.const 244
      i32.le_u
      i32.and
      if
        local.get $remain
        i32.const 4
        i32.lt_u
        if
          local.get $out
          local.get $i
          i32.const 0
          local.get $remain
          i32.const 4
          call $write_scan
          i32.const 5
          return
        end
        local.get $ptr
        local.get $i
        i32.const 1
        i32.add
        call $byte
        local.set $b1
        local.get $ptr
        local.get $i
        i32.const 2
        i32.add
        call $byte
        local.set $b2
        local.get $ptr
        local.get $i
        i32.const 3
        i32.add
        call $byte
        local.set $b3
        local.get $c
        i32.const 240
        i32.eq
        if
          local.get $b1
          i32.const 144
          i32.ge_u
          local.get $b1
          i32.const 191
          i32.le_u
          i32.and
          i32.eqz
          if
            local.get $out
            local.get $i
            i32.const 1
            i32.const 0
            i32.const 0
            call $write_scan
            i32.const 3
            return
          end
        else
          local.get $c
          i32.const 244
          i32.eq
          if
            local.get $b1
            i32.const 128
            i32.ge_u
            local.get $b1
            i32.const 143
            i32.le_u
            i32.and
            i32.eqz
            if
              local.get $out
              local.get $i
              i32.const 1
              i32.const 0
              i32.const 0
              call $write_scan
              i32.const 3
              return
            end
          else
            local.get $b1
            call $is_cont
            i32.eqz
            if
              local.get $out
              local.get $i
              i32.const 1
              i32.const 0
              i32.const 0
              call $write_scan
              i32.const 3
              return
            end
          end
        end
        local.get $b2
        call $is_cont
        local.get $b3
        call $is_cont
        i32.and
        i32.eqz
        if
          local.get $out
          local.get $i
          i32.const 1
          i32.const 0
          i32.const 0
          call $write_scan
          i32.const 3
          return
        end
        local.get $i
        i32.const 4
        i32.add
        local.set $i
        br $again
      end

      local.get $out
      local.get $i
      i32.const 1
      i32.const 0
      i32.const 0
      call $write_scan
      i32.const 3
      return)
    i32.const 0)

  (func $copy_bytes (param $src i32) (param $len i32) (param $dst i32)
    (local $i i32)
    (loop $copy
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $dst
        local.get $i
        i32.add
        local.get $src
        local.get $i
        i32.add
        i32.load8_u
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $copy
      end))

  (func $write_replacement (param $out_ptr i32) (param $written i32)
    local.get $out_ptr
    local.get $written
    i32.add
    i32.const 239
    i32.store8
    local.get $out_ptr
    local.get $written
    i32.add
    i32.const 1
    i32.add
    i32.const 191
    i32.store8
    local.get $out_ptr
    local.get $written
    i32.add
    i32.const 2
    i32.add
    i32.const 189
    i32.store8)

  (func (export "utf8_lossy_repair")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $pos i32)
    (local $written i32)
    (local $status i32)
    (local $valid i32)
    (local $err_len i32)
    (local $suffix_len i32)
    (loop $again
      local.get $pos
      local.get $in_len
      i32.ge_u
      if
        i32.const 0
        local.get $written
        call $pack
        return
      end
      local.get $in_ptr
      local.get $pos
      i32.add
      local.get $in_len
      local.get $pos
      i32.sub
      i32.const 0
      call $utf8_scan
      local.set $status
      i32.const 0
      i32.load
      local.set $valid
      local.get $written
      local.get $valid
      i32.add
      local.get $out_cap
      i32.gt_u
      if
        i32.const 2
        local.get $written
        call $pack
        return
      end
      local.get $in_ptr
      local.get $pos
      i32.add
      local.get $valid
      local.get $out_ptr
      local.get $written
      i32.add
      call $copy_bytes
      local.get $written
      local.get $valid
      i32.add
      local.set $written
      local.get $status
      i32.const 0
      i32.eq
      if
        i32.const 0
        local.get $written
        call $pack
        return
      end
      local.get $written
      i32.const 3
      i32.add
      local.get $out_cap
      i32.gt_u
      if
        i32.const 2
        local.get $written
        call $pack
        return
      end
      local.get $out_ptr
      local.get $written
      call $write_replacement
      local.get $written
      i32.const 3
      i32.add
      local.set $written
      local.get $status
      i32.const 5
      i32.eq
      if
        i32.const 0
        local.get $written
        call $pack
        return
      end
      i32.const 4
      i32.load
      local.set $err_len
      i32.const 8
      i32.load
      local.set $suffix_len
      local.get $pos
      local.get $valid
      i32.add
      local.get $err_len
      i32.add
      local.set $pos
      br $again)
    i32.const 0
    local.get $written
    call $pack)
)
