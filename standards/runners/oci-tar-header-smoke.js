#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/oci-tar-header.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function bytes(text) {
  return Array.from(Buffer.from(text, "utf8"));
}

function writeBytes(memory, data, ptr = 1024) {
  memory.fill(0, ptr, ptr + Math.max(512, data.length));
  memory.set(data, ptr);
  return ptr;
}

function writeString(memory, text, ptr = 4096) {
  const data = bytes(text);
  memory.fill(0, ptr, ptr + data.length + 8);
  memory.set(data, ptr);
  return [ptr, data.length];
}

function readFields(memory, ptr) {
  const view = new DataView(memory.buffer, ptr, 44);
  return {
    kind: view.getUint32(0, true),
    size: view.getUint32(4, true) + view.getUint32(8, true) * 0x100000000,
    mode: view.getUint32(12, true),
    uid: view.getUint32(16, true),
    gid: view.getUint32(20, true),
    mtime: view.getUint32(24, true) + view.getUint32(28, true) * 0x100000000,
    pathLen: view.getUint32(32, true),
    linkLen: view.getUint32(36, true),
    whiteoutKind: view.getUint32(40, true),
  };
}

function putString(header, offset, width, value) {
  const data = bytes(value);
  assert(data.length <= width, `${value} too long`);
  header.set(data, offset);
}

function putOctal(header, offset, width, value) {
  const text = value.toString(8).padStart(width - 1, "0").slice(-(width - 1)) + "\0";
  putString(header, offset, width, text);
}

function header(options) {
  const h = new Uint8Array(512);
  putString(h, 0, 100, options.name);
  putOctal(h, 100, 8, options.mode ?? 0o755);
  putOctal(h, 108, 8, options.uid ?? 0);
  putOctal(h, 116, 8, options.gid ?? 0);
  putOctal(h, 124, 12, options.size ?? 0);
  putOctal(h, 136, 12, options.mtime ?? 1);
  h.fill(0x20, 148, 156);
  putString(h, 156, 1, options.typeflag ?? "0");
  if (options.linkname) putString(h, 157, 100, options.linkname);
  putString(h, 257, 6, "ustar\0");
  putString(h, 263, 2, "00");
  if (options.prefix) putString(h, 345, 155, options.prefix);
  let sum = 0;
  for (const byte of h) sum += byte;
  putString(h, 148, 8, sum.toString(8).padStart(6, "0") + "\0 ");
  return h;
}

function parse(exports, memory, h) {
  const inPtr = writeBytes(memory, h);
  const outPtr = 8192;
  memory.fill(0, outPtr, outPtr + 44);
  const status = exports.oci_tar_header_parse(inPtr, h.length, outPtr);
  return [status, readFields(memory, outPtr)];
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300802);

  let [ptr, len] = writeString(memory, "usr/bin/app");
  assert.strictEqual(exports.oci_tar_path_safe(ptr, len), 1);
  [ptr, len] = writeString(memory, "/usr/bin/app");
  assert.strictEqual(exports.oci_tar_path_safe(ptr, len), 0);
  [ptr, len] = writeString(memory, "usr/../app");
  assert.strictEqual(exports.oci_tar_path_safe(ptr, len), 0);
  [ptr, len] = writeString(memory, "./");
  assert.strictEqual(exports.oci_tar_path_safe(ptr, len), 0);

  ptr = writeBytes(memory, bytes("00000000005\0"), 5000);
  const outPtr = 6000;
  assert.strictEqual(exports.oci_tar_octal_u64(ptr, 12, outPtr), 0);
  assert.strictEqual(new DataView(memory.buffer, outPtr, 8).getUint32(0, true), 5);
  ptr = writeBytes(memory, bytes("00000000008\0"), 5000);
  assert.strictEqual(exports.oci_tar_octal_u64(ptr, 12, outPtr), 3);

  let result = parse(exports, memory, header({ name: "usr/bin/app", size: 5, mode: 0o755 }));
  assert.strictEqual(result[0], 0);
  assert.deepStrictEqual(result[1], {
    kind: 0,
    size: 5,
    mode: 0o755,
    uid: 0,
    gid: 0,
    mtime: 1,
    pathLen: 11,
    linkLen: 0,
    whiteoutKind: 0,
  });

  result = parse(exports, memory, header({ name: "usr/bin", typeflag: "5", mode: 0o755 }));
  assert.strictEqual(result[0], 0);
  assert.strictEqual(result[1].kind, 5);

  result = parse(
    exports,
    memory,
    header({ name: "usr/bin/current", typeflag: "2", linkname: "app", mode: 0o777 }),
  );
  assert.strictEqual(result[0], 0);
  assert.strictEqual(result[1].kind, 2);
  assert.strictEqual(result[1].linkLen, 3);

  result = parse(exports, memory, header({ name: ".wh.old-app", typeflag: "0", size: 0 }));
  assert.strictEqual(result[0], 0);
  assert.strictEqual(result[1].whiteoutKind, 1);

  result = parse(exports, memory, header({ name: "usr/.wh..wh..opq", typeflag: "0" }));
  assert.strictEqual(result[0], 0);
  assert.strictEqual(result[1].whiteoutKind, 2);

  const badChecksum = header({ name: "usr/bin/app", size: 5 });
  badChecksum[0] = "U".charCodeAt(0);
  result = parse(exports, memory, badChecksum);
  assert.strictEqual(result[0], 3);

  result = parse(exports, memory, header({ name: "../escape", size: 1 }));
  assert.strictEqual(result[0], 3);

  result = parse(exports, memory, new Uint8Array(512));
  assert.strictEqual(result[0], 4);

  result = parse(exports, memory, header({ name: "pipe", typeflag: "z" }));
  assert.strictEqual(result[0], 1);

  result = parse(exports, memory, header({ name: "short" }).slice(0, 511));
  assert.strictEqual(result[0], 2);

  console.log(
    JSON.stringify({
      unit: "oci-tar-header",
      abi_version: exports.proto_abi_version(),
      standard_id: exports.proto_standard_id(),
      ok: true,
      cases: [
        "file",
        "directory",
        "symlink",
        "whiteout",
        "opaque_whiteout",
        "bad_checksum",
        "traversal_path",
        "zero_block",
        "unsupported_kind",
        "short_header",
        "octal_invalid",
        "path_safe",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
