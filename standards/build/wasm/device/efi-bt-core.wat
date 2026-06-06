(module
  (import "edgerun-core" "memory" (memory 1))
(func (export "efi_bt_rtl_vendor_id") (result i32) i32.const 3034)
  (func (export "efi_bt_rtl8922_product_id") (result i32) i32.const 35106)
  (func (export "efi_bt_usb_class_wireless") (result i32) i32.const 224)
  (func (export "efi_bt_usb_subclass_rf") (result i32) i32.const 1)
  (func (export "efi_bt_usb_protocol_bt") (result i32) i32.const 1)

  (func (export "efi_bt_default_event_endpoint") (result i32) i32.const 129)
  (func (export "efi_bt_default_acl_out_endpoint") (result i32) i32.const 2)
  (func (export "efi_bt_default_acl_in_endpoint") (result i32) i32.const 130)

  (func $hci_opcode (export "efi_bt_hci_opcode") (param $ogf i32) (param $ocf i32) (result i32)
    (i32.or (i32.shl (local.get $ogf) (i32.const 10)) (local.get $ocf)))

  (func (export "efi_bt_hci_op_reset") (result i32)
    (call $hci_opcode (i32.const 3) (i32.const 3)))

  (func (export "efi_bt_hci_op_read_local_version") (result i32)
    (call $hci_opcode (i32.const 4) (i32.const 1)))

  (func (export "efi_bt_hci_op_read_bd_addr") (result i32)
    (call $hci_opcode (i32.const 4) (i32.const 9)))

  (func (export "efi_bt_hci_op_le") (param $ocf i32) (result i32)
    (call $hci_opcode (i32.const 8) (local.get $ocf)))

  (func (export "efi_bt_hci_op_vendor") (param $ocf i32) (result i32)
    (call $hci_opcode (i32.const 63) (local.get $ocf)))

  (func (export "efi_bt_usb_identity_matches") (param $vendor i32) (param $class i32) (param $subclass i32) (param $protocol i32) (result i32)
    (i32.and
      (i32.and (i32.eq (local.get $vendor) (i32.const 3034)) (i32.eq (local.get $class) (i32.const 224)))
      (i32.and (i32.eq (local.get $subclass) (i32.const 1)) (i32.eq (local.get $protocol) (i32.const 1)))))

  ;; 1 interrupt IN event, 2 bulk IN ACL RX, 3 bulk OUT ACL TX, 0 ignored.
  (func (export "efi_bt_endpoint_role") (param $transfer_type i32) (param $endpoint_address i32) (result i32)
    (local $is_in i32)
    (local.set $is_in (i32.ne (i32.and (local.get $endpoint_address) (i32.const 128)) (i32.const 0)))
    (if (i32.and (i32.eq (local.get $transfer_type) (i32.const 3)) (local.get $is_in))
      (then (return (i32.const 1))))
    (if (i32.and (i32.eq (local.get $transfer_type) (i32.const 2)) (local.get $is_in))
      (then (return (i32.const 2))))
    (if (i32.and (i32.eq (local.get $transfer_type) (i32.const 2)) (i32.eqz (local.get $is_in)))
      (then (return (i32.const 3))))
    i32.const 0)

  (func (export "efi_bt_endpoint_set_complete") (param $event_ep i32) (param $acl_out_ep i32) (param $acl_in_ep i32) (result i32)
    (i32.and
      (i32.and (i32.ne (local.get $event_ep) (i32.const 0)) (i32.ne (local.get $acl_out_ep) (i32.const 0)))
      (i32.ne (local.get $acl_in_ep) (i32.const 0))))

  ;; 0 ok, 1 bad device/packet, 2 params too large, 3 missing params.
  (func (export "efi_bt_hci_command_args_result") (param $device_ok i32) (param $params_len i32) (param $params_present i32) (param $event_ok i32) (result i32)
    (if (i32.or (i32.eqz (local.get $device_ok)) (i32.eqz (local.get $event_ok)))
      (then (return (i32.const 1))))
    (if (i32.gt_u (local.get $params_len) (i32.const 255))
      (then (return (i32.const 2))))
    (if (i32.and (i32.gt_u (local.get $params_len) (i32.const 0)) (i32.eqz (local.get $params_present)))
      (then (return (i32.const 3))))
    i32.const 0)

  (func (export "efi_bt_hci_command_packet_len") (param $params_len i32) (result i32)
    (i32.add (local.get $params_len) (i32.const 3)))

  ;; 0 continue polling, 1 complete ok, 2 command failed, 3 malformed, 4 opcode mismatch.
  (func (export "efi_bt_hci_event_result") (param $event_code i32) (param $param_len i32) (param $opcode i32) (param $expected_opcode i32) (param $hci_status i32) (param $event_len i32) (result i32)
    (if (i32.lt_u (local.get $event_len) (i32.const 3))
      (then (return (i32.const 0))))
    (if (i32.eq (local.get $event_code) (i32.const 14))
      (then
        (if (i32.or (i32.lt_u (local.get $param_len) (i32.const 4)) (i32.lt_u (local.get $event_len) (i32.const 6)))
          (then (return (i32.const 3))))
        (if (i32.ne (local.get $opcode) (local.get $expected_opcode))
          (then (return (i32.const 4))))
        (if (i32.ne (local.get $hci_status) (i32.const 0))
          (then (return (i32.const 2))))
        (return (i32.const 1))))
    (if (i32.eq (local.get $event_code) (i32.const 15))
      (then
        (if (i32.or (i32.lt_u (local.get $param_len) (i32.const 4)) (i32.lt_u (local.get $event_len) (i32.const 6)))
          (then (return (i32.const 3))))
        (if (i32.and (i32.eq (local.get $opcode) (local.get $expected_opcode)) (i32.ne (local.get $hci_status) (i32.const 0)))
          (then (return (i32.const 2))))))
    i32.const 0)

  (func (export "efi_bt_read_local_version_event_len_ok") (param $event_len i32) (result i32)
    (i32.ge_u (local.get $event_len) (i32.const 14)))

  (func (export "efi_bt_read_bd_addr_event_len_ok") (param $event_len i32) (result i32)
    (i32.ge_u (local.get $event_len) (i32.const 12)))

  (func (export "efi_bt_looks_like_rtl8922a") (param $hci_version i32) (param $hci_revision i32) (param $manufacturer i32) (param $lmp_subversion i32) (result i32)
    (i32.and
      (i32.and (i32.eq (local.get $manufacturer) (i32.const 93)) (i32.eq (local.get $lmp_subversion) (i32.const 35106)))
      (i32.and (i32.eq (local.get $hci_revision) (i32.const 10)) (i32.eq (local.get $hci_version) (i32.const 12)))))

  (func (export "efi_bt_rtl_project_valid") (param $project_id i32) (param $lmp_subversion i32) (result i32)
    (i32.and (i32.eq (local.get $project_id) (i32.const 44)) (i32.eq (local.get $lmp_subversion) (i32.const 35106))))

  (func (export "efi_bt_rtl_extension_signature_ok") (param $b0 i32) (param $b1 i32) (param $b2 i32) (param $b3 i32) (result i32)
    (i32.and
      (i32.and (i32.eq (local.get $b0) (i32.const 81)) (i32.eq (local.get $b1) (i32.const 4)))
      (i32.and (i32.eq (local.get $b2) (i32.const 253)) (i32.eq (local.get $b3) (i32.const 119)))))

  ;; 1 legacy Realtech, 2 RTBTCore v2, 0 unknown.
  (func (export "efi_bt_rtl_signature_kind") (param $sig0 i32) (param $sig1 i32) (result i32)
    (if (i32.and (i32.eq (local.get $sig0) (i32.const 1818322258)) (i32.eq (local.get $sig1) (i32.const 1751344500)))
      (then (return (i32.const 1))))
    (if (i32.and (i32.eq (local.get $sig0) (i32.const 1413633106)) (i32.eq (local.get $sig1) (i32.const 1701998403)))
      (then (return (i32.const 2))))
    i32.const 0)

  (func (export "efi_bt_rtl_legacy_target_chip_id") (param $rom_version i32) (result i32)
    (i32.add (local.get $rom_version) (i32.const 1)))

  (func (export "efi_bt_rtl_legacy_patch_bounds_ok") (param $fw_len i32) (param $patch_off i32) (param $patch_len i32) (result i32)
    (i32.and
      (i32.and (i32.ge_u (local.get $patch_len) (i32.const 4)) (i32.le_u (local.get $patch_off) (local.get $fw_len)))
      (i32.le_u (local.get $patch_len) (i32.sub (local.get $fw_len) (local.get $patch_off)))))

  (func (export "efi_bt_rtl_v2_subsection_selected") (param $opcode i32) (param $eco i32) (param $rom_version i32) (param $key_id i32) (param $section_key i32) (result i32)
    (if (i32.and
          (i32.eq (local.get $opcode) (i32.const 3))
          (i32.or (i32.eqz (local.get $key_id)) (i32.ne (local.get $section_key) (local.get $key_id))))
      (then (return (i32.const 0))))
    (i32.eq (local.get $eco) (i32.add (local.get $rom_version) (i32.const 1))))

  (func (export "efi_bt_rtl_v2_append_config") (param $key_id i32) (param $cfg_len i32) (result i32)
    (i32.and (i32.eqz (local.get $key_id)) (i32.gt_u (local.get $cfg_len) (i32.const 0))))

  (func (export "efi_bt_rtl_patch_output_len") (param $patch_len i32) (param $cfg_len i32) (param $append_cfg i32) (result i32)
    (i32.add (local.get $patch_len) (select (local.get $cfg_len) (i32.const 0) (local.get $append_cfg))))

  (func (export "efi_bt_rtl_frag_len") (result i32) i32.const 252)

  (func (export "efi_bt_rtl_download_frag_count") (param $data_len i32) (result i32)
    (i32.add (i32.div_u (local.get $data_len) (i32.const 252)) (i32.const 1)))

  (func (export "efi_bt_rtl_download_frag_payload_len") (param $data_len i32) (param $frag_index i32) (result i32)
    (local $frag_count i32)
    (local.set $frag_count (call 32 (local.get $data_len)))
    (if (i32.eq (local.get $frag_index) (i32.sub (local.get $frag_count) (i32.const 1)))
      (then (return (i32.rem_u (local.get $data_len) (i32.const 252)))))
    i32.const 252)

  (func (export "efi_bt_rtl_download_index_byte") (param $frag_index i32) (param $frag_count i32) (result i32)
    (local $index_byte i32)
    (local.set $index_byte
      (select
        (i32.add (i32.and (local.get $frag_index) (i32.const 127)) (i32.const 1))
        (local.get $frag_index)
        (i32.gt_u (local.get $frag_index) (i32.const 127))))
    (if (i32.eq (local.get $frag_index) (i32.sub (local.get $frag_count) (i32.const 1)))
      (then (local.set $index_byte (i32.or (local.get $index_byte) (i32.const 128)))))
    local.get $index_byte)

  (func (export "efi_bt_adv_name_len_valid") (param $name_len i32) (result i32)
    (i32.and (i32.gt_u (local.get $name_len) (i32.const 0)) (i32.le_u (local.get $name_len) (i32.const 24))))

  (func (export "efi_bt_adv_payload_len") (param $name_len i32) (result i32)
    (i32.add (local.get $name_len) (i32.const 5)))

  (func (export "efi_bt_adv_interval_units") (result i32) i32.const 160)
  (func (export "efi_bt_adv_channels_mask") (result i32) i32.const 7)
  (func (export "efi_bt_adv_flags") (result i32) i32.const 6)
  (func (export "efi_bt_adv_type_complete_name") (result i32) i32.const 9)

  ;; 1 firmware, 2 firmware .bin, 3 root firmware, 4 root firmware .bin, 0 none.
  (func (export "efi_bt_firmware_path_rank") (param $candidate i32) (result i32)
    (select (local.get $candidate) (i32.const 0) (i32.and (i32.ge_u (local.get $candidate) (i32.const 1)) (i32.le_u (local.get $candidate) (i32.const 4)))))

  (func (export "efi_bt_build_requires_edk2_mdepkg") (param $mdepkg_present i32) (result i32)
    (local.get $mdepkg_present))

  (func (export "efi_bt_build_target_x64_gcc5_release") (result i32)
    i32.const 1)

  (func (export "efi_bt_main_stall_micros") (result i32)
    i32.const 250000)