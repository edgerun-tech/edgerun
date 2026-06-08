;; ESP32-S3 TCL AP/display bridge semantics plundered from devices/edgerun-tcl-bridge-esp32s3.

  (func (export "tcl_bridge_abi_version") (result i32) i32.const 1)
  (func (export "lcd_width") (result i32) i32.const 320)
  (func (export "lcd_height") (result i32) i32.const 480)
  (func (export "font_char_w") (result i32) i32.const 5)
  (func (export "font_char_h") (result i32) i32.const 7)
  (func (export "font_char_step") (result i32) i32.const 6)
  (func (export "max_clients") (result i32) i32.const 8)
  (func (export "ap_ip_u32") (result i32) i32.const 168634881) ;; 10.13.38.1
  (func (export "ap_lease_base_u32") (result i32) i32.const 168634882) ;; 10.13.38.2

  (func (export "rgb565") (param $r i32) (param $g i32) (param $b i32) (result i32)
    (i32.or
      (i32.shl (i32.and (local.get $r) (i32.const 0xf8)) (i32.const 8))
      (i32.or (i32.shl (i32.and (local.get $g) (i32.const 0xfc)) (i32.const 3)) (i32.shr_u (local.get $b) (i32.const 3)))))

  (func (export "glyph_index") (param $ch i32) (result i32)
    (if (i32.eq (local.get $ch) (i32.const 32)) (then (return (i32.const 0))))
    (if (i32.eq (local.get $ch) (i32.const 45)) (then (return (i32.const 13))))
    (if (i32.eq (local.get $ch) (i32.const 46)) (then (return (i32.const 14))))
    (if (i32.and (i32.ge_u (local.get $ch) (i32.const 48)) (i32.le_u (local.get $ch) (i32.const 57)))
      (then (return (i32.add (i32.sub (local.get $ch) (i32.const 48)) (i32.const 16)))))
    (if (i32.and (i32.ge_u (local.get $ch) (i32.const 65)) (i32.le_u (local.get $ch) (i32.const 90)))
      (then (return (i32.add (i32.sub (local.get $ch) (i32.const 65)) (i32.const 33)))))
    i32.const 0)

  (func (export "glyph_column_result") (param $ch i32) (param $col i32) (result i32)
    ;; 0 ok, 1 out-of-column gives blank.
    (if (i32.ge_u (local.get $col) (i32.const 5)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "text_width") (param $len i32) (param $scale i32) (result i32)
    (i32.mul (i32.mul (local.get $len) (i32.const 6)) (local.get $scale)))

  (func (export "draw_pixel_result") (param $x i32) (param $y i32) (result i32)
    ;; 0 drawn, 1 clipped.
    (if (i32.or (i32.ge_u (local.get $x) (i32.const 320)) (i32.ge_u (local.get $y) (i32.const 480))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "draw_rgb565_with_result") (param $width i32) (param $height i32) (result i32)
    ;; 0 drawn, 1 rejected because dimensions must be exact LCD size.
    (if (i32.and (i32.eq (local.get $width) (i32.const 320)) (i32.eq (local.get $height) (i32.const 480))) (then (return (i32.const 0))))
    i32.const 1)

  (func (export "fill_rect_result") (param $w i32) (param $h i32) (result i32)
    ;; 0 drawn, 1 no-op.
    (if (i32.or (i32.eqz (local.get $w)) (i32.eqz (local.get $h))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "spi_write_chunks") (param $len i32) (result i32)
    ;; SPI writes chunks of at most 64 bytes.
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (i32.div_u (i32.add (local.get $len) (i32.const 63)) (i32.const 64)))

  (func (export "spi_keep_cs_for_chunk") (param $remaining i32) (param $chunk_len i32) (param $caller_keep_cs i32) (result i32)
    (if (i32.or (local.get $caller_keep_cs) (i32.gt_u (local.get $remaining) (local.get $chunk_len))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "io_mux_offset") (param $pin i32) (result i32)
    ;; Returns register offset or -1 for unsupported pin.
    (if (i32.le_u (local.get $pin) (i32.const 14)) (then (return (i32.add (i32.const 4) (i32.mul (local.get $pin) (i32.const 4))))))
    (if (i32.and (i32.ge_u (local.get $pin) (i32.const 19)) (i32.le_u (local.get $pin) (i32.const 21)))
      (then (return (i32.add (i32.const 80) (i32.mul (i32.sub (local.get $pin) (i32.const 19)) (i32.const 4))))))
    (if (i32.and (i32.ge_u (local.get $pin) (i32.const 33)) (i32.le_u (local.get $pin) (i32.const 38)))
      (then (return (i32.add (i32.const 136) (i32.mul (i32.sub (local.get $pin) (i32.const 33)) (i32.const 4))))))
    (if (i32.and (i32.ge_u (local.get $pin) (i32.const 39)) (i32.le_u (local.get $pin) (i32.const 48)))
      (then (return (i32.add (i32.const 160) (i32.mul (i32.sub (local.get $pin) (i32.const 39)) (i32.const 4))))))
    i32.const -1)

  (func (export "gpio_bank") (param $pin i32) (result i32)
    ;; 0 lower GPIO bank, 1 upper bank.
    (if (i32.lt_u (local.get $pin) (i32.const 32)) (then (return (i32.const 0))))
    i32.const 1)

  (func (export "gpio_bit") (param $pin i32) (result i32)
    (if (i32.lt_u (local.get $pin) (i32.const 32)) (then (return (i32.shl (i32.const 1) (local.get $pin)))))
    (i32.shl (i32.const 1) (i32.sub (local.get $pin) (i32.const 32))))

  (func (export "client_add_result") (param $count i32) (result i32)
    ;; 0 added, 1 table full/no-op.
    (if (i32.ge_u (local.get $count) (i32.const 8)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "client_count_after_add") (param $count i32) (result i32)
    (if (i32.ge_u (local.get $count) (i32.const 8)) (then (return (i32.const 8))))
    (i32.add (local.get $count) (i32.const 1)))

  (func (export "dhcp_reply_type") (param $op i32) (param $magic_ok i32) (param $msg_type i32) (result i32)
    ;; 2 offer for discover, 5 ack for request, 0 invalid/no reply.
    (if (i32.or (i32.ne (local.get $op) (i32.const 1)) (i32.eqz (local.get $magic_ok))) (then (return (i32.const 0))))
    (if (i32.eq (local.get $msg_type) (i32.const 1)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $msg_type) (i32.const 3)) (then (return (i32.const 5))))
    i32.const 0)

  (func (export "dhcp_reply_len") (param $out_pos i32) (result i32)
    (if (i32.lt_u (local.get $out_pos) (i32.const 300)) (then (return (i32.const 300))))
    local.get $out_pos)

  (func (export "dhcp_option_result") (param $packet_len i32) (param $pos i32) (param $value_len i32) (result i32)
    ;; 0 ok, 1 overflow or u8 length violation.
    (if (i32.gt_u (local.get $value_len) (i32.const 255)) (then (return (i32.const 1))))
    (if (i32.gt_u (i32.add (i32.add (local.get $pos) (i32.const 2)) (local.get $value_len)) (local.get $packet_len)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "lease_last_octet") (param $lease_index i32) (result i32)
    ;; AP_LEASE_BASE last octet plus low byte of lease index.
    (i32.and (i32.add (i32.const 2) (local.get $lease_index)) (i32.const 255)))

  (func (export "ap_auth_method") (param $password_len i32) (result i32)
    ;; 0 open, 1 WPA2 personal.
    (if (i32.eqz (local.get $password_len)) (then (return (i32.const 0))))
    i32.const 1)

  (func (export "wait_ap_link_result") (param $already_up i32) (param $timeout_ok i32) (result i32)
    ;; 1 link up, 0 timeout.
    (if (local.get $already_up) (then (return (i32.const 1))))
    (if (local.get $timeout_ok) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "linker_hint_code") (param $kind_code i32) (param $symbol_code i32) (result i32)
    ;; kind 1 undefined-symbol. Symbol codes: 1 defmt, 2 stack, 3 esp_rtos, 4 embedded-test, 5 alloc.
    ;; returns 0 no hint, 1 hint printed, 2 unsupported kind exits error.
    (if (i32.ne (local.get $kind_code) (i32.const 1)) (then (return (i32.const 2))))
    (if (i32.and (i32.ge_u (local.get $symbol_code) (i32.const 1)) (i32.le_u (local.get $symbol_code) (i32.const 5))) (then (return (i32.const 1))))
    i32.const 0)
