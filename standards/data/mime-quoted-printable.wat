

  (func $m133hex_upper (param $n i32) (result i32)
    local.get $n
    i32.const 10
    i32.lt_u
    if (result i32)
      local.get $n
      i32.const 48
      i32.add
    else
      local.get $n
      i32.const 55
      i32.add
    end)

  (func $is_pass_byte (param $b i32) (result i32)
    local.get $b
    i32.const 33
    i32.ge_u
    local.get $b
    i32.const 60
    i32.le_u
    i32.and
    local.get $b
    i32.const 62
    i32.ge_u
    local.get $b
    i32.const 126
    i32.le_u
    i32.and
    i32.or
    local.get $b
    i32.const 32
    i32.eq
    i32.or
    local.get $b
    i32.const 9
    i32.eq
    i32.or)

  (func $encoded_len (param $in_ptr i32) (param $in_len i32) (result i32)
    (local $i i32)
    (local $b i32)
    (local $col i32)
    (local $required i32)
    loop $loop
      local.get $i
      local.get $in_len
      i32.lt_u
      if
        local.get $in_ptr
        local.get $i
        i32.add
        i32.load8_u
        local.set $b
        local.get $b
        i32.const 13
        i32.eq
        local.get $b
        i32.const 10
        i32.eq
        i32.or
        if
          local.get $required
          i32.const 1
          i32.add
          local.set $required
          i32.const 0
          local.set $col
        else
          local.get $b
          call $is_pass_byte
          if
            local.get $required
            i32.const 1
            i32.add
            local.set $required
            local.get $col
            i32.const 1
            i32.add
            local.set $col
          else
            local.get $required
            i32.const 3
            i32.add
            local.set $required
            local.get $col
            i32.const 3
            i32.add
            local.set $col
          end
          local.get $col
          i32.const 73
          i32.ge_u
          if
            local.get $required
            i32.const 3
            i32.add
            local.set $required
            i32.const 0
            local.set $col
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $required)

  (func (export "quoted_printable_encode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $required i32)
    (local $i i32)
    (local $j i32)
    (local $b i32)
    (local $col i32)
    local.get $in_ptr
    local.get $in_len
    call $encoded_len
    local.set $required
    local.get $required
    local.get $out_cap
    i32.gt_u
    if (result i64)
      i32.const 2
      i32.const 0
      call $pack
    else
      loop $loop
        local.get $i
        local.get $in_len
        i32.lt_u
        if
          local.get $in_ptr
          local.get $i
          i32.add
          i32.load8_u
          local.set $b
          local.get $b
          i32.const 13
          i32.eq
          local.get $b
          i32.const 10
          i32.eq
          i32.or
          if
            local.get $out_ptr
            local.get $j
            i32.add
            local.get $b
            i32.store8
            local.get $j
            i32.const 1
            i32.add
            local.set $j
            i32.const 0
            local.set $col
          else
            local.get $b
            call $is_pass_byte
            if
              local.get $out_ptr
              local.get $j
              i32.add
              local.get $b
              i32.store8
              local.get $j
              i32.const 1
              i32.add
              local.set $j
              local.get $col
              i32.const 1
              i32.add
              local.set $col
            else
              local.get $out_ptr
              local.get $j
              i32.add
              i32.const 61
              i32.store8
              local.get $out_ptr
              local.get $j
              i32.add
              i32.const 1
              i32.add
              local.get $b
              i32.const 4
              i32.shr_u
              call $m133hex_upper
              i32.store8
              local.get $out_ptr
              local.get $j
              i32.add
              i32.const 2
              i32.add
              local.get $b
              i32.const 15
              i32.and
              call $m133hex_upper
              i32.store8
              local.get $j
              i32.const 3
              i32.add
              local.set $j
              local.get $col
              i32.const 3
              i32.add
              local.set $col
            end
            local.get $col
            i32.const 73
            i32.ge_u
            if
              local.get $out_ptr
              local.get $j
              i32.add
              i32.const 61
              i32.store8
              local.get $out_ptr
              local.get $j
              i32.add
              i32.const 1
              i32.add
              i32.const 13
              i32.store8
              local.get $out_ptr
              local.get $j
              i32.add
              i32.const 2
              i32.add
              i32.const 10
              i32.store8
              local.get $j
              i32.const 3
              i32.add
              local.set $j
              i32.const 0
              local.set $col
            end
          end
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $loop
        end
      end
      i32.const 0
      local.get $j
      call $pack
    end)
