(module
  (import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))
  (import "edgerun" "lo" (func $lo (param i64) (result i32)))
  (import "edgerun" "hi" (func $hi (param i64) (result i32)))
  (memory (export "memory") 1)

  ;; Status values:
  ;; 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 incomplete,
  ;; 5 too_large, 6 unaligned.

  (func $m196read_u16_le (param $ptr i32) (result i32)
    (i32.or
      (i32.load8_u (local.get $ptr))
      (i32.shl
        (i32.load8_u (i32.add (local.get $ptr) (i32.const 1)))
        (i32.const 8))))

  (func $m196read_u32_le (param $ptr i32) (result i32)
    (i32.or
      (i32.or
        (i32.load8_u (local.get $ptr))
        (i32.shl
          (i32.load8_u (i32.add (local.get $ptr) (i32.const 1)))
          (i32.const 8)))
      (i32.or
        (i32.shl
          (i32.load8_u (i32.add (local.get $ptr) (i32.const 2)))
          (i32.const 16))
        (i32.shl
          (i32.load8_u (i32.add (local.get $ptr) (i32.const 3)))
          (i32.const 24)))))

  (func $write_u16_le (param $ptr i32) (param $v i32)
    (i32.store8 (local.get $ptr) (local.get $v))
    (i32.store8
      (i32.add (local.get $ptr) (i32.const 1))
      (i32.shr_u (local.get $v) (i32.const 8))))

  (func $write_u32_le (param $ptr i32) (param $v i32)
    (i32.store8 (local.get $ptr) (local.get $v))
    (i32.store8
      (i32.add (local.get $ptr) (i32.const 1))
      (i32.shr_u (local.get $v) (i32.const 8)))
    (i32.store8
      (i32.add (local.get $ptr) (i32.const 2))
      (i32.shr_u (local.get $v) (i32.const 16)))
    (i32.store8
      (i32.add (local.get $ptr) (i32.const 3))
      (i32.shr_u (local.get $v) (i32.const 24))))

  (func $wayland_align4 (export "wayland_align4") (param $n i32) (result i32)
    (i32.and
      (i32.add (local.get $n) (i32.const 3))
      (i32.const -4)))

  (func $wayland_object_id_status (export "wayland_object_id_status")
    (param $id i32) (param $nullable i32)
    (result i32)
    (if (i32.eqz (local.get $id))
      (then
        (return
          (select
            (i32.const 0)
            (i32.const 3)
            (i32.ne (local.get $nullable) (i32.const 0))))))
    (i32.const 0))

  (func $wayland_message_size_status (export "wayland_message_size_status")
    (param $available_len i32) (param $declared_size i32)
    (result i32)
    (if (i32.lt_u (local.get $available_len) (i32.const 8))
      (then (return (i32.const 1))))
    (if (i32.lt_u (local.get $declared_size) (i32.const 8))
      (then (return (i32.const 3))))
    (if (i32.gt_u (local.get $declared_size) (i32.const 65536))
      (then (return (i32.const 5))))
    (if (i32.gt_u (local.get $declared_size) (local.get $available_len))
      (then (return (i32.const 4))))
    (if (i32.ne (i32.and (local.get $declared_size) (i32.const 3)) (i32.const 0))
      (then (return (i32.const 6))))
    (i32.const 0))

  ;; Wayland wire header: u32 object id, u16 opcode, u16 message size, all little-endian.
  ;; Output record:
  ;; 0:object_id, 4:opcode, 8:size, 12:args_len, 16:size_opcode_word,
  ;; 20:object_status, 24:size_status, 28:aligned.
  (func (export "wayland_header_decode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32)
    (result i32)
    (local $object_id i32)
    (local $opcode i32)
    (local $size i32)
    (local $status i32)
    (if (i32.lt_u (local.get $in_len) (i32.const 8))
      (then (return (i32.const 1))))
    (local.set $object_id (call $m196read_u32_le (local.get $in_ptr)))
    (local.set $opcode (call $m196read_u16_le (i32.add (local.get $in_ptr) (i32.const 4))))
    (local.set $size (call $m196read_u16_le (i32.add (local.get $in_ptr) (i32.const 6))))
    (local.set $status
      (call $wayland_message_size_status
        (local.get $in_len)
        (local.get $size)))
    (if
      (i32.and
        (i32.eq (local.get $status) (i32.const 0))
        (i32.ne
          (call $wayland_object_id_status (local.get $object_id) (i32.const 0))
          (i32.const 0)))
      (then (local.set $status (i32.const 3))))
    (i32.store (local.get $out_ptr) (local.get $object_id))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $opcode))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $size))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 12))
      (select
        (i32.sub (local.get $size) (i32.const 8))
        (i32.const 0)
        (i32.ge_u (local.get $size) (i32.const 8))))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 16))
      (i32.or
        (i32.and (local.get $opcode) (i32.const 65535))
        (i32.shl (local.get $size) (i32.const 16))))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 20))
      (call $wayland_object_id_status (local.get $object_id) (i32.const 0)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 24)) (local.get $status))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 28))
      (i32.eq (i32.and (local.get $size) (i32.const 3)) (i32.const 0)))
    (local.get $status))

  ;; Return bits: low32=status, high32=written.
  (func (export "wayland_header_encode")
    (param $object_id i32) (param $opcode i32) (param $size i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $status i32)
    (if (i32.lt_u (local.get $out_cap) (i32.const 8))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (if
      (i32.or
        (i32.ne (call $wayland_object_id_status (local.get $object_id) (i32.const 0)) (i32.const 0))
        (i32.gt_u (local.get $opcode) (i32.const 65535)))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (local.set $status
      (call $wayland_message_size_status (local.get $size) (local.get $size)))
    (if (i32.ne (local.get $status) (i32.const 0))
      (then (return (call $pack (local.get $status) (i32.const 0)))))
    (call $write_u32_le (local.get $out_ptr) (local.get $object_id))
    (call $write_u16_le (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $opcode))
    (call $write_u16_le (i32.add (local.get $out_ptr) (i32.const 6)) (local.get $size))
    (call $pack (i32.const 0) (i32.const 8)))

  ;; Class IDs for compact runners and generated callers:
  ;; 1 wl_display, 2 wl_registry, 3 wl_callback, 4 wl_seat, 5 wl_keyboard,
  ;; 6 wl_pointer, 7 wl_touch, 8 wl_data_device_manager, 9 wl_data_source,
  ;; 10 wl_data_offer, 11 wl_data_device, 12 zwp_text_input_manager_v3,
  ;; 13 zwp_text_input_v3.
  (func $wayland_interface_classify (export "wayland_interface_classify") (param $interface_id i32) (result i32)
    (if (i32.and (i32.ge_u (local.get $interface_id) (i32.const 1)) (i32.le_u (local.get $interface_id) (i32.const 13)))
      (then (return (local.get $interface_id))))
    (i32.const 0))

  (func $max_opcode (param $interface_class i32) (param $direction i32) (result i32)
    (if (i32.eq (local.get $interface_class) (i32.const 1))
      (then (return (i32.const 1)))) ;; wl_display requests/events
    (if (i32.eq (local.get $interface_class) (i32.const 2))
      (then (return (i32.const 1)))) ;; wl_registry requests/events
    (if (i32.eq (local.get $interface_class) (i32.const 3))
      (then
        (return
          (select (i32.const 0) (i32.const -1) (i32.eq (local.get $direction) (i32.const 1))))))
    (if (i32.eq (local.get $interface_class) (i32.const 4))
      (then
        (return
          (select (i32.const 1) (i32.const 3) (i32.eq (local.get $direction) (i32.const 1))))))
    (if (i32.eq (local.get $interface_class) (i32.const 5))
      (then
        (return
          (select (i32.const 5) (i32.const 0) (i32.eq (local.get $direction) (i32.const 1))))))
    (if (i32.eq (local.get $interface_class) (i32.const 6))
      (then
        (return
          (select (i32.const 8) (i32.const 1) (i32.eq (local.get $direction) (i32.const 1))))))
    (if (i32.eq (local.get $interface_class) (i32.const 7))
      (then
        (return
          (select (i32.const 6) (i32.const 0) (i32.eq (local.get $direction) (i32.const 1))))))
    (if (i32.eq (local.get $interface_class) (i32.const 8))
      (then
        (return
          (select (i32.const -1) (i32.const 2) (i32.eq (local.get $direction) (i32.const 1))))))
    (if (i32.eq (local.get $interface_class) (i32.const 9))
      (then
        (return
          (select (i32.const 4) (i32.const 2) (i32.eq (local.get $direction) (i32.const 1))))))
    (if (i32.eq (local.get $interface_class) (i32.const 10))
      (then
        (return
          (select (i32.const 1) (i32.const 4) (i32.eq (local.get $direction) (i32.const 1))))))
    (if (i32.eq (local.get $interface_class) (i32.const 11))
      (then
        (return
          (select (i32.const 5) (i32.const 2) (i32.eq (local.get $direction) (i32.const 1))))))
    (if (i32.eq (local.get $interface_class) (i32.const 12))
      (then
        (return
          (select (i32.const -1) (i32.const 1) (i32.eq (local.get $direction) (i32.const 1))))))
    (if (i32.eq (local.get $interface_class) (i32.const 13))
      (then
        (return
          (select (i32.const 5) (i32.const 6) (i32.eq (local.get $direction) (i32.const 1))))))
    (i32.const -1))

  ;; direction: 0 request, 1 event. Returns 0 for unknown; otherwise a stable class word.
  (func $wayland_message_classify (export "wayland_message_classify")
    (param $interface_class i32) (param $direction i32) (param $opcode i32)
    (result i32)
    (local $max i32)
    (if (i32.gt_u (local.get $direction) (i32.const 1))
      (then (return (i32.const 0))))
    (local.set $max (call $max_opcode (local.get $interface_class) (local.get $direction)))
    (if
      (i32.or
        (i32.lt_s (local.get $max) (i32.const 0))
        (i32.gt_u (local.get $opcode) (local.get $max)))
      (then (return (i32.const 0))))
    (i32.or
      (i32.shl (local.get $interface_class) (i32.const 16))
      (i32.or
        (i32.shl (local.get $direction) (i32.const 8))
        (local.get $opcode))))

  (func $wayland_message_status (export "wayland_message_status")
    (param $interface_class i32) (param $direction i32) (param $opcode i32)
    (result i32)
    (select
      (i32.const 0)
      (i32.const 3)
      (i32.ne
        (call $wayland_message_classify
          (local.get $interface_class)
          (local.get $direction)
          (local.get $opcode))
        (i32.const 0))))

  (func $wayland_seat_capabilities_status (export "wayland_seat_capabilities_status") (param $caps i32) (result i32)
    (select
      (i32.const 0)
      (i32.const 3)
      (i32.eqz (i32.and (local.get $caps) (i32.const -8)))))

  ;; Returns bit0 pointer, bit1 keyboard, bit2 touch, or zero for invalid extra bits.
  (func (export "wayland_seat_capabilities_classify") (param $caps i32) (result i32)
    (if (i32.ne (call $wayland_seat_capabilities_status (local.get $caps)) (i32.const 0))
      (then (return (i32.const 0))))
    (local.get $caps))

  (func $wayland_key_state_status (export "wayland_key_state_status") (param $state i32) (result i32)
    (select (i32.const 0) (i32.const 3) (i32.le_u (local.get $state) (i32.const 1))))

  (func (export "wayland_button_state_status") (param $state i32) (result i32)
    (call $wayland_key_state_status (local.get $state)))

  (func (export "wayland_axis_source_status") (param $source i32) (result i32)
    (select (i32.const 0) (i32.const 3) (i32.le_u (local.get $source) (i32.const 3))))

  (func (export "wayland_dnd_action_status") (param $actions i32) (result i32)
    (select
      (i32.const 0)
      (i32.const 3)
      (i32.eqz (i32.and (local.get $actions) (i32.const -8)))))

  (func (export "wayland_dnd_preferred_action_status") (param $action i32) (result i32)
    (select
      (i32.const 0)
      (i32.const 3)
      (i32.or
        (i32.eq (local.get $action) (i32.const 0))
        (i32.or
          (i32.eq (local.get $action) (i32.const 1))
          (i32.or
            (i32.eq (local.get $action) (i32.const 2))
            (i32.eq (local.get $action) (i32.const 4)))))))

  (func (export "wayland_text_change_cause_status") (param $cause i32) (result i32)
    (select (i32.const 0) (i32.const 3) (i32.le_u (local.get $cause) (i32.const 1))))

  (func (export "wayland_string_arg_status")
    (param $arg_ptr i32) (param $arg_len i32) (param $offset i32)
    (result i32)
    (local $len i32)
    (local $end i32)
    (if (i32.gt_u (i32.add (local.get $offset) (i32.const 4)) (local.get $arg_len))
      (then (return (i32.const 1))))
    (local.set $len (call $m196read_u32_le (i32.add (local.get $arg_ptr) (local.get $offset))))
    (if (i32.eqz (local.get $len))
      (then (return (i32.const 0))))
    (local.set $end
      (call $wayland_align4
        (i32.add
          (i32.add (local.get $offset) (i32.const 4))
          (local.get $len))))
    (if (i32.gt_u (local.get $end) (local.get $arg_len))
      (then (return (i32.const 1))))
    (if
      (i32.ne
        (i32.load8_u
          (i32.sub
            (i32.add
              (i32.add (local.get $arg_ptr) (local.get $offset))
              (i32.add (i32.const 4) (local.get $len)))
            (i32.const 1)))
        (i32.const 0))
      (then (return (i32.const 3))))
    (i32.const 0))

  (func (export "wayland_array_arg_status")
    (param $arg_ptr i32) (param $arg_len i32) (param $offset i32)
    (result i32)
    (local $len i32)
    (local $end i32)
    (if (i32.gt_u (i32.add (local.get $offset) (i32.const 4)) (local.get $arg_len))
      (then (return (i32.const 1))))
    (local.set $len (call $m196read_u32_le (i32.add (local.get $arg_ptr) (local.get $offset))))
    (local.set $end
      (call $wayland_align4
        (i32.add
          (i32.add (local.get $offset) (i32.const 4))
          (local.get $len))))
    (select
      (i32.const 0)
      (i32.const 1)
      (i32.le_u (local.get $end) (local.get $arg_len))))
)
