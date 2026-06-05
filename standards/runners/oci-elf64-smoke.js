#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/oci-elf64.wat";

const OK = 0;
const UNSUPPORTED = 1;
const SHORT = 2;
const INVALID = 3;
const PT_LOAD = 1;
const PF_X = 1;
const PF_W = 2;
const PF_R = 4;

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function write(memory, ptr, bytes) {
  memory.fill(0, ptr, ptr + bytes.length + 128);
  memory.set(bytes, ptr);
}

function u16(buf, offset, value) {
  buf.writeUInt16LE(value, offset);
}

function u32(buf, offset, value) {
  buf.writeUInt32LE(value, offset);
}

function u64(buf, offset, value) {
  buf.writeBigUInt64LE(BigInt(value), offset);
}

function makeElf(overrides = {}) {
  const phoff = overrides.phoff ?? 64;
  const phnum = overrides.phnum ?? 1;
  const phentsize = overrides.phentsize ?? 56;
  const total = Math.max(64, phoff + phnum * phentsize);
  const elf = Buffer.alloc(total);
  elf.set([0x7f, 0x45, 0x4c, 0x46], 0);
  elf[4] = overrides.class ?? 2;
  elf[5] = overrides.endian ?? 1;
  elf[6] = overrides.version ?? 1;
  u16(elf, 16, overrides.type ?? 2);
  u16(elf, 18, overrides.machine ?? 62);
  u64(elf, 24, overrides.entry ?? 0x401000n);
  u64(elf, 32, phoff);
  u16(elf, 54, phentsize);
  u16(elf, 56, phnum);

  if (phnum > 0 && phentsize >= 56 && phoff + 56 <= elf.length) {
    const ph = overrides.ph ?? {};
    u32(elf, phoff + 0, ph.kind ?? PT_LOAD);
    u32(elf, phoff + 4, ph.flags ?? (PF_R | PF_X));
    u64(elf, phoff + 8, ph.offset ?? 0n);
    u64(elf, phoff + 16, ph.vaddr ?? 0x400000n);
    u64(elf, phoff + 24, ph.paddr ?? 0x400000n);
    u64(elf, phoff + 32, ph.filesz ?? 0x2000n);
    u64(elf, phoff + 40, ph.memsz ?? 0x2000n);
    u64(elf, phoff + 48, ph.align ?? 0x1000n);
  }
  return elf;
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = module.instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const view = new DataView(e.memory.buffer);
  const inPtr = 1024;
  const outPtr = 4096;

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300084);

  const valid = makeElf();
  write(memory, inPtr, valid);
  assert.equal(e.elf64_header_parse(inPtr, valid.length, outPtr), OK);
  assert.equal(view.getUint32(outPtr + 0, true), 2);
  assert.equal(view.getUint32(outPtr + 4, true), 62);
  assert.equal(view.getBigUint64(outPtr + 8, true), 0x401000n);
  assert.equal(view.getBigUint64(outPtr + 16, true), 64n);
  assert.equal(view.getUint32(outPtr + 24, true), 56);
  assert.equal(view.getUint32(outPtr + 28, true), 1);

  assert.equal(e.elf64_program_header_parse(inPtr, valid.length, 0, outPtr), OK);
  assert.equal(view.getUint32(outPtr + 0, true), PT_LOAD);
  assert.equal(view.getUint32(outPtr + 4, true), PF_R | PF_X);
  assert.equal(view.getBigUint64(outPtr + 8, true), 0n);
  assert.equal(view.getBigUint64(outPtr + 16, true), 0x400000n);
  assert.equal(view.getBigUint64(outPtr + 24, true), 0x2000n);
  assert.equal(view.getBigUint64(outPtr + 32, true), 0x2000n);
  assert.equal(view.getBigUint64(outPtr + 40, true), 0x1000n);
  assert.equal(e.elf64_entry_executable(inPtr, valid.length), 1);

  const badMagic = Buffer.from(valid);
  badMagic[0] = 0;
  write(memory, inPtr, badMagic);
  assert.equal(e.elf64_header_parse(inPtr, badMagic.length, outPtr), INVALID);
  assert.equal(e.elf64_entry_executable(inPtr, badMagic.length), 0);

  const wrongClass = makeElf({ class: 1 });
  write(memory, inPtr, wrongClass);
  assert.equal(e.elf64_header_parse(inPtr, wrongClass.length, outPtr), UNSUPPORTED);

  const badPhentsize = makeElf({ phentsize: 48 });
  write(memory, inPtr, badPhentsize);
  assert.equal(e.elf64_header_parse(inPtr, badPhentsize.length, outPtr), UNSUPPORTED);

  write(memory, inPtr, valid.subarray(0, 63));
  assert.equal(e.elf64_header_parse(inPtr, 63, outPtr), SHORT);

  write(memory, inPtr, valid);
  assert.equal(e.elf64_program_header_parse(inPtr, valid.length, 1, outPtr), INVALID);

  const truncatedPh = valid.subarray(0, valid.length - 1);
  write(memory, inPtr, truncatedPh);
  assert.equal(e.elf64_program_header_parse(inPtr, truncatedPh.length, 0, outPtr), SHORT);

  const notExecutable = makeElf({ ph: { flags: PF_R } });
  write(memory, inPtr, notExecutable);
  assert.equal(e.elf64_entry_executable(inPtr, notExecutable.length), 0);

  const entryOutside = makeElf({ entry: 0x500000n });
  write(memory, inPtr, entryOutside);
  assert.equal(e.elf64_entry_executable(inPtr, entryOutside.length), 0);

  assert.equal(e.elf64_flags_permissions(PF_R), 1);
  assert.equal(e.elf64_flags_permissions(PF_W), 2);
  assert.equal(e.elf64_flags_permissions(PF_X), 4);
  assert.equal(e.elf64_flags_permissions(PF_R | PF_W | PF_X), 7);

  console.log(
    JSON.stringify({
      unit: "oci-elf64",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      cases: [
        "valid_x86_64_executable",
        "program_header_parse",
        "bad_magic",
        "wrong_class",
        "bad_phentsize",
        "short_header",
        "index_out_of_range",
        "short_program_header",
        "permission_bits",
      ],
    })
  );
})();
