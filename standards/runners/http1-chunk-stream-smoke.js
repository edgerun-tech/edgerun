#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/http1-chunk-stream.wat";

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function write(memory, offset, text) {
  const bytes = Buffer.from(text, "ascii");
  memory.fill(0, offset, offset + Math.max(256, bytes.length + 64));
  memory.set(bytes, offset);
  return bytes.length;
}

function chunkRecord(view, ptr) {
  return {
    size: view.getUint32(ptr, true),
    size_line_off: view.getUint32(ptr + 4, true),
    size_line_len: view.getUint32(ptr + 8, true),
    data_off: view.getUint32(ptr + 12, true),
    data_len: view.getUint32(ptr + 16, true),
    next_off: view.getUint32(ptr + 20, true),
    is_final: view.getUint32(ptr + 24, true),
    flags: view.getUint32(ptr + 28, true),
  };
}

function summaryRecord(view, ptr) {
  return {
    decoded_len: view.getBigUint64(ptr, true),
    chunk_count: view.getUint32(ptr + 8, true),
    trailers_off: view.getUint32(ptr + 12, true),
    trailers_len: view.getUint32(ptr + 16, true),
    end_off: view.getUint32(ptr + 20, true),
    flags: view.getUint32(ptr + 24, true),
  };
}

(async () => {
  const { instance } = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);
  const ptr = 1024;
  const out = 4096;

  assert.equal(exports.proto_abi_version(), 2);
  assert.equal(exports.proto_standard_id(), 300102);

  let len = write(memory, ptr, "5\r\nHello\r\n0\r\n\r\n");
  assert.equal(exports.http1_chunk_next(ptr, len, 0, out), 0);
  assert.deepEqual(chunkRecord(view, out), {
    size: 5,
    size_line_off: 0,
    size_line_len: 1,
    data_off: 3,
    data_len: 5,
    next_off: 10,
    is_final: 0,
    flags: 0,
  });
  assert.equal(exports.http1_chunk_next(ptr, len, 10, out), 0);
  assert.deepEqual(chunkRecord(view, out), {
    size: 0,
    size_line_off: 10,
    size_line_len: 1,
    data_off: 13,
    data_len: 0,
    next_off: 15,
    is_final: 1,
    flags: 0,
  });

  assert.equal(exports.http1_chunk_scan_body(ptr, len, out), 0);
  assert.deepEqual(summaryRecord(view, out), {
    decoded_len: 5n,
    chunk_count: 1,
    trailers_off: 13,
    trailers_len: 0,
    end_off: 15,
    flags: 0,
  });
  assert.equal(chunkRecord(view, out + 32).size, 5);
  assert.equal(chunkRecord(view, out + 64).is_final, 1);

  len = write(memory, ptr, "7;foo=bar\r\nMozilla\r\n9\r\nDeveloper\r\n0\r\nX-Checksum: abc123\r\n\r\n");
  assert.equal(exports.http1_chunk_scan_body(ptr, len, out), 0);
  assert.deepEqual(summaryRecord(view, out), {
    decoded_len: 16n,
    chunk_count: 2,
    trailers_off: 37,
    trailers_len: 18,
    end_off: 59,
    flags: 2,
  });
  assert.equal(chunkRecord(view, out + 32).flags, 1);
  assert.equal(chunkRecord(view, out + 96).is_final, 1);
  assert.equal(chunkRecord(view, out + 96).flags, 2);

  len = write(memory, ptr, "0\r\n\r\n");
  assert.equal(exports.http1_chunk_scan_body(ptr, len, out), 0);
  assert.deepEqual(summaryRecord(view, out), {
    decoded_len: 0n,
    chunk_count: 0,
    trailers_off: 3,
    trailers_len: 0,
    end_off: 5,
    flags: 0,
  });

  len = write(memory, ptr, "g\r\nbad\r\n");
  assert.equal(exports.http1_chunk_next(ptr, len, 0, out), 3);

  len = write(memory, ptr, "100000000\r\nbad\r\n");
  assert.equal(exports.http1_chunk_next(ptr, len, 0, out), 4);

  len = write(memory, ptr, "5\r\nHelloXX");
  assert.equal(exports.http1_chunk_next(ptr, len, 0, out), 3);

  len = write(memory, ptr, "5\r\nHell");
  assert.equal(exports.http1_chunk_next(ptr, len, 0, out), 1);

  len = write(memory, ptr, "0\r\nTrailer: yes\r\n");
  assert.equal(exports.http1_chunk_next(ptr, len, 0, out), 1);

  len = write(memory, ptr, "1\u0001\r\nx\r\n0\r\n\r\n");
  assert.equal(exports.http1_chunk_next(ptr, len, 0, out), 3);

  console.log(
    JSON.stringify(
      {
        unit: "http1-chunk-stream",
        abi_version: exports.proto_abi_version(),
        standard_id: exports.proto_standard_id(),
        ok: true,
      },
      null,
      2,
    ),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
