#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { spawnSync } = require("child_process");

const root = path.resolve(__dirname, "../..");
const watRoot = path.join(root, "standards/build/wasm/codec-primitives");
const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "codec-composition-tls-clienthello-name-"));
const wat2wasm = resolveTool("wat2wasm");
const wasmValidate = resolveTool("wasm-validate");

process.on("exit", () => {
  fs.rmSync(tmpRoot, { recursive: true, force: true });
});

function resolveTool(name) {
  for (const dir of (process.env.PATH || "").split(path.delimiter)) {
    const candidate = path.join(dir, name);
    try {
      fs.accessSync(candidate, fs.constants.X_OK);
      return candidate;
    } catch (_) {
      // Keep scanning PATH.
    }
  }
  return name;
}

function compileWat(name) {
  const watPath = path.join(watRoot, `${name}.wat`);
  const wasmPath = path.join(tmpRoot, `${name}.wasm`);
  fs.accessSync(watPath, fs.constants.R_OK);
  runTool(wat2wasm, [watPath, "-o", wasmPath]);
  runTool(wasmValidate, [wasmPath]);
  return fs.readFileSync(wasmPath);
}

function runTool(file, args) {
  const result = spawnSync(file, args, { stdio: "pipe" });
  if (result.status !== 0) {
    throw result.error || new Error(`${file} failed with status ${result.status}`);
  }
}

async function instantiate(name) {
  const { instance } = await WebAssembly.instantiate(compileWat(name), {});
  return {
    exports: instance.exports,
    memory: new Uint8Array(instance.exports.memory.buffer),
  };
}

function u16(n) {
  return [(n >>> 8) & 0xff, n & 0xff];
}

function u24(n) {
  return [(n >>> 16) & 0xff, (n >>> 8) & 0xff, n & 0xff];
}

function u32(memory, offset) {
  return (
    memory[offset] |
    (memory[offset + 1] << 8) |
    (memory[offset + 2] << 16) |
    (memory[offset + 3] << 24)
  ) >>> 0;
}

function tlsRecordParts(value) {
  return {
    status: Number(value & 0xffffn),
    contentType: Number((value >> 16n) & 0xffn),
    version: Number((value >> 24n) & 0xffffn),
    fragmentLen: Number((value >> 40n) & 0xffffn),
  };
}

function tlsHandshakeParts(value) {
  return {
    status: Number(value & 0xffffn),
    handshakeType: Number((value >> 16n) & 0xffn),
    bodyLen: Number((value >> 24n) & 0xffffffn),
  };
}

function clientHelloRecord(memory, offset) {
  return {
    extensionsOffset: u32(memory, offset + 40),
    extensionsLen: u32(memory, offset + 44),
  };
}

function span3(memory, offset) {
  return {
    dataOffset: u32(memory, offset),
    dataLen: u32(memory, offset + 4),
    nextOffset: u32(memory, offset + 8),
  };
}

function extRecord(memory, offset) {
  return {
    type: u32(memory, offset),
    dataOffset: u32(memory, offset + 4),
    dataLen: u32(memory, offset + 8),
    nextOffset: u32(memory, offset + 12),
  };
}

function packed32(value) {
  return {
    status: Number(value & 0xffffffffn),
    value: Number((value >> 32n) & 0xffffffffn),
  };
}

function ext(type, data) {
  return [...u16(type), ...u16(data.length), ...data];
}

function sni(host) {
  const hostBytes = [...Buffer.from(host, "ascii")];
  const name = [0, ...u16(hostBytes.length), ...hostBytes];
  return [...u16(name.length), ...name];
}

function clientHelloBody(host) {
  const random = Array.from({ length: 32 }, (_, i) => i);
  const ciphers = [0x13, 0x01, 0x13, 0x02];
  const compression = [0];
  const extensions = ext(0, sni(host));
  return [
    0x03, 0x03,
    ...random,
    0,
    ...u16(ciphers.length), ...ciphers,
    compression.length, ...compression,
    ...u16(extensions.length), ...extensions,
  ];
}

(async () => {
  const tlsFrame = await instantiate("tls-frame");
  const clientHello = await instantiate("tls-clienthello");
  const tlsVector = await instantiate("tls-vector");
  const tlsName = await instantiate("tls-name");

  assert.strictEqual(tlsFrame.exports.proto_abi_version(), 2);
  assert.strictEqual(clientHello.exports.proto_abi_version(), 2);
  assert.strictEqual(tlsVector.exports.proto_abi_version(), 2);
  assert.strictEqual(tlsName.exports.proto_abi_version(), 2);

  const body = clientHelloBody("Example.COM.");
  const record = [0x16, 0x03, 0x03, ...u16(body.length + 4), 0x01, ...u24(body.length), ...body];

  tlsFrame.memory.set(record, 1024);
  const recordHeader = tlsRecordParts(tlsFrame.exports.tls_record_header_decode(1024, record.length));
  assert.deepStrictEqual(recordHeader, {
    status: 0,
    contentType: 0x16,
    version: 0x0303,
    fragmentLen: body.length + 4,
  });

  const handshake = tlsHandshakeParts(tlsFrame.exports.tls_handshake_header_decode(1029, recordHeader.fragmentLen));
  assert.deepStrictEqual(handshake, { status: 0, handshakeType: 1, bodyLen: body.length });

  clientHello.memory.set(record.slice(9), 1024);
  assert.strictEqual(clientHello.exports.tls_clienthello_scan(1024, handshake.bodyLen, 2048), 0);
  const ch = clientHelloRecord(clientHello.memory, 2048);
  const extensions = Array.from(clientHello.memory.slice(1024 + ch.extensionsOffset, 1024 + ch.extensionsOffset + ch.extensionsLen));

  tlsVector.memory.set(extensions, 1024);
  assert.strictEqual(tlsVector.exports.tls_extension_next(1024, extensions.length, 0, 2048), 0);
  const extension = extRecord(tlsVector.memory, 2048);
  assert.deepStrictEqual(
    { type: extension.type, dataLen: extension.dataLen, nextOffset: extension.nextOffset },
    { type: 0, dataLen: extensions.length - 4, nextOffset: extensions.length },
  );

  assert.strictEqual(tlsVector.exports.tls_vector_u16_decode(1024 + extension.dataOffset, extension.dataLen, 2048), 0);
  const nameList = span3(tlsVector.memory, 2048);
  assert.strictEqual(tlsVector.memory[1024 + extension.dataOffset + nameList.dataOffset], 0);

  const hostVectorPtr = 1024 + extension.dataOffset + nameList.dataOffset + 1;
  assert.strictEqual(tlsVector.exports.tls_vector_u16_decode(hostVectorPtr, nameList.dataLen - 1, 2048), 0);
  const host = span3(tlsVector.memory, 2048);
  const hostBytes = Array.from(tlsVector.memory.slice(hostVectorPtr + host.dataOffset, hostVectorPtr + host.dataOffset + host.dataLen));

  tlsName.memory.set(hostBytes, 1024);
  const normalized = packed32(tlsName.exports.tls_dns_name_normalize(1024, hostBytes.length, 2048, 256));
  assert.deepStrictEqual(normalized, { status: 0, value: "example.com".length });
  assert.strictEqual(Buffer.from(tlsName.memory.slice(2048, 2048 + normalized.value)).toString("ascii"), "example.com");

  console.log(JSON.stringify({
    runner: "codec-composition-tls-clienthello-name",
    pipeline: "tls-frame -> tls-clienthello -> tls-vector -> tls-name",
    ok: true,
    host: "example.com",
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
