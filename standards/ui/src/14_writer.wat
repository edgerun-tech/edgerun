
  (func $er_ui_writer_begin (export "er_ui_writer_begin") (param $base i32) (param $cap i32) (param $node_count i32) (param $root_count i32) (param $axis i32) (param $gap i32) (param $padding i32) (result i32)
    (local $records_len i32)
    local.get $node_count
    i32.eqz
    local.get $root_count
    i32.eqz
    i32.or
    local.get $root_count
    local.get $node_count
    i32.gt_u
    i32.or
    if
      i32.const 0
      return
    end
    local.get $node_count
    i32.const 16
    i32.mul
    local.set $records_len
    local.get $cap
    i32.const 20
    local.get $records_len
    i32.add
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $cap
    call $zero
    local.get $base
    i32.const 0x49755245
    call $store32
    local.get $base
    i32.const 4
    i32.add
    i32.const 0
    call $store32
    local.get $base
    i32.const 8
    i32.add
    i32.const 1
    call $store16
    local.get $base
    i32.const 10
    i32.add
    local.get $axis
    call $store16
    local.get $base
    i32.const 12
    i32.add
    local.get $gap
    call $store16
    local.get $base
    i32.const 14
    i32.add
    local.get $padding
    call $store16
    local.get $base
    i32.const 16
    i32.add
    local.get $node_count
    call $store16
    local.get $base
    i32.const 18
    i32.add
    local.get $root_count
    call $store16
    local.get $node_count
    global.set $writer_node_count
    local.get $base
    global.set $writer_base
    local.get $cap
    global.set $writer_cap
    local.get $base
    i32.const 20
    i32.add
    local.get $records_len
    i32.add
    global.set $writer_cursor
    global.get $writer_cursor)

  (func $er_ui_writer_cursor  (result i32)
    global.get $writer_cursor)

  (func $writer_store_used_len
    global.get $writer_base
    i32.const 4
    i32.add
    global.get $writer_cursor
    global.get $writer_base
    i32.sub
    call $store32)

  (func $er_ui_writer_string (export "er_ui_writer_string") (param $src i32) (param $len i32) (result i32)
    (local $offset i32)
    (local $table i32)
    local.get $len
    i32.const 65535
    i32.gt_u
    if
      i32.const 0
      return
    end
    global.get $writer_base
    global.get $writer_node_count
    call $table_start
    local.set $table
    global.get $writer_cursor
    local.get $table
    i32.sub
    local.set $offset
    local.get $offset
    i32.const 65535
    i32.gt_u
    if
      i32.const 0
      return
    end
    global.get $writer_cursor
    local.get $len
    i32.add
    global.get $writer_base
    global.get $writer_cap
    i32.add
    i32.gt_u
    if
      i32.const 0
      return
    end
    global.get $writer_cursor
    local.get $src
    local.get $len
    call $copy
    global.get $writer_cursor
    local.get $len
    i32.add
    global.set $writer_cursor
    call $writer_store_used_len
    local.get $offset
    local.get $len
    call $string_ref)

  (func $er_ui_writer_record  (param $index i32) (param $kind i32) (param $id i32) (param $first_ref i32) (param $second_ref i32) (result i32)
    (local $p i32)
    local.get $index
    global.get $writer_node_count
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $index
    call $record_ptr
    local.set $p
    local.get $p
    local.get $kind
    call $store16
    local.get $p
    i32.const 4
    i32.add
    local.get $id
    call $store32
    local.get $p
    i32.const 8
    i32.add
    local.get $first_ref
    i32.const 65535
    i32.and
    call $store16
    local.get $p
    i32.const 10
    i32.add
    local.get $first_ref
    i32.const 16
    i32.shr_u
    call $store16
    local.get $p
    i32.const 12
    i32.add
    local.get $second_ref
    i32.const 65535
    i32.and
    call $store16
    local.get $p
    i32.const 14
    i32.add
    local.get $second_ref
    i32.const 16
    i32.shr_u
    call $store16
    call $writer_store_used_len
    i32.const 1)

  (func $er_ui_writer_record_child  (param $index i32) (param $parent i32) (param $kind i32) (param $id i32) (param $first_ref i32) (param $second_ref i32) (result i32)
    (local $p i32)
    local.get $index
    global.get $writer_node_count
    i32.ge_u
    local.get $parent
    global.get $writer_node_count
    i32.ge_u
    i32.or
    local.get $index
    local.get $parent
    i32.eq
    i32.or
    if
      i32.const 0
      return
    end
    local.get $index
    local.get $kind
    local.get $id
    local.get $first_ref
    local.get $second_ref
    call $er_ui_writer_record
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $index
    call $record_ptr
    local.set $p
    local.get $p
    i32.const 2
    i32.add
    local.get $parent
    i32.const 1
    i32.add
    call $store16
    i32.const 1)
