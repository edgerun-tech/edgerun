  (func $er_ui_command_hit_test (export "er_ui_command_hit_test") (param $commands i32) (param $count i32) (param $x f32) (param $y f32) (result i32)
    (local $i i32)
    (local $p i32)
    (local $hit i32)
    (local $rx f32)
    (local $ry f32)
    (local $rw f32)
    (local $rh f32)
    i32.const -1
    local.set $hit
    loop $loop
      local.get $i
      local.get $count
      i32.lt_u
      if
        local.get $commands
        local.get $i
        i32.const 48
        i32.mul
        i32.add
        local.set $p
        local.get $p
        i32.load
        i32.const 0
        i32.ne
        if
          local.get $p
          i32.const 8
          i32.add
          f32.load
          local.set $rx
          local.get $p
          i32.const 12
          i32.add
          f32.load
          local.set $ry
          local.get $p
          i32.const 16
          i32.add
          f32.load
          local.set $rw
          local.get $p
          i32.const 20
          i32.add
          f32.load
          local.set $rh
          local.get $x
          local.get $rx
          f32.ge
          local.get $y
          local.get $ry
          f32.ge
          i32.and
          local.get $x
          local.get $rx
          local.get $rw
          f32.add
          f32.lt
          i32.and
          local.get $y
          local.get $ry
          local.get $rh
          f32.add
          f32.lt
          i32.and
          if
            local.get $i
            local.set $hit
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $hit)

  (func (export "er_ui_command_hit_id") (param $commands i32) (param $count i32) (param $x f32) (param $y f32) (result i32)
    (local $index i32)
    local.get $commands
    local.get $count
    local.get $x
    local.get $y
    call $er_ui_command_hit_test
    local.tee $index
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_owner_id_at)

  (func $er_ui_command_tag (export "er_ui_command_tag") (param $command i32) (result i32)
    local.get $command
    i32.load)

  (func $er_ui_command_color (export "er_ui_command_color") (param $command i32) (result i32)
    local.get $command
    i32.const 4
    i32.add
    i32.load)

  (func $er_ui_command_x (export "er_ui_command_x") (param $command i32) (result f32)
    local.get $command
    i32.const 8
    i32.add
    f32.load)

  (func $er_ui_command_y (export "er_ui_command_y") (param $command i32) (result f32)
    local.get $command
    i32.const 12
    i32.add
    f32.load)

  (func $er_ui_command_w (export "er_ui_command_w") (param $command i32) (result f32)
    local.get $command
    i32.const 16
    i32.add
    f32.load)

  (func $er_ui_command_h (export "er_ui_command_h") (param $command i32) (result f32)
    local.get $command
    i32.const 20
    i32.add
    f32.load)

  (func $er_ui_command_text_ptr (export "er_ui_command_text_ptr") (param $command i32) (result i32)
    local.get $command
    i32.const 24
    i32.add
    i32.load)

  (func $er_ui_command_text_len (export "er_ui_command_text_len") (param $command i32) (result i32)
    local.get $command
    i32.const 28
    i32.add
    i32.load)

  (func $er_ui_command_owner_id (export "er_ui_command_owner_id") (param $command i32) (result i32)
    local.get $command
    i32.const 32
    i32.add
    i32.load)

  (func $er_ui_command_owner_kind (export "er_ui_command_owner_kind") (param $command i32) (result i32)
    local.get $command
    i32.const 36
    i32.add
    i32.load)

  (func $er_ui_command_role (export "er_ui_command_role") (param $command i32) (result i32)
    local.get $command
    i32.const 40
    i32.add
    i32.load)

  (func $er_ui_command_meta (export "er_ui_command_meta") (param $command i32) (result i32)
    local.get $command
    i32.const 44
    i32.add
    i32.load)

  (func (export "er_ui_command_set_owner") (param $command i32) (param $id i32) (param $kind i32) (param $role i32) (result i32)
    local.get $command
    i32.const 32
    i32.add
    local.get $id
    call $store32
    local.get $command
    i32.const 36
    i32.add
    local.get $kind
    call $store32
    local.get $command
    i32.const 40
    i32.add
    local.get $role
    call $store32
    i32.const 1)

  (func (export "er_ui_command_set_meta") (param $command i32) (param $meta i32) (result i32)
    local.get $command
    i32.const 44
    i32.add
    local.get $meta
    call $store32
    i32.const 1)

  (func $er_ui_command_ptr (export "er_ui_command_ptr") (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $index
    local.get $count
    i32.ge_u
    if
      i32.const -1
      return
    end
    local.get $commands
    local.get $index
    i32.const 48
    i32.mul
    i32.add)

  (func (export "er_ui_command_valid_at") (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $index
    local.get $count
    i32.lt_u)

  (func (export "er_ui_command_copy") (param $commands i32) (param $count i32) (param $index i32) (param $out i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $commands
    i32.const 48
    call $copy
    i32.const 1)

  (func (export "er_ui_command_tag_at") (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    call $er_ui_command_tag)

  (func (export "er_ui_command_color_at") (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    call $er_ui_command_color)

  (func (export "er_ui_command_x_at") (param $commands i32) (param $count i32) (param $index i32) (result f32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      f32.const nan
      return
    end
    local.get $commands
    call $er_ui_command_x)

  (func (export "er_ui_command_y_at") (param $commands i32) (param $count i32) (param $index i32) (result f32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      f32.const nan
      return
    end
    local.get $commands
    call $er_ui_command_y)

  (func (export "er_ui_command_w_at") (param $commands i32) (param $count i32) (param $index i32) (result f32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      f32.const nan
      return
    end
    local.get $commands
    call $er_ui_command_w)

  (func (export "er_ui_command_h_at") (param $commands i32) (param $count i32) (param $index i32) (result f32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      f32.const nan
      return
    end
    local.get $commands
    call $er_ui_command_h)

  (func (export "er_ui_command_text_ptr_at") (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    call $er_ui_command_text_ptr)

  (func (export "er_ui_command_text_len_at") (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    call $er_ui_command_text_len)

  (func $er_ui_command_owner_id_at (export "er_ui_command_owner_id_at") (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    call $er_ui_command_owner_id)

  (func (export "er_ui_command_owner_kind_at") (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    call $er_ui_command_owner_kind)

  (func (export "er_ui_command_role_at") (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    call $er_ui_command_role)

  (func (export "er_ui_command_meta_at") (param $commands i32) (param $count i32) (param $index i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const -1
      return
    end
    local.get $commands
    call $er_ui_command_meta)

  (func (export "er_ui_command_set_owner_at") (param $commands i32) (param $count i32) (param $index i32) (param $id i32) (param $kind i32) (param $role i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const 0
      return
    end
    local.get $commands
    i32.const 32
    i32.add
    local.get $id
    call $store32
    local.get $commands
    i32.const 36
    i32.add
    local.get $kind
    call $store32
    local.get $commands
    i32.const 40
    i32.add
    local.get $role
    call $store32
    i32.const 1)

  (func (export "er_ui_command_set_meta_at") (param $commands i32) (param $count i32) (param $index i32) (param $meta i32) (result i32)
    local.get $commands
    local.get $count
    local.get $index
    call $er_ui_command_ptr
    local.tee $commands
    i32.const -1
    i32.eq
    if
      i32.const 0
      return
    end
    local.get $commands
    i32.const 44
    i32.add
    local.get $meta
    call $store32
    i32.const 1)

