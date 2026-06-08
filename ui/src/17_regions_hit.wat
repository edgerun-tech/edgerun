
  (func $er_ui_region_size  (result i32)
    i32.const 28)

  (func $er_ui_regions_add  (param $base i32) (param $cap i32) (param $len i32) (param $slot i32) (param $kind i32) (param $id i32) (param $x f32) (param $y f32) (param $w f32) (param $h f32) (result i32)
    (local $p i32)
    local.get $kind
    i32.const 58
    i32.ge_u
    if
      i32.const -1
      return
    end
    local.get $w
    local.get $h
    call $valid_rect
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $len
    i32.const 28
    i32.mul
    i32.const 28
    i32.add
    local.get $cap
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $base
    local.get $len
    i32.const 28
    i32.mul
    i32.add
    local.set $p
    local.get $p
    local.get $slot
    call $store32
    local.get $p
    i32.const 4
    i32.add
    local.get $kind
    call $store32
    local.get $p
    i32.const 8
    i32.add
    local.get $id
    call $store32
    local.get $p
    i32.const 12
    i32.add
    local.get $x
    f32.store
    local.get $p
    i32.const 16
    i32.add
    local.get $y
    f32.store
    local.get $p
    i32.const 20
    i32.add
    local.get $w
    f32.store
    local.get $p
    i32.const 24
    i32.add
    local.get $h
    f32.store
    local.get $len
    i32.const 1
    i32.add)

  (func $er_ui_regions_hit_test  (param $base i32) (param $len i32) (param $x f32) (param $y f32) (param $out i32) (result i32)
    (local $i i32)
    (local $p i32)
    local.get $len
    local.set $i
    loop $loop
      local.get $i
      i32.const 0
      i32.gt_u
      if
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        local.get $base
        local.get $i
        i32.const 28
        i32.mul
        i32.add
        local.set $p
        local.get $x
        local.get $p
        i32.const 12
        i32.add
        f32.load
        f32.ge
        local.get $y
        local.get $p
        i32.const 16
        i32.add
        f32.load
        f32.ge
        i32.and
        local.get $x
        local.get $p
        i32.const 12
        i32.add
        f32.load
        local.get $p
        i32.const 20
        i32.add
        f32.load
        f32.add
        f32.lt
        i32.and
        local.get $y
        local.get $p
        i32.const 16
        i32.add
        f32.load
        local.get $p
        i32.const 24
        i32.add
        f32.load
        f32.add
        f32.lt
        i32.and
        if
          local.get $out
          i32.eqz
          i32.eqz
          if
            local.get $out
            local.get $p
            i32.const 28
            call $copy
          end
          i32.const 1
          return
        end
        br $loop
      end
    end
    i32.const 0)

  (func $er_ui_hit_id  (param $namespace_ptr i32) (param $namespace_len i32) (param $role_ptr i32) (param $role_len i32) (param $key_ptr i32) (param $key_len i32) (result i32)
    (local $hash i32)
    i32.const 0x811c9dc5
    i32.const 65000
    i32.const 17
    call $fnv1a_range
    local.set $hash
    local.get $hash
    local.get $namespace_ptr
    local.get $namespace_len
    call $fnv1a_range
    i32.const 0
    call $fnv1a_byte
    local.set $hash
    local.get $hash
    local.get $role_ptr
    local.get $role_len
    call $fnv1a_range
    i32.const 0
    call $fnv1a_byte
    local.set $hash
    local.get $hash
    local.get $key_ptr
    local.get $key_len
    call $fnv1a_range
    local.tee $hash
    i32.eqz
    if
      i32.const 1
      return
    end
    local.get $hash)

  (func $er_ui_hit_event_size  (result i32)
    i32.const 20)

  (func $er_ui_hit_event_encode  (param $out i32) (param $cap i32) (param $kind i32) (param $id i32) (param $location_tag i32) (param $action_tag i32) (param $source_tag i32) (result i32)
    local.get $cap
    i32.const 12
    i32.lt_u
    local.get $kind
    i32.const 58
    i32.ge_u
    i32.or
    local.get $location_tag
    i32.eqz
    i32.eqz
    i32.or
    local.get $action_tag
    i32.const 4
    i32.gt_u
    i32.or
    local.get $source_tag
    i32.const 4
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $kind
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $id
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $location_tag
    i32.store8
    local.get $out
    i32.const 9
    i32.add
    local.get $action_tag
    i32.store8
    local.get $out
    i32.const 10
    i32.add
    local.get $source_tag
    i32.store8
    local.get $out
    i32.const 11
    i32.add
    i32.const 0
    i32.store8
    i32.const 12)

  (func $er_ui_hit_event_encode_point  (param $out i32) (param $cap i32) (param $kind i32) (param $id i32) (param $action_tag i32) (param $source_tag i32) (param $x f32) (param $y f32) (result i32)
    local.get $cap
    i32.const 20
    i32.lt_u
    local.get $kind
    i32.const 58
    i32.ge_u
    i32.or
    local.get $action_tag
    i32.const 4
    i32.gt_u
    i32.or
    local.get $source_tag
    i32.const 4
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $kind
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $id
    i32.store
    local.get $out
    i32.const 8
    i32.add
    i32.const 1
    i32.store8
    local.get $out
    i32.const 9
    i32.add
    local.get $action_tag
    i32.store8
    local.get $out
    i32.const 10
    i32.add
    local.get $source_tag
    i32.store8
    local.get $out
    i32.const 11
    i32.add
    i32.const 0
    i32.store8
    local.get $out
    i32.const 12
    i32.add
    local.get $x
    f32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $y
    f32.store
    i32.const 20)

  (func $er_ui_hit_event_kind  (param $event i32) (param $len i32) (result i32)
    local.get $len
    i32.const 12
    i32.lt_u
    if
      i32.const -1
      return
    end
    local.get $event
    i32.load)

  (func $er_ui_hit_event_id  (param $event i32) (param $len i32) (result i32)
    local.get $len
    i32.const 12
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $event
    i32.const 4
    i32.add
    i32.load)

  (func $er_ui_hit_event_location_tag  (param $event i32) (param $len i32) (result i32)
    local.get $len
    i32.const 12
    i32.lt_u
    if
      i32.const -1
      return
    end
    local.get $event
    i32.const 8
    i32.add
    i32.load8_u)

  (func $er_ui_hit_event_action_tag  (param $event i32) (param $len i32) (result i32)
    local.get $len
    i32.const 12
    i32.lt_u
    if
      i32.const -1
      return
    end
    local.get $event
    i32.const 9
    i32.add
    i32.load8_u)

  (func $er_ui_hit_event_source_tag  (param $event i32) (param $len i32) (result i32)
    local.get $len
    i32.const 12
    i32.lt_u
    if
      i32.const -1
      return
    end
    local.get $event
    i32.const 10
    i32.add
    i32.load8_u)

  (func $er_ui_hit_event_validate  (param $event i32) (param $len i32) (result i32)
    local.get $len
    i32.const 12
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $event
    i32.load
    i32.const 58
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $event
    i32.const 8
    i32.add
    i32.load8_u
    i32.const 1
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $event
    i32.const 9
    i32.add
    i32.load8_u
    i32.const 4
    i32.gt_u
    local.get $event
    i32.const 10
    i32.add
    i32.load8_u
    i32.const 4
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $event
    i32.const 8
    i32.add
    i32.load8_u
    i32.eqz
    if (result i32)
      local.get $len
      i32.const 12
      i32.ge_u
    else
      local.get $len
      i32.const 20
      i32.ge_u
    end)

  (func $er_ui_hit_event_point_x  (param $event i32) (param $len i32) (result f32)
    local.get $len
    i32.const 20
    i32.lt_u
    local.get $event
    i32.const 8
    i32.add
    i32.load8_u
    i32.const 1
    i32.ne
    i32.or
    if
      f32.const nan
      return
    end
    local.get $event
    i32.const 12
    i32.add
    f32.load)

  (func $er_ui_hit_event_point_y  (param $event i32) (param $len i32) (result f32)
    local.get $len
    i32.const 20
    i32.lt_u
    local.get $event
    i32.const 8
    i32.add
    i32.load8_u
    i32.const 1
    i32.ne
    i32.or
    if
      f32.const nan
      return
    end
    local.get $event
    i32.const 16
    i32.add
    f32.load)
