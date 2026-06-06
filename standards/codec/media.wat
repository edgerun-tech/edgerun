(module
  (import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))
  (import "edgerun" "lo" (func $lo (param i64) (result i32)))
  (import "edgerun" "hi" (func $hi (param i64) (result i32)))
  (import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))
  (memory (export "memory") 1)

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

;; Baseline JPEG entropy decoder for the retained title.jpg metadata.
  ;;
  ;; Decodes the first MCU block from a JPEG entropy-coded scan using the
  ;; caller-populated Huffman and quantization tables.
  ;;
  ;; State buffer layout (caller-allocated, passed via state_ptr):
  ;;   Offset  Size  Field
  ;;       0     4   entropy_ptr (offset into scan_data within state buffer)
  ;;       4     4   entropy_end  (total scan data length)
  ;;       8     4   bit_buf
  ;;      12     4   bit_count
  ;;      16    12   dc_predictors[3] (i32 × 3)
  ;;      28     4   component_count
  ;;      32     4   scan_component_count
  ;;      36     4   max_h_samp
  ;;      40     4   max_v_samp
  ;;      44    16   scan_component_ids[4]    (i32 × 4)
  ;;      60    16   frame_component_ids[4]   (i32 × 4)
  ;;      76    16   frame_component_qtables[4] (i32 × 4)
  ;;      92    16   scan_component_dc_tables[4] (i32 × 4)
  ;;     108    16   scan_component_ac_tables[4] (i32 × 4)
  ;;     124   128   huffman_code_counts (8 tables × 16 bytes)
  ;;     252    32   huffman_symbol_counts (8 tables × i32)
  ;;     284  2048   huffman_symbols (8 tables × 256 bytes)
  ;;    2332   256   quant_tables (4 tables × 64 bytes)
  ;;    2588   N     scan data (variable length)
  ;; Total header: 2588 bytes.  Callers should allocate ≥ 4096 bytes.
  ;;
  ;; The zigzag-to-natural mapping table is at memory address 0 (64 bytes).
  ;;
  ;; Exports:
  ;;   jpeg_decode_scan_start(state_ptr) → 0 ok, -1 fail
  ;;   jpeg_decode_first_mcu_block(state_ptr, output_64coeffs) → 0 ok, -1 fail
  ;;   jpeg_decode_next_mcu_dequantized(state_ptr, dst_y, dst_cb, dst_cr) → 0 ok, -1 fail(data (i32.const 0) "\00\01\08\10\09\02\03\0a")

  ;; Read byte from entropy stream, handling 0xFF-stuffing.
  ;; state_ptr → byte (0-255) or -1 on failure.
  (func $entropy_read_byte (param $s i32) (result i32)
    (local $ptr i32) (local $end i32) (local $byte i32) (local $scan_base i32)
    i32.const 2588 local.set $scan_base
    local.get $s i32.load local.set $ptr
    local.get $s i32.load offset=4 local.set $end
    local.get $ptr local.get $end i32.ge_u if i32.const -1 return end
    local.get $s local.get $scan_base i32.add local.get $ptr i32.add i32.load8_u
    local.set $byte
    local.get $ptr i32.const 1 i32.add local.set $ptr
    local.get $byte i32.const 0xff i32.ne
    if
      local.get $s local.get $ptr i32.store
      local.get $byte return
    end
    local.get $ptr local.get $end i32.ge_u if i32.const -1 return end
    local.get $s local.get $scan_base i32.add local.get $ptr i32.add i32.load8_u
    i32.const 0x00 i32.ne if i32.const -1 return end
    local.get $ptr i32.const 1 i32.add local.set $ptr
    local.get $s local.get $ptr i32.store
    i32.const 0xff
  )

  ;; Read one MSB-first entropy-coded bit.
  ;; state_ptr → bit (0/1) or -1 on failure.
  (func $entropy_read_bit (param $s i32) (result i32)
    (local $bc i32) (local $bb i32) (local $bit i32)
    local.get $s i32.load offset=12
    local.tee $bc
    if
      local.get $bc i32.const 1 i32.sub local.set $bc
      local.get $s i32.load offset=8
      local.get $bc i32.shr_u
      i32.const 1 i32.and
      local.set $bit
      local.get $s local.get $bc i32.store offset=12
      local.get $bit return
    end
    local.get $s call $entropy_read_byte
    local.tee $bb
    i32.const -1 i32.eq if i32.const -1 return end
    local.get $s local.get $bb i32.store offset=8
    local.get $s i32.const 8 i32.store offset=12
    local.get $bb i32.const 7 i32.shr_u i32.const 1 i32.and
    local.get $s i32.const 7 i32.store offset=12
  )

  ;; Receive and sign-extend a JPEG bit pattern.
  ;; state_ptr, category (0-15) → signed value, error_flag (0=ok, -1=fail)
  (func $receive_extend (param $s i32) (param $cat i32) (result i32 i32)
    (local $val i32) (local $i i32) (local $bit i32) (local $thresh i32)
    local.get $cat i32.const 16 i32.gt_u if i32.const 0 i32.const -1 return end
    local.get $cat i32.eqz if i32.const 0 i32.const 0 return end
    i32.const 0 local.set $val
    i32.const 0 local.set $i
    block $read_done
    loop $read_loop
      local.get $i local.get $cat i32.ge_u br_if $read_done
      local.get $s call $entropy_read_bit
      local.tee $bit
      i32.const -1 i32.eq if i32.const 0 i32.const -1 return end
      local.get $val i32.const 1 i32.shl local.get $bit i32.or local.set $val
      local.get $i i32.const 1 i32.add local.set $i
      br $read_loop
    end
    end
    i32.const 1
    local.get $cat i32.const 1 i32.sub
    i32.shl
    local.set $thresh
    local.get $val local.get $thresh i32.ge_u
    if
      local.get $val i32.const 0 return
    end
    local.get $val
    i32.const 1
    local.get $cat
    i32.shl
    i32.const 1 i32.sub
    i32.sub
    i32.const 0
  )

  ;; Decode a canonical Huffman code symbol.
  ;; state_ptr, table_index → symbol (0-255) or -1 on failure.
  (func $huffman_decode_symbol (param $s i32) (param $tbl i32) (result i32)
    (local $sym_cnt i32) (local $code i32) (local $first_code i32)
    (local $first_sym i32) (local $len_idx i32) (local $cnt i32)
    (local $codecnt_off i32) (local $sym_off i32) (local $bit i32)
    (local $diff i32)

    local.get $tbl i32.const 8 i32.ge_u if i32.const -1 return end
    local.get $s i32.const 252 i32.add local.get $tbl i32.const 2 i32.shl i32.add i32.load
    local.tee $sym_cnt
    i32.eqz if i32.const -1 return end
    local.get $sym_cnt i32.const 256 i32.gt_u if i32.const -1 return end

    local.get $tbl i32.const 4 i32.shl local.set $codecnt_off
    local.get $tbl i32.const 8 i32.shl local.set $sym_off
    i32.const 0 local.set $code
    i32.const 0 local.set $first_code
    i32.const 0 local.set $first_sym
    i32.const 0 local.set $len_idx

    block $fail
    loop $len_loop
      local.get $len_idx i32.const 16 i32.ge_u br_if $fail

      local.get $s call $entropy_read_bit
      local.tee $bit
      i32.const -1 i32.eq br_if $fail

      local.get $code i32.const 1 i32.shl local.get $bit i32.or local.set $code
      local.get $first_code i32.const 1 i32.shl local.set $first_code

      local.get $s i32.const 124 i32.add
      local.get $codecnt_off local.get $len_idx i32.add i32.add
      i32.load8_u
      local.tee $cnt
      i32.eqz
      if else
        local.get $code local.get $first_code i32.ge_u
        if
          local.get $code local.get $first_code i32.sub
          local.tee $diff
          local.get $cnt i32.lt_u
          if
            local.get $first_sym local.get $diff i32.add
            local.tee $diff
            local.get $sym_cnt i32.ge_u br_if $fail
            local.get $s i32.const 284 i32.add
            local.get $sym_off i32.add
            local.get $diff i32.add
            i32.load8_u
            return
          end
        end
      end

      local.get $first_code local.get $cnt i32.add local.set $first_code
      local.get $first_sym local.get $cnt i32.add local.set $first_sym
      local.get $len_idx i32.const 1 i32.add local.set $len_idx
      br $len_loop
    end
    end
    i32.const -1
  )

  ;; Decode one component's 64 coefficients (DC + AC) with dequantization.
  ;; state_ptr, component_index (0-2), output_dst → 0 ok, -1 fail
  (func $decode_component_dequantized (param $s i32) (param $comp i32) (param $dst i32) (result i32)
    (local $cursor i32) (local $symbol i32) (local $value i32)
    (local $dc_tbl i32) (local $ac_tbl i32) (local $qtbl_idx i32)
    (local $nat_idx i32) (local $qtbl_off i32) (local $err i32)

    local.get $comp i32.const 3 i32.ge_u if i32.const -1 return end

    ;; Validate component IDs match
    local.get $s i32.const 44 i32.add local.get $comp i32.const 2 i32.shl i32.add i32.load
    local.get $s i32.const 60 i32.add local.get $comp i32.const 2 i32.shl i32.add i32.load
    i32.ne if i32.const -1 return end

    ;; Get quant table index
    local.get $s i32.const 76 i32.add local.get $comp i32.const 2 i32.shl i32.add i32.load
    local.tee $qtbl_idx
    i32.const 4 i32.ge_u if i32.const -1 return end

    ;; Zero output buffer (64 int16 = 128 bytes)
    i32.const 0 local.set $cursor
    block $zero_done
    loop $zero_loop
      local.get $cursor i32.const 128 i32.ge_s br_if $zero_done
      local.get $dst local.get $cursor i32.add i32.const 0 i32.store16
      local.get $cursor i32.const 2 i32.add local.set $cursor
      br $zero_loop
    end
    end

    ;; Get DC table index for this component
    local.get $s i32.const 92 i32.add local.get $comp i32.const 2 i32.shl i32.add i32.load
    local.tee $dc_tbl
    i32.const 4 i32.ge_u if i32.const -1 return end

    ;; Decode DC coefficient
    local.get $s local.get $dc_tbl call $huffman_decode_symbol
    local.tee $symbol
    i32.const -1 i32.eq if i32.const -1 return end
    local.get $symbol i32.const 16 i32.gt_u if i32.const -1 return end

    local.get $s local.get $symbol call $receive_extend
    local.set $err
    local.set $value
    local.get $err if i32.const -1 return end

    ;; Update DC predictor
    local.get $s i32.const 16 i32.add local.get $comp i32.const 2 i32.shl i32.add
    local.get $s i32.const 16 i32.add local.get $comp i32.const 2 i32.shl i32.add i32.load
    local.get $value i32.add
    local.tee $value
    i32.store
    local.get $dst local.get $value i32.store16

    ;; Get AC table index
    local.get $s i32.const 108 i32.add local.get $comp i32.const 2 i32.shl i32.add i32.load
    local.tee $ac_tbl
    i32.const 4 i32.ge_u if i32.const -1 return end

    i32.const 1 local.set $cursor
    block $ac_done
    loop $ac_loop
      local.get $cursor i32.const 64 i32.ge_u br_if $ac_done

      local.get $s local.get $ac_tbl i32.const 4 i32.add call $huffman_decode_symbol
      local.tee $symbol
      i32.const -1 i32.eq if i32.const -1 return end

      local.get $symbol i32.eqz if br $ac_done end

      local.get $symbol i32.const 0xf0 i32.eq
      if
        local.get $cursor i32.const 16 i32.add local.set $cursor
        local.get $cursor i32.const 64 i32.gt_u if i32.const -1 return end
        br $ac_loop
      end

      ;; Extract zero-run and amplitude bits
      local.get $symbol i32.const 4 i32.shr_u
      local.get $cursor i32.add local.set $cursor
      local.get $symbol i32.const 15 i32.and
      local.tee $symbol
      i32.eqz if i32.const -1 return end
      local.get $cursor i32.const 64 i32.ge_u if i32.const -1 return end

      local.get $s local.get $symbol call $receive_extend
      local.set $err
      local.set $value
      local.get $err if i32.const -1 return end

      local.get $cursor i32.load8_u
      local.tee $nat_idx
      i32.const 1 i32.shl
      local.get $dst i32.add
      local.get $value i32.store16

      local.get $cursor i32.const 1 i32.add local.set $cursor
      br $ac_loop
    end
    end

    ;; Dequantize
    local.get $qtbl_idx i32.const 6 i32.shl
    local.set $qtbl_off
    i32.const 0 local.set $cursor
    block $dequant_done
    loop $dequant_loop
      local.get $cursor i32.const 64 i32.ge_u br_if $dequant_done

      local.get $cursor i32.load8_u
      local.tee $nat_idx
      i32.const 1 i32.shl
      local.get $dst i32.add

      local.get $dst
      local.get $nat_idx i32.const 1 i32.shl i32.add
      i32.load16_s
      local.get $s i32.const 2332 i32.add local.get $qtbl_off local.get $cursor i32.add i32.add i32.load8_u
      i32.mul
      i32.store16

      local.get $cursor i32.const 1 i32.add local.set $cursor
      br $dequant_loop
    end
    end
    i32.const 0
  )

  ;; Initialize entropy decoder state for a new scan.
  ;; Resets bit buffer, bit count, and DC predictors.  Sets entropy pointer
  ;; to the beginning of the scan data within the state buffer.
  ;; state_ptr → 0 ok, -1 if no scan data
  (func (export "jpeg_decode_scan_start") (param $s i32) (result i32)
    local.get $s i32.load offset=4
    i32.eqz if i32.const -1 return end
    local.get $s i32.const 0 i32.store
    local.get $s i32.const 0 i32.store offset=8
    local.get $s i32.const 0 i32.store offset=12
    local.get $s i32.const 0 i32.store offset=16
    local.get $s i32.const 0 i32.store offset=20
    local.get $s i32.const 0 i32.store offset=24
    i32.const 0
  )

  ;; Decode the first MCU block (64 raw int16 coefficients) from the
  ;; entropy-coded scan, using hardcoded DC table 0 and AC table 4.
  ;; This does NOT use component tables or dequantization — it decodes
  ;; the raw DC+AC coefficients exactly as `title_jpeg_decode_first_mcu_block`.
  ;; state_ptr, output_64coeffs → 0 ok, -1 fail
  (func (export "jpeg_decode_first_mcu_block") (param $s i32) (param $out i32) (result i32)
    (local $cursor i32) (local $symbol i32) (local $value i32)
    (local $nat_idx i32) (local $err i32)

    local.get $s i32.load offset=4 i32.eqz if i32.const -1 return end
    local.get $s i32.const 0 i32.store
    local.get $s i32.const 0 i32.store offset=8
    local.get $s i32.const 0 i32.store offset=12

    ;; Zero output (64 int16 = 128 bytes)
    i32.const 0 local.set $cursor
    block $zero_done
    loop $zero_loop
      local.get $cursor i32.const 128 i32.ge_s br_if $zero_done
      local.get $out local.get $cursor i32.add i32.const 0 i32.store16
      local.get $cursor i32.const 2 i32.add local.set $cursor
      br $zero_loop
    end
    end

    ;; Decode DC coefficient (table 0)
    local.get $s i32.const 0 call $huffman_decode_symbol
    local.tee $symbol
    i32.const -1 i32.eq if i32.const -1 return end
    local.get $symbol i32.const 16 i32.gt_u if i32.const -1 return end

    local.get $s local.get $symbol call $receive_extend
    local.set $err
    local.set $value
    local.get $err if i32.const -1 return end
    local.get $out local.get $value i32.store16

    ;; Decode AC coefficients (table 4 = AC table 0)
    i32.const 1 local.set $cursor
    block $ac_done
    loop $ac_loop
      local.get $cursor i32.const 64 i32.ge_u br_if $ac_done

      local.get $s i32.const 4 call $huffman_decode_symbol
      local.tee $symbol
      i32.const -1 i32.eq if i32.const -1 return end

      local.get $symbol i32.eqz if br $ac_done end

      local.get $symbol i32.const 0xf0 i32.eq
      if
        local.get $cursor i32.const 16 i32.add local.set $cursor
        local.get $cursor i32.const 64 i32.gt_u if i32.const -1 return end
        br $ac_loop
      end

      local.get $symbol i32.const 4 i32.shr_u
      local.get $cursor i32.add local.set $cursor
      local.get $symbol i32.const 15 i32.and
      local.tee $symbol
      i32.eqz if i32.const -1 return end
      local.get $cursor i32.const 64 i32.ge_u if i32.const -1 return end

      local.get $s local.get $symbol call $receive_extend
      local.set $err
      local.set $value
      local.get $err if i32.const -1 return end

      local.get $cursor i32.load8_u
      local.tee $nat_idx
      i32.const 1 i32.shl
      local.get $out i32.add
      local.get $value i32.store16

      local.get $cursor i32.const 1 i32.add local.set $cursor
      br $ac_loop
    end
    end
    i32.const 0
  )

  ;; Decode and dequantize a complete 3-component 1x1-sampled MCU.
  ;; state_ptr must be initialized with jpeg_decode_scan_start first.
  ;; Components are decoded in natural order (Y, Cb, Cr).
  ;; Each output buffer must be 64 int16 coefficients (128 bytes).
  ;; state_ptr, dst_y, dst_cb, dst_cr → 0 ok, -1 fail
  (func (export "jpeg_decode_next_mcu_dequantized")
    (param $s i32) (param $y i32) (param $cb i32) (param $cr i32)
    (result i32)

    local.get $y i32.eqz if i32.const -1 return end
    local.get $cb i32.eqz if i32.const -1 return end
    local.get $cr i32.eqz if i32.const -1 return end

    local.get $s i32.load offset=28
    i32.const 3 i32.ne if i32.const -1 return end
    local.get $s i32.load offset=32
    i32.const 3 i32.ne if i32.const -1 return end
    local.get $s i32.load offset=36
    i32.const 1 i32.ne if i32.const -1 return end
    local.get $s i32.load offset=40
    i32.const 1 i32.ne if i32.const -1 return end

    local.get $s i32.const 0 local.get $y call $decode_component_dequantized
    if i32.const -1 return end

    local.get $s i32.const 1 local.get $cb call $decode_component_dequantized
    if i32.const -1 return end

    local.get $s i32.const 2 local.get $cr call $decode_component_dequantized
    if i32.const -1 return end

    i32.const 0
  )

