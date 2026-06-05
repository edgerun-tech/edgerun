#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/device-primitives/linux-adapter-core.wat");
const wasm = path.join(os.tmpdir(), `linux-adapter-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.sysfs_hex_prefix_len("0".charCodeAt(0), "x".charCodeAt(0)), 2);
  assert.strictEqual(e.sysfs_bool_flag(1), 1);
  assert.strictEqual(e.sysfs_bool_flag(2), 2);
  assert.strictEqual(e.sysfs_display_refresh_millihz(0, 0), 60000);
  assert.strictEqual(e.sysfs_display_refresh_millihz(1, 59940), 59940);
  assert.strictEqual(e.sysfs_pci_address_shape(12, 58, 58, 46), 1);
  assert.strictEqual(e.sysfs_pci_address_shape(11, 58, 58, 46), 0);
  assert.strictEqual(e.sysfs_parent_valid(1, 0), 0);
  assert.strictEqual(e.sysfs_parent_valid(1, 1), 1);

  assert.strictEqual(e.usb_speed_kind(15), 1);
  assert.strictEqual(e.usb_speed_kind(12), 2);
  assert.strictEqual(e.usb_speed_kind(480), 3);
  assert.strictEqual(e.usb_speed_kind(5000), 4);
  assert.strictEqual(e.usb_speed_kind(20000), 5);
  assert.strictEqual(e.usb_device_dir_valid(1, 1, 0, 1), 1);
  assert.strictEqual(e.usb_device_dir_valid(1, 0, 1, 1), 0);
  assert.strictEqual(e.usb_interface_dir_valid(1, 1, 1), 1);
  assert.strictEqual(e.usb_parent_depth_action(1, 2), 1);
  assert.strictEqual(e.usb_parent_depth_action(1, 0), 0);

  assert.strictEqual(e.netif_link_state(1), 1);
  assert.strictEqual(e.netif_link_state(7), 0);
  assert.strictEqual(e.netif_kind_from_sysfs(1, 0, 0, 1, 0, 1), 3);
  assert.strictEqual(e.netif_kind_from_sysfs(0, 1, 0, 1, 0, 1), 4);
  assert.strictEqual(e.netif_kind_from_sysfs(0, 0, 0, 772, 0, 1), 2);
  assert.strictEqual(e.netif_kind_from_sysfs(0, 0, 0, 1, 1, 1), 4);
  assert.strictEqual(e.netif_kind_from_sysfs(0, 0, 0, 1, 2, 1), 5);
  assert.strictEqual(e.netif_kind_from_sysfs(0, 0, 0, 1, 0, 0), 7);
  assert.strictEqual(e.netif_kind_from_sysfs(0, 0, 0, 1, 0, 1), 1);

  assert.strictEqual(e.wifi_rfkill_state(0, 0, 0), 0);
  assert.strictEqual(e.wifi_rfkill_state(1, 0, 0), 1);
  assert.strictEqual(e.wifi_rfkill_state(1, 1, 0), 2);
  assert.strictEqual(e.wifi_power_fallback(1), 1);
  assert.strictEqual(e.wifi_power_fallback(2), 3);
  assert.strictEqual(e.wifi_mode_to_nl80211(1), 2);
  assert.strictEqual(e.wifi_mode_to_nl80211(2), 3);
  assert.strictEqual(e.wifi_mode_from_nl80211(6), 4);
  assert.strictEqual(e.wifi_nla_aligned_len(1), 8);
  assert.strictEqual(e.wifi_nla_aligned_len(4), 8);
  assert.strictEqual(e.wifi_nested_attr_type(11), 0x800b);
  assert.strictEqual(e.wifi_scan_ssid_attr_count(0), 1);
  assert.strictEqual(e.wifi_scan_ssid_attr_count(3), 3);
  assert.strictEqual(e.wifi_scan_poll_decision(3, 1), 1);
  assert.strictEqual(e.wifi_scan_poll_decision(3, 0), 2);
  assert.strictEqual(e.wifi_scan_poll_decision(20, 0), 3);
  assert.strictEqual(e.wifi_ie_security_seen(48, 0), 1);
  assert.strictEqual(e.wifi_ie_security_seen(221, 1), 1);
  assert.strictEqual(e.wifi_ie_security_seen(221, 0), 0);
  assert.strictEqual(e.wifi_bss_observation_valid(0), 0);
  assert.strictEqual(e.wifi_bss_observation_valid(4), 1);
  assert.strictEqual(e.wifi_signal_dbm(-5500), -55);
  assert.strictEqual(e.wifi_rsn_ie_len(), 22);
  assert.strictEqual(e.wifi_connect_mode(1), 1);
  assert.strictEqual(e.wifi_connect_mode(0), 2);
  assert.strictEqual(e.wifi_eapol_parse_result(80, 2, 0), -1);
  assert.strictEqual(e.wifi_eapol_parse_result(99, 1, 0), -2);
  assert.strictEqual(e.wifi_eapol_parse_result(100, 2, 1), 1);
  assert.strictEqual(e.wifi_eapol_parse_result(99, 2, 1), 0);
  assert.strictEqual(e.wifi_eapol_message_type(0x80), 1);
  assert.strictEqual(e.wifi_eapol_message_type(0x100), 2);
  assert.strictEqual(e.wifi_eapol_message_type(0x80 | 0x100 | 0x200), 3);
  assert.strictEqual(e.wifi_eapol_message_type(0x100 | 0x200), 4);
  assert.strictEqual(e.wifi_eapol_is_pairwise(0x8), 1);
  assert.strictEqual(e.wifi_ptk_order_bit(1, 0), 1);
  assert.strictEqual(e.wifi_ptk_order_bit(1, 1), 3);
  assert.strictEqual(e.wifi_eapol_mic_offset(), 81);

  assert.strictEqual(e.evdev_product_field_count(6), 4);
  assert.strictEqual(e.evdev_bitmap_word_index(130), 2);
  assert.strictEqual(e.evdev_bitmap_bit_mask_low(33), 2);
  assert.strictEqual(e.evdev_classify(1, 1, 1, 1, 1, 1), 1);
  assert.strictEqual(e.evdev_classify(0, 1, 1, 1, 1, 1), 2);
  assert.strictEqual(e.evdev_classify(0, 0, 1, 1, 1, 1), 3);
  assert.strictEqual(e.evdev_classify(0, 0, 0, 1, 1, 1), 4);
  assert.strictEqual(e.evdev_classify(0, 0, 0, 0, 1, 1), 5);
  assert.strictEqual(e.evdev_classify(0, 0, 0, 0, 0, 1), 6);
  assert.strictEqual(e.evdev_event_kind(0), 1);
  assert.strictEqual(e.evdev_event_kind(1), 2);
  assert.strictEqual(e.evdev_event_kind(3), 4);
  assert.strictEqual(e.evdev_read_loop_decision(0, 0, 1), 1);
  assert.strictEqual(e.evdev_read_loop_decision(1, 8, 1), 2);
  assert.strictEqual(e.evdev_read_loop_decision(1, 0, 0), 3);
  assert.strictEqual(e.evdev_read_loop_decision(1, 0, -11), 5);
  assert.strictEqual(e.evdev_read_loop_decision(1, 0, 24), 4);

  assert.strictEqual(e.gatt_address_kind_from_u8(1), 1);
  assert.strictEqual(e.gatt_address_kind_from_u8(3), 0);
  assert.strictEqual(e.gatt_address_kind_to_u8(3), 3);
  assert.strictEqual(e.gatt_service_kind(0x180a), 1);
  assert.strictEqual(e.gatt_service_kind(0x180f), 2);
  assert.strictEqual(e.gatt_characteristic_access(1, 0, 1, 0, 1), 7);
  assert.strictEqual(e.gatt_connection_state_connected(1), 0);
  assert.strictEqual(e.gatt_connection_state_connected(3), 1);
  assert.strictEqual(e.gatt_att_error_class(4), 4);
  assert.strictEqual(e.gatt_att_error_class(8), 99);
  assert.strictEqual(e.gatt_error_category(1), 1);
  assert.strictEqual(e.gatt_error_category(2), 2);
  assert.strictEqual(e.gatt_error_category(3), 3);
  assert.strictEqual(e.hci_default_fast_param(0), 0x0060);
  assert.strictEqual(e.hci_default_fast_param(4), 0x0028);
  assert.strictEqual(e.hci_default_fast_param(6), 0x01c0);
  assert.strictEqual(e.l2cap_sockaddr_mode(1), 4);
  assert.strictEqual(e.l2cap_sockaddr_mode(0), 0);

  console.log("linux adapter core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
