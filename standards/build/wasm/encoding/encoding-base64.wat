(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "pack" (func $pack (param i32 i32) (result i64)))

  (func (export "proto_standard_id") (result i32)
    i32.const 300046)


  (func $m85b64_char (param $n i32) (result i32)
    local.get $n
    i32.const 26
    i32.lt_u
    if (result i32)
      local.get $n
      i32.const 65
      i32.add
    else
      local.get $n
      i32.const 52
      i32.lt_u
      if (result i32)
        local.get $n
        i32.const 26
        i32.sub
        i32.const 97
        i32.add
      else
        local.get $n
        i32.const 62
        i32.lt_u
        if (result i32)
          local.get $n
          i32.const 52
          i32.sub
          i32.const 48
          i32.add
        else
          local.get $n
          i32.const 62
          i32.eq
          if (result i32)
            i32.const 43
          else
            i32.const 47
          end
        end
      end
    end)

  (func $m85b64_value (param $c i32) (result i32)
    local.get $c
    i32.const 65
    i32.ge_u
    local.get $c
    i32.const 90
    i32.le_u
    i32.and
    if (result i32)
      local.get $c
      i32.const 65
      i32.sub
    else
      local.get $c
      i32.const 97
      i32.ge_u
      local.get $c
      i32.const 122
      i32.le_u
      i32.and
      if (result i32)
        local.get $c
        i32.const 97
        i32.sub
        i32.const 26
        i32.add
      else
        local.get $c
        i32.const 48
        i32.ge_u
        local.get $c
        i32.const 57
        i32.le_u
        i32.and
        if (result i32)
          local.get $c
          i32.const 48
          i32.sub
          i32.const 52
          i32.add
        else
          local.get $c
          i32.const 43
          i32.eq
          if (result i32)
            i32.const 62
          else
            local.get $c
            i32.const 47
            i32.eq
            if (result i32)
              i32.const 63
            else
              i32.const -1
            end
          end
        end
      end
    end)

  (func (export "base64_standard_encode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $groups i32)
    (local $rem i32)
    (local $out_len i32)
    (local $i i32)
    (local $j i32)
    (local $b0 i32)
    (local $b1 i32)
    (local $b2 i32)
    local.get $in_len
    i32.const 3
    i32.div_u
    local.set $groups
    local.get $in_len
    i32.const 3
    i32.rem_u
    local.set $rem
    local.get $groups
    i32.const 1073741823
    i32.gt_u
    if (result i64)
      i32.const 4
      i32.const 0
      call $pack
    else
      local.get $groups
      i32.const 1073741823
      i32.eq
      local.get $rem
      i32.const 0
      i32.ne
      i32.and
      if (result i64)
        i32.const 4
        i32.const 0
        call $pack
      else
        local.get $groups
        local.get $rem
        i32.const 0
        i32.ne
        i32.add
        i32.const 2
        i32.shl
        local.set $out_len
        local.get $out_len
        local.get $out_cap
        i32.gt_u
        if (result i64)
          i32.const 2
          i32.const 0
          call $pack
        else
          loop $loop
            local.get $i
            local.get $groups
            i32.lt_u
            if
              local.get $in_ptr
              local.get $i
              i32.const 3
              i32.mul
              i32.add
              i32.load8_u
              local.set $b0
              local.get $in_ptr
              local.get $i
              i32.const 3
              i32.mul
              i32.add
              i32.const 1
              i32.add
              i32.load8_u
              local.set $b1
              local.get $in_ptr
              local.get $i
              i32.const 3
              i32.mul
              i32.add
              i32.const 2
              i32.add
              i32.load8_u
              local.set $b2
              local.get $out_ptr
              local.get $j
              i32.add
              local.get $b0
              i32.const 2
              i32.shr_u
              call $m85b64_char
              i32.store8
              local.get $out_ptr
              local.get $j
              i32.add
              i32.const 1
              i32.add
              local.get $b0
              i32.const 4
              i32.shl
              local.get $b1
              i32.const 4
              i32.shr_u
              i32.or
              i32.const 63
              i32.and
              call $m85b64_char
              i32.store8
              local.get $out_ptr
              local.get $j
              i32.add
              i32.const 2
              i32.add
              local.get $b1
              i32.const 2
              i32.shl
              local.get $b2
              i32.const 6
              i32.shr_u
              i32.or
              i32.const 63
              i32.and
              call $m85b64_char
              i32.store8
              local.get $out_ptr
              local.get $j
              i32.add
              i32.const 3
              i32.add
              local.get $b2
              i32.const 63
              i32.and
              call $m85b64_char
              i32.store8
              local.get $i
              i32.const 1
              i32.add
              local.set $i
              local.get $j
              i32.const 4
              i32.add
              local.set $j
              br $loop
            end
          end
          local.get $rem
          i32.const 1
          i32.eq
          if
            local.get $in_ptr
            local.get $groups
            i32.const 3
            i32.mul
            i32.add
            i32.load8_u
            local.set $b0
            local.get $out_ptr
            local.get $j
            i32.add
            local.get $b0
            i32.const 2
            i32.shr_u
            call $m85b64_char
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 1
            i32.add
            local.get $b0
            i32.const 4
            i32.shl
            i32.const 63
            i32.and
            call $m85b64_char
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 2
            i32.add
            i32.const 61
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 3
            i32.add
            i32.const 61
            i32.store8
          end
          local.get $rem
          i32.const 2
          i32.eq
          if
            local.get $in_ptr
            local.get $groups
            i32.const 3
            i32.mul
            i32.add
            i32.load8_u
            local.set $b0
            local.get $in_ptr
            local.get $groups
            i32.const 3
            i32.mul
            i32.add
            i32.const 1
            i32.add
            i32.load8_u
            local.set $b1
            local.get $out_ptr
            local.get $j
            i32.add
            local.get $b0
            i32.const 2
            i32.shr_u
            call $m85b64_char
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 1
            i32.add
            local.get $b0
            i32.const 4
            i32.shl
            local.get $b1
            i32.const 4
            i32.shr_u
            i32.or
            i32.const 63
            i32.and
            call $m85b64_char
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 2
            i32.add
            local.get $b1
            i32.const 2
            i32.shl
            i32.const 63
            i32.and
            call $m85b64_char
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 3
            i32.add
            i32.const 61
            i32.store8
          end
          i32.const 0
          local.get $out_len
          call $pack
        end
      end
    end)

  (func (export "base64_standard_decode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $groups i32)
    (local $full_groups i32)
    (local $pad i32)
    (local $out_len i32)
    (local $i i32)
    (local $j i32)
    (local $v0 i32)
    (local $v1 i32)
    (local $v2 i32)
    (local $v3 i32)
    local.get $in_len
    i32.const 3
    i32.and
    if (result i64)
      i32.const 3
      i32.const 0
      call $pack
    else
      local.get $in_len
      i32.const 4
      i32.div_u
      local.set $groups
      local.get $groups
      local.set $full_groups
      local.get $in_len
      if
        local.get $in_ptr
        local.get $in_len
        i32.add
        i32.const 1
        i32.sub
        i32.load8_u
        i32.const 61
        i32.eq
        if
          i32.const 1
          local.set $pad
          local.get $in_ptr
          local.get $in_len
          i32.add
          i32.const 2
          i32.sub
          i32.load8_u
          i32.const 61
          i32.eq
          if
            i32.const 2
            local.set $pad
          end
        end
      end
      local.get $pad
      if
        local.get $groups
        i32.const 0
        i32.eq
        if
          i32.const 3
          i32.const 0
          call $pack
          return
        end
        local.get $groups
        i32.const 1
        i32.sub
        local.set $full_groups
      end
      local.get $groups
      i32.const 3
      i32.mul
      local.get $pad
      i32.sub
      local.set $out_len
      local.get $out_len
      local.get $out_cap
      i32.gt_u
      if (result i64)
        i32.const 2
        i32.const 0
        call $pack
      else
        loop $loop
          local.get $i
          local.get $full_groups
          i32.lt_u
          if
            local.get $in_ptr
            local.get $i
            i32.const 4
            i32.mul
            i32.add
            i32.load8_u
            call $m85b64_value
            local.tee $v0
            i32.const 0
            i32.lt_s
            if
              i32.const 3
              i32.const 0
              call $pack
              return
            end
            local.get $in_ptr
            local.get $i
            i32.const 4
            i32.mul
            i32.add
            i32.const 1
            i32.add
            i32.load8_u
            call $m85b64_value
            local.tee $v1
            i32.const 0
            i32.lt_s
            if
              i32.const 3
              i32.const 0
              call $pack
              return
            end
            local.get $in_ptr
            local.get $i
            i32.const 4
            i32.mul
            i32.add
            i32.const 2
            i32.add
            i32.load8_u
            call $m85b64_value
            local.tee $v2
            i32.const 0
            i32.lt_s
            if
              i32.const 3
              i32.const 0
              call $pack
              return
            end
            local.get $in_ptr
            local.get $i
            i32.const 4
            i32.mul
            i32.add
            i32.const 3
            i32.add
            i32.load8_u
            call $m85b64_value
            local.tee $v3
            i32.const 0
            i32.lt_s
            if
              i32.const 3
              i32.const 0
              call $pack
              return
            end
            local.get $out_ptr
            local.get $j
            i32.add
            local.get $v0
            i32.const 2
            i32.shl
            local.get $v1
            i32.const 4
            i32.shr_u
            i32.or
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 1
            i32.add
            local.get $v1
            i32.const 4
            i32.shl
            local.get $v2
            i32.const 2
            i32.shr_u
            i32.or
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 2
            i32.add
            local.get $v2
            i32.const 6
            i32.shl
            local.get $v3
            i32.or
            i32.store8
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            local.get $j
            i32.const 3
            i32.add
            local.set $j
            br $loop
          end
        end
        local.get $pad
        i32.const 1
        i32.eq
        if
          local.get $in_ptr
          local.get $full_groups
          i32.const 4
          i32.mul
          i32.add
          i32.load8_u
          call $m85b64_value
          local.tee $v0
          i32.const 0
          i32.lt_s
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $in_ptr
          local.get $full_groups
          i32.const 4
          i32.mul
          i32.add
          i32.const 1
          i32.add
          i32.load8_u
          call $m85b64_value
          local.tee $v1
          i32.const 0
          i32.lt_s
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $in_ptr
          local.get $full_groups
          i32.const 4
          i32.mul
          i32.add
          i32.const 2
          i32.add
          i32.load8_u
          call $m85b64_value
          local.tee $v2
          i32.const 0
          i32.lt_s
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $v2
          i32.const 3
          i32.and
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $out_ptr
          local.get $j
          i32.add
          local.get $v0
          i32.const 2
          i32.shl
          local.get $v1
          i32.const 4
          i32.shr_u
          i32.or
          i32.store8
          local.get $out_ptr
          local.get $j
          i32.add
          i32.const 1
          i32.add
          local.get $v1
          i32.const 4
          i32.shl
          local.get $v2
          i32.const 2
          i32.shr_u
          i32.or
          i32.store8
        end
        local.get $pad
        i32.const 2
        i32.eq
        if
          local.get $in_ptr
          local.get $full_groups
          i32.const 4
          i32.mul
          i32.add
          i32.load8_u
          call $m85b64_value
          local.tee $v0
          i32.const 0
          i32.lt_s
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $in_ptr
          local.get $full_groups
          i32.const 4
          i32.mul
          i32.add
          i32.const 1
          i32.add
          i32.load8_u
          call $m85b64_value
          local.tee $v1
          i32.const 0
          i32.lt_s
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $v1
          i32.const 15
          i32.and
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $out_ptr
          local.get $j
          i32.add
          local.get $v0
          i32.const 2
          i32.shl
          local.get $v1
          i32.const 4
          i32.shr_u
          i32.or
          i32.store8
        end
        i32.const 0
        local.get $out_len
        call $pack
      end
    end)
)