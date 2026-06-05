#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  "standards/build/wasm/codec-primitives/esp32s3-platform-map.wat";

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasmPath = path.join(
    os.tmpdir(),
    `edgerun-${path.basename(file)}-${process.pid}.wasm`,
  );
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

(async () => {
  const { instance } = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = instance.exports;

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300094);

  assert.equal(e.esp32s3_wifi_domain_id(), 1);
  assert.equal(e.esp32s3_ble_domain_id(), 2);
  assert.equal(e.esp32s3_domain_valid(1), 1);
  assert.equal(e.esp32s3_domain_valid(2), 1);
  assert.equal(e.esp32s3_domain_valid(3), 0);

  assert.equal(e.esp32s3_region_kind(0x3fcef800), 1);
  assert.equal(e.esp32s3_region_kind(0x60004000), 2);
  assert.equal(e.esp32s3_region_kind(0x60009000), 3);
  assert.equal(e.esp32s3_region_kind(0x60008090), 4);
  assert.equal(e.esp32s3_region_kind(0x600060c8), 5);
  assert.equal(e.esp32s3_region_kind(0x6000e0c4), 6);
  assert.equal(e.esp32s3_region_kind(0x6001c400), 7);
  assert.equal(e.esp32s3_region_kind(0x60026014), 8);
  assert.equal(e.esp32s3_region_kind(0x60024000), 9);
  assert.equal(e.esp32s3_region_kind(0x60033d14), 10);
  assert.equal(e.esp32s3_region_kind(0x600c0018), 11);
  assert.equal(e.esp32s3_region_kind(0x50000000), 0);

  assert.equal(e.esp32s3_region_contains(1, 0x3fcef800, 0x800), 1);
  assert.equal(e.esp32s3_region_contains(1, 0x3fcef800, 0x801), 0);
  assert.equal(e.esp32s3_region_contains(10, 0x60033c40, 4), 1);
  assert.equal(e.esp32s3_region_contains(10, 0x600351fc, 4), 1);
  assert.equal(e.esp32s3_region_contains(10, 0x600351fc, 8), 0);

  assert.equal(e.esp32s3_mmio_addr_valid(0x60026014, 4), 1);
  assert.equal(e.esp32s3_mmio_addr_valid(0x60026014, 4096), 0);
  assert.equal(e.esp32s3_mmio_addr_valid(0x60026014, 0), 0);
  assert.equal(e.esp32s3_mmio_reg32_valid(0x60026014), 1);
  assert.equal(e.esp32s3_mmio_reg32_valid(0x60026015), 0);
  assert.equal(e.esp32s3_mmio_reg32_valid(0x6002ffff), 0);

  for (let kind = 1; kind <= 8; kind += 1) {
    assert.equal(e.esp32s3_blob_section_valid(kind), 1);
  }
  assert.equal(e.esp32s3_blob_section_valid(0), 0);
  assert.equal(e.esp32s3_blob_section_valid(9), 0);
  assert.equal(e.esp32s3_blob_section_domain(1), 1);
  assert.equal(e.esp32s3_blob_section_domain(4), 1);
  assert.equal(e.esp32s3_blob_section_domain(5), 2);
  assert.equal(e.esp32s3_blob_section_domain(8), 2);
  assert.equal(e.esp32s3_blob_section_domain(9), 0);

  assert.equal(e.esp32s3_blob_section_meta(1), 0x800);
  assert.equal(e.esp32s3_blob_section_meta(2), 256);
  assert.equal(e.esp32s3_blob_section_meta(3), 0x1f2f3f4f);
  assert.equal(e.esp32s3_blob_section_meta(4) >>> 0, 0xdeadbeaf);
  assert.equal(e.esp32s3_blob_section_meta(5), 64);
  assert.equal(e.esp32s3_blob_section_meta(6), 0x5a5aa5a5);
  assert.equal(e.esp32s3_blob_section_meta(7) >>> 0, 0xfadebead);
  assert.equal(e.esp32s3_blob_section_meta(8), 32);

  assert.equal(e.esp32s3_wifi_ap_config_field_offset(1), 0);
  assert.equal(e.esp32s3_wifi_ap_config_field_offset(2), 96);
  assert.equal(e.esp32s3_wifi_ap_config_field_offset(3), 97);
  assert.equal(e.esp32s3_wifi_ap_config_field_offset(4), 100);
  assert.equal(e.esp32s3_wifi_ap_config_field_offset(5), 105);
  assert.equal(e.esp32s3_wifi_ap_config_field_offset(6), 106);
  assert.equal(e.esp32s3_wifi_ap_config_field_offset(7), 109);
  assert.equal(e.esp32s3_wifi_ap_config_field_offset(8), -1);
  assert.equal(e.esp32s3_wifi_ap_config_valid(1, 1), 1);
  assert.equal(e.esp32s3_wifi_ap_config_valid(32, 14), 1);
  assert.equal(e.esp32s3_wifi_ap_config_valid(0, 6), 0);
  assert.equal(e.esp32s3_wifi_ap_config_valid(33, 6), 0);
  assert.equal(e.esp32s3_wifi_ap_config_valid(8, 0), 0);
  assert.equal(e.esp32s3_wifi_ap_config_valid(8, 15), 0);

  assert.equal(e.esp32s3_wifi_init_status_kind(0), 0);
  assert.equal(e.esp32s3_wifi_init_status_kind(-1), 1);
  assert.equal(e.esp32s3_wifi_init_status_kind(0x3003), 2);
  assert.equal(e.esp32s3_wifi_init_status_kind(20017), 3);
  assert.equal(e.esp32s3_wifi_init_status_kind(30017), 4);
  assert.equal(e.esp32s3_wifi_init_status_kind(40017), 5);
  assert.equal(e.esp32s3_wifi_init_status_kind(50000), 6);

  assert.equal(e.esp32s3_ble_status_field_valid(1, 0), 1);
  assert.equal(e.esp32s3_ble_status_field_valid(4, -2147483648), 1);
  assert.equal(e.esp32s3_ble_status_field_valid(4, 7), 0);
  assert.equal(e.esp32s3_ble_status_field_valid(5, 0), 1);
  assert.equal(e.esp32s3_ble_status_field_valid(5, -1), 0);
  assert.equal(e.esp32s3_ble_version_byte_valid(0x20), 1);
  assert.equal(e.esp32s3_ble_version_byte_valid(0x7e), 1);
  assert.equal(e.esp32s3_ble_version_byte_valid(0x1f), 0);
  assert.equal(e.esp32s3_ble_version_byte_valid(0x7f), 0);

  for (const opcode of [0x0c03, 0x2005, 0x2006, 0x2008, 0x200a, 0x200c]) {
    assert.equal(e.esp32s3_ble_hci_opcode_valid(opcode), 1);
  }
  assert.equal(e.esp32s3_ble_hci_opcode_valid(0x1234), 0);
  assert.equal(e.esp32s3_ble_hci_packet_kind_valid(1), 1);
  assert.equal(e.esp32s3_ble_hci_packet_kind_valid(2), 1);
  assert.equal(e.esp32s3_ble_hci_packet_kind_valid(4), 1);
  assert.equal(e.esp32s3_ble_hci_packet_kind_valid(3), 0);

  assert.equal(e.esp32s3_wifi_raw_80211_len_valid(24), 1);
  assert.equal(e.esp32s3_wifi_raw_80211_len_valid(2352), 1);
  assert.equal(e.esp32s3_wifi_raw_80211_len_valid(23), 0);
  assert.equal(e.esp32s3_wifi_raw_80211_len_valid(2353), 0);
  assert.equal(e.esp32s3_wifi_promisc_sig_payload_len(28), 24);
  assert.equal(e.esp32s3_wifi_promisc_sig_payload_len(2356), 2352);
  assert.equal(e.esp32s3_wifi_promisc_sig_payload_len(27), -1);

  console.log(
    JSON.stringify({
      unit: "esp32s3-platform-map",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      cases: [
        "wifi_ble_domain_ids",
        "known_mmio_regions",
        "region_span_bounds",
        "reg32_alignment",
        "blob_section_kind_domain_meta",
        "wifi_ap_config_offsets",
        "wifi_init_status_bands",
        "ble_status_and_version_bytes",
        "ble_hci_kinds_and_opcodes",
        "wifi_raw_80211_lengths",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
