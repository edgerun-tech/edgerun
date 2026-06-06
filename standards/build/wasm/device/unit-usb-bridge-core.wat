(func (export "edgerun_unit_abi_version") (result i32)
    i32.const 2)

  (func (export "edgerun_unit_scalar_abi_type_valid") (param $type_code i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $type_code) (i32.const 1))
      (i32.le_u (local.get $type_code) (i32.const 4))))

  (func (export "edgerun_unit_pointer_is_memory_offset") (result i32)
    i32.const 1)

  (func (export "edgerun_unit_imports_host_memory_allowed") (result i32)
    i32.const 0)

  (func (export "edgerun_unit_no_alloc_result") (result i32)
    i32.const 0)

  (func (export "edgerun_unit_panic_strategy") (result i32)
    i32.const 1)

  (func (export "usb_ppp_ap_default_channel") (result i32)
    i32.const 6)

  (func (export "usb_ppp_ap_max_connections") (result i32)
    i32.const 4)

  (func (export "usb_ppp_ap_status_bit") (param $ap_started i32) (param $ppp_got_ip i32) (result i32)
    (i32.or
      (select (i32.const 1) (i32.const 0) (local.get $ap_started))
      (select (i32.const 2) (i32.const 0) (local.get $ppp_got_ip))))

  ;; 0 ok, 1 invalid ssid, 2 no event group memory.
  (func (export "usb_ppp_ap_start_result") (param $ssid_len i32) (param $event_group_allocated i32) (result i32)
    (if (i32.eqz (local.get $ssid_len))
      (then (return (i32.const 1))))
    (if (i32.eqz (local.get $event_group_allocated))
      (then (return (i32.const 2))))
    i32.const 0)

  (func (export "usb_ppp_ap_password_auth_mode") (param $password_len i32) (result i32)
    (select (i32.const 2) (i32.const 0) (i32.gt_u (local.get $password_len) (i32.const 0))))

  (func (export "usb_ppp_ap_effective_channel") (param $requested_channel i32) (result i32)
    (select (local.get $requested_channel) (i32.const 6) (local.get $requested_channel)))

  (func (export "usb_ppp_ap_napt_result_ok") (param $esp_result i32) (result i32)
    (i32.or
      (i32.eqz (local.get $esp_result))
      (i32.eq (local.get $esp_result) (i32.const 259))))

  (func (export "usb_ppp_ap_ip_event_bits_after") (param $event_id i32) (param $bits i32) (result i32)
    (if (i32.eq (local.get $event_id) (i32.const 0))
      (then (return (i32.or (local.get $bits) (i32.const 2)))))
    (if (i32.eq (local.get $event_id) (i32.const 1))
      (then (return (i32.and (local.get $bits) (i32.const -3)))))
    local.get $bits)

  (func (export "usb_ppp_ap_wifi_event_bits_after") (param $event_id i32) (param $bits i32) (result i32)
    (if (i32.eq (local.get $event_id) (i32.const 0))
      (then (return (i32.or (local.get $bits) (i32.const 1)))))
    local.get $bits)

  (func (export "usb_ppp_ap_cdc_rx_should_receive") (param $itf_matches i32) (param $ppp_netif_present i32) (param $rx_size i32) (param $read_ok i32) (result i32)
    (i32.and
      (i32.and (local.get $itf_matches) (local.get $ppp_netif_present))
      (i32.and (local.get $read_ok) (i32.gt_u (local.get $rx_size) (i32.const 0)))))

  (func (export "usb_ppp_ap_nvs_init_action") (param $init_result i32) (result i32)
    (if (i32.or
          (i32.eq (local.get $init_result) (i32.const 4354))
          (i32.eq (local.get $init_result) (i32.const 4355)))
      (then (return (i32.const 1))))
    i32.const 0)

  (func (export "usb_ppp_ap_main_stack_size") (result i32)
    i32.const 8192)

  (func (export "usb_ppp_ap_event_stack_size") (result i32)
    i32.const 4096)

  (func (export "usb_ppp_ap_cdc_rx_bufsize") (result i32)
    i32.const 1024)

  (func (export "usb_ppp_ap_cdc_tx_bufsize") (result i32)
    i32.const 1024)

  (func (export "usb_ppp_ap_ppp_ipv4_enabled") (result i32)
    i32.const 1)

  (func (export "usb_ppp_ap_ipv4_napt_enabled") (result i32)
    i32.const 1)

  (func (export "usb_ppp_ap_component_version_code") (param $component i32) (result i32)
    (if (i32.eq (local.get $component) (i32.const 1))
      (then (return (i32.const 20101))))
    (if (i32.eq (local.get $component) (i32.const 2))
      (then (return (i32.const 19003))))
    (if (i32.eq (local.get $component) (i32.const 3))
      (then (return (i32.const 50503))))
    i32.const 0)

  (func (export "usb_ppp_ap_component_target_supported") (param $target i32) (result i32)
    (i32.or
      (i32.or (i32.eq (local.get $target) (i32.const 2)) (i32.eq (local.get $target) (i32.const 3)))
      (i32.or (i32.eq (local.get $target) (i32.const 4)) (i32.eq (local.get $target) (i32.const 5)))))

  (func (export "usb_ppp_ap_main_loop_sleep_secs") (result i32)
    i32.const 5)