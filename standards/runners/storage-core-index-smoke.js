#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/storage-core-index.wat");

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function write(memory, ptr, value) {
  const bytes = Buffer.from(value, "utf8");
  memory.fill(0, ptr, ptr + Math.max(128, bytes.length + 16));
  memory.set(bytes, ptr);
  return bytes.length;
}

function fnv(bytes, seed = 0x811c9dc5) {
  let hash = seed >>> 0;
  for (const byte of bytes) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash >>> 0;
}

(async () => {
  const { instance } = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);
  const ptr = 1024;

  assert.equal(exports.proto_abi_version(), 2);
  assert.equal(exports.proto_standard_id(), 300106);

  for (let kind = 0; kind <= 10; kind += 1) {
    assert.equal(exports.storage_object_kind_valid(kind), 1);
  }
  assert.equal(exports.storage_object_kind_valid(11), 0);
  assert.equal(exports.storage_object_kind_portable(0), 0);
  assert.equal(exports.storage_object_kind_portable(1), 1);
  assert.equal(exports.storage_action_status_valid(3), 1);
  assert.equal(exports.storage_action_status_valid(4), 0);
  assert.equal(exports.storage_op_event_type_valid(100), 1);
  assert.equal(exports.storage_op_event_type_valid(112), 1);
  assert.equal(exports.storage_op_event_type_valid(113), 0);
  assert.equal(exports.storage_op_event_type_valid(119), 1);

  assert.equal(exports.storage_fetch_status_code(ptr, write(memory, ptr, "pending")), 1);
  assert.equal(exports.storage_fetch_status_code(ptr, write(memory, ptr, "done")), 2);
  assert.equal(exports.storage_fetch_status_code(ptr, write(memory, ptr, "failed")), 3);
  assert.equal(exports.storage_fetch_status_code(ptr, write(memory, ptr, "queued")), 0);
  assert.equal(exports.storage_peer_status_code(ptr, write(memory, ptr, "discovered")), 1);
  assert.equal(exports.storage_peer_status_code(ptr, write(memory, ptr, "status_changed")), 2);
  assert.equal(exports.storage_peer_status_code(ptr, write(memory, ptr, "unreachable")), 3);

  const payload = Buffer.from("abc", "ascii");
  memory.set(payload, ptr);
  assert.equal(exports.derived_db_header_checksum(ptr, payload.length) >>> 0, fnv(payload));
  assert.equal(
    exports.derived_db_record_checksum(2, ptr, payload.length) >>> 0,
    fnv(payload, fnv(Buffer.from([2]))),
  );
  for (const kind of [1, 2, 3, 4, 5]) {
    assert.equal(exports.derived_db_record_kind_valid(kind), 1);
  }
  assert.equal(exports.derived_db_record_kind_valid(0), 0);
  assert.equal(exports.derived_db_record_kind_valid(6), 0);

  const header = Buffer.alloc(20);
  header.set(Buffer.from("ERDB0001", "ascii"), 0);
  header.writeUInt16LE(2, 8);
  header.writeUInt16LE(1, 10);
  header.writeUInt32LE(0, 12);
  header.writeUInt32LE(fnv(header.subarray(0, 16)), 16);
  memory.set(header, ptr);
  assert.equal(exports.derived_db_header_validate(ptr, header.length), 0);
  memory[ptr + 8] = 3;
  assert.equal(exports.derived_db_header_validate(ptr, header.length), 3);
  memory.set(header, ptr);

  assert.equal(exports.derived_db_access_path(1, 1, 0, 0, 0, 0), 1);
  assert.equal(exports.derived_db_access_path(2, 0, 1, 0, 0, 0), 2);
  assert.equal(exports.derived_db_access_path(2, 0, 0, 1, 1, 0), 4);
  assert.equal(exports.derived_db_access_path(2, 0, 0, 1, 0, 0), 3);
  assert.equal(exports.derived_db_access_path(3, 0, 0, 0, 0, 1), 5);
  assert.equal(exports.derived_db_access_path(3, 0, 0, 0, 0, 0), 0);

  const fileKinds = {
    "stream_heads.bin": [1, 4],
    "events.bin": [2, 6],
    "replay_cache.bin": [3, 4],
    "peers.bin": [4, 6],
    "snapshots.bin": [5, 2],
    "delegations.bin": [6, 7],
    "revocations.bin": [7, 6],
    "credentials.bin": [8, 4],
    "object_presence.bin": [9, 4],
    "controller_changes.bin": [10, 3],
    "fetch_queue.bin": [11, 6],
  };
  for (const [name, [kind, shape]] of Object.entries(fileKinds)) {
    assert.equal(exports.file_index_filename_kind(ptr, write(memory, ptr, name)), kind, name);
    assert.equal(exports.file_index_record_shape(kind), shape, name);
  }
  assert.equal(exports.file_index_filename_kind(ptr, write(memory, ptr, "index.bin")), 0);
  assert.equal(exports.file_index_credential_key_valid(ptr, write(memory, ptr, "gmail/token")), 1);
  assert.equal(exports.file_index_credential_key_valid(ptr, write(memory, ptr, "/token")), 0);
  assert.equal(exports.file_index_credential_key_valid(ptr, write(memory, ptr, "gmail/")), 0);
  assert.equal(exports.file_index_credential_key_valid(ptr, write(memory, ptr, "gmail/a/b")), 0);

  const blockHeader = Buffer.alloc(16);
  blockHeader.set(Buffer.from("ERLG", "ascii"), 0);
  blockHeader.writeBigUInt64LE(512n, 4);
  blockHeader[12] = 1;
  memory.set(blockHeader, ptr);
  assert.equal(exports.block_event_log_header_validate(ptr, blockHeader.length, 512, 4n), 0);
  blockHeader.writeBigUInt64LE(2048n, 4);
  memory.set(blockHeader, ptr);
  assert.equal(exports.block_event_log_header_validate(ptr, blockHeader.length, 512, 4n), 3);
  assert.equal(exports.block_event_stream_append_status(0, 0n, 0n, 0, 0, 0), 0);
  assert.equal(exports.block_event_stream_append_status(0, 0n, 1n, 0, 0, 0), 3);
  assert.equal(exports.block_event_stream_append_status(1, 4n, 5n, 1, 1, 1), 0);
  assert.equal(exports.block_event_stream_append_status(1, 4n, 6n, 1, 1, 1), 3);
  assert.equal(exports.block_event_stream_append_status(1, 4n, 5n, 1, 2, 1), 3);

  for (let kind = 1; kind <= 7; kind += 1) {
    assert.equal(exports.edgefs_entry_kind_valid(kind), 1);
  }
  assert.equal(exports.edgefs_entry_kind_valid(8), 0);
  assert.equal(exports.edgefs_entry_kind_class(1), 1);
  assert.equal(exports.edgefs_entry_kind_class(2), 2);
  assert.equal(exports.edgefs_entry_kind_class(3), 3);
  assert.equal(exports.edgefs_entry_kind_class(4), 3);
  assert.equal(exports.edgefs_entry_kind_class(5), 4);
  assert.equal(exports.edgefs_entry_kind_class(6), 4);
  assert.equal(exports.edgefs_entry_kind_class(7), 5);
  assert.equal(exports.edgefs_create_node_kind_valid(1), 0);
  assert.equal(exports.edgefs_create_node_kind_valid(2), 1);
  assert.equal(exports.edgefs_create_node_kind_valid(5), 1);
  assert.equal(exports.edgefs_create_node_kind_valid(7), 1);

  assert.equal(exports.edgefs_path_classify(ptr, write(memory, ptr, "a/./b"), 0), 0);
  assert.equal(exports.edgefs_path_classify(ptr, write(memory, ptr, "/"), 1), 1);
  assert.equal(exports.edgefs_path_classify(ptr, write(memory, ptr, "."), 1), 1);
  assert.equal(exports.edgefs_path_classify(ptr, write(memory, ptr, ""), 0), 2);
  assert.equal(exports.edgefs_path_classify(ptr, write(memory, ptr, "../x"), 1), 2);
  assert.equal(exports.edgefs_path_classify(ptr, write(memory, ptr, "a/../../x"), 1), 2);
  memory.set(Buffer.from([97, 0, 98]), ptr);
  assert.equal(exports.edgefs_path_classify(ptr, 3, 1), 3);

  console.log(
    JSON.stringify({
      unit: "storage-core-index",
      abi_version: exports.proto_abi_version(),
      standard_id: exports.proto_standard_id(),
      ok: true,
      cases: [
        "object_kind_mapping",
        "action_and_op_event_status_mapping",
        "fetch_and_peer_status_mapping",
        "derived_db_header_and_record_checksums",
        "derived_db_access_paths",
        "file_index_filename_and_record_shapes",
        "credential_key_shape",
        "block_event_log_header_and_append_status",
        "edgefs_entry_kind_classification",
        "edgefs_path_validation",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
