(func $er_ui_record_set_id (export "er_ui_record_set_id") (param $base i32) (param $len i32) (param $index i32) (param $id i32) (result i32)
    (local $n i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $index
    local.get $n
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $index
    call $record_at
    i32.const 4
    i32.add
    local.get $id
    i32.store
    i32.const 1)

  (func (export "er_ui_record_set_parent") (param $base i32) (param $len i32) (param $index i32) (param $parent i32) (result i32)
    (local $n i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $index
    local.get $n
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $parent
    i32.const -1
    i32.eq
    if
      local.get $base
      local.get $index
      call $record_at
      i32.const 2
      i32.add
      i32.const 0
      call $store16
      i32.const 1
      return
    end
    local.get $parent
    local.get $n
    i32.ge_u
    local.get $parent
    local.get $index
    i32.eq
    i32.or
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $index
    call $record_at
    i32.const 2
    i32.add
    local.get $parent
    i32.const 1
    i32.add
    call $store16
    i32.const 1)

  (func $er_ui_record_set_first_ref (export "er_ui_record_set_first_ref") (param $base i32) (param $len i32) (param $index i32) (param $ref i32) (result i32)
    (local $n i32)
    (local $r i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $index
    local.get $n
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $index
    call $record_at
    local.set $r
    local.get $r
    i32.const 8
    i32.add
    local.get $ref
    i32.const 65535
    i32.and
    call $store16
    local.get $r
    i32.const 10
    i32.add
    local.get $ref
    i32.const 16
    i32.shr_u
    call $store16
    i32.const 1)

  (func $er_ui_record_set_second_ref (export "er_ui_record_set_second_ref") (param $base i32) (param $len i32) (param $index i32) (param $ref i32) (result i32)
    (local $n i32)
    (local $r i32)
    local.get $base
    local.get $len
    call $er_ui_validate
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 16
    i32.add
    call $load16
    local.set $n
    local.get $index
    local.get $n
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $base
    local.get $index
    call $record_at
    local.set $r
    local.get $r
    i32.const 12
    i32.add
    local.get $ref
    i32.const 65535
    i32.and
    call $store16
    local.get $r
    i32.const 14
    i32.add
    local.get $ref
    i32.const 16
    i32.shr_u
    call $store16
    i32.const 1)

  (func $er_ui_write_empty (export "er_ui_write_empty") (param $base i32) (param $cap i32) (param $kind i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 1
    i32.const 1
    i32.const 0
    i32.const 0
    i32.const 0
    call $er_ui_writer_begin
    i32.eqz
    if
      i32.const 0
      return
    end
    i32.const 0
    local.get $kind
    i32.const 0
    i32.const 0
    i32.const 0
    call $er_ui_writer_record
    drop
    global.get $writer_cursor)

  (func $er_ui_write_ref (export "er_ui_write_ref") (param $base i32) (param $cap i32) (param $kind i32) (param $id i32) (param $ref i32) (result i32)
    local.get $base
    local.get $cap
    i32.const 1
    i32.const 1
    i32.const 0
    i32.const 0
    i32.const 0
    call $er_ui_writer_begin
    i32.eqz
    if
      i32.const 0
      return
    end
    i32.const 0
    local.get $kind
    local.get $id
    i32.const 0
    local.get $ref
    call $er_ui_writer_record
    drop
    global.get $writer_cursor)

  (func $er_ui_write_two_strings (export "er_ui_write_two_strings") (param $base i32) (param $cap i32) (param $kind i32) (param $id i32) (param $a_ptr i32) (param $a_len i32) (param $b_ptr i32) (param $b_len i32) (result i32)
    (local $a_ref i32)
    (local $b_ref i32)
    local.get $base
    local.get $cap
    i32.const 1
    i32.const 1
    i32.const 0
    i32.const 0
    i32.const 0
    call $er_ui_writer_begin
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $a_ptr
    local.get $a_len
    call $er_ui_writer_string
    local.set $a_ref
    local.get $a_len
    i32.eqz
    i32.eqz
    local.get $a_ref
    i32.eqz
    i32.and
    if
      i32.const 0
      return
    end
    local.get $b_ptr
    local.get $b_len
    call $er_ui_writer_string
    local.set $b_ref
    local.get $b_len
    i32.eqz
    i32.eqz
    local.get $b_ref
    i32.eqz
    i32.and
    if
      i32.const 0
      return
    end
    i32.const 0
    local.get $kind
    local.get $id
    local.get $a_ref
    local.get $b_ref
    call $er_ui_writer_record
    drop
    global.get $writer_cursor)
