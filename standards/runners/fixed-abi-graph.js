#!/usr/bin/env node

const fs = require("fs");
const crypto = require("crypto");

const graphPath = process.argv[2] || "standards/build/wasm/udp-tftp-fixed-abi/graph.json";
const corpusDir = process.argv[3] || "standards/corpus/udp-tftp";

const graph = JSON.parse(fs.readFileSync(graphPath, "utf8"));
const nodeById = Object.fromEntries(graph.nodes.map((node) => [node.id, node]));

const cases = [
  ["valid-ack", "valid-ack.hex", { udp: 0, tftp: 0, opcode: 4, bits: 7 }],
  ["invalid-udp-length", "invalid-length.hex", { udp: 2, tftp: null }],
  ["invalid-ack-length", "invalid-ack-length.hex", { udp: 0, tftp: 2, opcode: 4, bits: 5 }],
  ["invalid-data-length", "invalid-data-length.hex", { udp: 0, tftp: 2, opcode: 3, bits: 3 }],
  ["invalid-opcode", "invalid-opcode.hex", { udp: 0, tftp: 2, opcode: 9, bits: 6 }],
];

function readHex(path) {
  return Buffer.from(fs.readFileSync(path, "utf8").replace(/\s+/g, ""), "hex");
}

function writeU16LE(buf, offset, value) {
  buf[offset] = value & 0xff;
  buf[offset + 1] = (value >>> 8) & 0xff;
}

function writeU32LE(buf, offset, value) {
  buf[offset] = value & 0xff;
  buf[offset + 1] = (value >>> 8) & 0xff;
  buf[offset + 2] = (value >>> 16) & 0xff;
  buf[offset + 3] = (value >>> 24) & 0xff;
}

function inputDatagramFrame(payload) {
  const frame = Buffer.alloc(8 + payload.length);
  writeU16LE(frame, 0, 3);
  writeU16LE(frame, 2, 0);
  writeU32LE(frame, 4, payload.length);
  payload.copy(frame, 8);
  return frame;
}

function call(unit, payload) {
  const frame = inputDatagramFrame(payload);
  const memory = new Uint8Array(unit.exports.memory.buffer);
  memory.set(frame, 1024);
  const packed = unit.exports.proto_push(unit.handle, 1024, frame.length);
  const ptr = Number(packed >> 32n);
  const len = Number(packed & 0xffffffffn);
  return Buffer.from(memory.slice(ptr, ptr + len));
}

async function instantiate(id) {
  const spec = nodeById[id];
  if (!spec) {
    throw new Error(`graph is missing node ${id}`);
  }
  const module = await WebAssembly.instantiate(fs.readFileSync(spec.wasm), {});
  const exports = module.instance.exports;
  return {
    exports,
    handle: exports.proto_open(0, 0),
  };
}

function digestFromEvent(frame, digestLen) {
  const kind = frame.readUInt16LE(0);
  const status = frame.readUInt16LE(2);
  const len = frame.readUInt32LE(4);
  if (kind !== 11 || status !== 0 || len !== digestLen) {
    return null;
  }
  return frame.slice(8, 8 + digestLen).toString("hex");
}

(async () => {
  const udp = await instantiate("udp-rfc768");
  const tftp = await instantiate("tftp-rfc1350");
  const sha256 = nodeById["sha256-fips180"] ? await instantiate("sha256-fips180") : null;

  const result = {
    graph: graph.id,
    sha256: graph.sha256,
    abi_version: 1,
    hashed: Boolean(sha256),
    ok: true,
    cases: [],
  };

  for (const [id, file, expect] of cases) {
    const datagram = readHex(`${corpusDir}/${file}`);
    const datagramSha256 = sha256 ? digestFromEvent(call(sha256, datagram), 32) : null;
    const udpOut = call(udp, datagram);
    const udpStatus = udpOut.readUInt16LE(8);
    const src = udpOut.readUInt16LE(10);
    const dst = udpOut.readUInt16LE(12);
    const payloadOffset = udpOut.readUInt16LE(16);
    const payloadLen = udpOut.readUInt16LE(18);
    const udpBits = udpOut.readUInt16LE(20);

    let tftpStatus = null;
    let opcode = null;
    let tftpBits = null;
    let payloadSha256 = null;
    if (udpStatus === 0 && (src === 69 || dst === 69)) {
      const payload = datagram.slice(payloadOffset, payloadOffset + payloadLen);
      payloadSha256 = sha256 ? digestFromEvent(call(sha256, payload), 32) : null;
      const tftpOut = call(tftp, payload);
      tftpStatus = tftpOut.readUInt16LE(8);
      opcode = tftpOut.readUInt16LE(10);
      tftpBits = tftpOut.readUInt16LE(14);
    }

    const ok =
      udpStatus === expect.udp &&
      udpBits === (expect.udp === 0 ? 1 : 0) &&
      tftpStatus === expect.tftp &&
      (expect.opcode == null || opcode === expect.opcode) &&
      (expect.bits == null || tftpBits === expect.bits) &&
      (!sha256 || datagramSha256 === crypto.createHash("sha256").update(datagram).digest("hex")) &&
      (!sha256 ||
        payloadSha256 === null ||
        payloadSha256 ===
          crypto
            .createHash("sha256")
            .update(datagram.slice(payloadOffset, payloadOffset + payloadLen))
            .digest("hex"));

    result.ok &&= ok;
    result.cases.push({
      id,
      ok,
      udpStatus,
      udpBits,
      datagramSha256,
      tftpStatus,
      opcode,
      tftpBits,
      payloadSha256,
    });
  }

  console.log(JSON.stringify(result, null, 2));
  process.exit(result.ok ? 0 : 1);
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
