#!/usr/bin/env node

const fs = require("fs");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/sse-event-stream.wat";
const wasmPath = process.argv[3] || "/tmp/sse-event-stream-smoke.wasm";

function status(packed) {
  return Number(packed & 0xffffffffn);
}

function count(packed) {
  return Number(packed >> 32n);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function write(memory, ptr, bytes) {
  memory.fill(0, ptr, ptr + bytes.length + 512);
  memory.set(bytes, ptr);
}

function readString(memory, ptr, len) {
  return Buffer.from(memory.slice(ptr, ptr + len)).toString("utf8");
}

function record(view, base, index) {
  const off = base + index * 40;
  return {
    eventPtr: view.getUint32(off, true),
    eventLen: view.getUint32(off + 4, true),
    dataPtr: view.getUint32(off + 8, true),
    dataLen: view.getUint32(off + 12, true),
    idPtr: view.getUint32(off + 16, true),
    idLen: view.getUint32(off + 20, true),
    retryLo: view.getUint32(off + 24, true),
    retryHi: view.getUint32(off + 28, true),
    flags: view.getUint32(off + 32, true),
    endOffset: view.getUint32(off + 36, true),
  };
}

function parse(exports, input, options = {}) {
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);
  const inPtr = options.inPtr || 1024;
  const lastPtr = options.lastPtr || 8192;
  const recPtr = options.recPtr || 12288;
  const textPtr = options.textPtr || 20480;
  const last = Buffer.from(options.last || "", "utf8");
  const recCap = options.recCap ?? 8;
  const textCap = options.textCap ?? 8192;
  const bytes = Buffer.isBuffer(input) ? input : Buffer.from(input, "utf8");

  write(memory, inPtr, bytes);
  write(memory, lastPtr, last);
  memory.fill(0, recPtr, recPtr + recCap * 40 + 512);
  memory.fill(0, textPtr, textPtr + textCap + 512);

  const packed = exports.sse_parse_events(
    inPtr,
    bytes.length,
    lastPtr,
    last.length,
    recPtr,
    recCap,
    textPtr,
    textCap,
  );
  const n = count(packed);
  const events = [];
  for (let i = 0; i < n; i += 1) {
    const rec = record(view, recPtr, i);
    events.push({
      event: readString(memory, rec.eventPtr, rec.eventLen),
      data: readString(memory, rec.dataPtr, rec.dataLen),
      id: readString(memory, rec.idPtr, rec.idLen),
      retry: rec.flags & 1 ? rec.retryLo : null,
      flags: rec.flags,
      endOffset: rec.endOffset,
    });
  }
  return { status: status(packed), count: n, events };
}

(async () => {
  execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "inherit" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });

  const module = await WebAssembly.instantiate(fs.readFileSync(wasmPath), {});
  const exports = module.instance.exports;

  assert(exports.proto_abi_version() === 2, "unexpected ABI version");
  assert(exports.proto_standard_id() === 300072, "unexpected standard id");

  const cases = [];

  let out = parse(exports, "data: hello\n\n");
  assert(out.status === 0 && out.count === 1, "data-only event count");
  assert(out.events[0].event === "message", "data-only default event");
  assert(out.events[0].data === "hello", "data-only payload");
  assert(out.events[0].id === "", "data-only empty id");
  cases.push("data-only");

  out = parse(exports, "data: first\r\ndata: second\r\n\r\n");
  assert(out.status === 0 && out.count === 1, "multi-line event count");
  assert(out.events[0].data === "first\nsecond", "multi-line data join");
  cases.push("multi-line-data");

  out = parse(exports, "event: update\ndata: body\n\n");
  assert(out.events[0].event === "update", "event type");
  assert(out.events[0].data === "body", "event type data");
  cases.push("event-type");

  out = parse(exports, "id: first\ndata: one\n\ndata: two\n\nid:\ndata: three\n\n", {
    last: "old",
  });
  assert(out.status === 0 && out.count === 3, "id carry count");
  assert(out.events[0].id === "first", "id update");
  assert(out.events[1].id === "first", "id carry");
  assert(out.events[2].id === "", "empty id clears");
  cases.push("id-carry-update-clear");

  out = parse(exports, ": ignored comment\nunknown: nope\ndata: kept\n\n");
  assert(out.status === 0 && out.count === 1, "comment event count");
  assert(out.events[0].data === "kept", "comment and unknown ignored");
  cases.push("comments-unknown-fields");

  out = parse(exports, "retry: 1500\ndata: wait\n\nretry: nope\ndata: no-retry\n\n");
  assert(out.status === 0 && out.count === 2, "retry event count");
  assert(out.events[0].retry === 1500, "numeric retry");
  assert(out.events[1].retry === null, "invalid retry ignored");
  cases.push("retry");

  out = parse(exports, "data:test\n\n");
  assert(out.events[0].data === "test", "colon without leading space");
  cases.push("optional-space-after-colon");

  out = parse(exports, "event: final\ndata: eof");
  assert(out.status === 0 && out.count === 1, "EOF final frame count");
  assert(out.events[0].event === "final", "EOF final frame event");
  assert(out.events[0].data === "eof", "EOF final frame data");
  cases.push("final-frame-eof");

  out = parse(exports, Buffer.from([0x64, 0x61, 0x74, 0x61, 0x3a, 0x20, 0xc3, 0x28, 0x0a, 0x0a]));
  assert(out.status === 3 && out.count === 0, "malformed UTF-8 rejected");
  cases.push("malformed-utf8");

  out = parse(exports, "data: one\n\ndata: two\n\n", { recCap: 1 });
  assert(out.status === 2 && out.count === 1, "record cap");
  cases.push("record-cap");

  out = parse(exports, "data: too long\n\n", { textCap: 4 });
  assert(out.status === 2 && out.count === 0, "text cap");
  cases.push("text-cap");

  console.log(
    JSON.stringify({
      unit: "sse-event-stream",
      abi: exports.proto_abi_version(),
      standard_id: exports.proto_standard_id(),
      ok: true,
      cases,
    }),
  );
})();