(global $VORBIS_OK i32 (i32.const 0))
  (global $VORBIS_ERR_BOUNDS i32 (i32.const -1))
  (global $VORBIS_ERR_BAD_ARCHIVE i32 (i32.const -2))
  (global $VORBIS_ERR_TOO_MANY_PACKETS i32 (i32.const -3))
  (global $VORBIS_ERR_PCM_TOO_LARGE i32 (i32.const -4))
  (global $VORBIS_ERR_TODO_DECODE i32 (i32.const -5))
  (global $VORBIS_ERR_TOO_MANY_BOOKS i32 (i32.const -6))
  (global $VORBIS_ERR_TOO_MANY_ENTRIES i32 (i32.const -7))
  (global $VORBIS_ERR_BAD_CODEBOOK i32 (i32.const -8))
  (global $VORBIS_ERR_BAD_INDEX i32 (i32.const -9))
  (global $VORBIS_ERR_TOO_MANY_SETUP i32 (i32.const -10))
  (global $VORBIS_MAX_PACKETS i32 (i32.const 1024))
  (global $VORBIS_MAX_PCM_FRAMES i32 (i32.const 262144))
  (global $VORBIS_MAX_CODEBOOKS i32 (i32.const 64))
  (global $VORBIS_MAX_CODEBOOK_ENTRIES i32 (i32.const 8192))
  (global $VORBIS_MAX_MULTIPLICANDS i32 (i32.const 512))
  (global $VORBIS_CODEBOOK_SYNC i32 (i32.const 5653314))
  (global $VORBIS_MAX_SETUP_OBJS i32 (i32.const 64))
  (global $VORBIS_MAX_FLOOR_PARTS i32 (i32.const 64))
  (global $VORBIS_MAX_FLOOR_CLASSES i32 (i32.const 64))
  (global $VORBIS_MAX_FLOOR_VALUES i32 (i32.const 512))
  (global $VORBIS_MAX_RESIDUE_CLASSES i32 (i32.const 64))
  (global $VORBIS_MAX_RESIDUE_BOOKS i32 (i32.const 512))
  (global $VORBIS_MAX_MAPPING_SUBMAPS i32 (i32.const 16))
  (global $VORBIS_MAX_BLOCK_SIZE i32 (i32.const 8192))
  (global $VORBIS_MAX_OVERLAP_FRAMES i32 (i32.const 4096))
  (global $VORBIS_MAX_PACKET_FLOOR_CLASSES i32 (i32.const 65536))
  (global $VORBIS_MAX_PACKET_FLOOR_VALUES i32 (i32.const 524288))
  (global $VORBIS_MAX_PACKET_FLOOR_SEGMENTS i32 (i32.const 524288))
  (global $VORBIS_MAX_PACKET_RESIDUE_PARTS i32 (i32.const 524288))
  (global $VORBIS_MAX_PACKET_RESIDUE_VALUES i32 (i32.const 524288))
  (global $VORBIS_PACKET_RESIDUE_PARTS_PER_PACKET i32 (i32.const 512))
  (global $VORBIS_PACKET_RESIDUE_VALUES_PER_PACKET i32 (i32.const 512))
  (global $SIZEOF_SETUP_STATE i32 (i32.const 68))
  (global $SIZEOF_SAMPLE_STATE i32 (i32.const 12328))
  (global $SIZEOF_RAWSOUND_VIEW i32 (i32.const 28))
  (global $SIZEOF_BITREADER i32 (i32.const 32))
  (global $SIZEOF_FLOOR_HEADER i32 (i32.const 32))
  (global $SIZEOF_RESIDUE_HEADER i32 (i32.const 28))
  (global $SIZEOF_MAPPING_HEADER i32 (i32.const 20))
  (global $SIZEOF_MODE_HEADER i32 (i32.const 8))
  (global $SIZEOF_CODEBOOK_HEADER i32 (i32.const 60))
  (global $BSS_SETUP_STATE i32 (i32.const 0x00000000))
  (global $BSS_SAMPLE_STATE i32 (i32.const 0x00000100))
  (global $BSS_RAWSOUND_VIEW i32 (i32.const 0x00003200))
  (global $BSS_BITREADER i32 (i32.const 0x00003300))
  (global $BSS_SILENCE_PCM i32 (i32.const 0x00003400))
  (global $BSS_FLOOR_RESIDUE_PCM i32 (i32.const 0x000C3400))
  (global $BSS_MDCT_SYNTH_PCM i32 (i32.const 0x00143400))
  (global $BSS_CODEBOOK_HEADERS i32 (i32.const 0x001C3400))
  (global $BSS_CODEBOOK_LENGTHS i32 (i32.const 0x001C4400))
  (global $BSS_CODEBOOK_CODES i32 (i32.const 0x001CC400))
  (global $BSS_CODEBOOK_MULTIPLICANDS i32 (i32.const 0x001D4400))
  (global $BSS_CANONICAL_TEMP i32 (i32.const 0x001D4C00))
  (global $BSS_FLOOR_HEADERS i32 (i32.const 0x001D5000))
  (global $BSS_FLOOR_PARTITIONS i32 (i32.const 0x001D5800))
  (global $BSS_FLOOR_CLASS_DIMS i32 (i32.const 0x001D5A00))
  (global $BSS_FLOOR_CLASS_SUBBITS i32 (i32.const 0x001D5C00))
  (global $BSS_FLOOR_CLASS_MASTER i32 (i32.const 0x001D5E00))
  (global $BSS_FLOOR_CLASS_BOOKS i32 (i32.const 0x001D6000))
  (global $BSS_FLOOR_VALUES i32 (i32.const 0x001D6800))
  (global $BSS_RESIDUE_HEADERS i32 (i32.const 0x001D7000))
  (global $BSS_RESIDUE_CASCADES i32 (i32.const 0x001D7800))
  (global $BSS_RESIDUE_BOOKS i32 (i32.const 0x001D7A00))
  (global $BSS_MAPPING_HEADERS i32 (i32.const 0x001D8200))
  (global $BSS_MAPPING_FLOORS i32 (i32.const 0x001D8800))
  (global $BSS_MAPPING_RESIDUES i32 (i32.const 0x001D9800))
  (global $BSS_MODE_HEADERS i32 (i32.const 0x001DA800))
  (global $BSS_PACKET_MODES i32 (i32.const 0x001DAC00))
  (global $BSS_PACKET_MAPPINGS i32 (i32.const 0x001DBC00))
  (global $BSS_PACKET_FLOORS i32 (i32.const 0x001DCC00))
  (global $BSS_PACKET_RESIDUES i32 (i32.const 0x001DDC00))
  (global $BSS_PACKET_FLOOR_NONZERO i32 (i32.const 0x001DEC00))
  (global $BSS_PACKET_FLOOR_Y0 i32 (i32.const 0x001DFC00))
  (global $BSS_PACKET_FLOOR_Y1 i32 (i32.const 0x001E0C00))
  (global $BSS_PACKET_FLOOR_CLASS_COUNT i32 (i32.const 0x001E1C00))
  (global $BSS_PACKET_FLOOR_CLASS_IDS i32 (i32.const 0x001E2C00))
  (global $BSS_PACKET_FLOOR_CLASS_SELECTORS i32 (i32.const 0x00222C00))
  (global $BSS_PACKET_FLOOR_CLASS_DIMS i32 (i32.const 0x00262C00))
  (global $BSS_PACKET_FLOOR_VALUE_BOOKS i32 (i32.const 0x002A2C00))
  (global $BSS_PACKET_FLOOR_VALUES i32 (i32.const 0x004A2C00))
  (global $BSS_PACKET_FLOOR_POINT_COUNT i32 (i32.const 0x006A2C00))
  (global $BSS_PACKET_FLOOR_POINT_X i32 (i32.const 0x006A3C00))
  (global $BSS_PACKET_FLOOR_POINT_Y i32 (i32.const 0x008A3C00))
  (global $BSS_PACKET_FLOOR_POINT_ORDER i32 (i32.const 0x00AA3C00))
  (global $BSS_PACKET_FLOOR_SEGMENT_COUNT i32 (i32.const 0x00CA3C00))
  (global $BSS_PACKET_FLOOR_SEGMENT_X0 i32 (i32.const 0x00CA4C00))
  (global $BSS_PACKET_FLOOR_SEGMENT_Y0 i32 (i32.const 0x00EA4C00))
  (global $BSS_PACKET_FLOOR_SEGMENT_X1 i32 (i32.const 0x010A4C00))
  (global $BSS_PACKET_FLOOR_SEGMENT_Y1 i32 (i32.const 0x012A4C00))
  (global $BSS_PACKET_RESIDUE_PART_COUNT i32 (i32.const 0x014A4C00))
  (global $BSS_PACKET_RESIDUE_CLASSIFICATIONS i32 (i32.const 0x014A5C00))
  (global $BSS_PACKET_RESIDUE_VALUE_COUNT i32 (i32.const 0x016A5C00))
  (global $BSS_PACKET_RESIDUE_VALUE_BOOKS i32 (i32.const 0x016A6C00))
  (global $BSS_PACKET_RESIDUE_VALUE_ENTRIES i32 (i32.const 0x018A6C00))
  (global $BSS_PACKET_RESIDUE_VALUE_DIMS i32 (i32.const 0x01AA6C00))
  (global $BSS_PACKET_RESIDUE_VALUE_TARGETS i32 (i32.const 0x01CA6C00))
  (global $BSS_PACKET_RESIDUE_VALUE_VALUES i32 (i32.const 0x01EA6C00))
  (global $BSS_PACKET_BLOCK_SIZES i32 (i32.const 0x020A6C00))
  (global $BSS_PACKET_PREV_WINDOW_FLAGS i32 (i32.const 0x020A7C00))
  (global $BSS_PACKET_NEXT_WINDOW_FLAGS i32 (i32.const 0x020A8C00))
  (global $BSS_PACKET_WINDOW_LEFT_START i32 (i32.const 0x020A9C00))
  (global $BSS_PACKET_WINDOW_LEFT_END i32 (i32.const 0x020AAC00))
  (global $BSS_PACKET_WINDOW_RIGHT_START i32 (i32.const 0x020ABC00))
  (global $BSS_PACKET_WINDOW_RIGHT_END i32 (i32.const 0x020ACC00))
  (global $BSS_PACKET_WINDOW_LEFT_FRAMES i32 (i32.const 0x020ADC00))
  (global $BSS_PACKET_WINDOW_RIGHT_FRAMES i32 (i32.const 0x020AEC00))
  (global $BSS_MDCT_OUTPUT_Q15 i32 (i32.const 0x020AFC00))
  (global $BSS_WINDOW_Q15 i32 (i32.const 0x020B7C00))
  (global $BSS_LAST_CODEBOOK_STATUS i32 (i32.const 0x020BFC00))
  (global $BSS_LAST_CODEBOOK_COUNT i32 (i32.const 0x020BFC04))
  (global $BSS_LAST_CODEBOOK_ENTRIES i32 (i32.const 0x020BFC08))
  (global $BSS_LAST_CODEBOOK_NONZERO i32 (i32.const 0x020BFC0C))
  (global $BSS_LAST_CODEBOOK_MAX_LEN i32 (i32.const 0x020BFC10))
  (global $BSS_LAST_LOOKUP_VALUES i32 (i32.const 0x020BFC14))
  (global $BSS_LAST_SETUP_BYTE i32 (i32.const 0x020BFC18))
  (global $BSS_LAST_SETUP_BIT i32 (i32.const 0x020BFC1C))
  (global $BSS_LAST_FLOOR_STATUS i32 (i32.const 0x020BFC20))
  (global $BSS_LAST_RESIDUE_STATUS i32 (i32.const 0x020BFC24))
  (global $BSS_LAST_MAPPING_STATUS i32 (i32.const 0x020BFC28))
  (global $BSS_LAST_MODE_STATUS i32 (i32.const 0x020BFC2C))
  (global $BSS_LAST_FLOOR_VALUES i32 (i32.const 0x020BFC30))
  (global $BSS_LAST_RESIDUE_BOOKS i32 (i32.const 0x020BFC34))
  (global $BSS_LAST_PACKET_STATUS i32 (i32.const 0x020BFC38))
  (global $BSS_LAST_PACKET_COUNT i32 (i32.const 0x020BFC3C))
  (global $BSS_LAST_PACKET_MODE i32 (i32.const 0x020BFC40))
  (global $BSS_LAST_PACKET_MAPPING i32 (i32.const 0x020BFC44))
  (global $BSS_LAST_PACKET_FLOOR i32 (i32.const 0x020BFC48))
  (global $BSS_LAST_PACKET_RESIDUE i32 (i32.const 0x020BFC4C))
  (global $BSS_LAST_MODE_BITS i32 (i32.const 0x020BFC50))
  (global $BSS_LAST_WINDOW_STATUS i32 (i32.const 0x020BFC54))
  (global $BSS_LAST_WINDOW_PACKET i32 (i32.const 0x020BFC58))
  (global $BSS_LAST_WINDOW_BLOCK_SIZE i32 (i32.const 0x020BFC5C))
  (global $BSS_LAST_WINDOW_PREV_FLAG i32 (i32.const 0x020BFC60))
  (global $BSS_LAST_WINDOW_NEXT_FLAG i32 (i32.const 0x020BFC64))
  (global $BSS_LAST_WINDOW_LEFT_START i32 (i32.const 0x020BFC68))
  (global $BSS_LAST_WINDOW_LEFT_END i32 (i32.const 0x020BFC6C))
  (global $BSS_LAST_WINDOW_RIGHT_START i32 (i32.const 0x020BFC70))
  (global $BSS_LAST_WINDOW_RIGHT_END i32 (i32.const 0x020BFC74))
  (global $BSS_LAST_WINDOW_LEFT_FRAMES i32 (i32.const 0x020BFC78))
  (global $BSS_LAST_WINDOW_RIGHT_FRAMES i32 (i32.const 0x020BFC7C))
  (global $BSS_LAST_MDCT_BUFFER_FRAMES i32 (i32.const 0x020BFC80))
  (global $BSS_LAST_OVERLAP_FRAMES i32 (i32.const 0x020BFC84))
  (global $BSS_LAST_FLOOR_PACKET_STATUS i32 (i32.const 0x020BFC88))
  (global $BSS_LAST_FLOOR_PACKET_COUNT i32 (i32.const 0x020BFC8C))
  (global $BSS_LAST_FLOOR_PACKET_NONZERO i32 (i32.const 0x020BFC90))
  (global $BSS_LAST_FLOOR_PACKET_Y0 i32 (i32.const 0x020BFC94))
  (global $BSS_LAST_FLOOR_PACKET_Y1 i32 (i32.const 0x020BFC98))
  (global $BSS_LAST_FLOOR_PACKET_Y_RANGE i32 (i32.const 0x020BFC9C))
  (global $BSS_LAST_FLOOR_PACKET_CLASS_COUNT i32 (i32.const 0x020BFCA0))
  (global $BSS_LAST_FLOOR_PACKET_VALUE_COUNT i32 (i32.const 0x020BFCA4))
  (global $BSS_LAST_FLOOR_PACKET_SCALAR_COUNT i32 (i32.const 0x020BFCA8))
  (global $BSS_LAST_FLOOR_PACKET_POINT_COUNT i32 (i32.const 0x020BFCAC))
  (global $BSS_LAST_FLOOR_PACKET_SORTED_COUNT i32 (i32.const 0x020BFCB0))
  (global $BSS_LAST_FLOOR_PACKET_SEGMENT_COUNT i32 (i32.const 0x020BFCB4))
  (global $BSS_LAST_RESIDUE_PACKET_STATUS i32 (i32.const 0x020BFCB8))
  (global $BSS_LAST_RESIDUE_PACKET_COUNT i32 (i32.const 0x020BFCBC))
  (global $BSS_LAST_RESIDUE_PACKET_PARTITION_COUNT i32 (i32.const 0x020BFCC0))
  (global $BSS_LAST_RESIDUE_PACKET_CLASS_COUNT i32 (i32.const 0x020BFCC4))
  (global $BSS_LAST_RESIDUE_PACKET_BOOK_COUNT i32 (i32.const 0x020BFCC8))
  (global $BSS_LAST_RESIDUE_PACKET_SCALAR_COUNT i32 (i32.const 0x020BFCCC))
  (global $BSS_LAST_RESIDUE_PACKET_VALUE_COUNT i32 (i32.const 0x020BFCD0))
  (global $BSS_LAST_RESIDUE_WORK_VECTOR_STATUS i32 (i32.const 0x020BFCD4))
  (global $BSS_LAST_RESIDUE_WORK_VECTOR_COUNT i32 (i32.const 0x020BFCD8))
  (global $BSS_LAST_RESIDUE_WORK_VECTOR_NONZERO i32 (i32.const 0x020BFCDC))
  (global $BSS_LAST_FLOOR_APPLY_STATUS i32 (i32.const 0x020BFCE0))
  (global $BSS_LAST_FLOOR_APPLY_COUNT i32 (i32.const 0x020BFCE4))
  (global $BSS_LAST_FLOOR_APPLY_NONZERO i32 (i32.const 0x020BFCE8))
  (global $BSS_LAST_FLOOR_RESIDUE_PCM_STATUS i32 (i32.const 0x020BFCEC))
  (global $BSS_LAST_FLOOR_RESIDUE_PCM_FRAMES i32 (i32.const 0x020BFCF0))
  (global $BSS_LAST_PCM_STATUS i32 (i32.const 0x020BFCF4))
  (global $BSS_LAST_SILENT_PACKET_COUNT i32 (i32.const 0x020BFCF8))
  (global $BSS_LAST_BAD_CODEBOOK_INDEX i32 (i32.const 0x020BFCFC))
  (global $BSS_LAST_BAD_CODEBOOK_ENTRIES i32 (i32.const 0x020BFD00))
  (global $BSS_LAST_BAD_CODEBOOK_MAX_LENGTH i32 (i32.const 0x020BFD04))
  (global $BSS_LAST_MDCT_SYNTH_STATUS i32 (i32.const 0x020BFD08))
  (global $BSS_LAST_MDCT_SYNTH_PACKETS i32 (i32.const 0x020BFD0C))
  (global $BSS_LAST_MDCT_SYNTH_FRAMES i32 (i32.const 0x020BFD10))
  (global $BSS_LAST_MDCT_SYNTH_NONZERO i32 (i32.const 0x020BFD14))
  (global $BSS_LAST_MDCT_SYNTH_OVERLAP_NONZERO i32 (i32.const 0x020BFD18))
  (global $BSS_LAST_IMDCT_KERNEL_BINS i32 (i32.const 0x020BFD1C))
  (global $BSS_LAST_IMDCT_KERNEL_SAMPLES i32 (i32.const 0x020BFD20))
  (global $BSS_LAST_IMDCT_KERNEL_CROSS_TERMS i32 (i32.const 0x020BFD24))
  (global $BSS_MDCT_INPUT_Q15 i32 (i32.const 0x020BFD40))
  (global $BSS_OVERLAP_Q15 i32 (i32.const 0x020C7D40))
  (global $SS_READY i32 (i32.const 0))
  (global $SS_BLOCK_SIZE_SHORT i32 (i32.const 4))
  (global $SS_BLOCK_SIZE_LONG i32 (i32.const 8))
  (global $SS_CODEBOOK_COUNT i32 (i32.const 12))
  (global $SS_CODEBOOK_STATUS i32 (i32.const 16))
  (global $SS_CODEBOOK_PARSED i32 (i32.const 20))
  (global $SS_CODEBOOK_ENTRIES i32 (i32.const 24))
  (global $SS_CODEBOOK_NONZERO i32 (i32.const 28))
  (global $SS_CODEBOOK_MAX_LEN i32 (i32.const 32))
  (global $SS_LOOKUP_VALUES i32 (i32.const 36))
  (global $SS_SETUP_BIT_BYTE i32 (i32.const 40))
  (global $SS_SETUP_BIT_OFFSET i32 (i32.const 44))
  (global $SS_TIME_COUNT i32 (i32.const 48))
  (global $SS_FLOOR_COUNT i32 (i32.const 52))
  (global $SS_RESIDUE_COUNT i32 (i32.const 56))
  (global $SS_MAPPING_COUNT i32 (i32.const 60))
  (global $SS_MODE_COUNT i32 (i32.const 64))
  (global $CB_DIMENSIONS i32 (i32.const 0))
  (global $CB_ENTRIES i32 (i32.const 4))
  (global $CB_LENGTH_OFFSET i32 (i32.const 8))
  (global $CB_CODE_OFFSET i32 (i32.const 12))
  (global $CB_NONZERO_COUNT i32 (i32.const 16))
  (global $CB_MAX_LENGTH i32 (i32.const 20))
  (global $CB_ORDERED i32 (i32.const 24))
  (global $CB_SPARSE i32 (i32.const 28))
  (global $CB_LOOKUP_TYPE i32 (i32.const 32))
  (global $CB_LOOKUP_MIN i32 (i32.const 36))
  (global $CB_LOOKUP_DELTA i32 (i32.const 40))
  (global $CB_LOOKUP_VALUE_BITS i32 (i32.const 44))
  (global $CB_LOOKUP_SEQUENCE i32 (i32.const 48))
  (global $CB_MULTIPLICAND_COUNT i32 (i32.const 52))
  (global $CB_MULTIPLICAND_OFFSET i32 (i32.const 56))
  (global $FH_PARTITION_COUNT i32 (i32.const 0))
  (global $FH_CLASS_COUNT i32 (i32.const 4))
  (global $FH_MULTIPLIER i32 (i32.const 8))
  (global $FH_RANGE_BITS i32 (i32.const 12))
  (global $FH_VALUE_COUNT i32 (i32.const 16))
  (global $FH_PART_OFFSET i32 (i32.const 20))
  (global $FH_CLASS_OFFSET i32 (i32.const 24))
  (global $FH_VALUE_OFFSET i32 (i32.const 28))
  (global $RH_RESIDUE_TYPE i32 (i32.const 0))
  (global $RH_BEGIN i32 (i32.const 4))
  (global $RH_END i32 (i32.const 8))
  (global $RH_PARTITION_SIZE i32 (i32.const 12))
  (global $RH_CLASSIFICATIONS i32 (i32.const 16))
  (global $RH_CLASSBOOK i32 (i32.const 20))
  (global $RH_BOOK_OFFSET i32 (i32.const 24))
  (global $MH_SUBMAPS i32 (i32.const 0))
  (global $MH_COUPLING_STEPS i32 (i32.const 4))
  (global $MH_MUX i32 (i32.const 8))
  (global $MH_FLOOR_OFFSET i32 (i32.const 12))
  (global $MH_RESIDUE_OFFSET i32 (i32.const 16))
  (global $MD_BLOCK_FLAG i32 (i32.const 0))
  (global $MD_MAPPING i32 (i32.const 4))
  (global $BR_DATA i32 (i32.const 0))
  (global $BR_LEN i32 (i32.const 8))
  (global $BR_BYTE_POS i32 (i32.const 16))
  (global $BR_BIT_POS i32 (i32.const 24))
  (global $BR_ERROR i32 (i32.const 28))
  (global $SS_SAMPLE_RATE i32 (i32.const 0))
  (global $SS_FRAME_COUNT i32 (i32.const 4))
  (global $SS_LOOP_START i32 (i32.const 8))
  (global $SS_LOOP_END i32 (i32.const 12))
  (global $SS_LOOP_FLAG i32 (i32.const 16))
  (global $SS_PACKET_COUNT i32 (i32.const 20))
  (global $SS_DECODE_STATUS i32 (i32.const 24))
  (global $SS_PCM_PTR i32 (i32.const 32))
  (global $SS_PACKET_PTRS i32 (i32.const 40))
  (global $SS_PACKET_LENS i32 (i32.const 8232))
  (global $RS_SAMPLE_RATE i32 (i32.const 0))
  (global $RS_FRAME_COUNT i32 (i32.const 4))
  (global $RS_LOOP_START i32 (i32.const 8))
  (global $RS_LOOP_END i32 (i32.const 12))
  (global $RS_LOOP_FLAG i32 (i32.const 16))
  (global $RS_PCM_PTR i32 (i32.const 24))
  (global (export "vorbis_codebook_headers") i32 (i32.const 0x001C3400))
  (global (export "vorbis_codebook_lengths") i32 (i32.const 0x001C4400))
  (global (export "vorbis_codebook_codes") i32 (i32.const 0x001CC400))
  (global (export "vorbis_codebook_multiplicands") i32 (i32.const 0x001D4400))
  (global (export "vorbis_floor_headers") i32 (i32.const 0x001D5000))
  (global (export "vorbis_floor_partitions") i32 (i32.const 0x001D5800))
  (global (export "vorbis_floor_class_dims") i32 (i32.const 0x001D5A00))
  (global (export "vorbis_floor_class_subbits") i32 (i32.const 0x001D5C00))
  (global (export "vorbis_floor_class_master") i32 (i32.const 0x001D5E00))
  (global (export "vorbis_floor_class_books") i32 (i32.const 0x001D6000))
  (global (export "vorbis_floor_values") i32 (i32.const 0x001D6800))
  (global (export "vorbis_residue_headers") i32 (i32.const 0x001D7000))
  (global (export "vorbis_residue_cascades") i32 (i32.const 0x001D7800))
  (global (export "vorbis_residue_books") i32 (i32.const 0x001D7A00))
  (global (export "vorbis_mapping_headers") i32 (i32.const 0x001D8200))
  (global (export "vorbis_mapping_floors") i32 (i32.const 0x001D8800))
  (global (export "vorbis_mapping_residues") i32 (i32.const 0x001D9800))
  (global (export "vorbis_mode_headers") i32 (i32.const 0x001DA800))
  (global (export "vorbis_packet_modes") i32 (i32.const 0x001DAC00))
  (global (export "vorbis_packet_mappings") i32 (i32.const 0x001DBC00))
  (global (export "vorbis_packet_floors") i32 (i32.const 0x001DCC00))
  (global (export "vorbis_packet_residues") i32 (i32.const 0x001DDC00))
  (global (export "vorbis_packet_floor_nonzero") i32 (i32.const 0x001DEC00))
  (global (export "vorbis_packet_floor_y0") i32 (i32.const 0x001DFC00))
  (global (export "vorbis_packet_floor_y1") i32 (i32.const 0x001E0C00))
  (global (export "vorbis_packet_floor_class_count") i32 (i32.const 0x001E1C00))
  (global (export "vorbis_packet_floor_class_ids") i32 (i32.const 0x001E2C00))
  (global (export "vorbis_packet_floor_class_selectors") i32 (i32.const 0x00222C00))
  (global (export "vorbis_packet_floor_class_dims") i32 (i32.const 0x00262C00))
  (global (export "vorbis_packet_floor_value_books") i32 (i32.const 0x002A2C00))
  (global (export "vorbis_packet_floor_values") i32 (i32.const 0x004A2C00))
  (global (export "vorbis_packet_floor_point_count") i32 (i32.const 0x006A2C00))
  (global (export "vorbis_packet_floor_point_x") i32 (i32.const 0x006A3C00))
  (global (export "vorbis_packet_floor_point_y") i32 (i32.const 0x008A3C00))
  (global (export "vorbis_packet_floor_point_order") i32 (i32.const 0x00AA3C00))
  (global (export "vorbis_packet_floor_segment_count") i32 (i32.const 0x00CA3C00))
  (global (export "vorbis_packet_floor_segment_x0") i32 (i32.const 0x00CA4C00))
  (global (export "vorbis_packet_floor_segment_y0") i32 (i32.const 0x00EA4C00))
  (global (export "vorbis_packet_floor_segment_x1") i32 (i32.const 0x010A4C00))
  (global (export "vorbis_packet_floor_segment_y1") i32 (i32.const 0x012A4C00))
  (global (export "vorbis_packet_residue_partition_count") i32 (i32.const 0x014A4C00))
  (global (export "vorbis_packet_residue_classifications") i32 (i32.const 0x014A5C00))
  (global (export "vorbis_packet_residue_value_count") i32 (i32.const 0x016A5C00))
  (global (export "vorbis_packet_residue_value_books") i32 (i32.const 0x016A6C00))
  (global (export "vorbis_packet_residue_value_entries") i32 (i32.const 0x018A6C00))
  (global (export "vorbis_packet_residue_value_dims") i32 (i32.const 0x01AA6C00))
  (global (export "vorbis_packet_residue_value_targets") i32 (i32.const 0x01CA6C00))
  (global (export "vorbis_packet_residue_values") i32 (i32.const 0x01EA6C00))
  (global (export "vorbis_packet_block_sizes") i32 (i32.const 0x020A6C00))
  (global (export "vorbis_packet_prev_window_flags") i32 (i32.const 0x020A7C00))
  (global (export "vorbis_packet_next_window_flags") i32 (i32.const 0x020A8C00))
  (global (export "vorbis_packet_window_left_start") i32 (i32.const 0x020A9C00))
  (global (export "vorbis_packet_window_left_end") i32 (i32.const 0x020AAC00))
  (global (export "vorbis_packet_window_right_start") i32 (i32.const 0x020ABC00))
  (global (export "vorbis_packet_window_right_end") i32 (i32.const 0x020ACC00))
  (global (export "vorbis_packet_window_left_frames") i32 (i32.const 0x020ADC00))
  (global (export "vorbis_packet_window_right_frames") i32 (i32.const 0x020AEC00))
  (global (export "vorbis_mdct_input_q15") i32 (i32.const 0x020BFD40))
  (global (export "vorbis_mdct_output_q15") i32 (i32.const 0x020AFC00))
  (global (export "vorbis_window_q15") i32 (i32.const 0x020B7C00))
  (global (export "vorbis_overlap_q15") i32 (i32.const 0x020C7D40))

  (func $read_u32be (param $data i32) (param $off i32) (result i32)
    local.get $data local.get $off i32.add i32.load8_u i32.const 24 i32.shl
    local.get $data local.get $off i32.const 1 i32.add i32.add i32.load8_u i32.const 16 i32.shl
    i32.or
    local.get $data local.get $off i32.const 2 i32.add i32.add i32.load8_u i32.const 8 i32.shl
    i32.or
    local.get $data local.get $off i32.const 3 i32.add i32.add i32.load8_u
    i32.or
  )

  (func $ilog_u32 (param $val i32) (result i32)
    (local $n i32)
    i32.const 0 local.set $n
    (block $done
      (loop $loop
        local.get $val i32.eqz br_if $done
        local.get $val i32.const 1 i32.shr_u local.set $val
        local.get $n i32.const 1 i32.add local.set $n
        br $loop
      )
    )
    local.get $n
  )

  (func $pow_u32_capped (param $val i32) (param $pow i32) (result i32)
    (local $r i32)
    i32.const 1 local.set $r
    (block $done
      (loop $loop
        local.get $pow i32.eqz br_if $done
        local.get $r local.get $val i32.mul local.tee $r
        i32.const 0 i32.lt_s
        if
          i32.const 0x7fffffff return
        end
        local.get $pow i32.const 1 i32.sub local.set $pow
        br $loop
      )
    )
    local.get $r
  )

  (func $reverse_bits (param $val i32) (param $bits i32) (result i32)
    (local $r i32) (local $i i32)
    i32.const 0 local.set $r
    i32.const 0 local.set $i
    (block $done
      (loop $loop
        local.get $i local.get $bits i32.ge_u br_if $done
        local.get $r i32.const 1 i32.shl local.set $r
        local.get $r local.get $val i32.const 1 i32.and i32.or local.set $r
        local.get $val i32.const 1 i32.shr_u local.set $val
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      )
    )
    local.get $r
  )

  (func $unpack_float_q15 (param $val i32) (result i32)
    (local $mant i32) (local $exp i32)
    local.get $val i32.const 0x1fffff i32.and local.set $mant
    local.get $val i32.const 0x80000000 i32.and
    if
      i32.const 0 local.get $mant i32.sub local.set $mant
    end
    local.get $val i32.const 21 i32.shr_u i32.const 0x3ff i32.and i32.const 788 i32.sub local.set $exp
    local.get $exp i32.const 0 i32.ge_s
    if
      local.get $exp i32.const 15 i32.gt_s
      if
        local.get $mant i32.const 0 i32.lt_s
        if
          i32.const -32768 return
        else
          i32.const 32767 return
        end
      end
      local.get $mant local.get $exp i32.shl local.set $mant
    else
      i32.const 0 local.get $exp i32.sub local.set $exp
      local.get $exp i32.const 31 i32.ge_s
      if
        i32.const 0 return
      end
      local.get $mant local.get $exp i32.shr_s local.set $mant
    end
    local.get $mant i32.const 32767 i32.gt_s
    if
      i32.const 32767 local.set $mant
    end
    local.get $mant i32.const -32768 i32.lt_s
    if
      i32.const -32768 local.set $mant
    end
    local.get $mant
  )

  (func $floor1_predict_y (param $x0 i32) (param $y0 i32) (param $x1 i32) (param $y1 i32) (param $x i32) (result i32)
    (local $dx i32)
    local.get $x1 local.get $x0 i32.sub local.tee $dx
    i32.eqz
    if
      local.get $y0 return
    end
    local.get $x local.get $x0 i32.sub
    local.get $y1 local.get $y0 i32.sub i32.mul
    local.get $dx i32.div_s
    local.get $y0 i32.add
  )

  (func $cos_q15_from_phase (param $phase i32) (result i32)
    (local $idx i32) (local $frac i32) (local $sign i32) (local $v0 i32) (local $v1 i32)
    local.get $phase i32.const 0xffff i32.and local.set $phase
    i32.const 0 local.set $sign
    local.get $phase i32.const 32768 i32.ge_u
    if
      local.get $phase i32.const 32768 i32.sub local.set $phase
      i32.const 1 local.set $sign
    end
    local.get $phase i32.const 16384 i32.gt_u
    if
      i32.const 32768 local.get $phase i32.sub local.set $phase
    end
    local.get $phase i32.const 6 i32.shr_u local.set $idx
    local.get $phase i32.const 63 i32.and local.set $frac
    local.get $idx i32.const 256 i32.lt_u
    if
      i32.const 34389312 local.get $idx i32.const 1 i32.shl i32.add i32.load16_s local.set $v0
      i32.const 34389312 local.get $idx i32.const 1 i32.shl i32.add i32.const 2 i32.add i32.load16_s local.set $v1
      local.get $v1 local.get $v0 i32.sub
      local.get $frac i32.mul
      i32.const 6 i32.shr_s
      local.get $v0 i32.add
      local.set $phase
    else
      i32.const 34389312 i32.const 512 i32.add i32.load16_s local.set $phase
    end
    local.get $sign
    if
      i32.const 0 local.get $phase i32.sub local.set $phase
    end
    local.get $phase
  )

  (func $read_i32be_sample (param $data i32) (param $len i32) (param $cursor_ptr i32) (result i32)
    (local $cursor i32) (local $val i32)
    local.get $cursor_ptr i32.load offset=0 local.set $cursor
    local.get $cursor i32.const 4 i32.add local.get $len i32.gt_u
    if
      i32.const -1 return
    end
    local.get $data local.get $cursor i32.add i32.load8_u i32.const 24 i32.shl
    local.get $data local.get $cursor i32.const 1 i32.add i32.add i32.load8_u i32.const 16 i32.shl
    i32.or
    local.get $data local.get $cursor i32.const 2 i32.add i32.add i32.load8_u i32.const 8 i32.shl
    i32.or
    local.get $data local.get $cursor i32.const 3 i32.add i32.add i32.load8_u
    i32.or
    local.set $val
    local.get $cursor_ptr local.get $cursor i32.const 4 i32.add i32.store offset=0
    local.get $val
  )

  (func $vorbis_bitreader_init (export "vorbis_bitreader_init")
    (param $br i32) (param $data i32) (param $len i32)
    local.get $br global.get $BR_DATA i32.add local.get $data i64.extend_i32_u i64.store offset=0
    local.get $br global.get $BR_LEN i32.add local.get $len i64.extend_i32_u i64.store offset=0
    local.get $br global.get $BR_BYTE_POS i32.add i64.const 0 i64.store offset=0
    local.get $br global.get $BR_BIT_POS i32.add i32.const 0 i32.store offset=0
    local.get $br global.get $BR_ERROR i32.add i32.const 0 i32.store offset=0
  )

  (func $read_bits_internal (param $br i32) (param $n i32) (result i32)
    (local $val i32) (local $byte_pos i32) (local $bit_pos i32) (local $data i32) (local $len i32)
    local.get $br global.get $BR_BYTE_POS i32.add i64.load offset=0 i32.wrap_i64 local.set $byte_pos
    local.get $br global.get $BR_BIT_POS i32.add i32.load offset=0 local.set $bit_pos
    local.get $br global.get $BR_DATA i32.add i64.load offset=0 i32.wrap_i64 local.set $data
    local.get $br global.get $BR_LEN i32.add i64.load offset=0 i32.wrap_i64 local.set $len
    i32.const 0 local.set $val
    (block $done
      (loop $read
        local.get $n i32.eqz br_if $done
        local.get $byte_pos local.get $len i32.ge_u
        if
          local.get $br global.get $BR_ERROR i32.add i32.const -1 i32.store offset=0
          i32.const 0 return
        end
        local.get $data local.get $byte_pos i32.add i32.load8_u
        local.get $bit_pos i32.shr_u
        i32.const 1 i32.and
        local.get $n i32.const 1 i32.sub local.tee $n
        i32.shl
        local.get $val i32.or
        local.set $val
        local.get $bit_pos i32.const 1 i32.add local.tee $bit_pos
        i32.const 8 i32.ne
        br_if $read
        i32.const 0 local.set $bit_pos
        local.get $byte_pos i32.const 1 i32.add local.set $byte_pos
        br $read
      )
    )
    local.get $br global.get $BR_BYTE_POS i32.add local.get $byte_pos i64.extend_i32_u i64.store offset=0
    local.get $br global.get $BR_BIT_POS i32.add local.get $bit_pos i32.store offset=0
    local.get $val
  )

  (func $read_bit_internal (param $br i32) (result i32)
    local.get $br i32.const 1 call $read_bits_internal
  )

  (func (export "vorbis_bitreader_read_bits")
    (param $br i32) (param $n i32) (result i32)
    local.get $br local.get $n call $read_bits_internal
  )

  (func (export "vorbis_bitreader_read_bit")
    (param $br i32) (result i32)
    local.get $br i32.const 1 call $read_bits_internal
  )

  (func $vorbis_setup_init (export "vorbis_setup_init")
    (param $setup i32)
    (local $i i32)
    i32.const 0 local.set $i
    (block $done
      (loop $loop
        local.get $i i32.const 17 i32.ge_u br_if $done
        local.get $setup local.get $i i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      )
    )
  )

  (func $vorbis_codebook_get_header (export "vorbis_codebook_get_header")
    (param $index i32) (result i32)
    (local $count i32)
    global.get $BSS_LAST_CODEBOOK_COUNT i32.load offset=0 local.set $count
    local.get $index local.get $count i32.ge_u
    if
      i32.const 0 return
    end
    global.get $BSS_CODEBOOK_HEADERS local.get $index global.get $SIZEOF_CODEBOOK_HEADER i32.mul i32.add
  )

  (func (export "vorbis_codebook_get_lengths")
    (param $index i32) (result i32)
    (local $hdr i32)
    local.get $index call $vorbis_codebook_get_header
    local.tee $hdr i32.eqz
    if
      i32.const 0 return
    end
    global.get $BSS_CODEBOOK_LENGTHS
    local.get $hdr global.get $CB_LENGTH_OFFSET i32.add i32.load offset=0
    i32.const 2 i32.shl i32.add
  )

  (func (export "vorbis_codebook_get_codes")
    (param $index i32) (result i32)
    (local $hdr i32)
    local.get $index call $vorbis_codebook_get_header
    local.tee $hdr i32.eqz
    if
      i32.const 0 return
    end
    global.get $BSS_CODEBOOK_CODES
    local.get $hdr global.get $CB_CODE_OFFSET i32.add i32.load offset=0
    i32.const 2 i32.shl i32.add
  )

  (func (export "vorbis_codebook_get_multiplicands")
    (param $index i32) (result i32)
    (local $hdr i32)
    local.get $index call $vorbis_codebook_get_header
    local.tee $hdr i32.eqz
    if
      i32.const 0 return
    end
    global.get $BSS_CODEBOOK_MULTIPLICANDS
    local.get $hdr global.get $CB_MULTIPLICAND_OFFSET i32.add i32.load offset=0
    i32.const 2 i32.shl i32.add
  )

  (func (export "vorbis_setup_get_codebook_tables")
    (result i32) (result i32) (result i32) (result i32) (result i32)
    global.get $BSS_CODEBOOK_HEADERS
    global.get $BSS_CODEBOOK_LENGTHS
    global.get $BSS_CODEBOOK_CODES
    global.get $BSS_CODEBOOK_MULTIPLICANDS
    global.get $BSS_LAST_CODEBOOK_COUNT i32.load offset=0
  )

  (func $vorbis_codebook_lookup1_values (export "vorbis_codebook_lookup1_values")
    (param $entries i32) (param $dimensions i32) (result i32)
    (local $val i32)
    i32.const 1 local.set $val
    (block $done
      (loop $loop
        local.get $val local.get $dimensions i32.gt_u br_if $done
        local.get $val local.get $entries i32.mul local.set $val
        br $loop
      )
    )
    local.get $val
  )

  (func $vorbis_codebook_build_canonical (export "vorbis_codebook_build_canonical")
    (param $book_index i32) (param $setup i32) (result i32)
    (local $hdr i32) (local $entry_count i32) (local $len_offset i32) (local $code_offset i32)
    (local $i i32) (local $length i32) (local $code i32) (local $ishift i32) (local $i4 i32)
    (local $carry_val i32) (local $j i32) (local $temp_val i32)
    local.get $book_index call $vorbis_codebook_get_header
    local.tee $hdr i32.eqz
    if
      global.get $VORBIS_ERR_BAD_INDEX return
    end
    i32.const 0 local.set $i
    (block $zero_done
      (loop $zero_loop
        local.get $i i32.const 33 i32.ge_u br_if $zero_done
        global.get $BSS_CANONICAL_TEMP local.get $i i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $zero_loop
      )
    )
    local.get $hdr global.get $CB_ENTRIES i32.add i32.load offset=0 local.set $entry_count
    local.get $hdr global.get $CB_LENGTH_OFFSET i32.add i32.load offset=0 local.set $len_offset
    local.get $hdr global.get $CB_CODE_OFFSET i32.add i32.load offset=0 local.set $code_offset
    i32.const 0 local.set $i
    (block $done
      (loop $loop
        local.get $i local.get $entry_count i32.ge_u br_if $done
        global.get $BSS_CODEBOOK_LENGTHS local.get $len_offset local.get $i i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $length
        local.get $length i32.eqz
        if
          local.get $i i32.const 1 i32.add local.set $i
          br $loop
        end
        i32.const 32 local.get $length i32.sub local.set $ishift
        i32.const 1 local.get $ishift i32.shl local.set $i4
        global.get $BSS_CANONICAL_TEMP local.get $length i32.const 2 i32.shl i32.add i32.load offset=0 local.set $code
        global.get $BSS_CODEBOOK_CODES local.get $code_offset local.get $i i32.add i32.const 2 i32.shl i32.add local.get $code i32.store offset=0
        local.get $code local.get $i4 i32.and
        if
          global.get $BSS_CANONICAL_TEMP local.get $length i32.const 2 i32.shl i32.add i32.const -4 i32.add i32.load offset=0 local.set $carry_val
        else
          local.get $code local.get $i4 i32.or local.set $carry_val
          local.get $length i32.const 1 i32.sub local.set $j
          (block $carry_done
            (loop $carry_loop
              local.get $j i32.const 1 i32.lt_s br_if $carry_done
              global.get $BSS_CANONICAL_TEMP local.get $j i32.const 2 i32.shl i32.add i32.load offset=0 local.set $temp_val
              local.get $temp_val local.get $code i32.ne br_if $carry_done
              i32.const 32 local.get $j i32.sub local.set $ishift
              i32.const 1 local.get $ishift i32.shl local.set $i4
              local.get $temp_val local.get $i4 i32.and
              if
                global.get $BSS_CANONICAL_TEMP local.get $j i32.const 2 i32.shl i32.add i32.const -4 i32.add i32.load offset=0
                local.set $temp_val
                global.get $BSS_CANONICAL_TEMP local.get $j i32.const 2 i32.shl i32.add local.get $temp_val i32.store offset=0
                br $carry_done
              else
                global.get $BSS_CANONICAL_TEMP local.get $j i32.const 2 i32.shl i32.add
                local.get $temp_val local.get $i4 i32.or i32.store offset=0
              end
              local.get $j i32.const 1 i32.sub local.set $j
              br $carry_loop
            )
          )
        end
        global.get $BSS_CANONICAL_TEMP local.get $length i32.const 2 i32.shl i32.add local.get $carry_val i32.store offset=0
        local.get $length i32.const 1 i32.add local.set $j
        (block $prop_done
          (loop $prop_loop
            local.get $j i32.const 32 i32.gt_u br_if $prop_done
            global.get $BSS_CANONICAL_TEMP local.get $j i32.const 2 i32.shl i32.add i32.load offset=0
            local.get $code i32.ne br_if $prop_done
            global.get $BSS_CANONICAL_TEMP local.get $j i32.const 2 i32.shl i32.add local.get $carry_val i32.store offset=0
            local.get $j i32.const 1 i32.add local.set $j
            br $prop_loop
          )
        )
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      )
    )
    i32.const 0
  )

  (func $vorbis_codebook_decode_scalar (export "vorbis_codebook_decode_scalar")
    (param $br i32) (param $book_index i32) (param $setup i32) (result i32)
    (local $hdr i32) (local $entries i32) (local $max_len i32) (local $len_offset i32) (local $code_offset i32)
    (local $val i32) (local $len i32) (local $j i32) (local $candidate i32)
    global.get $BSS_LAST_BAD_CODEBOOK_INDEX local.get $book_index i32.store offset=0
    local.get $book_index call $vorbis_codebook_get_header
    local.tee $hdr i32.eqz
    if
      i32.const 0 return
    end
    local.get $hdr global.get $CB_ENTRIES i32.add i32.load offset=0 local.set $entries
    global.get $BSS_LAST_BAD_CODEBOOK_ENTRIES local.get $entries i32.store offset=0
    local.get $hdr global.get $CB_MAX_LENGTH i32.add i32.load offset=0 local.set $max_len
    global.get $BSS_LAST_BAD_CODEBOOK_MAX_LENGTH local.get $max_len i32.store offset=0
    local.get $max_len i32.eqz if i32.const 0 return end
    local.get $max_len i32.const 32 i32.gt_u if i32.const 0 return end
    local.get $hdr global.get $CB_LENGTH_OFFSET i32.add i32.load offset=0 local.set $len_offset
    local.get $hdr global.get $CB_CODE_OFFSET i32.add i32.load offset=0 local.set $code_offset
    i32.const 0 local.set $val
    i32.const 0 local.set $len
    (block $done
      (loop $read_loop
        local.get $len local.get $max_len i32.ge_u br_if $done
        local.get $br call $read_bit_internal
        local.get $len i32.shl
        local.get $val i32.or
        local.set $val
        local.get $len i32.const 1 i32.add local.set $len
        i32.const 0 local.set $j
        (block $entry_done
          (loop $entry_loop
            local.get $j local.get $entries i32.ge_u br_if $entry_done
            global.get $BSS_CODEBOOK_LENGTHS
            local.get $len_offset local.get $j i32.add i32.const 2 i32.shl i32.add
            i32.load offset=0
            local.get $len i32.ne
            if
              local.get $j i32.const 1 i32.add local.set $j
              br $entry_loop
            end
            global.get $BSS_CODEBOOK_CODES
            local.get $code_offset local.get $j i32.add i32.const 2 i32.shl i32.add
            i32.load offset=0
            local.set $candidate
            local.get $candidate
            i32.const 32 local.get $len i32.sub i32.shr_u
            local.get $len
            call $reverse_bits
            local.get $val i32.ne
            if
              local.get $j i32.const 1 i32.add local.set $j
              br $entry_loop
            end
            local.get $j return
          )
        )
        br $read_loop
      )
    )
    i32.const 0
  )

  ;; TODO: remaining functions

  (func (export "vorbis_last_codebook_status") (result i32)
    global.get $BSS_LAST_CODEBOOK_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_codebook_count") (result i32)
    global.get $BSS_LAST_CODEBOOK_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_codebook_entries") (result i32)
    global.get $BSS_LAST_CODEBOOK_ENTRIES i32.load offset=0
  )

  (func (export "vorbis_last_codebook_nonzero") (result i32)
    global.get $BSS_LAST_CODEBOOK_NONZERO i32.load offset=0
  )

  (func (export "vorbis_last_codebook_max_len") (result i32)
    global.get $BSS_LAST_CODEBOOK_MAX_LEN i32.load offset=0
  )

  (func (export "vorbis_last_lookup_values") (result i32)
    global.get $BSS_LAST_LOOKUP_VALUES i32.load offset=0
  )

  (func (export "vorbis_last_setup_byte") (result i32)
    global.get $BSS_LAST_SETUP_BYTE i32.load offset=0
  )

  (func (export "vorbis_last_setup_bit") (result i32)
    global.get $BSS_LAST_SETUP_BIT i32.load offset=0
  )

  (func (export "vorbis_last_floor_status") (result i32)
    global.get $BSS_LAST_FLOOR_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_residue_status") (result i32)
    global.get $BSS_LAST_RESIDUE_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_mapping_status") (result i32)
    global.get $BSS_LAST_MAPPING_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_mode_status") (result i32)
    global.get $BSS_LAST_MODE_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_floor_values") (result i32)
    global.get $BSS_LAST_FLOOR_VALUES i32.load offset=0
  )

  (func (export "vorbis_last_residue_books") (result i32)
    global.get $BSS_LAST_RESIDUE_BOOKS i32.load offset=0
  )

  (func (export "vorbis_last_packet_status") (result i32)
    global.get $BSS_LAST_PACKET_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_packet_count") (result i32)
    global.get $BSS_LAST_PACKET_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_packet_mode") (result i32)
    global.get $BSS_LAST_PACKET_MODE i32.load offset=0
  )

  (func (export "vorbis_last_packet_mapping") (result i32)
    global.get $BSS_LAST_PACKET_MAPPING i32.load offset=0
  )

  (func (export "vorbis_last_packet_floor") (result i32)
    global.get $BSS_LAST_PACKET_FLOOR i32.load offset=0
  )

  (func (export "vorbis_last_packet_residue") (result i32)
    global.get $BSS_LAST_PACKET_RESIDUE i32.load offset=0
  )

  (func (export "vorbis_last_mode_bits") (result i32)
    global.get $BSS_LAST_MODE_BITS i32.load offset=0
  )

  (func (export "vorbis_last_window_status") (result i32)
    global.get $BSS_LAST_WINDOW_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_window_packet") (result i32)
    global.get $BSS_LAST_WINDOW_PACKET i32.load offset=0
  )

  (func (export "vorbis_last_window_block_size") (result i32)
    global.get $BSS_LAST_WINDOW_BLOCK_SIZE i32.load offset=0
  )

  (func (export "vorbis_last_window_prev_flag") (result i32)
    global.get $BSS_LAST_WINDOW_PREV_FLAG i32.load offset=0
  )

  (func (export "vorbis_last_window_next_flag") (result i32)
    global.get $BSS_LAST_WINDOW_NEXT_FLAG i32.load offset=0
  )

  (func (export "vorbis_last_window_left_start") (result i32)
    global.get $BSS_LAST_WINDOW_LEFT_START i32.load offset=0
  )

  (func (export "vorbis_last_window_left_end") (result i32)
    global.get $BSS_LAST_WINDOW_LEFT_END i32.load offset=0
  )

  (func (export "vorbis_last_window_right_start") (result i32)
    global.get $BSS_LAST_WINDOW_RIGHT_START i32.load offset=0
  )

  (func (export "vorbis_last_window_right_end") (result i32)
    global.get $BSS_LAST_WINDOW_RIGHT_END i32.load offset=0
  )

  (func (export "vorbis_last_window_left_frames") (result i32)
    global.get $BSS_LAST_WINDOW_LEFT_FRAMES i32.load offset=0
  )

  (func (export "vorbis_last_window_right_frames") (result i32)
    global.get $BSS_LAST_WINDOW_RIGHT_FRAMES i32.load offset=0
  )

  (func (export "vorbis_last_mdct_buffer_frames") (result i32)
    global.get $BSS_LAST_MDCT_BUFFER_FRAMES i32.load offset=0
  )

  (func (export "vorbis_last_overlap_frames") (result i32)
    global.get $BSS_LAST_OVERLAP_FRAMES i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_status") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_count") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_nonzero") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_NONZERO i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_y0") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_Y0 i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_y1") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_Y1 i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_y_range") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_Y_RANGE i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_class_count") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_CLASS_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_value_count") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_VALUE_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_scalar_count") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_SCALAR_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_point_count") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_POINT_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_sorted_count") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_SORTED_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_segment_count") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_SEGMENT_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_residue_packet_status") (result i32)
    global.get $BSS_LAST_RESIDUE_PACKET_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_residue_packet_count") (result i32)
    global.get $BSS_LAST_RESIDUE_PACKET_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_residue_packet_partition_count") (result i32)
    global.get $BSS_LAST_RESIDUE_PACKET_PARTITION_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_residue_packet_class_count") (result i32)
    global.get $BSS_LAST_RESIDUE_PACKET_CLASS_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_residue_packet_book_count") (result i32)
    global.get $BSS_LAST_RESIDUE_PACKET_BOOK_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_residue_packet_scalar_count") (result i32)
    global.get $BSS_LAST_RESIDUE_PACKET_SCALAR_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_residue_packet_value_count") (result i32)
    global.get $BSS_LAST_RESIDUE_PACKET_VALUE_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_residue_work_vector_status") (result i32)
    global.get $BSS_LAST_RESIDUE_WORK_VECTOR_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_residue_work_vector_count") (result i32)
    global.get $BSS_LAST_RESIDUE_WORK_VECTOR_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_residue_work_vector_nonzero") (result i32)
    global.get $BSS_LAST_RESIDUE_WORK_VECTOR_NONZERO i32.load offset=0
  )

  (func (export "vorbis_last_floor_apply_status") (result i32)
    global.get $BSS_LAST_FLOOR_APPLY_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_floor_apply_count") (result i32)
    global.get $BSS_LAST_FLOOR_APPLY_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_floor_apply_nonzero") (result i32)
    global.get $BSS_LAST_FLOOR_APPLY_NONZERO i32.load offset=0
  )

  (func (export "vorbis_last_floor_residue_pcm_status") (result i32)
    global.get $BSS_LAST_FLOOR_RESIDUE_PCM_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_floor_residue_pcm_frames") (result i32)
    global.get $BSS_LAST_FLOOR_RESIDUE_PCM_FRAMES i32.load offset=0
  )

  (func (export "vorbis_last_pcm_status") (result i32)
    global.get $BSS_LAST_PCM_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_silent_packet_count") (result i32)
    global.get $BSS_LAST_SILENT_PACKET_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_bad_codebook_index") (result i32)
    global.get $BSS_LAST_BAD_CODEBOOK_INDEX i32.load offset=0
  )

  (func (export "vorbis_last_bad_codebook_entries") (result i32)
    global.get $BSS_LAST_BAD_CODEBOOK_ENTRIES i32.load offset=0
  )

  (func (export "vorbis_last_bad_codebook_max_length") (result i32)
    global.get $BSS_LAST_BAD_CODEBOOK_MAX_LENGTH i32.load offset=0
  )

  (func (export "vorbis_last_mdct_synth_status") (result i32)
    global.get $BSS_LAST_MDCT_SYNTH_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_mdct_synth_packets") (result i32)
    global.get $BSS_LAST_MDCT_SYNTH_PACKETS i32.load offset=0
  )

  (func (export "vorbis_last_mdct_synth_frames") (result i32)
    global.get $BSS_LAST_MDCT_SYNTH_FRAMES i32.load offset=0
  )

  (func (export "vorbis_last_mdct_synth_nonzero") (result i32)
    global.get $BSS_LAST_MDCT_SYNTH_NONZERO i32.load offset=0
  )

  (func (export "vorbis_last_mdct_synth_overlap_nonzero") (result i32)
    global.get $BSS_LAST_MDCT_SYNTH_OVERLAP_NONZERO i32.load offset=0
  )

  (func (export "vorbis_last_imdct_kernel_bins") (result i32)
    global.get $BSS_LAST_IMDCT_KERNEL_BINS i32.load offset=0
  )

  (func (export "vorbis_last_imdct_kernel_samples") (result i32)
    global.get $BSS_LAST_IMDCT_KERNEL_SAMPLES i32.load offset=0
  )

  (func (export "vorbis_last_imdct_kernel_cross_terms") (result i32)
    global.get $BSS_LAST_IMDCT_KERNEL_CROSS_TERMS i32.load offset=0
  )

  
  (func $vorbis_decode_codebooks (export "vorbis_decode_codebooks")
    (param $data i32) (param $len i32) (param $setup i32) (result i32)
    (local $br i32) (local $i i32) (local $hdr i32) (local $entries i32)
    (local $ordered i32) (local $sparse i32) (local $j i32) (local $clen i32)
    (local $entry_offset i32) (local $mult_offset i32) (local $nonzero i32)
    (local $max_len i32) (local $book_count i32) (local $lookup_type i32)
    (local $lookup_val_bits i32) (local $mult_count i32) (local $k i32)
    (local $cur_len i32) (local $run i32) (local $sync i32)
    global.get $BSS_BITREADER local.set $br
    local.get $br local.get $data local.get $len call $vorbis_bitreader_init
    global.get $BSS_LAST_CODEBOOK_STATUS i32.const 0 i32.store offset=0
    global.get $BSS_LAST_CODEBOOK_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_CODEBOOK_ENTRIES i32.const 0 i32.store offset=0
    global.get $BSS_LAST_CODEBOOK_NONZERO i32.const 0 i32.store offset=0
    global.get $BSS_LAST_CODEBOOK_MAX_LEN i32.const 0 i32.store offset=0
    global.get $BSS_LAST_LOOKUP_VALUES i32.const 0 i32.store offset=0
    local.get $setup global.get $SS_CODEBOOK_COUNT i32.add i32.load offset=0 local.set $book_count
    local.get $book_count global.get $VORBIS_MAX_CODEBOOKS i32.gt_u
    if
      global.get $BSS_LAST_CODEBOOK_STATUS i32.const -6 i32.store offset=0
      global.get $VORBIS_ERR_TOO_MANY_BOOKS return
    end
    global.get $BSS_LAST_CODEBOOK_COUNT local.get $book_count i32.store offset=0
    i32.const 0 local.set $i
    i32.const 0 local.set $entry_offset
    i32.const 0 local.set $mult_offset
    i32.const 0 local.set $nonzero
    i32.const 0 local.set $max_len
    (block $book_done
      (loop $book_loop
        local.get $i local.get $book_count i32.ge_u br_if $book_done
        global.get $BSS_CODEBOOK_HEADERS local.get $i global.get $SIZEOF_CODEBOOK_HEADER i32.mul i32.add local.set $hdr
        i32.const 0 local.set $j
        (block $hdr_zero_done
          (loop $hdr_zero_loop
            local.get $j i32.const 15 i32.ge_u br_if $hdr_zero_done
            local.get $hdr local.get $j i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
            local.get $j i32.const 1 i32.add local.set $j
            br $hdr_zero_loop
          )
        )
        local.get $br i32.const 24 call $read_bits_internal local.set $sync
        local.get $sync global.get $VORBIS_CODEBOOK_SYNC i32.ne
        if
          global.get $BSS_LAST_CODEBOOK_STATUS i32.const -8 i32.store offset=0
          global.get $VORBIS_ERR_BAD_CODEBOOK return
        end
        local.get $hdr global.get $CB_DIMENSIONS i32.add local.get $br i32.const 16 call $read_bits_internal i32.store offset=0
        local.get $br i32.const 24 call $read_bits_internal local.set $entries
        local.get $hdr global.get $CB_ENTRIES i32.add local.get $entries i32.store offset=0
        local.get $hdr global.get $CB_LENGTH_OFFSET i32.add local.get $entry_offset i32.store offset=0
        local.get $hdr global.get $CB_CODE_OFFSET i32.add local.get $entry_offset i32.store offset=0
        local.get $entries local.get $entry_offset i32.add local.set $j
        local.get $j global.get $VORBIS_MAX_CODEBOOK_ENTRIES i32.gt_u
        if
          global.get $BSS_LAST_CODEBOOK_STATUS i32.const -7 i32.store offset=0
          global.get $VORBIS_ERR_TOO_MANY_ENTRIES return
        end
        global.get $BSS_LAST_CODEBOOK_ENTRIES local.get $j i32.store offset=0
        local.get $br call $read_bit_internal local.set $ordered
        local.get $hdr global.get $CB_ORDERED i32.add local.get $ordered i32.store offset=0
        local.get $ordered i32.eqz
        if
          local.get $br call $read_bit_internal local.set $sparse
          local.get $hdr global.get $CB_SPARSE i32.add local.get $sparse i32.store offset=0
          i32.const 0 local.set $j
          (block $unordered_done
            (loop $unordered_loop
              local.get $j local.get $entries i32.ge_u br_if $unordered_done
              i32.const 0 local.set $clen
              local.get $hdr global.get $CB_SPARSE i32.add i32.load offset=0
              if
                local.get $br call $read_bit_internal i32.eqz
                if
                  global.get $BSS_CODEBOOK_LENGTHS local.get $entry_offset local.get $j i32.add i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
                  local.get $j i32.const 1 i32.add local.set $j
                  br $unordered_loop
                end
              end
              local.get $br i32.const 5 call $read_bits_internal i32.const 1 i32.add local.set $clen
              global.get $BSS_CODEBOOK_LENGTHS local.get $entry_offset local.get $j i32.add i32.const 2 i32.shl i32.add local.get $clen i32.store offset=0
              local.get $clen i32.eqz if local.get $j i32.const 1 i32.add local.set $j br $unordered_loop end
              local.get $hdr global.get $CB_NONZERO_COUNT i32.add local.get $hdr global.get $CB_NONZERO_COUNT i32.add i32.load offset=0 i32.const 1 i32.add i32.store offset=0
              local.get $nonzero i32.const 1 i32.add local.set $nonzero
              local.get $clen local.get $hdr global.get $CB_MAX_LENGTH i32.add i32.load offset=0 i32.gt_u if local.get $hdr global.get $CB_MAX_LENGTH i32.add local.get $clen i32.store offset=0 end
              local.get $clen local.get $max_len i32.gt_u if local.get $clen local.set $max_len end
              local.get $j i32.const 1 i32.add local.set $j
              br $unordered_loop
            )
          )
        else
          local.get $br i32.const 5 call $read_bits_internal i32.const 1 i32.add local.set $cur_len
          i32.const 0 local.set $j
          (block $ordered_done
            (loop $ordered_loop
              local.get $j local.get $entries i32.ge_u br_if $ordered_done
              local.get $entries local.get $j i32.sub call $ilog_u32 local.set $run
              local.get $br local.get $run call $read_bits_internal local.set $run
              (block $ordered_run_done
                (loop $ordered_run_loop
                  local.get $run i32.eqz br_if $ordered_run_done
                  local.get $j local.get $entries i32.ge_u
                  if
                    global.get $BSS_LAST_CODEBOOK_STATUS i32.const -8 i32.store offset=0
                    global.get $VORBIS_ERR_BAD_CODEBOOK return
                  end
                  global.get $BSS_CODEBOOK_LENGTHS local.get $entry_offset local.get $j i32.add i32.const 2 i32.shl i32.add local.get $cur_len i32.store offset=0
                  local.get $hdr global.get $CB_NONZERO_COUNT i32.add local.get $hdr global.get $CB_NONZERO_COUNT i32.add i32.load offset=0 i32.const 1 i32.add i32.store offset=0
                  local.get $nonzero i32.const 1 i32.add local.set $nonzero
                  local.get $cur_len local.get $hdr global.get $CB_MAX_LENGTH i32.add i32.load offset=0 i32.gt_u if local.get $hdr global.get $CB_MAX_LENGTH i32.add local.get $cur_len i32.store offset=0 end
                  local.get $cur_len local.get $max_len i32.gt_u if local.get $cur_len local.set $max_len end
                  local.get $j i32.const 1 i32.add local.set $j
                  local.get $run i32.const 1 i32.sub local.set $run
                  br $ordered_run_loop
                )
              )
              local.get $cur_len i32.const 1 i32.add local.set $cur_len
              br $ordered_loop
            )
          )
        end
        local.get $i local.get $setup call $vorbis_codebook_build_canonical drop
        local.get $br i32.const 4 call $read_bits_internal local.set $lookup_type
        local.get $hdr global.get $CB_LOOKUP_TYPE i32.add local.get $lookup_type i32.store offset=0
        local.get $lookup_type i32.eqz
        if
          local.get $entry_offset local.get $entries i32.add local.set $entry_offset
          local.get $i i32.const 1 i32.add local.set $i
          br $book_loop
        end
        local.get $hdr global.get $CB_LOOKUP_MIN i32.add local.get $br i32.const 32 call $read_bits_internal i32.store offset=0
        local.get $hdr global.get $CB_LOOKUP_DELTA i32.add local.get $br i32.const 32 call $read_bits_internal i32.store offset=0
        local.get $br i32.const 4 call $read_bits_internal i32.const 1 i32.add local.set $lookup_val_bits
        local.get $hdr global.get $CB_LOOKUP_VALUE_BITS i32.add local.get $lookup_val_bits i32.store offset=0
        local.get $hdr global.get $CB_LOOKUP_SEQUENCE i32.add local.get $br call $read_bit_internal i32.store offset=0
        local.get $lookup_type i32.const 1 i32.eq
        if
          local.get $hdr global.get $CB_ENTRIES i32.add i32.load offset=0
          local.get $hdr global.get $CB_DIMENSIONS i32.add i32.load offset=0
          call $vorbis_codebook_lookup1_values
          local.set $mult_count
        else
          local.get $hdr global.get $CB_ENTRIES i32.add i32.load offset=0
          local.get $hdr global.get $CB_DIMENSIONS i32.add i32.load offset=0
          i32.mul
          local.set $mult_count
        end
        local.get $hdr global.get $CB_MULTIPLICAND_COUNT i32.add local.get $mult_count i32.store offset=0
        local.get $hdr global.get $CB_MULTIPLICAND_OFFSET i32.add local.get $mult_offset i32.store offset=0
        local.get $mult_count local.get $mult_offset i32.add local.set $k
        local.get $k global.get $VORBIS_MAX_MULTIPLICANDS i32.gt_u
        if
          global.get $BSS_LAST_CODEBOOK_STATUS i32.const -7 i32.store offset=0
          global.get $VORBIS_ERR_TOO_MANY_ENTRIES return
        end
        global.get $BSS_LAST_LOOKUP_VALUES local.get $k i32.store offset=0
        i32.const 0 local.set $k
        (block $mult_done
          (loop $mult_loop
            local.get $k local.get $mult_count i32.ge_u br_if $mult_done
            global.get $BSS_CODEBOOK_MULTIPLICANDS local.get $mult_offset local.get $k i32.add i32.const 2 i32.shl i32.add
            local.get $br local.get $lookup_val_bits call $read_bits_internal i32.store offset=0
            local.get $k i32.const 1 i32.add local.set $k
            br $mult_loop
          )
        )
        local.get $entry_offset local.get $entries i32.add local.set $entry_offset
        local.get $mult_offset local.get $mult_count i32.add local.set $mult_offset
        local.get $i i32.const 1 i32.add local.set $i
        br $book_loop
      )
    )
    global.get $BSS_LAST_CODEBOOK_ENTRIES local.get $entry_offset i32.store offset=0
    global.get $BSS_LAST_CODEBOOK_NONZERO local.get $nonzero i32.store offset=0
    global.get $BSS_LAST_CODEBOOK_MAX_LEN local.get $max_len i32.store offset=0
    global.get $BSS_LAST_LOOKUP_VALUES local.get $mult_offset i32.store offset=0
    local.get $setup global.get $SS_CODEBOOK_ENTRIES i32.add local.get $entry_offset i32.store offset=0
    local.get $setup global.get $SS_CODEBOOK_NONZERO i32.add local.get $nonzero i32.store offset=0
    local.get $setup global.get $SS_CODEBOOK_MAX_LEN i32.add local.get $max_len i32.store offset=0
    local.get $setup global.get $SS_LOOKUP_VALUES i32.add local.get $mult_offset i32.store offset=0
    local.get $setup global.get $SS_CODEBOOK_STATUS i32.add i32.const 0 i32.store offset=0
    i32.const 0
  )

  (func $vorbis_decode_floors (export "vorbis_decode_floors")
    (param $data i32) (param $len i32) (param $setup i32) (result i32)
    (local $br i32) (local $floor_count i32) (local $i i32) (local $fh i32)
    (local $val i32) (local $j i32) (local $k i32) (local $class_count i32)
    (local $max_class i32) (local $dim_val i32) (local $subbits i32)
    (local $master_val i32) (local $books_per i32) (local $b i32)
    (local $multiplier i32) (local $range_bits i32) (local $range_val i32)
    (local $part_idx i32) (local $dim_idx i32) (local $value_idx i32)
    local.get $setup global.get $SS_FLOOR_COUNT i32.add i32.load offset=0 local.set $floor_count
    local.get $floor_count global.get $VORBIS_MAX_SETUP_OBJS i32.gt_u
    if
      global.get $VORBIS_ERR_TOO_MANY_SETUP return
    end
    global.get $BSS_BITREADER local.set $br
    local.get $br local.get $data local.get $len call $vorbis_bitreader_init
    global.get $BSS_LAST_FLOOR_STATUS i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_VALUES i32.const 0 i32.store offset=0
    i32.const 0 local.set $i
    (block $floor_done
      (loop $floor_loop
        local.get $i local.get $floor_count i32.ge_u br_if $floor_done
        global.get $BSS_FLOOR_HEADERS local.get $i global.get $SIZEOF_FLOOR_HEADER i32.mul i32.add local.set $fh
        i32.const 0 local.set $j
        (block $fh_zero_done
          (loop $fh_zero_loop
            local.get $j i32.const 8 i32.ge_u br_if $fh_zero_done
            local.get $fh local.get $j i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
            local.get $j i32.const 1 i32.add local.set $j
            br $fh_zero_loop
          )
        )
        local.get $br i32.const 16 call $read_bits_internal
        i32.const 1 i32.ne
        if
          global.get $BSS_LAST_FLOOR_STATUS i32.const -2 i32.store offset=0
          global.get $VORBIS_ERR_BAD_ARCHIVE return
        end
        local.get $br i32.const 5 call $read_bits_internal local.set $val
        local.get $val global.get $VORBIS_MAX_FLOOR_PARTS i32.gt_u
        if
          global.get $BSS_LAST_FLOOR_STATUS i32.const -10 i32.store offset=0
          global.get $VORBIS_ERR_TOO_MANY_SETUP return
        end
        local.get $fh global.get $FH_PARTITION_COUNT i32.add local.get $val i32.store offset=0
        i32.const 0 local.set $max_class
        i32.const 0 local.set $j
        (block $part_done
          (loop $part_loop
            local.get $j local.get $val i32.ge_u br_if $part_done
            local.get $br i32.const 4 call $read_bits_internal local.set $k
            global.get $BSS_FLOOR_PARTITIONS local.get $j i32.const 2 i32.shl i32.add local.get $k i32.store offset=0
            local.get $k local.get $max_class i32.ge_u if local.get $k i32.const 1 i32.add local.set $max_class end
            local.get $j i32.const 1 i32.add local.set $j
            br $part_loop
          )
        )
        local.get $max_class global.get $VORBIS_MAX_FLOOR_CLASSES i32.gt_u
        if
          global.get $BSS_LAST_FLOOR_STATUS i32.const -10 i32.store offset=0
          global.get $VORBIS_ERR_TOO_MANY_SETUP return
        end
        local.get $fh global.get $FH_CLASS_COUNT i32.add local.get $max_class i32.store offset=0
        i32.const 0 local.set $j
        (block $class_done
          (loop $class_loop
            local.get $j local.get $max_class i32.ge_u br_if $class_done
            local.get $br i32.const 3 call $read_bits_internal i32.const 1 i32.add local.set $dim_val
            global.get $BSS_FLOOR_CLASS_DIMS local.get $j i32.const 2 i32.shl i32.add local.get $dim_val i32.store offset=0
            local.get $br i32.const 2 call $read_bits_internal local.set $subbits
            global.get $BSS_FLOOR_CLASS_SUBBITS local.get $j i32.const 2 i32.shl i32.add local.get $subbits i32.store offset=0
            local.get $subbits i32.eqz
            if
              global.get $BSS_FLOOR_CLASS_MASTER local.get $j i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
              i32.const 1 local.set $books_per
            else
              local.get $br i32.const 8 call $read_bits_internal local.set $master_val
              global.get $BSS_FLOOR_CLASS_MASTER local.get $j i32.const 2 i32.shl i32.add local.get $master_val i32.store offset=0
              i32.const 1 local.get $subbits i32.shl local.set $books_per
            end
            i32.const 0 local.set $b
            (block $book_done
              (loop $book_loop
                local.get $b local.get $books_per i32.ge_u br_if $book_done
                local.get $br i32.const 8 call $read_bits_internal i32.const 1 i32.sub local.set $k
                local.get $j i32.const 3 i32.shl local.get $b i32.or local.set $k
                local.get $k global.get $VORBIS_MAX_RESIDUE_BOOKS i32.ge_u
                if
                  global.get $BSS_LAST_FLOOR_STATUS i32.const -10 i32.store offset=0
                  global.get $VORBIS_ERR_TOO_MANY_SETUP return
                end
                global.get $BSS_FLOOR_CLASS_BOOKS local.get $k i32.const 2 i32.shl i32.add local.get $val i32.store offset=0
                local.get $b i32.const 1 i32.add local.set $b
                br $book_loop
              )
            )
            local.get $j i32.const 1 i32.add local.set $j
            br $class_loop
          )
        )
        local.get $br i32.const 2 call $read_bits_internal i32.const 1 i32.add local.set $multiplier
        local.get $fh global.get $FH_MULTIPLIER i32.add local.get $multiplier i32.store offset=0
        local.get $br i32.const 4 call $read_bits_internal local.set $range_bits
        local.get $fh global.get $FH_RANGE_BITS i32.add local.get $range_bits i32.store offset=0
        i32.const 1 local.get $range_bits i32.shl local.set $range_val
        global.get $BSS_FLOOR_VALUES i32.const 4 i32.add local.get $range_val i32.store offset=0
        i32.const 2 local.set $value_idx
        i32.const 0 local.set $j
        (block $fv_done
          (loop $fv_loop
            local.get $j local.get $fh global.get $FH_PARTITION_COUNT i32.add i32.load offset=0 i32.ge_u br_if $fv_done
            global.get $BSS_FLOOR_PARTITIONS local.get $j i32.const 2 i32.shl i32.add i32.load offset=0 local.set $part_idx
            global.get $BSS_FLOOR_CLASS_DIMS local.get $part_idx i32.const 2 i32.shl i32.add i32.load offset=0 local.set $dim_idx
            (block $dim_done
              (loop $dim_loop
                local.get $dim_idx i32.eqz br_if $dim_done
                local.get $value_idx global.get $VORBIS_MAX_FLOOR_VALUES i32.ge_u
                if
                  global.get $BSS_LAST_FLOOR_STATUS i32.const -10 i32.store offset=0
                  global.get $VORBIS_ERR_TOO_MANY_SETUP return
                end
                global.get $BSS_FLOOR_VALUES local.get $value_idx i32.const 2 i32.shl i32.add
                local.get $br local.get $range_bits call $read_bits_internal i32.store offset=0
                local.get $value_idx i32.const 1 i32.add local.set $value_idx
                local.get $dim_idx i32.const 1 i32.sub local.set $dim_idx
                br $dim_loop
              )
            )
            local.get $j i32.const 1 i32.add local.set $j
            br $fv_loop
          )
        )
        local.get $fh global.get $FH_VALUE_COUNT i32.add local.get $value_idx i32.store offset=0
        global.get $BSS_LAST_FLOOR_VALUES local.get $value_idx i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $floor_loop
      )
    )
    i32.const 0
  )



  (func $vorbis_decode_residues (export "vorbis_decode_residues")
    (param $data i32) (param $len i32) (param $setup i32) (result i32)
    (local $br i32) (local $residue_count i32) (local $i i32) (local $rh i32)
    (local $j i32) (local $k i32) (local $cascade i32) (local $has_extra i32)
    (local $val i32) (local $casc i32) (local $book_global i32)
    (local $classifications i32) (local $num_books i32)
    local.get $setup global.get $SS_RESIDUE_COUNT i32.add i32.load offset=0 local.set $residue_count
    local.get $residue_count global.get $VORBIS_MAX_SETUP_OBJS i32.gt_u
    if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
    global.get $BSS_BITREADER local.set $br
    local.get $br local.get $data local.get $len call $vorbis_bitreader_init
    global.get $BSS_LAST_RESIDUE_STATUS i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_BOOKS i32.const 0 i32.store offset=0
    i32.const 0 local.set $i
    i32.const 0 local.set $book_global
    (block $res_done
      (loop $res_loop
        local.get $i local.get $residue_count i32.ge_u br_if $res_done
        global.get $BSS_RESIDUE_HEADERS local.get $i global.get $SIZEOF_RESIDUE_HEADER i32.mul i32.add local.set $rh
        local.get $rh global.get $RH_RESIDUE_TYPE i32.add local.get $br i32.const 16 call $read_bits_internal i32.store offset=0
        local.get $rh global.get $RH_BEGIN i32.add local.get $br i32.const 24 call $read_bits_internal i32.store offset=0
        local.get $rh global.get $RH_END i32.add local.get $br i32.const 24 call $read_bits_internal i32.store offset=0
        local.get $rh global.get $RH_PARTITION_SIZE i32.add local.get $br i32.const 24 call $read_bits_internal i32.const 1 i32.add i32.store offset=0
        local.get $br i32.const 6 call $read_bits_internal i32.const 1 i32.add local.set $classifications
        local.get $classifications global.get $VORBIS_MAX_RESIDUE_CLASSES i32.gt_u
        if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
        local.get $rh global.get $RH_CLASSIFICATIONS i32.add local.get $classifications i32.store offset=0
        local.get $rh global.get $RH_CLASSBOOK i32.add local.get $br i32.const 8 call $read_bits_internal i32.store offset=0
        local.get $rh global.get $RH_BOOK_OFFSET i32.add local.get $book_global i32.store offset=0
        i32.const 0 local.set $j
        (block $cascade_done
          (loop $cascade_loop
            local.get $j local.get $classifications i32.ge_u br_if $cascade_done
            local.get $br i32.const 3 call $read_bits_internal local.set $cascade
            local.get $br call $read_bit_internal local.set $has_extra
            local.get $has_extra
            if
              local.get $br i32.const 5 call $read_bits_internal i32.const 3 i32.shl local.get $cascade i32.or local.set $cascade
            end
            global.get $BSS_RESIDUE_CASCADES local.get $j i32.const 2 i32.shl i32.add local.get $cascade i32.store offset=0
            local.get $j i32.const 1 i32.add local.set $j
            br $cascade_loop
          )
        )
        local.get $classifications i32.const 3 i32.shl local.set $num_books
        i32.const 0 local.set $j
        (block $book_done
          (loop $book_loop
            local.get $j local.get $num_books i32.ge_u br_if $book_done
            local.get $j i32.const 3 i32.shr_u local.set $k
            global.get $BSS_RESIDUE_CASCADES local.get $k i32.const 2 i32.shl i32.add i32.load offset=0 local.set $casc
            i32.const 1 local.get $j i32.const 7 i32.and i32.shl local.set $k
            local.get $casc local.get $k i32.and
            if
              local.get $br i32.const 8 call $read_bits_internal local.set $val
            else
              i32.const -1 local.set $val
            end
            local.get $book_global global.get $VORBIS_MAX_RESIDUE_BOOKS i32.ge_u
            if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
            global.get $BSS_RESIDUE_BOOKS local.get $book_global i32.const 2 i32.shl i32.add local.get $val i32.store offset=0
            local.get $book_global i32.const 1 i32.add local.set $book_global
            local.get $j i32.const 1 i32.add local.set $j
            br $book_loop
          )
        )
        global.get $BSS_LAST_RESIDUE_BOOKS local.get $book_global i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $res_loop
      )
    )
    i32.const 0
  )

  (func $vorbis_decode_mappings (export "vorbis_decode_mappings")
    (param $data i32) (param $len i32) (param $setup i32) (result i32)
    (local $br i32) (local $mapping_count i32) (local $i i32) (local $mh i32)
    (local $j i32) (local $val i32) (local $submaps i32) (local $mux i32)
    (local $floor_offset i32) (local $coupling i32)
    local.get $setup global.get $SS_MAPPING_COUNT i32.add i32.load offset=0 local.set $mapping_count
    local.get $mapping_count global.get $VORBIS_MAX_SETUP_OBJS i32.gt_u
    if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
    global.get $BSS_BITREADER local.set $br
    local.get $br local.get $data local.get $len call $vorbis_bitreader_init
    global.get $BSS_LAST_MAPPING_STATUS i32.const 0 i32.store offset=0
    i32.const 0 local.set $i
    (block $map_done
      (loop $map_loop
        local.get $i local.get $mapping_count i32.ge_u br_if $map_done
        global.get $BSS_MAPPING_HEADERS local.get $i global.get $SIZEOF_MAPPING_HEADER i32.mul i32.add local.set $mh
        local.get $br i32.const 16 call $read_bits_internal drop
        local.get $br call $read_bit_internal
        if
          local.get $br i32.const 4 call $read_bits_internal i32.const 1 i32.add local.set $submaps
        else
          i32.const 1 local.set $submaps
        end
        local.get $submaps global.get $VORBIS_MAX_MAPPING_SUBMAPS i32.gt_u
        if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
        local.get $mh global.get $MH_SUBMAPS i32.add local.get $submaps i32.store offset=0
        local.get $br call $read_bit_internal
        if
          local.get $br i32.const 8 call $read_bits_internal drop
        end
        local.get $br i32.const 2 call $read_bits_internal drop
        local.get $submaps i32.const 1 i32.le_u
        if
          i32.const 0 local.set $mux
        else
          local.get $br i32.const 4 call $read_bits_internal local.set $mux
        end
        local.get $mh global.get $MH_MUX i32.add local.get $mux i32.store offset=0
        local.get $i global.get $VORBIS_MAX_MAPPING_SUBMAPS i32.mul local.set $floor_offset
        local.get $mh global.get $MH_FLOOR_OFFSET i32.add local.get $floor_offset i32.store offset=0
        local.get $mh global.get $MH_RESIDUE_OFFSET i32.add local.get $floor_offset i32.store offset=0
        i32.const 0 local.set $j
        (block $submap_done
          (loop $submap_loop
            local.get $j local.get $submaps i32.ge_u br_if $submap_done
            local.get $br i32.const 8 call $read_bits_internal drop
            local.get $br i32.const 8 call $read_bits_internal local.set $val
            global.get $BSS_MAPPING_FLOORS local.get $floor_offset local.get $j i32.add i32.const 2 i32.shl i32.add local.get $val i32.store offset=0
            local.get $br i32.const 8 call $read_bits_internal local.set $val
            global.get $BSS_MAPPING_RESIDUES local.get $floor_offset local.get $j i32.add i32.const 2 i32.shl i32.add local.get $val i32.store offset=0
            local.get $j i32.const 1 i32.add local.set $j
            br $submap_loop
          )
        )
        local.get $i i32.const 1 i32.add local.set $i
        br $map_loop
      )
    )
    i32.const 0
  )

  (func $vorbis_decode_modes (export "vorbis_decode_modes")
    (param $data i32) (param $len i32) (param $setup i32) (result i32)
    (local $br i32) (local $mode_count i32) (local $i i32) (local $mh i32)
    local.get $setup global.get $SS_MODE_COUNT i32.add i32.load offset=0 local.set $mode_count
    local.get $mode_count global.get $VORBIS_MAX_SETUP_OBJS i32.gt_u
    if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
    global.get $BSS_BITREADER local.set $br
    local.get $br local.get $data local.get $len call $vorbis_bitreader_init
    global.get $BSS_LAST_MODE_STATUS i32.const 0 i32.store offset=0
    i32.const 0 local.set $i
    (block $mode_done
      (loop $mode_loop
        local.get $i local.get $mode_count i32.ge_u br_if $mode_done
        global.get $BSS_MODE_HEADERS local.get $i global.get $SIZEOF_MODE_HEADER i32.mul i32.add local.set $mh
        local.get $mh global.get $MD_BLOCK_FLAG i32.add local.get $br call $read_bit_internal i32.store offset=0
        local.get $br i32.const 16 call $read_bits_internal drop
        local.get $br i32.const 16 call $read_bits_internal drop
        local.get $mh global.get $MD_MAPPING i32.add local.get $br i32.const 8 call $read_bits_internal i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $mode_loop
      )
    )
    i32.const 0
  )

  (func (export "vorbis_decode_setup_archive")
    (param $data i32) (param $len i32) (param $setup i32) (result i32)
    (local $br i32) (local $val i32) (local $count i32) (local $res i32)
    global.get $BSS_BITREADER local.set $br
    local.get $br local.get $data local.get $len call $vorbis_bitreader_init
    local.get $setup call $vorbis_setup_init
    local.get $br i32.const 4 call $read_bits_internal local.set $val
    local.get $setup global.get $SS_BLOCK_SIZE_SHORT i32.add i32.const 1 local.get $val i32.shl i32.store offset=0
    local.get $br i32.const 4 call $read_bits_internal local.set $val
    local.get $setup global.get $SS_BLOCK_SIZE_LONG i32.add i32.const 1 local.get $val i32.shl i32.store offset=0
    local.get $br i32.const 8 call $read_bits_internal i32.const 1 i32.add local.set $val
    local.get $setup global.get $SS_CODEBOOK_COUNT i32.add local.get $val i32.store offset=0
    local.get $data local.get $len local.get $setup call $vorbis_decode_codebooks
    local.tee $res
    if
      local.get $res return
    end
    local.get $br i32.const 6 call $read_bits_internal i32.const 1 i32.add local.set $val
    local.get $setup global.get $SS_TIME_COUNT i32.add local.get $val i32.store offset=0
    (block $time_done
      (loop $time_loop
        local.get $val i32.eqz br_if $time_done
        local.get $br i32.const 16 call $read_bits_internal drop
        local.get $val i32.const 1 i32.sub local.set $val
        br $time_loop
      )
    )
    local.get $br i32.const 6 call $read_bits_internal i32.const 1 i32.add local.set $val
    local.get $setup global.get $SS_FLOOR_COUNT i32.add local.get $val i32.store offset=0
    local.get $data local.get $len local.get $setup call $vorbis_decode_floors
    local.tee $res
    if
      local.get $res return
    end
    local.get $br i32.const 6 call $read_bits_internal i32.const 1 i32.add local.set $val
    local.get $setup global.get $SS_RESIDUE_COUNT i32.add local.get $val i32.store offset=0
    local.get $data local.get $len local.get $setup call $vorbis_decode_residues
    local.tee $res
    if
      local.get $res return
    end
    local.get $br i32.const 6 call $read_bits_internal i32.const 1 i32.add local.set $val
    local.get $setup global.get $SS_MAPPING_COUNT i32.add local.get $val i32.store offset=0
    local.get $data local.get $len local.get $setup call $vorbis_decode_mappings
    local.tee $res
    if
      local.get $res return
    end
    local.get $br i32.const 6 call $read_bits_internal i32.const 1 i32.add local.set $val
    local.get $setup global.get $SS_MODE_COUNT i32.add local.get $val i32.store offset=0
    local.get $data local.get $len local.get $setup call $vorbis_decode_modes
    local.tee $res
    if
      local.get $res return
    end
    local.get $br global.get $BR_BYTE_POS i32.add i64.load offset=0 i32.wrap_i64 local.set $val
    local.get $setup global.get $SS_SETUP_BIT_BYTE i32.add local.get $val i32.store offset=0
    global.get $BSS_LAST_SETUP_BYTE local.get $val i32.store offset=0
    local.get $br global.get $BR_BIT_POS i32.add i32.load offset=0 local.set $val
    local.get $setup global.get $SS_SETUP_BIT_OFFSET i32.add local.get $val i32.store offset=0
    global.get $BSS_LAST_SETUP_BIT local.get $val i32.store offset=0
    local.get $setup global.get $SS_READY i32.add i32.const 1 i32.store offset=0
    i32.const 0
  )

  (func $vorbis_decode_sample_packets (export "vorbis_decode_sample_packets")
    (param $sample i32) (param $setup i32) (result i32)
    (local $br i32) (local $mode_count i32) (local $mode_bits i32) (local $index i32)
    (local $packet_ptr i32) (local $packet_len i32) (local $mode_index i32)
    (local $mode_header i32) (local $prev_flag i32) (local $next_flag i32)
    (local $result i32) (local $mapping i32) (local $mapping_header i32)
    (local $mux i32) (local $floor i32) (local $floor_header i32)
    (local $residue i32) (local $residue_header i32) (local $error i32)
    (local $flags i32)

    local.get $setup
    if (result i32)
      local.get $setup
    else
      global.get $BSS_SETUP_STATE
    end
    local.set $setup

    global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_TODO_DECODE i32.store offset=0
    global.get $BSS_LAST_PACKET_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_PACKET_MODE i32.const 0 i32.store offset=0
    global.get $BSS_LAST_PACKET_MAPPING i32.const 0 i32.store offset=0
    global.get $BSS_LAST_PACKET_FLOOR i32.const 0 i32.store offset=0
    global.get $BSS_LAST_PACKET_RESIDUE i32.const 0 i32.store offset=0
    global.get $BSS_LAST_MODE_BITS i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_STATUS global.get $VORBIS_ERR_TODO_DECODE i32.store offset=0
    global.get $BSS_LAST_WINDOW_PACKET i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_BLOCK_SIZE i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_PREV_FLAG i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_NEXT_FLAG i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_LEFT_START i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_LEFT_END i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_RIGHT_START i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_RIGHT_END i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_LEFT_FRAMES i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_RIGHT_FRAMES i32.const 0 i32.store offset=0
    global.get $BSS_LAST_MDCT_BUFFER_FRAMES i32.const 0 i32.store offset=0
    global.get $BSS_LAST_OVERLAP_FRAMES i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_STATUS global.get $VORBIS_ERR_TODO_DECODE i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_NONZERO i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_Y0 i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_Y1 i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_TODO_DECODE i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_PARTITION_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_CLASS_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_BOOK_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_SCALAR_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_VALUE_COUNT i32.const 0 i32.store offset=0

    local.get $setup global.get $SS_READY i32.add i32.load offset=0
    i32.const 1 i32.ne
    if
      global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
      global.get $VORBIS_ERR_BAD_ARCHIVE return
    end

    local.get $setup global.get $SS_MODE_COUNT i32.add i32.load offset=0
    local.tee $mode_count
    i32.eqz
    if
      global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
      global.get $VORBIS_ERR_BAD_ARCHIVE return
    end

    local.get $mode_count i32.const 1 i32.sub
    call $ilog_u32
    local.tee $mode_bits
    global.get $BSS_LAST_MODE_BITS i32.store offset=0

    global.get $BSS_BITREADER local.set $br

    i32.const 0 local.set $index

    (block $packet_frontier
      (loop $packet_loop
        local.get $index
        local.get $sample global.get $SS_PACKET_COUNT i32.add i32.load offset=0
        i32.ge_u
        br_if $packet_frontier

        local.get $sample global.get $SS_PACKET_PTRS i32.add
        local.get $index i32.const 3 i32.shl i32.add
        i64.load offset=0 i32.wrap_i64
        local.tee $packet_ptr
        i32.eqz
        if
          global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
          global.get $VORBIS_ERR_BAD_ARCHIVE return
        end

        local.get $sample global.get $SS_PACKET_LENS i32.add
        local.get $index i32.const 2 i32.shl i32.add
        i32.load offset=0
        local.tee $packet_len
        i32.eqz
        if
          global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
          global.get $VORBIS_ERR_BAD_ARCHIVE return
        end

        local.get $br local.get $packet_ptr local.get $packet_len call $vorbis_bitreader_init

        local.get $br call $read_bit_internal drop
        local.get $br global.get $BR_ERROR i32.add i32.load offset=0
        local.tee $error
        if
          global.get $BSS_LAST_PACKET_STATUS local.get $error i32.store offset=0
          local.get $error return
        end

        local.get $br local.get $mode_bits call $read_bits_internal
        local.set $mode_index
        local.get $br global.get $BR_ERROR i32.add i32.load offset=0
        local.tee $error
        if
          global.get $BSS_LAST_PACKET_STATUS local.get $error i32.store offset=0
          local.get $error return
        end

        local.get $mode_index local.get $mode_count i32.ge_u
        if
          global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_INDEX i32.store offset=0
          global.get $VORBIS_ERR_BAD_INDEX return
        end

        global.get $BSS_PACKET_MODES local.get $index i32.const 2 i32.shl i32.add
        local.get $mode_index i32.store offset=0
        global.get $BSS_LAST_PACKET_MODE local.get $mode_index i32.store offset=0

        global.get $BSS_MODE_HEADERS
        local.get $mode_index global.get $SIZEOF_MODE_HEADER i32.mul i32.add
        local.set $mode_header

        i32.const 0 local.set $prev_flag
        i32.const 0 local.set $next_flag
        local.get $mode_header global.get $MD_BLOCK_FLAG i32.add i32.load offset=0
        if
          local.get $br i32.const 2 call $read_bits_internal
          local.set $flags
          local.get $br global.get $BR_ERROR i32.add i32.load offset=0
          local.tee $error
          if
            global.get $BSS_LAST_PACKET_STATUS local.get $error i32.store offset=0
            local.get $error return
          end
          local.get $flags i32.const 1 i32.and local.set $prev_flag
          local.get $flags i32.const 1 i32.shr_u i32.const 1 i32.and local.set $next_flag
        end

        local.get $setup
        local.get $mode_header
        local.get $index
        local.get $prev_flag
        local.get $next_flag
        call $vorbis_prepare_packet_window
        local.tee $result
        if
          global.get $BSS_LAST_PACKET_STATUS local.get $result i32.store offset=0
          local.get $result return
        end

        local.get $mode_header global.get $MD_MAPPING i32.add i32.load offset=0
        local.tee $mapping
        local.get $setup global.get $SS_MAPPING_COUNT i32.add i32.load offset=0
        i32.ge_u
        if
          global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_INDEX i32.store offset=0
          global.get $VORBIS_ERR_BAD_INDEX return
        end

        global.get $BSS_PACKET_MAPPINGS local.get $index i32.const 2 i32.shl i32.add
        local.get $mapping i32.store offset=0
        global.get $BSS_LAST_PACKET_MAPPING local.get $mapping i32.store offset=0

        global.get $BSS_MAPPING_HEADERS
        local.get $mapping global.get $SIZEOF_MAPPING_HEADER i32.mul i32.add
        local.set $mapping_header

        local.get $mapping_header global.get $MH_MUX i32.add i32.load offset=0
        local.tee $mux
        local.get $mapping_header global.get $MH_SUBMAPS i32.add i32.load offset=0
        i32.ge_u
        if
          global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_INDEX i32.store offset=0
          global.get $VORBIS_ERR_BAD_INDEX return
        end

        global.get $BSS_MAPPING_FLOORS
        local.get $mapping_header global.get $MH_FLOOR_OFFSET i32.add i32.load offset=0
        local.get $mux i32.add
        i32.const 2 i32.shl i32.add
        i32.load offset=0
        local.tee $floor
        local.get $setup global.get $SS_FLOOR_COUNT i32.add i32.load offset=0
        i32.ge_u
        if
          global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_INDEX i32.store offset=0
          global.get $VORBIS_ERR_BAD_INDEX return
        end

        global.get $BSS_PACKET_FLOORS local.get $index i32.const 2 i32.shl i32.add
        local.get $floor i32.store offset=0
        global.get $BSS_LAST_PACKET_FLOOR local.get $floor i32.store offset=0

        global.get $BSS_FLOOR_HEADERS
        local.get $floor global.get $SIZEOF_FLOOR_HEADER i32.mul i32.add
        local.set $floor_header

        local.get $br local.get $floor_header local.get $index
        call $vorbis_decode_packet_floor
        local.tee $result
        if
          global.get $BSS_LAST_PACKET_STATUS local.get $result i32.store offset=0
          local.get $result return
        end

        global.get $BSS_MAPPING_RESIDUES
        local.get $mapping_header global.get $MH_RESIDUE_OFFSET i32.add i32.load offset=0
        local.get $mux i32.add
        i32.const 2 i32.shl i32.add
        i32.load offset=0
        local.tee $residue
        local.get $setup global.get $SS_RESIDUE_COUNT i32.add i32.load offset=0
        i32.ge_u
        if
          global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_INDEX i32.store offset=0
          global.get $VORBIS_ERR_BAD_INDEX return
        end

        global.get $BSS_PACKET_RESIDUES local.get $index i32.const 2 i32.shl i32.add
        local.get $residue i32.store offset=0
        global.get $BSS_LAST_PACKET_RESIDUE local.get $residue i32.store offset=0

        global.get $BSS_RESIDUE_HEADERS
        local.get $residue global.get $SIZEOF_RESIDUE_HEADER i32.mul i32.add
        local.set $residue_header

        local.get $br local.get $residue_header local.get $index
        call $vorbis_decode_packet_residue
        local.tee $result
        if
          global.get $BSS_LAST_PACKET_STATUS local.get $result i32.store offset=0
          local.get $result return
        end

        local.get $index i32.const 1 i32.add
        local.tee $index
        global.get $BSS_LAST_PACKET_COUNT i32.store offset=0

        br $packet_loop
      )
    )

    global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_TODO_DECODE i32.store offset=0
    i32.const 0
  )

  (func $vorbis_decode_packet_floor (export "vorbis_decode_packet_floor")
    (param $br i32) (param $packet_index i32) (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )

  (func $vorbis_decode_packet_residue (export "vorbis_decode_packet_residue")
    (param $br i32) (param $rh i32) (param $packet_index i32) (result i32)
    (local $classbook_dims i32) (local $classifications i32)
    (local $partition_size i32) (local $part_count i32)
    (local $base_parts i32) (local $base_values i32)
    (local $hdr i32) (local $book_index i32)
    (local $i i32) (local $j i32) (local $k i32) (local $pass_idx i32)
    (local $class_id i32) (local $classbook_entry i32)
    (local $scalar_val i32) (local $tmp i32) (local $tmp2 i32)
    (local $value_idx i32) (local $target_idx i32)
    (local $lookup1_val i32) (local $lookup_min_q15 i32)
    (local $lookup_delta_q15 i32) (local $seq_acc i32)
    (local $lookup_type i32) (local $mult_val i32)
    global.get $BSS_PACKET_RESIDUE_PART_COUNT
    local.get $packet_index i32.const 2 i32.shl i32.add
    i32.const 0 i32.store offset=0
    global.get $BSS_PACKET_RESIDUE_VALUE_COUNT
    local.get $packet_index i32.const 2 i32.shl i32.add
    i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_PARTITION_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_CLASS_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_BOOK_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_SCALAR_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_VALUE_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_PACKET_FLOOR_NONZERO
    local.get $packet_index i32.const 2 i32.shl i32.add
    i32.load offset=0
    if
      local.get $rh global.get $RH_RESIDUE_TYPE i32.add i32.load offset=0
      i32.const 2 i32.gt_u
      if
        global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
        global.get $VORBIS_ERR_BAD_ARCHIVE return
      end
      local.get $rh global.get $RH_PARTITION_SIZE i32.add i32.load offset=0
      local.tee $partition_size
      i32.eqz
      if
        global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
        global.get $VORBIS_ERR_BAD_ARCHIVE return
      end
      local.get $rh global.get $RH_CLASSIFICATIONS i32.add i32.load offset=0
      local.tee $classifications
      i32.eqz
      if
        global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
        global.get $VORBIS_ERR_BAD_ARCHIVE return
      end
      local.get $classifications global.get $VORBIS_MAX_RESIDUE_CLASSES i32.gt_u
      if
        global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_TOO_MANY_SETUP i32.store offset=0
        global.get $VORBIS_ERR_TOO_MANY_SETUP return
      end
      local.get $rh global.get $RH_CLASSBOOK i32.add i32.load offset=0
      local.tee $book_index
      global.get $BSS_LAST_CODEBOOK_COUNT i32.load offset=0
      i32.ge_u
      if
        global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_BAD_INDEX i32.store offset=0
        global.get $VORBIS_ERR_BAD_INDEX return
      end
      local.get $book_index call $vorbis_codebook_get_header
      local.tee $hdr
      i32.eqz
      if
        global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_BAD_INDEX i32.store offset=0
        global.get $VORBIS_ERR_BAD_INDEX return
      end
      local.get $hdr global.get $CB_DIMENSIONS i32.add i32.load offset=0
      local.tee $classbook_dims
      i32.eqz
      if
        global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
        global.get $VORBIS_ERR_BAD_ARCHIVE return
      end
      local.get $classbook_dims global.get $VORBIS_PACKET_RESIDUE_PARTS_PER_PACKET i32.gt_u
      if
        global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_TOO_MANY_SETUP i32.store offset=0
        global.get $VORBIS_ERR_TOO_MANY_SETUP return
      end
      local.get $rh global.get $RH_END i32.add i32.load offset=0
      local.get $rh global.get $RH_BEGIN i32.add i32.load offset=0
      local.tee $tmp
      i32.lt_u
      if
        global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
        global.get $VORBIS_ERR_BAD_ARCHIVE return
      end
      local.get $rh global.get $RH_END i32.add i32.load offset=0
      local.get $tmp i32.sub
      local.get $partition_size
      i32.div_u
      local.tee $part_count
      if
        local.get $part_count global.get $VORBIS_PACKET_RESIDUE_PARTS_PER_PACKET i32.gt_u
        if
          global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_TOO_MANY_SETUP i32.store offset=0
          global.get $VORBIS_ERR_TOO_MANY_SETUP return
        end
        global.get $BSS_PACKET_RESIDUE_PART_COUNT
        local.get $packet_index i32.const 2 i32.shl i32.add
        local.get $part_count i32.store offset=0
        global.get $BSS_LAST_RESIDUE_PACKET_PARTITION_COUNT local.get $part_count i32.store offset=0
        local.get $packet_index global.get $VORBIS_PACKET_RESIDUE_PARTS_PER_PACKET i32.mul
        local.set $base_parts
        local.get $packet_index global.get $VORBIS_PACKET_RESIDUE_VALUES_PER_PACKET i32.mul
        local.set $base_values
        i32.const 0 local.set $i
        (block $class_done
          (loop $class_loop
            local.get $i
            global.get $BSS_PACKET_RESIDUE_PART_COUNT
            local.get $packet_index i32.const 2 i32.shl i32.add
            i32.load offset=0
            i32.ge_u
            br_if $class_done
            local.get $br
            local.get $book_index
            i32.const 0
            call $vorbis_codebook_decode_scalar
            local.set $classbook_entry
            i32.const 0 local.set $j
            (block $dim_done
              (loop $dim_loop
                local.get $j
                local.get $classbook_dims
                i32.ge_u
                br_if $dim_done
                local.get $i local.get $j i32.add
                local.tee $tmp
                global.get $BSS_PACKET_RESIDUE_PART_COUNT
                local.get $packet_index i32.const 2 i32.shl i32.add
                i32.load offset=0
                i32.ge_u
                br_if $dim_done
                local.get $classbook_entry
                local.get $classifications
                i32.rem_u
                local.set $tmp2
                local.get $classbook_entry
                local.get $classifications
                i32.div_u
                local.set $classbook_entry
                local.get $base_parts local.get $i i32.add local.get $j i32.add
                local.tee $tmp
                global.get $VORBIS_MAX_PACKET_RESIDUE_PARTS
                i32.ge_u
                if
                  global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_TOO_MANY_SETUP i32.store offset=0
                  global.get $VORBIS_ERR_TOO_MANY_SETUP return
                end
                global.get $BSS_PACKET_RESIDUE_CLASSIFICATIONS
                local.get $tmp i32.const 2 i32.shl i32.add
                local.get $tmp2 i32.store offset=0
                global.get $BSS_LAST_RESIDUE_PACKET_CLASS_COUNT
                global.get $BSS_LAST_RESIDUE_PACKET_CLASS_COUNT i32.load offset=0
                i32.const 1 i32.add
                i32.store offset=0
                local.get $j i32.const 1 i32.add local.set $j
                br $dim_loop
              )
            )
            local.get $i local.get $classbook_dims i32.add local.set $i
            br $class_loop
          )
        )
        i32.const 0 local.set $pass_idx
        (block $pass_done
          (loop $pass_loop
            local.get $pass_idx i32.const 8 i32.ge_u br_if $pass_done
            i32.const 0 local.set $i
            (block $part_pass_done
              (loop $part_pass_loop
                local.get $i
                global.get $BSS_PACKET_RESIDUE_PART_COUNT
                local.get $packet_index i32.const 2 i32.shl i32.add
                i32.load offset=0
                i32.ge_u
                br_if $part_pass_done
                local.get $base_parts local.get $i i32.add
                local.tee $tmp
                global.get $VORBIS_MAX_PACKET_RESIDUE_PARTS
                i32.ge_u
                if
                  global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_TOO_MANY_SETUP i32.store offset=0
                  global.get $VORBIS_ERR_TOO_MANY_SETUP return
                end
                global.get $BSS_PACKET_RESIDUE_CLASSIFICATIONS
                local.get $tmp i32.const 2 i32.shl i32.add
                i32.load offset=0
                local.tee $class_id
                local.get $classifications
                i32.ge_u
                if
                  global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
                  global.get $VORBIS_ERR_BAD_ARCHIVE return
                end
                local.get $class_id i32.const 3 i32.shl
                local.get $pass_idx i32.add
                local.get $rh global.get $RH_BOOK_OFFSET i32.add i32.load offset=0
                i32.add
                local.tee $tmp
                global.get $VORBIS_MAX_RESIDUE_BOOKS
                i32.ge_u
                if
                  global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_TOO_MANY_SETUP i32.store offset=0
                  global.get $VORBIS_ERR_TOO_MANY_SETUP return
                end
                global.get $BSS_RESIDUE_BOOKS
                local.get $tmp i32.const 2 i32.shl i32.add
                i32.load offset=0
                local.tee $book_index
                i32.const -1 i32.eq
                if
                  local.get $i i32.const 1 i32.add local.set $i
                  br $part_pass_loop
                end
                local.get $book_index
                global.get $BSS_LAST_CODEBOOK_COUNT i32.load offset=0
                i32.ge_u
                if
                  global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_BAD_INDEX i32.store offset=0
                  global.get $VORBIS_ERR_BAD_INDEX return
                end
                local.get $br
                local.get $book_index
                i32.const 0
                call $vorbis_codebook_decode_scalar
                local.set $scalar_val
                global.get $BSS_LAST_RESIDUE_PACKET_SCALAR_COUNT
                global.get $BSS_LAST_RESIDUE_PACKET_SCALAR_COUNT i32.load offset=0
                i32.const 1 i32.add
                i32.store offset=0
                global.get $BSS_LAST_RESIDUE_PACKET_BOOK_COUNT
                global.get $BSS_LAST_RESIDUE_PACKET_BOOK_COUNT i32.load offset=0
                i32.const 1 i32.add
                i32.store offset=0
                local.get $book_index call $vorbis_codebook_get_header
                local.tee $hdr
                i32.eqz
                if
                  global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_BAD_INDEX i32.store offset=0
                  global.get $VORBIS_ERR_BAD_INDEX return
                end
                local.get $hdr global.get $CB_DIMENSIONS i32.add i32.load offset=0
                local.tee $classbook_dims
                i32.eqz
                if
                  global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
                  global.get $VORBIS_ERR_BAD_ARCHIVE return
                end
                local.get $hdr global.get $CB_LOOKUP_TYPE i32.add i32.load offset=0
                local.set $lookup_type
                local.get $lookup_type
                i32.const 1
                i32.eq
                if
                  local.get $hdr global.get $CB_ENTRIES i32.add i32.load offset=0
                  local.get $classbook_dims
                  call $vorbis_codebook_lookup1_values
                  local.set $lookup1_val
                else
                  local.get $lookup_type
                  i32.const 2
                  i32.eq
                  if
                    local.get $hdr global.get $CB_ENTRIES i32.add i32.load offset=0
                    local.set $lookup1_val
                  else
                    i32.const 0 local.set $lookup1_val
                  end
                end
                local.get $hdr global.get $CB_LOOKUP_MIN i32.add i32.load offset=0
                call $unpack_float_q15
                local.set $lookup_min_q15
                local.get $hdr global.get $CB_LOOKUP_DELTA i32.add i32.load offset=0
                call $unpack_float_q15
                local.set $lookup_delta_q15
                i32.const 0 local.set $j
                i32.const 0 local.set $seq_acc
                (block $expand_done
                  (loop $expand_loop
                    local.get $j
                    local.get $classbook_dims
                    i32.ge_u
                    br_if $expand_done
                    global.get $BSS_LAST_RESIDUE_PACKET_VALUE_COUNT i32.load offset=0
                    local.tee $tmp
                    global.get $VORBIS_PACKET_RESIDUE_VALUES_PER_PACKET
                    i32.ge_u
                    if
                      global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_TOO_MANY_SETUP i32.store offset=0
                      global.get $VORBIS_ERR_TOO_MANY_SETUP return
                    end
                    local.get $base_values
                    local.get $tmp i32.add
                    local.tee $value_idx
                    global.get $VORBIS_MAX_PACKET_RESIDUE_VALUES
                    i32.ge_u
                    if
                      global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_TOO_MANY_SETUP i32.store offset=0
                      global.get $VORBIS_ERR_TOO_MANY_SETUP return
                    end
                    global.get $BSS_PACKET_RESIDUE_VALUE_BOOKS
                    local.get $value_idx i32.const 2 i32.shl i32.add
                    local.get $book_index i32.store offset=0
                    global.get $BSS_PACKET_RESIDUE_VALUE_ENTRIES
                    local.get $value_idx i32.const 2 i32.shl i32.add
                    local.get $scalar_val i32.store offset=0
                    global.get $BSS_PACKET_RESIDUE_VALUE_DIMS
                    local.get $value_idx i32.const 2 i32.shl i32.add
                    local.get $j i32.store offset=0
                    local.get $i
                    local.get $rh global.get $RH_PARTITION_SIZE i32.add i32.load offset=0
                    i32.mul
                    local.get $rh global.get $RH_BEGIN i32.add i32.load offset=0
                    i32.add
                    local.get $j i32.add
                    local.tee $target_idx
                    global.get $VORBIS_MAX_BLOCK_SIZE
                    i32.ge_u
                    if
                      global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_TOO_MANY_SETUP i32.store offset=0
                      global.get $VORBIS_ERR_TOO_MANY_SETUP return
                    end
                    global.get $BSS_PACKET_RESIDUE_VALUE_TARGETS
                    local.get $value_idx i32.const 2 i32.shl i32.add
                    local.get $target_idx i32.store offset=0
                    i32.const 0 local.set $mult_val
                    local.get $lookup_type
                    i32.const 1
                    i32.eq
                    if
                      local.get $lookup1_val
                      if
                        local.get $lookup1_val
                        local.get $j
                        call $pow_u32_capped
                        local.tee $k
                        if
                          local.get $scalar_val
                          local.get $k
                          i32.div_u
                          local.tee $k
                          local.get $lookup1_val
                          i32.rem_u
                          local.set $k
                          local.get $hdr global.get $CB_MULTIPLICAND_OFFSET i32.add i32.load offset=0
                          local.get $k i32.add
                          local.tee $k
                          global.get $VORBIS_MAX_MULTIPLICANDS
                          i32.ge_u
                          if
                            global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_TOO_MANY_SETUP i32.store offset=0
                            global.get $VORBIS_ERR_TOO_MANY_SETUP return
                          end
                          global.get $BSS_CODEBOOK_MULTIPLICANDS
                          local.get $k i32.const 2 i32.shl i32.add
                          i32.load offset=0
                          local.set $mult_val
                        end
                      end
                    else
                      local.get $lookup_type
                      i32.const 2
                      i32.eq
                      if
                        local.get $scalar_val
                        local.get $classbook_dims i32.mul
                        local.get $j i32.add
                        local.get $hdr global.get $CB_MULTIPLICAND_OFFSET i32.add i32.load offset=0
                        i32.add
                        local.tee $k
                        global.get $VORBIS_MAX_MULTIPLICANDS
                        i32.ge_u
                        if
                          global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_TOO_MANY_SETUP i32.store offset=0
                          global.get $VORBIS_ERR_TOO_MANY_SETUP return
                        end
                        global.get $BSS_CODEBOOK_MULTIPLICANDS
                        local.get $k i32.const 2 i32.shl i32.add
                        i32.load offset=0
                        local.set $mult_val
                      end
                    end
                    local.get $lookup_type
                    i32.eqz
                    if
                      i32.const 0 local.set $mult_val
                    else
                      local.get $mult_val
                      local.get $lookup_delta_q15 i32.mul
                      local.get $lookup_min_q15 i32.add
                      local.set $k
                      local.get $hdr global.get $CB_LOOKUP_SEQUENCE i32.add i32.load offset=0
                      if
                        local.get $k
                        local.get $seq_acc i32.add
                        local.set $k
                      end
                      i32.const 32767
                      local.get $k
                      i32.gt_s
                      if
                        i32.const 32767 local.set $k
                      end
                      i32.const -32768
                      local.get $k
                      i32.lt_s
                      if
                        i32.const -32768 local.set $k
                      end
                      local.get $k local.set $mult_val
                    end
                    global.get $BSS_PACKET_RESIDUE_VALUE_VALUES
                    local.get $value_idx i32.const 2 i32.shl i32.add
                    local.get $mult_val i32.store offset=0
                    global.get $BSS_LAST_RESIDUE_PACKET_VALUE_COUNT
                    global.get $BSS_LAST_RESIDUE_PACKET_VALUE_COUNT i32.load offset=0
                    i32.const 1 i32.add
                    i32.store offset=0
                    local.get $j i32.const 1 i32.add local.set $j
                    br $expand_loop
                  )
                )
                local.get $i i32.const 1 i32.add local.set $i
                br $part_pass_loop
              )
            )
            local.get $pass_idx i32.const 1 i32.add local.set $pass_idx
            br $pass_loop
          )
        )
      end
    end
    global.get $BSS_LAST_RESIDUE_PACKET_VALUE_COUNT i32.load offset=0
    global.get $BSS_PACKET_RESIDUE_VALUE_COUNT
    local.get $packet_index i32.const 2 i32.shl i32.add
    i32.store offset=0
    local.get $packet_index
    call $vorbis_build_residue_work_vector_q15
    local.tee $tmp
    if
      global.get $BSS_LAST_RESIDUE_PACKET_STATUS local.get $tmp i32.store offset=0
      local.get $tmp return
    end
    local.get $packet_index
    call $vorbis_apply_floor_curve_to_residue_q15
    local.tee $tmp
    if
      global.get $BSS_LAST_RESIDUE_PACKET_STATUS local.get $tmp i32.store offset=0
      local.get $tmp return
    end
    global.get $BSS_LAST_RESIDUE_PACKET_COUNT
    global.get $BSS_LAST_RESIDUE_PACKET_COUNT i32.load offset=0
    i32.const 1 i32.add
    i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_TODO_DECODE i32.store offset=0
    i32.const 0
  )

  (func $vorbis_prepare_packet_window (export "vorbis_prepare_packet_window")
    (param $setup i32) (param $packet_index i32) (param $prev_flag i32) (param $next_flag i32) (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )

  (func (export "vorbis_reconstruct_floor1_point_y")
    (param $packet_index i32) (param $point_index i32) (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )

  (func $vorbis_build_residue_work_vector_q15 (export "vorbis_build_residue_work_vector_q15")
    (param $packet_index i32) (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )

  (func $vorbis_apply_floor_curve_to_residue_q15 (export "vorbis_apply_floor_curve_to_residue_q15")
    (param $packet_index i32) (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )

  (func $vorbis_build_packet_window_q15 (export "vorbis_build_packet_window_q15")
    (param $packet_index i32) (param $block_size i32) (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )

  (func $vorbis_run_owned_lapped_transform_q15 (export "vorbis_run_owned_lapped_transform_q15")
    (param $block_size i32) (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )

  (func (export "vorbis_publish_floor_residue_pcm")
    (param $sample i32) (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )

  (func $vorbis_synthesize_mdct_window_pcm (export "vorbis_synthesize_mdct_window_pcm")
    (param $sample i32) (result i32)
    (local $packet_idx i32) (local $frame_cursor i32) (local $block_size i32) (local $half i32)
    (local $i i32) (local $val i32) (local $frame_count i32) (local $packet_count i32)
    (local $synth_nonzero i32) (local $overlap_nonzero i32) (local $synth_packets i32)
    global.get $BSS_LAST_MDCT_SYNTH_PACKETS i32.const 0 i32.store offset=0
    global.get $BSS_LAST_MDCT_SYNTH_FRAMES i32.const 0 i32.store offset=0
    global.get $BSS_LAST_MDCT_SYNTH_NONZERO i32.const 0 i32.store offset=0
    global.get $BSS_LAST_MDCT_SYNTH_OVERLAP_NONZERO i32.const 0 i32.store offset=0
    global.get $BSS_LAST_IMDCT_KERNEL_BINS i32.const 0 i32.store offset=0
    global.get $BSS_LAST_IMDCT_KERNEL_SAMPLES i32.const 0 i32.store offset=0
    global.get $BSS_LAST_IMDCT_KERNEL_CROSS_TERMS i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_RESIDUE_PCM_STATUS global.get $VORBIS_ERR_TODO_DECODE i32.store offset=0
    global.get $BSS_LAST_FLOOR_RESIDUE_PCM_FRAMES i32.const 0 i32.store offset=0
    local.get $sample global.get $SS_FRAME_COUNT i32.add i32.load offset=0
    local.tee $frame_count
    i32.const 0 i32.le_s
    if
      global.get $BSS_LAST_MDCT_SYNTH_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
      global.get $VORBIS_ERR_BAD_ARCHIVE return
    end
    local.get $frame_count global.get $VORBIS_MAX_PCM_FRAMES i32.gt_u
    if
      global.get $BSS_LAST_MDCT_SYNTH_STATUS global.get $VORBIS_ERR_PCM_TOO_LARGE i32.store offset=0
      global.get $VORBIS_ERR_PCM_TOO_LARGE return
    end
    local.get $sample global.get $SS_PACKET_COUNT i32.add i32.load offset=0
    local.tee $packet_count
    i32.eqz
    if
      global.get $BSS_LAST_MDCT_SYNTH_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
      global.get $VORBIS_ERR_BAD_ARCHIVE return
    end
    local.get $packet_count global.get $BSS_LAST_PACKET_COUNT i32.load offset=0
    i32.ne
    if
      global.get $BSS_LAST_MDCT_SYNTH_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
      global.get $VORBIS_ERR_BAD_ARCHIVE return
    end
    i32.const 0 local.set $i
    (block $zero_pcm_done
      (loop $zero_pcm_loop
        local.get $i global.get $VORBIS_MAX_PCM_FRAMES i32.ge_u br_if $zero_pcm_done
        global.get $BSS_MDCT_SYNTH_PCM local.get $i i32.const 1 i32.shl i32.add i32.const 0 i32.store16 offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $zero_pcm_loop
      )
    )
    i32.const 0 local.set $i
    (block $zero_overlap_done
      (loop $zero_overlap_loop
        local.get $i global.get $VORBIS_MAX_OVERLAP_FRAMES i32.ge_u br_if $zero_overlap_done
        global.get $BSS_OVERLAP_Q15 local.get $i i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $zero_overlap_loop
      )
    )
    i32.const 0 local.set $packet_idx
    i32.const 0 local.set $frame_cursor
    i32.const 0 local.set $synth_nonzero
    i32.const 0 local.set $overlap_nonzero
    i32.const 0 local.set $synth_packets
    (block $publish
      (loop $packet_loop
        local.get $packet_idx local.get $packet_count i32.ge_u br_if $publish
        local.get $frame_cursor local.get $frame_count i32.ge_u br_if $publish
        local.get $packet_idx call $vorbis_build_residue_work_vector_q15
        local.tee $val
        if
          global.get $BSS_LAST_MDCT_SYNTH_STATUS local.get $val i32.store offset=0
          local.get $val return
        end
        local.get $packet_idx call $vorbis_apply_floor_curve_to_residue_q15
        local.tee $val
        if
          global.get $BSS_LAST_MDCT_SYNTH_STATUS local.get $val i32.store offset=0
          local.get $val return
        end
        global.get $BSS_PACKET_BLOCK_SIZES local.get $packet_idx i32.const 2 i32.shl i32.add i32.load offset=0
        local.tee $block_size
        i32.eqz
        if
          global.get $BSS_LAST_MDCT_SYNTH_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
          global.get $VORBIS_ERR_BAD_ARCHIVE return
        end
        local.get $block_size global.get $VORBIS_MAX_BLOCK_SIZE i32.gt_u
        if
          global.get $BSS_LAST_MDCT_SYNTH_STATUS global.get $VORBIS_ERR_TOO_MANY_SETUP i32.store offset=0
          global.get $VORBIS_ERR_TOO_MANY_SETUP return
        end
        local.get $block_size i32.const 1 i32.shr_u
        local.tee $half
        global.get $VORBIS_MAX_OVERLAP_FRAMES i32.gt_u
        if
          global.get $BSS_LAST_MDCT_SYNTH_STATUS global.get $VORBIS_ERR_TOO_MANY_SETUP i32.store offset=0
          global.get $VORBIS_ERR_TOO_MANY_SETUP return
        end
        local.get $packet_idx local.get $block_size call $vorbis_build_packet_window_q15
        local.tee $val
        if
          global.get $BSS_LAST_MDCT_SYNTH_STATUS local.get $val i32.store offset=0
          local.get $val return
        end
        local.get $block_size call $vorbis_run_owned_lapped_transform_q15
        local.tee $val
        if
          global.get $BSS_LAST_MDCT_SYNTH_STATUS local.get $val i32.store offset=0
          local.get $val return
        end
        i32.const 0 local.set $i
        (block $save_overlap
          (loop $left_loop
            local.get $i local.get $half i32.ge_u br_if $save_overlap
            local.get $frame_cursor local.get $frame_count i32.ge_u br_if $save_overlap
            global.get $BSS_MDCT_OUTPUT_Q15 local.get $i i32.const 2 i32.shl i32.add i32.load offset=0
            global.get $BSS_OVERLAP_Q15 local.get $i i32.const 2 i32.shl i32.add i32.load offset=0
            i32.add
            local.tee $val
            i32.const 32767 i32.gt_s if i32.const 32767 local.set $val end
            local.get $val i32.const -32768 i32.lt_s if i32.const -32768 local.set $val end
            global.get $BSS_MDCT_SYNTH_PCM local.get $frame_cursor i32.const 1 i32.shl i32.add local.get $val i32.store16 offset=0
            local.get $val i32.eqz if else
              local.get $synth_nonzero i32.const 1 i32.add local.set $synth_nonzero
            end
            local.get $frame_cursor i32.const 1 i32.add local.set $frame_cursor
            local.get $i i32.const 1 i32.add local.set $i
            br $left_loop
          )
        )
        i32.const 0 local.set $i
        (block $next_packet
          (loop $overlap_loop
            local.get $i local.get $half i32.ge_u br_if $next_packet
            global.get $BSS_MDCT_OUTPUT_Q15 local.get $half local.get $i i32.add i32.const 2 i32.shl i32.add i32.load offset=0
            local.tee $val
            global.get $BSS_OVERLAP_Q15 local.get $i i32.const 2 i32.shl i32.add i32.store offset=0
            local.get $val i32.eqz if else
              local.get $overlap_nonzero i32.const 1 i32.add local.set $overlap_nonzero
            end
            local.get $i i32.const 1 i32.add local.set $i
            br $overlap_loop
          )
        )
        local.get $synth_packets i32.const 1 i32.add local.set $synth_packets
        local.get $packet_idx i32.const 1 i32.add local.set $packet_idx
        br $packet_loop
      )
    )
    local.get $frame_cursor i32.eqz
    if
      global.get $BSS_LAST_MDCT_SYNTH_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
      global.get $VORBIS_ERR_BAD_ARCHIVE return
    end
    (block $publish_ready
      (loop $tail_loop
        local.get $frame_cursor local.get $frame_count i32.ge_u br_if $publish_ready
        global.get $BSS_MDCT_SYNTH_PCM local.get $frame_cursor i32.const 1 i32.shl i32.add i32.const 0 i32.store16 offset=0
        local.get $frame_cursor i32.const 1 i32.add local.set $frame_cursor
        br $tail_loop
      )
    )
    local.get $sample global.get $SS_PCM_PTR i32.add global.get $BSS_MDCT_SYNTH_PCM i64.extend_i32_u i64.store offset=0
    global.get $BSS_LAST_MDCT_SYNTH_FRAMES local.get $frame_count i32.store offset=0
    global.get $BSS_LAST_MDCT_SYNTH_PACKETS local.get $synth_packets i32.store offset=0
    global.get $BSS_LAST_MDCT_SYNTH_NONZERO local.get $synth_nonzero i32.store offset=0
    global.get $BSS_LAST_MDCT_SYNTH_OVERLAP_NONZERO local.get $overlap_nonzero i32.store offset=0
    global.get $BSS_LAST_MDCT_SYNTH_STATUS i32.const 0 i32.store offset=0
    i32.const 0
  )

  (func (export "vorbis_decode_sample_archive")
    (param $data i32) (param $len i32) (param $sample i32) (param $setup i32) (result i32)
    (local $cursor_ptr i32) (local $cursor i32) (local $val i32) (local $br i32)
    (local $i i32) (local $packet_ptr i32) (local $packet_len i32)
    (local $frame_count i32)
    local.get $sample call $vorbis_setup_init
    global.get $BSS_BITREADER local.set $br
    i32.const 0 local.set $cursor
    local.get $cursor i32.const 4 i32.add local.get $len i32.gt_u
    if global.get $VORBIS_ERR_BOUNDS return end
    local.get $data local.get $cursor call $read_u32be local.set $val
    local.get $sample global.get $SS_SAMPLE_RATE i32.add local.get $val i32.store offset=0
    local.get $cursor i32.const 4 i32.add local.set $cursor
    local.get $data local.get $cursor call $read_u32be local.set $val
    local.get $cursor i32.const 4 i32.add local.set $cursor
    local.get $val i32.const 0 i32.lt_s
    if global.get $VORBIS_ERR_BAD_ARCHIVE return end
    local.get $val global.get $VORBIS_MAX_PCM_FRAMES i32.gt_u
    if global.get $VORBIS_ERR_PCM_TOO_LARGE return end
    local.get $sample global.get $SS_FRAME_COUNT i32.add local.get $val i32.store offset=0
    local.get $data local.get $cursor call $read_u32be local.set $val
    local.get $cursor i32.const 4 i32.add local.set $cursor
    local.get $sample global.get $SS_LOOP_START i32.add local.get $val i32.store offset=0
    local.get $data local.get $cursor call $read_u32be local.set $val
    local.get $cursor i32.const 4 i32.add local.set $cursor
    local.get $val i32.const 0 i32.ge_s
    if
      local.get $sample global.get $SS_LOOP_FLAG i32.add i32.const 0 i32.store offset=0
    else
      local.get $val i32.const -1 i32.xor local.set $val
      local.get $sample global.get $SS_LOOP_FLAG i32.add i32.const 1 i32.store offset=0
    end
    local.get $sample global.get $SS_LOOP_END i32.add local.get $val i32.store offset=0
    local.get $data local.get $cursor call $read_u32be local.set $val
    local.get $cursor i32.const 4 i32.add local.set $cursor
    local.get $val i32.const 0 i32.lt_s
    if global.get $VORBIS_ERR_BAD_ARCHIVE return end
    local.get $val global.get $VORBIS_MAX_PACKETS i32.gt_u
    if global.get $VORBIS_ERR_TOO_MANY_PACKETS return end
    local.get $sample global.get $SS_PACKET_COUNT i32.add local.get $val i32.store offset=0
    i32.const 0 local.set $i
    (block $pkt_done
      (loop $pkt_loop
        local.get $i local.get $sample global.get $SS_PACKET_COUNT i32.add i32.load offset=0 i32.ge_u br_if $pkt_done
        i32.const 0 local.set $packet_len
        (block $len_done
          (loop $len_loop
            local.get $cursor local.get $len i32.ge_u if global.get $VORBIS_ERR_BOUNDS return end
            local.get $data local.get $cursor i32.add i32.load8_u local.set $val
            local.get $cursor i32.const 1 i32.add local.set $cursor
            local.get $packet_len local.get $val i32.add local.set $packet_len
            local.get $val i32.const 255 i32.eq
            br_if $len_loop
          )
        )
        local.get $sample global.get $SS_PACKET_LENS i32.add local.get $i i32.const 2 i32.shl i32.add local.get $packet_len i32.store offset=0
        local.get $data local.get $cursor i32.add local.set $packet_ptr
        local.get $sample global.get $SS_PACKET_PTRS i32.add local.get $i i32.const 3 i32.shl i32.add local.get $packet_ptr i64.extend_i32_u i64.store offset=0
        local.get $cursor local.get $packet_len i32.add local.set $cursor
        local.get $cursor local.get $len i32.gt_u
        if global.get $VORBIS_ERR_BOUNDS return end
        local.get $i i32.const 1 i32.add local.set $i
        br $pkt_loop
      )
    )
    ;; For now, return ok without decoding packet headers
    local.get $sample global.get $SS_DECODE_STATUS i32.add i32.const 0 i32.store offset=0
    local.get $sample global.get $SS_PCM_PTR i32.add i64.const 0 i64.store offset=0
    i32.const 0
  )

  (func $vorbis_publish_silence_if_all_floor_false (export "vorbis_publish_silence_if_all_floor_false")
    (param $sample i32) (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )

  (func (export "vorbis_sample_get_rawsound")
    (param $sample i32) (param $raw i32) (result i32)
    (local $ptr i32)
    local.get $sample i32.eqz
    if
      global.get $BSS_SAMPLE_STATE local.set $sample
    end
    local.get $raw if else global.get $BSS_RAWSOUND_VIEW local.set $raw end
    local.get $raw global.get $RS_SAMPLE_RATE i32.add local.get $sample global.get $SS_SAMPLE_RATE i32.add i32.load offset=0 i32.store offset=0
    local.get $raw global.get $RS_FRAME_COUNT i32.add local.get $sample global.get $SS_FRAME_COUNT i32.add i32.load offset=0 i32.store offset=0
    local.get $raw global.get $RS_LOOP_START i32.add local.get $sample global.get $SS_LOOP_START i32.add i32.load offset=0 i32.store offset=0
    local.get $raw global.get $RS_LOOP_END i32.add local.get $sample global.get $SS_LOOP_END i32.add i32.load offset=0 i32.store offset=0
    local.get $raw global.get $RS_LOOP_FLAG i32.add local.get $sample global.get $SS_LOOP_FLAG i32.add i32.load offset=0 i32.store offset=0
    local.get $raw global.get $RS_PCM_PTR i32.add local.get $sample global.get $SS_PCM_PTR i32.add i64.load offset=0 i64.store offset=0
    local.get $raw
  )

  (func (export "vorbis_sample_get_pcm")
    (param $sample i32) (result i32) (result i32) (result i32)
    local.get $sample i32.eqz
    if
      global.get $BSS_SAMPLE_STATE local.set $sample
    end
    local.get $sample global.get $SS_PCM_PTR i32.add i64.load offset=0 i32.wrap_i64
    local.get $sample global.get $SS_FRAME_COUNT i32.add i32.load offset=0
    local.get $sample global.get $SS_SAMPLE_RATE i32.add i32.load offset=0
  )

  (func $vorbis_publish_rawsound (export "vorbis_publish_rawsound")
    (param $sample i32) (result i32)
    global.get $BSS_RAWSOUND_VIEW global.get $RS_SAMPLE_RATE i32.add local.get $sample global.get $SS_SAMPLE_RATE i32.add i32.load offset=0 i32.store offset=0
    global.get $BSS_RAWSOUND_VIEW global.get $RS_FRAME_COUNT i32.add local.get $sample global.get $SS_FRAME_COUNT i32.add i32.load offset=0 i32.store offset=0
    global.get $BSS_RAWSOUND_VIEW global.get $RS_LOOP_START i32.add local.get $sample global.get $SS_LOOP_START i32.add i32.load offset=0 i32.store offset=0
    global.get $BSS_RAWSOUND_VIEW global.get $RS_LOOP_END i32.add local.get $sample global.get $SS_LOOP_END i32.add i32.load offset=0 i32.store offset=0
    global.get $BSS_RAWSOUND_VIEW global.get $RS_LOOP_FLAG i32.add local.get $sample global.get $SS_LOOP_FLAG i32.add i32.load offset=0 i32.store offset=0
    global.get $BSS_RAWSOUND_VIEW global.get $RS_PCM_PTR i32.add local.get $sample global.get $SS_PCM_PTR i32.add i64.load offset=0 i64.store offset=0
    global.get $BSS_RAWSOUND_VIEW
  )

  ;; TODO stubs
  (func (export "vorbis_todo_decode_codebooks") (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )
  (func (export "vorbis_todo_decode_floors") (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )
  (func (export "vorbis_todo_decode_residues") (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )
  (func (export "vorbis_todo_decode_mappings") (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )
  (func (export "vorbis_todo_inverse_mdct") (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )

  (data (i32.const 8) "\11\18\20\19\12\0b\04\05")
  (data (i32.const 16) "\0c\13\1a\21\28\30\29\22")
  (data (i32.const 24) "\1b\14\0d\06\07\0e\15\1c")
  (data (i32.const 32) "\23\2a\31\38\39\32\2b\24")
  (data (i32.const 40) "\1d\16\0f\17\1e\25\2c\33")
  (data (i32.const 48) "\3a\3b\34\2d\26\1f\27\2e")
  (data (i32.const 56) "\35\3c\3d\36\2f\37\3e\3f")
)
