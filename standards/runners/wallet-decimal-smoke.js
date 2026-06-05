#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/wallet-decimal.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function packed(value) {
  return {
    status: Number(value & 0xffffffffn),
    written: Number((value >> 32n) & 0xffffffffn),
  };
}

function writeAscii(memory, value, ptr = 1024) {
  memory.fill(0, ptr, ptr + Math.max(128, value.length + 1));
  for (let i = 0; i < value.length; i += 1) {
    memory[ptr + i] = value.charCodeAt(i);
  }
  return ptr;
}

function readAscii(memory, ptr, len) {
  return Buffer.from(memory.slice(ptr, ptr + len)).toString("ascii");
}

function readAmount(view, ptr) {
  return {
    lo: view.getBigUint64(ptr, true),
    hi: view.getBigUint64(ptr + 8, true),
    scale: view.getUint32(ptr + 16, true),
  };
}

function expectAmount(amount, mantissa, scale) {
  const expected = BigInt(mantissa);
  assert.equal((amount.hi << 64n) | amount.lo, expected);
  assert.equal(amount.scale, scale);
}

function parse(exports, memory, view, value) {
  const ptr = writeAscii(memory, value);
  const out = 2048;
  const status = exports.wallet_decimal_parse(ptr, value.length, out);
  assert.equal(status, 0, `parse ${value}`);
  return readAmount(view, out);
}

function format(exports, memory, amount, cap = 128) {
  const out = 4096;
  memory.fill(0, out, out + cap);
  const result = packed(exports.wallet_decimal_format(amount.lo, amount.hi, amount.scale, out, cap));
  assert.equal(result.status, 0);
  return readAscii(memory, out, result.written);
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const view = new DataView(e.memory.buffer);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300086);

  const hundred = parse(e, memory, view, "100.00");
  expectAmount(hundred, 10000n, 2);
  assert.equal(format(e, memory, hundred), "100.00");

  const small = parse(e, memory, view, "0.12345678");
  expectAmount(small, 12345678n, 8);
  assert.equal(format(e, memory, small), "0.12345678");

  const leading = parse(e, memory, view, "000.500");
  expectAmount(leading, 500n, 3);
  assert.equal(format(e, memory, leading), "0.500");

  for (const invalid of ["-1", "+1", "1e2", `0.${"1".repeat(39)}`]) {
    const ptr = writeAscii(memory, invalid);
    assert.equal(e.wallet_decimal_parse(ptr, invalid.length, 2048), 3, `reject ${invalid}`);
  }

  const onePointTwo = parse(e, memory, view, "1.2");
  const threeCents = parse(e, memory, view, "0.03");
  assert.equal(
    e.wallet_decimal_add(
      onePointTwo.lo,
      onePointTwo.hi,
      onePointTwo.scale,
      threeCents.lo,
      threeCents.hi,
      threeCents.scale,
      2048,
    ),
    0,
  );
  const sum = readAmount(view, 2048);
  expectAmount(sum, 123n, 2);
  assert.equal(format(e, memory, sum), "1.23");

  assert.equal(
    e.wallet_decimal_sub(
      threeCents.lo,
      threeCents.hi,
      threeCents.scale,
      onePointTwo.lo,
      onePointTwo.hi,
      onePointTwo.scale,
      2048,
    ),
    3,
  );

  assert.equal(e.wallet_decimal_compare(12n, 0n, 1, 120n, 0n, 2), 0);
  assert.equal(e.wallet_decimal_compare(10000n, 0n, 2, 99999n, 0n, 3), 1);
  assert.equal(e.wallet_decimal_compare(99999n, 0n, 3, 10000n, 0n, 2), -1);

  assert.equal(e.wallet_decimal_normalize(15n, 0n, 1, 3, 2048), 0);
  const normalized = readAmount(view, 2048);
  expectAmount(normalized, 1500n, 3);
  assert.equal(format(e, memory, normalized), "1.500");

  assert.deepEqual(packed(e.wallet_decimal_format(500n, 0n, 3, 4096, 4)), {
    status: 2,
    written: 0,
  });

  console.log(
    JSON.stringify({
      unit: "wallet-decimal",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      cases: [
        "parse_100_00",
        "parse_0_12345678",
        "canonical_000_500",
        "reject_sign_exponent_scale39",
        "add_1_2_0_03",
        "sub_negative_reject",
        "compare_normalized",
        "normalize_up",
        "output_short",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
