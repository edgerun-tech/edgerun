#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/sdk-app-identity.wat");

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

function writeBytes(memory, bytes, ptr) {
  memory.fill(0, ptr, ptr + Math.max(256, bytes.length + 1));
  memory.set(bytes, ptr);
  return ptr;
}

function writeAscii(memory, value, ptr) {
  return writeBytes(memory, Buffer.from(value, "ascii"), ptr);
}

function readBytes(memory, ptr, len) {
  return Buffer.from(memory.slice(ptr, ptr + len));
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300088);

  for (const slug of ["edgerun-wallet-app", "sha256-fips180", "a1-b2-c3"]) {
    const ptr = writeAscii(memory, slug, 1024);
    assert.equal(e.sdk_app_slug_valid(ptr, slug.length), 0, slug);
  }

  for (const slug of [
    "",
    "EdgeRun",
    "-bad",
    "bad-",
    "bad--slug",
    "bad_slug",
    "a".repeat(65),
  ]) {
    const ptr = writeAscii(memory, slug, 1024);
    assert.equal(e.sdk_app_slug_valid(ptr, slug.length), 3, slug);
  }

  for (const version of ["0.1.0", "12.34.56", "1.2.3-alpha.1"]) {
    const ptr = writeAscii(memory, version, 2048);
    assert.equal(e.sdk_app_version_valid(ptr, version.length), 0, version);
  }

  for (const version of ["", "1", "1.2", "1.2.", "1.2.3-", "v1.2.3", "1.2.3+meta"]) {
    const ptr = writeAscii(memory, version, 2048);
    assert.equal(e.sdk_app_version_valid(ptr, version.length), 3, version);
  }

  const slug = "edgerun-wallet-app";
  const slugPtr = writeAscii(memory, slug, 1024);
  const dev = Buffer.from(Array.from({ length: 32 }, (_, i) => i + 1));
  const devPtr = writeBytes(memory, dev, 2048);
  let out = 4096;
  let result = packed(e.sdk_app_manifest_preimage(slugPtr, slug.length, devPtr, 32, out, 128));
  const manifestExpected = Buffer.concat([
    Buffer.from("edgerun-app:", "ascii"),
    Buffer.from(slug, "ascii"),
    Buffer.from([0]),
    dev,
  ]);
  assert.deepEqual(result, { status: 0, written: manifestExpected.length });
  assert.deepEqual(readBytes(memory, out, result.written), manifestExpected);
  assert.deepEqual(packed(e.sdk_app_manifest_preimage(slugPtr, slug.length, devPtr, 32, out, 8)), {
    status: 2,
    written: 0,
  });
  assert.deepEqual(packed(e.sdk_app_manifest_preimage(slugPtr, slug.length, devPtr, 31, out, 128)), {
    status: 3,
    written: 0,
  });

  const appId = Buffer.from(Array.from({ length: 32 }, (_, i) => 0xa0 + i));
  const hash = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x40 + i));
  const appPtr = writeBytes(memory, appId, 1024);
  const version = "1.2.3-alpha";
  const versionPtr = writeAscii(memory, version, 2048);
  const hashPtr = writeBytes(memory, hash, 3072);
  out = 5120;
  result = packed(
    e.sdk_release_preimage(appPtr, 32, versionPtr, version.length, hashPtr, 32, out, 128),
  );
  const releaseExpected = Buffer.concat([
    appId,
    Buffer.from([0]),
    Buffer.from(version, "ascii"),
    Buffer.from([0]),
    hash,
  ]);
  assert.deepEqual(result, { status: 0, written: releaseExpected.length });
  assert.deepEqual(readBytes(memory, out, result.written), releaseExpected);
  assert.deepEqual(
    packed(e.sdk_release_preimage(appPtr, 31, versionPtr, version.length, hashPtr, 32, out, 128)),
    { status: 3, written: 0 },
  );
  assert.deepEqual(
    packed(e.sdk_release_preimage(appPtr, 32, versionPtr, version.length, hashPtr, 32, out, 8)),
    { status: 2, written: 0 },
  );

  console.log(
    JSON.stringify({
      unit: "sdk-app-identity",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      cases: [
        "slug_good",
        "slug_bad",
        "version_good",
        "version_bad",
        "manifest_preimage",
        "manifest_output_short",
        "release_preimage",
        "release_output_short",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
