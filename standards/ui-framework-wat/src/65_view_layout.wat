  (func (export "er_ui_view_stack_cursor_size") (result i32)
    i32.const 24)

  (func (export "er_ui_view_row_cursor_size") (result i32)
    i32.const 24)

  (func (export "er_ui_view_split_size") (result i32)
    i32.const 32)

  (func (export "er_ui_view_stack_cursor_init") (param $cursor i32) (param $bounds i32) (param $gap f32) (result i32)
    local.get $cursor
    i32.eqz
    local.get $bounds
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $cursor
    local.get $bounds
    f32.load
    local.get $bounds
    i32.const 4
    i32.add
    f32.load
    local.get $bounds
    i32.const 8
    i32.add
    f32.load
    local.get $bounds
    i32.const 12
    i32.add
    f32.load
    call $rect_store
    drop
    local.get $cursor
    i32.const 16
    i32.add
    local.get $gap
    f32.store
    local.get $cursor
    i32.const 20
    i32.add
    local.get $bounds
    i32.const 4
    i32.add
    f32.load
    f32.store
    i32.const 1)

  (func $er_ui_view_stack_cursor_take (export "er_ui_view_stack_cursor_take") (param $cursor i32) (param $height f32) (param $out i32) (result i32)
    (local $resolved_h f32)
    (local $cursor_y f32)
    local.get $cursor
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $height
    f32.const 1
    call $max_f32
    local.set $resolved_h
    local.get $cursor
    i32.const 20
    i32.add
    f32.load
    local.set $cursor_y
    local.get $out
    local.get $cursor
    f32.load
    local.get $cursor_y
    local.get $cursor
    i32.const 8
    i32.add
    f32.load
    local.get $resolved_h
    call $rect_store
    drop
    local.get $cursor
    i32.const 20
    i32.add
    local.get $cursor_y
    local.get $resolved_h
    f32.add
    local.get $cursor
    i32.const 16
    i32.add
    f32.load
    f32.add
    f32.store
    i32.const 1)

  (func (export "er_ui_view_stack_cursor_take_if_fits") (param $cursor i32) (param $height f32) (param $out i32) (result i32)
    local.get $cursor
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $cursor
    i32.const 20
    i32.add
    f32.load
    local.get $height
    f32.add
    local.get $cursor
    i32.const 4
    i32.add
    f32.load
    local.get $cursor
    i32.const 12
    i32.add
    f32.load
    f32.add
    f32.gt
    if
      i32.const 0
      return
    end
    local.get $cursor
    local.get $height
    local.get $out
    call $er_ui_view_stack_cursor_take)

  (func (export "er_ui_view_stack_cursor_skip") (param $cursor i32) (param $amount f32) (result i32)
    local.get $cursor
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $cursor
    i32.const 20
    i32.add
    local.get $cursor
    i32.const 20
    i32.add
    f32.load
    local.get $amount
    f32.add
    f32.store
    i32.const 1)

  (func (export "er_ui_view_stack_cursor_remaining") (param $cursor i32) (param $out i32) (result i32)
    local.get $cursor
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $cursor
    f32.load
    local.get $cursor
    i32.const 20
    i32.add
    f32.load
    local.get $cursor
    i32.const 8
    i32.add
    f32.load
    local.get $cursor
    i32.const 4
    i32.add
    f32.load
    local.get $cursor
    i32.const 12
    i32.add
    f32.load
    f32.add
    local.get $cursor
    i32.const 20
    i32.add
    f32.load
    f32.sub
    f32.const 1
    call $max_f32
    call $rect_store)

  (func (export "er_ui_view_row_cursor_init") (param $cursor i32) (param $bounds i32) (param $gap f32) (result i32)
    local.get $cursor
    i32.eqz
    local.get $bounds
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $cursor
    local.get $bounds
    f32.load
    local.get $bounds
    i32.const 4
    i32.add
    f32.load
    local.get $bounds
    i32.const 8
    i32.add
    f32.load
    local.get $bounds
    i32.const 12
    i32.add
    f32.load
    call $rect_store
    drop
    local.get $cursor
    i32.const 16
    i32.add
    local.get $gap
    f32.store
    local.get $cursor
    i32.const 20
    i32.add
    local.get $bounds
    f32.load
    f32.store
    i32.const 1)

  (func (export "er_ui_view_row_cursor_take") (param $cursor i32) (param $width f32) (param $out i32) (result i32)
    (local $resolved_w f32)
    (local $cursor_x f32)
    local.get $cursor
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $width
    f32.const 1
    call $max_f32
    local.set $resolved_w
    local.get $cursor
    i32.const 20
    i32.add
    f32.load
    local.set $cursor_x
    local.get $out
    local.get $cursor_x
    local.get $cursor
    i32.const 4
    i32.add
    f32.load
    local.get $resolved_w
    local.get $cursor
    i32.const 12
    i32.add
    f32.load
    call $rect_store
    drop
    local.get $cursor
    i32.const 20
    i32.add
    local.get $cursor_x
    local.get $resolved_w
    f32.add
    local.get $cursor
    i32.const 16
    i32.add
    f32.load
    f32.add
    f32.store
    i32.const 1)

  (func (export "er_ui_view_row_cursor_remaining") (param $cursor i32) (param $out i32) (result i32)
    local.get $cursor
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $out
    local.get $cursor
    i32.const 20
    i32.add
    f32.load
    local.get $cursor
    i32.const 4
    i32.add
    f32.load
    local.get $cursor
    f32.load
    local.get $cursor
    i32.const 8
    i32.add
    f32.load
    f32.add
    local.get $cursor
    i32.const 20
    i32.add
    f32.load
    f32.sub
    f32.const 1
    call $max_f32
    local.get $cursor
    i32.const 12
    i32.add
    f32.load
    call $rect_store)

  (func (export "er_ui_view_grid_item") (param $bounds i32) (param $columns i32) (param $gap f32) (param $item_h f32) (param $index i32) (param $out i32) (result i32)
    (local $columns_value i32)
    (local $col i32)
    (local $row i32)
    (local $item_w f32)
    local.get $bounds
    i32.eqz
    local.get $out
    i32.eqz
    i32.or
    if
      i32.const 0
      return
    end
    local.get $columns
    i32.eqz
    if
      i32.const 1
      local.set $columns_value
    else
      local.get $columns
      local.set $columns_value
    end
    local.get $index
    local.get $columns_value
    i32.rem_u
    local.set $col
    local.get $index
    local.get $columns_value
    i32.div_u
    local.set $row
    local.get $bounds
    i32.const 8
    i32.add
    f32.load
    local.get $gap
    local.get $columns_value
    i32.const 1
    i32.sub
    f32.convert_i32_u
    f32.mul
    f32.sub
    local.get $columns_value
    f32.convert_i32_u
    f32.div
    f32.const 1
    call $max_f32
    local.set $item_w
    local.get $out
    local.get $bounds
    f32.load
    local.get $col
    f32.convert_i32_u
    local.get $item_w
    local.get $gap
    f32.add
    f32.mul
    f32.add
    local.get $bounds
    i32.const 4
    i32.add
    f32.load
    local.get $row
    f32.convert_i32_u
    local.get $item_h
    local.get $gap
    f32.add
    f32.mul
    f32.add
    local.get $item_w
    local.get $item_h
    call $rect_store)

  (func (export "er_ui_view_grid_height") (param $columns i32) (param $gap f32) (param $item_h f32) (param $item_count i32) (result f32)
    (local $columns_value i32)
    (local $rows i32)
    local.get $item_count
    i32.eqz
    if
      f32.const 0
      return
    end
    local.get $columns
    i32.eqz
    if
      i32.const 1
      local.set $columns_value
    else
      local.get $columns
      local.set $columns_value
    end
    local.get $item_count
    local.get $columns_value
    i32.add
    i32.const 1
    i32.sub
    local.get $columns_value
    i32.div_u
    local.set $rows
    local.get $rows
    f32.convert_i32_u
    local.get $item_h
    f32.mul
    local.get $rows
    i32.const 1
    i32.sub
    f32.convert_i32_u
    local.get $gap
    f32.mul
    f32.add)

  (func (export "er_ui_view_split_left") (param $bounds i32) (param $width f32) (param $gap f32) (param $out i32) (result i32)
    (local $first_w f32)
    (local $rest_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $width f32.const 1 call $max_f32 local.get $bounds i32.const 8 i32.add f32.load call $min_f32 local.set $first_w
    local.get $bounds f32.load local.get $first_w f32.add local.get $gap f32.add local.set $rest_x
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $first_w local.get $bounds i32.const 12 i32.add f32.load call $rect_store drop
    local.get $out i32.const 16 i32.add local.get $rest_x local.get $bounds i32.const 4 i32.add f32.load local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load f32.add local.get $rest_x f32.sub f32.const 1 call $max_f32 local.get $bounds i32.const 12 i32.add f32.load call $rect_store)

  (func (export "er_ui_view_split_right") (param $bounds i32) (param $width f32) (param $gap f32) (param $out i32) (result i32)
    (local $second_w f32)
    (local $second_x f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $width f32.const 1 call $max_f32 local.get $bounds i32.const 8 i32.add f32.load call $min_f32 local.set $second_w
    local.get $bounds f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $second_w f32.sub f32.const 0 call $max_f32 f32.add local.set $second_x
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $second_x local.get $gap f32.sub local.get $bounds f32.load f32.sub f32.const 1 call $max_f32 local.get $bounds i32.const 12 i32.add f32.load call $rect_store drop
    local.get $out i32.const 16 i32.add local.get $second_x local.get $bounds i32.const 4 i32.add f32.load local.get $second_w local.get $bounds i32.const 12 i32.add f32.load call $rect_store)

  (func (export "er_ui_view_split_top") (param $bounds i32) (param $height f32) (param $gap f32) (param $out i32) (result i32)
    (local $first_h f32)
    (local $rest_y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $height f32.const 1 call $max_f32 local.get $bounds i32.const 12 i32.add f32.load call $min_f32 local.set $first_h
    local.get $bounds i32.const 4 i32.add f32.load local.get $first_h f32.add local.get $gap f32.add local.set $rest_y
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $first_h call $rect_store drop
    local.get $out i32.const 16 i32.add local.get $bounds f32.load local.get $rest_y local.get $bounds i32.const 8 i32.add f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load f32.add local.get $rest_y f32.sub f32.const 1 call $max_f32 call $rect_store)

  (func (export "er_ui_view_split_bottom") (param $bounds i32) (param $height f32) (param $gap f32) (param $out i32) (result i32)
    (local $second_h f32)
    (local $second_y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $height f32.const 1 call $max_f32 local.get $bounds i32.const 12 i32.add f32.load call $min_f32 local.set $second_h
    local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 12 i32.add f32.load local.get $second_h f32.sub f32.const 0 call $max_f32 f32.add local.set $second_y
    local.get $out local.get $bounds f32.load local.get $bounds i32.const 4 i32.add f32.load local.get $bounds i32.const 8 i32.add f32.load local.get $second_y local.get $gap f32.sub local.get $bounds i32.const 4 i32.add f32.load f32.sub f32.const 1 call $max_f32 call $rect_store drop
    local.get $out i32.const 16 i32.add local.get $bounds f32.load local.get $second_y local.get $bounds i32.const 8 i32.add f32.load local.get $second_h call $rect_store)
