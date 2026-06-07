(func $svg_norm_x (param $x f32) (param $min_x f32) (param $vw f32) (result f32)
    local.get $x local.get $min_x f32.sub local.get $vw f32.div)
  (func $svg_norm_y (param $y f32) (param $min_y f32) (param $vh f32) (result f32)
    local.get $y local.get $min_y f32.sub local.get $vh f32.div)
  (func $svg_tx_norm_x (param $x f32) (param $y f32) (param $min_x f32) (param $vw f32) (param $a f32) (param $c f32) (param $tx f32) (result f32)
    local.get $a local.get $x f32.mul
    local.get $c local.get $y f32.mul
    f32.add
    local.get $tx
    f32.add
    local.get $min_x
    f32.sub
    local.get $vw
    f32.div)
  (func $svg_tx_norm_y (param $x f32) (param $y f32) (param $min_y f32) (param $vh f32) (param $b f32) (param $d f32) (param $ty f32) (result f32)
    local.get $b local.get $x f32.mul
    local.get $d local.get $y f32.mul
    f32.add
    local.get $ty
    f32.add
    local.get $min_y
    f32.sub
    local.get $vh
    f32.div)

  (func $er_ui_svg_path_parse_to_ir_transform_impl (export "er_ui_svg_path_parse_to_ir_transform") (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (param $min_x f32) (param $min_y f32) (param $vw f32) (param $vh f32) (param $a f32) (param $b f32) (param $c f32) (param $d f32) (param $tx f32) (param $ty f32) (result i32)
    (local $idx i32) (local $idx_slot i32) (local $cmd i32) (local $ch i32) (local $count i32) (local $started i32)
    (local $has_prev_c i32) (local $has_prev_q i32)
    (local $cx f32) (local $cy f32) (local $sx f32) (local $sy f32) (local $x f32) (local $y f32)
    (local $c0x f32) (local $c0y f32) (local $c1x f32) (local $c1y f32) (local $prev_cx f32) (local $prev_cy f32) (local $prev_qx f32) (local $prev_qy f32)
    (local $rx f32) (local $ry f32) (local $rot f32) (local $large f32) (local $sweep f32)
    (local $arc_rx f32) (local $arc_ry f32) (local $arc_sweep f32)
    local.get $ptr i32.eqz
    local.get $out i32.eqz i32.or
    local.get $vw f32.const 0 f32.le i32.or
    local.get $vh f32.const 0 f32.le i32.or
    if i32.const -1 return end
    i32.const 65520
    local.set $idx_slot
    block $done
      loop $loop
        local.get $ptr local.get $len local.get $idx call $svg_skip_separators local.set $idx
        local.get $idx local.get $len i32.ge_u br_if $done
        local.get $ptr local.get $idx i32.add i32.load8_u local.tee $ch call $svg_is_command
        if
          local.get $ch local.set $cmd
          local.get $idx i32.const 1 i32.add local.set $idx
        end
        local.get $cmd i32.eqz
        if i32.const -1 return end
        local.get $started
        i32.eqz
        local.get $cmd
        i32.const 77
        i32.ne
        local.get $cmd
        i32.const 109
        i32.ne
        i32.and
        i32.and
        if i32.const -1 return end
        local.get $idx_slot local.get $idx i32.store
        local.get $cmd i32.const 77 i32.eq
        local.get $cmd i32.const 109 i32.eq i32.or
        if
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $x
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $y
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          i32.const 1 local.set $started
          local.get $cmd i32.const 109 i32.eq
          if
            local.get $cx local.get $x f32.add local.set $x
            local.get $cy local.get $y f32.add local.set $y
          end
          local.get $x local.set $cx local.get $y local.set $cy local.get $x local.set $sx local.get $y local.set $sy
          i32.const 0 local.set $has_prev_c
          i32.const 0 local.set $has_prev_q
          local.get $out local.get $cap local.get $count f32.const 6
            local.get $x local.get $y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $x local.get $y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          local.get $cmd i32.const 109 i32.eq if i32.const 108 local.set $cmd else i32.const 76 local.set $cmd end
          br $loop
        end
        local.get $cmd i32.const 76 i32.eq
        local.get $cmd i32.const 108 i32.eq i32.or
        if
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $x
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $y
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          local.get $cmd i32.const 108 i32.eq if local.get $cx local.get $x f32.add local.set $x local.get $cy local.get $y f32.add local.set $y end
          local.get $x local.set $cx local.get $y local.set $cy
          i32.const 0 local.set $has_prev_c
          i32.const 0 local.set $has_prev_q
          local.get $out local.get $cap local.get $count f32.const 7
            local.get $x local.get $y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $x local.get $y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          br $loop
        end
        local.get $cmd i32.const 72 i32.eq
        local.get $cmd i32.const 104 i32.eq i32.or
        if
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $x
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          local.get $cmd i32.const 104 i32.eq if local.get $cx local.get $x f32.add local.set $x end
          local.get $x local.set $cx
          i32.const 0 local.set $has_prev_c
          i32.const 0 local.set $has_prev_q
          local.get $out local.get $cap local.get $count f32.const 7
            local.get $cx local.get $cy local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $cx local.get $cy local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          br $loop
        end
        local.get $cmd i32.const 86 i32.eq
        local.get $cmd i32.const 118 i32.eq i32.or
        if
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $y
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          local.get $cmd i32.const 118 i32.eq if local.get $cy local.get $y f32.add local.set $y end
          local.get $y local.set $cy
          i32.const 0 local.set $has_prev_c
          i32.const 0 local.set $has_prev_q
          local.get $out local.get $cap local.get $count f32.const 7
            local.get $cx local.get $cy local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $cx local.get $cy local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          br $loop
        end
        local.get $cmd i32.const 67 i32.eq
        local.get $cmd i32.const 99 i32.eq i32.or
        if
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $c0x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $c0y
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $c1x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $c1y
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $y
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          local.get $cmd i32.const 99 i32.eq
          if
            local.get $cx local.get $c0x f32.add local.set $c0x
            local.get $cy local.get $c0y f32.add local.set $c0y
            local.get $cx local.get $c1x f32.add local.set $c1x
            local.get $cy local.get $c1y f32.add local.set $c1y
            local.get $cx local.get $x f32.add local.set $x
            local.get $cy local.get $y f32.add local.set $y
          end
          local.get $c1x local.set $prev_cx
          local.get $c1y local.set $prev_cy
          i32.const 1 local.set $has_prev_c
          i32.const 0 local.set $has_prev_q
          local.get $x local.set $cx local.get $y local.set $cy
          local.get $out local.get $cap local.get $count f32.const 9
            local.get $c0x local.get $c0y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $c0x local.get $c0y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            local.get $c1x local.get $c1y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $c1x local.get $c1y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            local.get $x local.get $y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $x local.get $y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op7 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          br $loop
        end
        local.get $cmd i32.const 83 i32.eq
        local.get $cmd i32.const 115 i32.eq i32.or
        if
          local.get $has_prev_c
          if
            local.get $cx f32.const 2 f32.mul local.get $prev_cx f32.sub local.set $c0x
            local.get $cy f32.const 2 f32.mul local.get $prev_cy f32.sub local.set $c0y
          else
            local.get $cx local.set $c0x
            local.get $cy local.set $c0y
          end
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $c1x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $c1y
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $y
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          local.get $cmd i32.const 115 i32.eq
          if
            local.get $cx local.get $c1x f32.add local.set $c1x
            local.get $cy local.get $c1y f32.add local.set $c1y
            local.get $cx local.get $x f32.add local.set $x
            local.get $cy local.get $y f32.add local.set $y
          end
          local.get $c1x local.set $prev_cx
          local.get $c1y local.set $prev_cy
          i32.const 1 local.set $has_prev_c
          i32.const 0 local.set $has_prev_q
          local.get $x local.set $cx local.get $y local.set $cy
          local.get $out local.get $cap local.get $count f32.const 9
            local.get $c0x local.get $c0y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $c0x local.get $c0y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            local.get $c1x local.get $c1y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $c1x local.get $c1y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            local.get $x local.get $y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $x local.get $y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op7 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          br $loop
        end
        local.get $cmd i32.const 81 i32.eq
        local.get $cmd i32.const 113 i32.eq i32.or
        if
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $c0x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $c0y
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $y
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          local.get $cmd i32.const 113 i32.eq
          if
            local.get $cx local.get $c0x f32.add local.set $c0x
            local.get $cy local.get $c0y f32.add local.set $c0y
            local.get $cx local.get $x f32.add local.set $x
            local.get $cy local.get $y f32.add local.set $y
          end
          local.get $c0x local.set $prev_qx
          local.get $c0y local.set $prev_qy
          i32.const 0 local.set $has_prev_c
          i32.const 1 local.set $has_prev_q
          local.get $x local.set $cx local.get $y local.set $cy
          local.get $out local.get $cap local.get $count f32.const 8
            local.get $c0x local.get $c0y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $c0x local.get $c0y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            local.get $x local.get $y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $x local.get $y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op5 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          br $loop
        end
        local.get $cmd i32.const 84 i32.eq
        local.get $cmd i32.const 116 i32.eq i32.or
        if
          local.get $has_prev_q
          if
            local.get $cx f32.const 2 f32.mul local.get $prev_qx f32.sub local.set $c0x
            local.get $cy f32.const 2 f32.mul local.get $prev_qy f32.sub local.set $c0y
          else
            local.get $cx local.set $c0x
            local.get $cy local.set $c0y
          end
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $y
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          local.get $cmd i32.const 116 i32.eq
          if
            local.get $cx local.get $x f32.add local.set $x
            local.get $cy local.get $y f32.add local.set $y
          end
          local.get $c0x local.set $prev_qx
          local.get $c0y local.set $prev_qy
          i32.const 0 local.set $has_prev_c
          i32.const 1 local.set $has_prev_q
          local.get $x local.set $cx local.get $y local.set $cy
          local.get $out local.get $cap local.get $count f32.const 8
            local.get $c0x local.get $c0y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $c0x local.get $c0y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            local.get $x local.get $y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $x local.get $y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op5 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          br $loop
        end
        local.get $cmd i32.const 65 i32.eq
        local.get $cmd i32.const 97 i32.eq i32.or
        if
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number f32.abs local.set $rx
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number f32.abs local.set $ry
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $rot
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_flag local.set $large
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_flag local.set $sweep
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $x
          local.get $ptr local.get $len local.get $idx_slot call $svg_parse_number local.set $y
          local.get $idx_slot i32.load i32.const -1 i32.eq if i32.const -1 return end
          local.get $idx_slot i32.load local.set $idx
          local.get $cmd i32.const 97 i32.eq
          if
            local.get $cx local.get $x f32.add local.set $x
            local.get $cy local.get $y f32.add local.set $y
          end
          i32.const 0 local.set $has_prev_c
          i32.const 0 local.set $has_prev_q
          local.get $rx f32.const 0.00001 f32.le
          local.get $ry f32.const 0.00001 f32.le
          i32.or
          if
            local.get $x local.set $cx local.get $y local.set $cy
            local.get $out local.get $cap local.get $count f32.const 7
              local.get $x local.get $y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
              local.get $x local.get $y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
              call $svg_write_op3 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
            br $loop
          end
          local.get $x local.set $cx local.get $y local.set $cy
          local.get $rx
          local.get $a local.get $a f32.mul
          local.get $b local.get $b f32.mul
          f32.add
          f32.sqrt
          f32.mul
          local.set $arc_rx
          local.get $ry
          local.get $c local.get $c f32.mul
          local.get $d local.get $d f32.mul
          f32.add
          f32.sqrt
          f32.mul
          local.set $arc_ry
          local.get $sweep local.set $arc_sweep
          local.get $a local.get $d f32.mul
          local.get $b local.get $c f32.mul
          f32.sub
          f32.const 0
          f32.lt
          if
            f32.const 1
            local.get $sweep
            f32.sub
            local.set $arc_sweep
          end
          local.get $out local.get $cap local.get $count f32.const 10
            local.get $arc_rx local.get $vw f32.div
            local.get $arc_ry local.get $vh f32.div
            local.get $rot
            local.get $large
            local.get $arc_sweep
            local.get $x local.get $y local.get $min_x local.get $vw local.get $a local.get $c local.get $tx call $svg_tx_norm_x
            local.get $x local.get $y local.get $min_y local.get $vh local.get $b local.get $d local.get $ty call $svg_tx_norm_y
            call $svg_write_op8 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          br $loop
        end
        local.get $cmd i32.const 90 i32.eq
        local.get $cmd i32.const 122 i32.eq i32.or
        if
          local.get $sx local.set $cx local.get $sy local.set $cy
          i32.const 0 local.set $has_prev_c
          i32.const 0 local.set $has_prev_q
          local.get $out local.get $cap local.get $count f32.const 11 call $svg_write_op1 local.tee $count i32.const -1 i32.eq if i32.const -1 return end
          i32.const 0 local.set $cmd
          br $loop
        end
        i32.const -1
        return
      end
    end
    local.get $count)

  (func (export "er_ui_svg_path_parse_to_ir") (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (param $min_x f32) (param $min_y f32) (param $vw f32) (param $vh f32) (result i32)
    local.get $ptr
    local.get $len
    local.get $out
    local.get $cap
    local.get $min_x
    local.get $min_y
    local.get $vw
    local.get $vh
    f32.const 1
    f32.const 0
    f32.const 0
    f32.const 1
    f32.const 0
    f32.const 0
    call $er_ui_svg_path_parse_to_ir_transform_impl)
