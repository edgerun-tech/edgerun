  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300071)



  (func $is_sixel (param $c i32) (result i32)
    local.get $c
    i32.const 63
    i32.ge_u
    local.get $c
    i32.const 126
    i32.le_u
    i32.and)

  (func $parse_number (param $ptr i32) (param $len i32) (param $idx i32) (result i32 i32)
    (local $start i32)
    (local $value i32)
    local.get $idx
    local.set $start
    i32.const 0
    local.set $value
    block $done
      loop $digits
        local.get $idx
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $idx
        i32.add
        i32.load8_u
        call $is_digit
        i32.eqz
        br_if $done
        local.get $value
        i32.const 10
        i32.mul
        local.get $ptr
        local.get $idx
        i32.add
        i32.load8_u
        i32.const 48
        i32.sub
        i32.add
        local.set $value
        local.get $idx
        i32.const 1
        i32.add
        local.set $idx
        br $digits
      end
    end
    local.get $idx
    local.get $start
    i32.eq
    if
      i32.const 0
      i32.const -1
      return
    end
    local.get $value
    local.get $idx)

  (func $scale_percent (param $value i32) (result i32)
    local.get $value
    i32.const 100
    i32.gt_u
    if
      i32.const 100
      local.set $value
    end
    local.get $value
    i32.const 255
    i32.mul
    i32.const 100
    i32.div_u)

  (func $set_palette (param $base i32) (param $idx i32) (param $r i32) (param $g i32) (param $b i32)
    local.get $base
    local.get $idx
    i32.const 4
    i32.mul
    i32.add
    local.get $r
    i32.store8
    local.get $base
    local.get $idx
    i32.const 4
    i32.mul
    i32.add
    i32.const 1
    i32.add
    local.get $g
    i32.store8
    local.get $base
    local.get $idx
    i32.const 4
    i32.mul
    i32.add
    i32.const 2
    i32.add
    local.get $b
    i32.store8
    local.get $base
    local.get $idx
    i32.const 4
    i32.mul
    i32.add
    i32.const 3
    i32.add
    i32.const 255
    i32.store8)

  (func $init_palette (param $base i32)
    (local $i i32)
    i32.const 0
    local.set $i
    block $done
      loop $clear
        local.get $i
        i32.const 256
        i32.ge_u
        br_if $done
        local.get $base
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        i32.const 0
        i32.store8
        local.get $base
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        i32.const 1
        i32.add
        i32.const 0
        i32.store8
        local.get $base
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        i32.const 2
        i32.add
        i32.const 0
        i32.store8
        local.get $base
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        i32.const 3
        i32.add
        i32.const 255
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $clear
      end
    end
    local.get $base i32.const 0 i32.const 0 i32.const 0 i32.const 0 call $set_palette
    local.get $base i32.const 1 i32.const 51 i32.const 51 i32.const 204 call $set_palette
    local.get $base i32.const 2 i32.const 204 i32.const 51 i32.const 51 call $set_palette
    local.get $base i32.const 3 i32.const 51 i32.const 204 i32.const 51 call $set_palette
    local.get $base i32.const 4 i32.const 204 i32.const 51 i32.const 204 call $set_palette
    local.get $base i32.const 5 i32.const 51 i32.const 204 i32.const 204 call $set_palette
    local.get $base i32.const 6 i32.const 204 i32.const 204 i32.const 51 call $set_palette
    local.get $base i32.const 7 i32.const 229 i32.const 229 i32.const 229 call $set_palette)

  (func $parse_color_tail (param $ptr i32) (param $len i32) (param $idx i32) (param $palette i32) (param $color i32) (result i32)
    (local $mode i32)
    (local $r i32)
    (local $g i32)
    (local $b i32)
    (local $next i32)
    local.get $idx
    local.get $len
    i32.ge_u
    if
      local.get $idx
      return
    end
    local.get $ptr
    local.get $idx
    i32.add
    i32.load8_u
    i32.const 59
    i32.ne
    if
      local.get $idx
      return
    end
    local.get $ptr
    local.get $len
    local.get $idx
    i32.const 1
    i32.add
    call $parse_number
    local.set $next
    local.set $mode
    local.get $next
    i32.const -1
    i32.eq
    if i32.const -1 return end
    local.get $next
    local.set $idx
    local.get $idx
    local.get $len
    i32.ge_u
    if i32.const -1 return end
    local.get $ptr
    local.get $idx
    i32.add
    i32.load8_u
    i32.const 59
    i32.ne
    if i32.const -1 return end
    local.get $ptr
    local.get $len
    local.get $idx
    i32.const 1
    i32.add
    call $parse_number
    local.set $next
    local.set $r
    local.get $next
    i32.const -1
    i32.eq
    if i32.const -1 return end
    local.get $next
    local.set $idx
    local.get $idx
    local.get $len
    i32.ge_u
    if i32.const -1 return end
    local.get $ptr
    local.get $idx
    i32.add
    i32.load8_u
    i32.const 59
    i32.ne
    if i32.const -1 return end
    local.get $ptr
    local.get $len
    local.get $idx
    i32.const 1
    i32.add
    call $parse_number
    local.set $next
    local.set $g
    local.get $next
    i32.const -1
    i32.eq
    if i32.const -1 return end
    local.get $next
    local.set $idx
    local.get $idx
    local.get $len
    i32.ge_u
    if i32.const -1 return end
    local.get $ptr
    local.get $idx
    i32.add
    i32.load8_u
    i32.const 59
    i32.ne
    if i32.const -1 return end
    local.get $ptr
    local.get $len
    local.get $idx
    i32.const 1
    i32.add
    call $parse_number
    local.set $next
    local.set $b
    local.get $next
    i32.const -1
    i32.eq
    if i32.const -1 return end
    local.get $next
    local.set $idx
    local.get $mode
    i32.const 2
    i32.ne
    if i32.const -1 return end
    local.get $palette
    local.get $color
    local.get $r
    call $scale_percent
    local.get $g
    call $scale_percent
    local.get $b
    call $scale_percent
    call $set_palette
    local.get $idx)

  (func $scan_dims (param $ptr i32) (param $len i32) (param $zero_color i32) (param $grid_size i32) (param $meta i32) (result i32)
    (local $idx i32)
    (local $c i32)
    (local $cursor_x i32)
    (local $cursor_y i32)
    (local $max_x i32)
    (local $max_y i32)
    (local $width i32)
    (local $height i32)
    (local $number i32)
    (local $next i32)
    local.get $grid_size
    local.set $width
    local.get $grid_size
    i32.const 0
    i32.gt_u
    if
      i32.const 6
      local.set $height
    else
      i32.const 0
      local.set $height
    end
    local.get $grid_size
    local.set $max_y
    block $done
      loop $scan
        local.get $idx
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $idx
        i32.add
        i32.load8_u
        local.set $c
        local.get $c
        call $is_sixel
        if
          local.get $cursor_x
          i32.const 1
          i32.add
          local.set $cursor_x
          local.get $cursor_x
          local.get $max_x
          i32.gt_u
          if local.get $cursor_x local.set $max_x end
          local.get $cursor_y
          i32.const 6
          i32.add
          local.get $max_y
          i32.gt_u
          if local.get $cursor_y i32.const 6 i32.add local.set $max_y end
          local.get $idx
          i32.const 1
          i32.add
          local.set $idx
          br $scan
        end
        local.get $c
        i32.const 33
        i32.eq
        if
          local.get $ptr
          local.get $len
          local.get $idx
          i32.const 1
          i32.add
          call $parse_number
          local.set $next
          local.set $number
          local.get $next
          i32.const -1
          i32.eq
          if i32.const 3 return end
          local.get $next
          local.get $len
          i32.ge_u
          if i32.const 3 return end
          local.get $ptr
          local.get $next
          i32.add
          i32.load8_u
          call $is_sixel
          i32.eqz
          if i32.const 3 return end
          local.get $cursor_x
          local.get $number
          i32.add
          local.set $cursor_x
          local.get $cursor_x
          local.get $max_x
          i32.gt_u
          if local.get $cursor_x local.set $max_x end
          local.get $cursor_y
          i32.const 6
          i32.add
          local.get $max_y
          i32.gt_u
          if local.get $cursor_y i32.const 6 i32.add local.set $max_y end
          local.get $next
          i32.const 1
          i32.add
          local.set $idx
          br $scan
        end
        local.get $c
        i32.const 35
        i32.eq
        if
          local.get $ptr
          local.get $len
          local.get $idx
          i32.const 1
          i32.add
          call $parse_number
          local.set $next
          local.set $number
          local.get $next
          i32.const -1
          i32.eq
          if i32.const 3 return end
          local.get $next
          local.set $idx
          local.get $idx
          local.get $len
          i32.lt_u
          if
            local.get $ptr
            local.get $idx
            i32.add
            i32.load8_u
            i32.const 59
            i32.eq
            if
              local.get $ptr
              local.get $len
              local.get $idx
              i32.const 60000
              local.get $number
              i32.const 255
              i32.and
              call $parse_color_tail
              local.set $next
              local.get $next
              i32.const -1
              i32.eq
              if i32.const 3 return end
              local.get $next
              local.set $idx
            end
          end
          br $scan
        end
        local.get $c
        i32.const 34
        i32.eq
        if
          local.get $ptr
          local.get $len
          local.get $idx
          i32.const 1
          i32.add
          local.get $meta
          call $parse_raster_dims
          local.set $next
          local.get $next
          i32.const -1
          i32.ne
          if
            local.get $meta
            i32.load
            local.get $width
            i32.gt_u
            if local.get $meta i32.load local.set $width end
            local.get $meta
            i32.const 4
            i32.add
            i32.load
            local.get $height
            i32.gt_u
            if local.get $meta i32.const 4 i32.add i32.load local.set $height end
            local.get $width
            local.get $max_x
            i32.gt_u
            if local.get $width local.set $max_x end
            local.get $height
            local.get $max_y
            i32.gt_u
            if local.get $height local.set $max_y end
            local.get $next
            local.set $idx
            br $scan
          end
        end
        local.get $c
        i32.const 36
        i32.eq
        if
          i32.const 0
          local.set $cursor_x
          local.get $idx
          i32.const 1
          i32.add
          local.set $idx
          br $scan
        end
        local.get $c
        i32.const 45
        i32.eq
        if
          i32.const 0
          local.set $cursor_x
          local.get $cursor_y
          i32.const 6
          i32.add
          local.set $cursor_y
          local.get $cursor_y
          i32.const 6
          i32.add
          local.get $max_y
          i32.gt_u
          if local.get $cursor_y i32.const 6 i32.add local.set $max_y end
          local.get $idx
          i32.const 1
          i32.add
          local.set $idx
          br $scan
        end
        local.get $idx
        i32.const 1
        i32.add
        local.set $idx
        br $scan
      end
    end
    local.get $max_x
    local.get $width
    i32.lt_u
    if local.get $width local.set $max_x end
    local.get $max_y
    local.get $height
    i32.lt_u
    if local.get $height local.set $max_y end
    local.get $max_x
    i32.const 0
    i32.eq
    if i32.const 1 local.set $max_x end
    local.get $max_y
    i32.const 6
    i32.lt_u
    if i32.const 6 local.set $max_y end
    local.get $meta
    local.get $max_x
    i32.store
    local.get $meta
    i32.const 4
    i32.add
    local.get $max_y
    i32.store
    i32.const 0)

  (func $parse_raster_dims (param $ptr i32) (param $len i32) (param $idx i32) (param $meta i32) (result i32)
    (local $v0 i32)
    (local $v1 i32)
    (local $w i32)
    (local $h i32)
    (local $next i32)
    local.get $ptr local.get $len local.get $idx call $parse_number
    local.set $next
    local.set $v0
    local.get $next
    i32.const -1
    i32.eq
    if i32.const -1 return end
    local.get $next
    local.set $idx
    local.get $idx local.get $len i32.ge_u if i32.const -1 return end
    local.get $ptr local.get $idx i32.add i32.load8_u i32.const 59 i32.ne
    if i32.const -1 return end
    local.get $ptr local.get $len local.get $idx i32.const 1 i32.add call $parse_number
    local.set $next
    local.set $v1
    local.get $next i32.const -1 i32.eq if i32.const -1 return end
    local.get $next local.set $idx
    local.get $idx local.get $len i32.ge_u if i32.const -1 return end
    local.get $ptr local.get $idx i32.add i32.load8_u i32.const 59 i32.ne
    if i32.const -1 return end
    local.get $ptr local.get $len local.get $idx i32.const 1 i32.add call $parse_number
    local.set $next
    local.set $w
    local.get $next i32.const -1 i32.eq if i32.const -1 return end
    local.get $next local.set $idx
    local.get $idx local.get $len i32.ge_u if i32.const -1 return end
    local.get $ptr local.get $idx i32.add i32.load8_u i32.const 59 i32.ne
    if i32.const -1 return end
    local.get $ptr local.get $len local.get $idx i32.const 1 i32.add call $parse_number
    local.set $next
    local.set $h
    local.get $next i32.const -1 i32.eq if i32.const -1 return end
    local.get $meta local.get $w i32.store
    local.get $meta i32.const 4 i32.add local.get $h i32.const 6 i32.lt_u if (result i32) i32.const 6 else local.get $h end i32.store
    local.get $next)

  (func $paint (param $out i32) (param $width i32) (param $x i32) (param $y i32) (param $bits i32) (param $palette i32) (param $color i32)
    (local $bit i32)
    (local $dst i32)
    i32.const 0
    local.set $bit
    block $done
      loop $bits
        local.get $bit
        i32.const 6
        i32.ge_u
        br_if $done
        local.get $bits
        i32.const 1
        local.get $bit
        i32.shl
        i32.and
        if
          local.get $out
          local.get $y
          local.get $bit
          i32.add
          local.get $width
          i32.mul
          local.get $x
          i32.add
          i32.const 4
          i32.mul
          i32.add
          local.set $dst
          local.get $dst
          local.get $palette
          local.get $color
          i32.const 4
          i32.mul
          i32.add
          i32.load8_u
          i32.store8
          local.get $dst
          i32.const 1
          i32.add
          local.get $palette
          local.get $color
          i32.const 4
          i32.mul
          i32.add
          i32.const 1
          i32.add
          i32.load8_u
          i32.store8
          local.get $dst
          i32.const 2
          i32.add
          local.get $palette
          local.get $color
          i32.const 4
          i32.mul
          i32.add
          i32.const 2
          i32.add
          i32.load8_u
          i32.store8
          local.get $dst
          i32.const 3
          i32.add
          i32.const 255
          i32.store8
        end
        local.get $bit
        i32.const 1
        i32.add
        local.set $bit
        br $bits
      end
    end)

  (func (export "sixel_decode_rgba") (param $ptr i32) (param $len i32) (param $zero_color i32) (param $grid_size i32) (param $out i32) (param $out_cap i32) (param $meta i32) (result i64)
    (local $palette i32)
    (local $status i32)
    (local $width i32)
    (local $height i32)
    (local $written i32)
    (local $idx i32)
    (local $c i32)
    (local $cursor_x i32)
    (local $cursor_y i32)
    (local $color i32)
    (local $number i32)
    (local $next i32)
    (local $repeat i32)
    local.get $len
    i32.eqz
    if
      i32.const 1
      i32.const 0
      call $pack
      return
    end
    i32.const 60000
    local.set $palette
    local.get $palette
    call $init_palette
    local.get $ptr
    local.get $len
    local.get $zero_color
    local.get $grid_size
    local.get $meta
    call $scan_dims
    local.set $status
    local.get $status
    i32.const 0
    i32.ne
    if
      local.get $status
      i32.const 0
      call $pack
      return
    end
    local.get $meta
    i32.load
    local.set $width
    local.get $meta
    i32.const 4
    i32.add
    i32.load
    local.set $height
    local.get $width
    local.get $height
    i32.mul
    i32.const 4
    i32.mul
    local.set $written
    local.get $written
    local.get $out_cap
    i32.gt_u
    if
      local.get $meta
      i32.const 8
      i32.add
      local.get $written
      i32.store
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $out
    i32.const 0
    local.get $written
    memory.fill
    local.get $palette
    call $init_palette
    local.get $zero_color
    i32.const 255
    i32.and
    local.set $color
    block $done
      loop $decode
        local.get $idx
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $idx
        i32.add
        i32.load8_u
        local.set $c
        local.get $c
        call $is_sixel
        if
          local.get $out
          local.get $width
          local.get $cursor_x
          local.get $cursor_y
          local.get $c
          i32.const 63
          i32.sub
          local.get $palette
          local.get $color
          call $paint
          local.get $cursor_x
          i32.const 1
          i32.add
          local.set $cursor_x
          local.get $idx
          i32.const 1
          i32.add
          local.set $idx
          br $decode
        end
        local.get $c
        i32.const 33
        i32.eq
        if
          local.get $ptr local.get $len local.get $idx i32.const 1 i32.add call $parse_number
          local.set $next
          local.set $number
          local.get $next i32.const -1 i32.eq if i32.const 3 i32.const 0 call $pack return end
          local.get $next local.get $len i32.ge_u if i32.const 3 i32.const 0 call $pack return end
          local.get $ptr local.get $next i32.add i32.load8_u call $is_sixel i32.eqz
          if i32.const 3 i32.const 0 call $pack return end
          i32.const 0
          local.set $repeat
          block $repdone
            loop $reploop
              local.get $repeat
              local.get $number
              i32.ge_u
              br_if $repdone
              local.get $out
              local.get $width
              local.get $cursor_x
              local.get $cursor_y
              local.get $ptr
              local.get $next
              i32.add
              i32.load8_u
              i32.const 63
              i32.sub
              local.get $palette
              local.get $color
              call $paint
              local.get $cursor_x i32.const 1 i32.add local.set $cursor_x
              local.get $repeat i32.const 1 i32.add local.set $repeat
              br $reploop
            end
          end
          local.get $next i32.const 1 i32.add local.set $idx
          br $decode
        end
        local.get $c
        i32.const 35
        i32.eq
        if
          local.get $ptr local.get $len local.get $idx i32.const 1 i32.add call $parse_number
          local.set $next
          local.set $number
          local.get $next i32.const -1 i32.eq if i32.const 3 i32.const 0 call $pack return end
          local.get $number i32.const 255 i32.gt_u
          if i32.const 255 local.set $color else local.get $number local.set $color end
          local.get $next local.set $idx
          local.get $idx local.get $len i32.lt_u
          if
            local.get $ptr local.get $idx i32.add i32.load8_u i32.const 59 i32.eq
            if
              local.get $ptr local.get $len local.get $idx local.get $palette local.get $color call $parse_color_tail
              local.set $next
              local.get $next i32.const -1 i32.eq if i32.const 3 i32.const 0 call $pack return end
              local.get $next local.set $idx
            end
          end
          br $decode
        end
        local.get $c
        i32.const 36
        i32.eq
        if
          i32.const 0 local.set $cursor_x
          local.get $idx i32.const 1 i32.add local.set $idx
          br $decode
        end
        local.get $c
        i32.const 45
        i32.eq
        if
          i32.const 0 local.set $cursor_x
          local.get $cursor_y i32.const 6 i32.add local.set $cursor_y
          local.get $idx i32.const 1 i32.add local.set $idx
          br $decode
        end
        local.get $idx
        i32.const 1
        i32.add
        local.set $idx
        br $decode
      end
    end
    local.get $meta
    i32.const 8
    i32.add
    local.get $written
    i32.store
    i32.const 0
    local.get $written
    call $pack)