#!/usr/bin/env node
const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");
const { spawnSync } = require("node:child_process");

const wat = "standards/build/wasm/device-primitives/efi-bt-core.wat";
const wasm = "/tmp/efi-bt-core.wasm";
const compiled = spawnSync("wat2wasm", [wat, "-o", wasm], { encoding: "utf8" });
assert.equal(compiled.status, 0, compiled.stderr);

const instance = new WebAssembly.Instance(new WebAssembly.Module(readFileSync(wasm)), {});
const e = instance.exports;

assert.equal(e.efi_bt_rtl_vendor_id(), 0x0bda);
assert.equal(e.efi_bt_rtl8922_product_id(), 0x8922);
assert.equal(e.efi_bt_usb_identity_matches(0x0bda, 0xe0, 1, 1), 1);
assert.equal(e.efi_bt_usb_identity_matches(0x1234, 0xe0, 1, 1), 0);

assert.equal(e.efi_bt_default_event_endpoint(), 0x81);
assert.equal(e.efi_bt_default_acl_out_endpoint(), 0x02);
assert.equal(e.efi_bt_default_acl_in_endpoint(), 0x82);
assert.equal(e.efi_bt_endpoint_role(3, 0x81), 1);
assert.equal(e.efi_bt_endpoint_role(2, 0x82), 2);
assert.equal(e.efi_bt_endpoint_role(2, 0x02), 3);
assert.equal(e.efi_bt_endpoint_role(3, 0x01), 0);
assert.equal(e.efi_bt_endpoint_set_complete(0x81, 0x02, 0x82), 1);
assert.equal(e.efi_bt_endpoint_set_complete(0x81, 0, 0x82), 0);

assert.equal(e.efi_bt_hci_opcode(0x3f, 0x20), 0xfc20);
assert.equal(e.efi_bt_hci_op_reset(), 0x0c03);
assert.equal(e.efi_bt_hci_op_read_local_version(), 0x1001);
assert.equal(e.efi_bt_hci_op_read_bd_addr(), 0x1009);
assert.equal(e.efi_bt_hci_op_le(0x0a), 0x200a);
assert.equal(e.efi_bt_hci_op_vendor(0x6d), 0xfc6d);

assert.equal(e.efi_bt_hci_command_args_result(1, 0, 0, 1), 0);
assert.equal(e.efi_bt_hci_command_args_result(0, 0, 0, 1), 1);
assert.equal(e.efi_bt_hci_command_args_result(1, 256, 1, 1), 2);
assert.equal(e.efi_bt_hci_command_args_result(1, 1, 0, 1), 3);
assert.equal(e.efi_bt_hci_command_packet_len(12), 15);

assert.equal(e.efi_bt_hci_event_result(0x0e, 4, 0x1001, 0x1001, 0, 6), 1);
assert.equal(e.efi_bt_hci_event_result(0x0e, 4, 0x1001, 0x1001, 1, 6), 2);
assert.equal(e.efi_bt_hci_event_result(0x0e, 3, 0x1001, 0x1001, 0, 6), 3);
assert.equal(e.efi_bt_hci_event_result(0x0e, 4, 0x1002, 0x1001, 0, 6), 4);
assert.equal(e.efi_bt_hci_event_result(0x0f, 4, 0x1001, 0x1001, 1, 6), 2);
assert.equal(e.efi_bt_read_local_version_event_len_ok(14), 1);
assert.equal(e.efi_bt_read_local_version_event_len_ok(13), 0);
assert.equal(e.efi_bt_read_bd_addr_event_len_ok(12), 1);
assert.equal(e.efi_bt_read_bd_addr_event_len_ok(11), 0);

assert.equal(e.efi_bt_looks_like_rtl8922a(0x0c, 0x000a, 0x005d, 0x8922), 1);
assert.equal(e.efi_bt_looks_like_rtl8922a(0x0b, 0x000a, 0x005d, 0x8922), 0);
assert.equal(e.efi_bt_rtl_project_valid(44, 0x8922), 1);
assert.equal(e.efi_bt_rtl_project_valid(43, 0x8922), 0);
assert.equal(e.efi_bt_rtl_extension_signature_ok(0x51, 0x04, 0xfd, 0x77), 1);
assert.equal(e.efi_bt_rtl_extension_signature_ok(0x51, 0x04, 0xfd, 0x00), 0);
assert.equal(e.efi_bt_rtl_signature_kind(0x6c616552, 0x68636574), 1);
assert.equal(e.efi_bt_rtl_signature_kind(0x54425452, 0x65726f43), 2);
assert.equal(e.efi_bt_rtl_signature_kind(0, 0), 0);
assert.equal(e.efi_bt_rtl_legacy_target_chip_id(2), 3);
assert.equal(e.efi_bt_rtl_legacy_patch_bounds_ok(100, 10, 20), 1);
assert.equal(e.efi_bt_rtl_legacy_patch_bounds_ok(100, 90, 20), 0);
assert.equal(e.efi_bt_rtl_v2_subsection_selected(1, 3, 2, 0, 0), 1);
assert.equal(e.efi_bt_rtl_v2_subsection_selected(3, 3, 2, 7, 8), 0);
assert.equal(e.efi_bt_rtl_v2_subsection_selected(3, 3, 2, 7, 7), 1);
assert.equal(e.efi_bt_rtl_v2_append_config(0, 9), 1);
assert.equal(e.efi_bt_rtl_v2_append_config(7, 9), 0);
assert.equal(e.efi_bt_rtl_patch_output_len(100, 9, 1), 109);

assert.equal(e.efi_bt_rtl_frag_len(), 252);
assert.equal(e.efi_bt_rtl_download_frag_count(0), 1);
assert.equal(e.efi_bt_rtl_download_frag_count(252), 2);
assert.equal(e.efi_bt_rtl_download_frag_payload_len(253, 0), 252);
assert.equal(e.efi_bt_rtl_download_frag_payload_len(253, 1), 1);
assert.equal(e.efi_bt_rtl_download_frag_payload_len(252, 1), 0);
assert.equal(e.efi_bt_rtl_download_index_byte(0, 2), 0);
assert.equal(e.efi_bt_rtl_download_index_byte(1, 2), 0x81);
assert.equal(e.efi_bt_rtl_download_index_byte(128, 130), 1);
assert.equal(e.efi_bt_rtl_download_index_byte(129, 130), 0x82);

assert.equal(e.efi_bt_adv_name_len_valid(0), 0);
assert.equal(e.efi_bt_adv_name_len_valid(24), 1);
assert.equal(e.efi_bt_adv_name_len_valid(25), 0);
assert.equal(e.efi_bt_adv_payload_len(12), 17);
assert.equal(e.efi_bt_adv_interval_units(), 160);
assert.equal(e.efi_bt_adv_channels_mask(), 7);
assert.equal(e.efi_bt_adv_flags(), 6);
assert.equal(e.efi_bt_adv_type_complete_name(), 9);

assert.equal(e.efi_bt_firmware_path_rank(1), 1);
assert.equal(e.efi_bt_firmware_path_rank(4), 4);
assert.equal(e.efi_bt_firmware_path_rank(5), 0);
assert.equal(e.efi_bt_build_requires_edk2_mdepkg(1), 1);
assert.equal(e.efi_bt_build_target_x64_gcc5_release(), 1);
assert.equal(e.efi_bt_main_stall_micros(), 250000);

console.log("efi-bt-core smoke passed");
