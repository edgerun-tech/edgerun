#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/device-primitives/tcl-bridge-esp32s3-core.wat");
const wasm = path.join(os.tmpdir(), `tcl-bridge-esp32s3-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.tcl_bridge_abi_version(), 1);
  assert.strictEqual(e.lcd_width(), 320);
  assert.strictEqual(e.lcd_height(), 480);
  assert.strictEqual(e.rgb565(255, 0, 0), 0xf800);
  assert.strictEqual(e.rgb565(0, 255, 0), 0x07e0);
  assert.strictEqual(e.glyph_index("A".charCodeAt(0)), 33);
  assert.strictEqual(e.glyph_index("9".charCodeAt(0)), 25);
  assert.strictEqual(e.glyph_column_result("A".charCodeAt(0), 5), 1);
  assert.strictEqual(e.text_width(7, 4), 168);
  assert.strictEqual(e.draw_pixel_result(319, 479), 0);
  assert.strictEqual(e.draw_pixel_result(320, 0), 1);
  assert.strictEqual(e.draw_rgb565_with_result(320, 480), 0);
  assert.strictEqual(e.draw_rgb565_with_result(100, 480), 1);
  assert.strictEqual(e.fill_rect_result(0, 4), 1);
  assert.strictEqual(e.spi_write_chunks(0), 0);
  assert.strictEqual(e.spi_write_chunks(65), 2);
  assert.strictEqual(e.spi_keep_cs_for_chunk(65, 64, 0), 1);
  assert.strictEqual(e.io_mux_offset(0), 4);
  assert.strictEqual(e.io_mux_offset(21), 88);
  assert.strictEqual(e.io_mux_offset(48), 196);
  assert.strictEqual(e.io_mux_offset(49), -1);
  assert.strictEqual(e.gpio_bank(31), 0);
  assert.strictEqual(e.gpio_bank(32), 1);
  assert.strictEqual(e.gpio_bit(33), 2);

  assert.strictEqual(e.client_add_result(7), 0);
  assert.strictEqual(e.client_add_result(8), 1);
  assert.strictEqual(e.client_count_after_add(7), 8);
  assert.strictEqual(e.client_count_after_add(8), 8);
  assert.strictEqual(e.dhcp_reply_type(1, 1, 1), 2);
  assert.strictEqual(e.dhcp_reply_type(1, 1, 3), 5);
  assert.strictEqual(e.dhcp_reply_type(1, 0, 3), 0);
  assert.strictEqual(e.dhcp_reply_len(260), 300);
  assert.strictEqual(e.dhcp_option_result(300, 240, 4), 0);
  assert.strictEqual(e.dhcp_option_result(244, 240, 4), 1);
  assert.strictEqual(e.lease_last_octet(0), 2);
  assert.strictEqual(e.lease_last_octet(255), 1);
  assert.strictEqual(e.ap_auth_method(0), 0);
  assert.strictEqual(e.ap_auth_method(1), 1);
  assert.strictEqual(e.wait_ap_link_result(0, 0), 0);
  assert.strictEqual(e.wait_ap_link_result(0, 1), 1);
  assert.strictEqual(e.linker_hint_code(1, 5), 1);
  assert.strictEqual(e.linker_hint_code(2, 1), 2);

  console.log("tcl bridge esp32s3 core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
