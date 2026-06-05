#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/marketplace-policy.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function readU128(view, ptr) {
  return view.getBigUint64(ptr, true) + (view.getBigUint64(ptr + 8, true) << 64n);
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const view = new DataView(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300087);

  for (const id of [0, 1, 2, 3]) {
    assert.strictEqual(exports.marketplace_listing_status_valid(id), 1);
  }
  for (const id of [-1, 4, 99]) {
    assert.strictEqual(exports.marketplace_listing_status_valid(id), 0);
  }

  for (const id of [0, 1, 2, 3, 4, 5]) {
    assert.strictEqual(exports.marketplace_checkout_status_valid(id), 1);
  }
  for (const id of [-1, 6, 99]) {
    assert.strictEqual(exports.marketplace_checkout_status_valid(id), 0);
  }

  assert.strictEqual(
    exports.marketplace_listing_can_checkout(1, 1_000n, 0n, 0n, 0n),
    1,
  );
  assert.strictEqual(
    exports.marketplace_listing_can_checkout(1, 1_000n, 0n, 1_000n, 0n),
    1,
  );
  assert.strictEqual(
    exports.marketplace_listing_can_checkout(1, 1_001n, 0n, 1_000n, 0n),
    0,
  );
  assert.strictEqual(
    exports.marketplace_listing_can_checkout(0, 1n, 0n, 0n, 0n),
    0,
  );
  assert.strictEqual(
    exports.marketplace_listing_can_checkout(2, 1n, 0n, 0n, 0n),
    0,
  );
  assert.strictEqual(
    exports.marketplace_listing_can_checkout(3, 1n, 0n, 0n, 0n),
    0,
  );

  assert.strictEqual(exports.marketplace_commission_policy_valid(100, 200, 50, 350), 1);
  assert.strictEqual(exports.marketplace_commission_policy_valid(100, 200, 51, 350), 0);
  assert.strictEqual(exports.marketplace_commission_policy_valid(100, 200, 50, 10001), 0);
  assert.strictEqual(exports.marketplace_commission_policy_valid(10000, 0, 0, 10000), 1);
  assert.strictEqual(exports.marketplace_commission_policy_valid(10000, 1, 0, 10000), 0);

  const outPtr = 2048;
  assert.strictEqual(
    exports.marketplace_payout_split(1_000_000n, 0n, 100, 200, 50, outPtr),
    0,
  );
  assert.strictEqual(readU128(view, outPtr), 965000n);
  assert.strictEqual(readU128(view, outPtr + 16), 10000n);
  assert.strictEqual(readU128(view, outPtr + 32), 20000n);
  assert.strictEqual(readU128(view, outPtr + 48), 5000n);

  assert.strictEqual(exports.marketplace_payout_split(1_000_000n, 0n, 10001, 0, 0, outPtr), 3);

  console.log(
    JSON.stringify({
      unit: "marketplace-policy",
      abi_version: exports.proto_abi_version(),
      standard_id: exports.proto_standard_id(),
      ok: true,
      cases: [
        "listing_status_valid",
        "listing_status_invalid",
        "checkout_status_valid",
        "checkout_status_invalid",
        "active_no_expiry_checkout",
        "active_expiry_boundary",
        "expired_checkout_reject",
        "inactive_checkout_reject",
        "commission_policy_valid",
        "commission_policy_over_max_reject",
        "commission_policy_max_over_10000_reject",
        "payout_split_1000000_100_200_50",
        "payout_split_invalid_bps",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
