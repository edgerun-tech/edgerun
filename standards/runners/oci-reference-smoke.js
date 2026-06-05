#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/oci-reference.wat";

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function put(memory, text, ptr = 1024) {
  const data = Buffer.from(text, "utf8");
  memory.fill(0, ptr, ptr + data.length + 64);
  memory.set(data, ptr);
  return [ptr, data.length];
}

function fields(memory, out) {
  const view = new DataView(memory.buffer, out, 36);
  return {
    registryStart: view.getUint32(0, true),
    registryLen: view.getUint32(4, true),
    repoStart: view.getUint32(8, true),
    repoLen: view.getUint32(12, true),
    tagStart: view.getUint32(16, true),
    tagLen: view.getUint32(20, true),
    digestStart: view.getUint32(24, true),
    digestLen: view.getUint32(28, true),
    kind: view.getUint32(32, true),
  };
}

function span(text, f, name) {
  const start = f[`${name}Start`];
  const len = f[`${name}Len`];
  return text.slice(start, start + len);
}

(async () => {
  const { instance } = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const out = 8192;
  const digest = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300090);

  let written = put(memory, digest);
  assert.strictEqual(e.oci_digest_validate(written[0], written[1]), 0);
  written = put(memory, digest.toUpperCase());
  assert.strictEqual(e.oci_digest_validate(written[0], written[1]), 1);
  written = put(memory, "sha512:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
  assert.strictEqual(e.oci_digest_validate(written[0], written[1]), 1);
  written = put(memory, "sha256:not-hex");
  assert.strictEqual(e.oci_digest_validate(written[0], written[1]), 1);

  let ref = "alpine:latest";
  written = put(memory, ref);
  memory.fill(0, out, out + 36);
  assert.strictEqual(e.oci_reference_scan(written[0], written[1], out), 0);
  let scanned = fields(memory, out);
  assert.strictEqual(scanned.kind, 1);
  assert.strictEqual(span(ref, scanned, "registry"), "");
  assert.strictEqual(span(ref, scanned, "repo"), "alpine");
  assert.strictEqual(span(ref, scanned, "tag"), "latest");

  ref = "docker.io/library/alpine:3.18";
  written = put(memory, ref);
  memory.fill(0, out, out + 36);
  assert.strictEqual(e.oci_reference_scan(written[0], written[1], out), 0);
  scanned = fields(memory, out);
  assert.strictEqual(scanned.kind, 1);
  assert.strictEqual(span(ref, scanned, "registry"), "docker.io");
  assert.strictEqual(span(ref, scanned, "repo"), "library/alpine");
  assert.strictEqual(span(ref, scanned, "tag"), "3.18");

  ref = `registry.example/ns/app@${digest}`;
  written = put(memory, ref);
  memory.fill(0, out, out + 36);
  assert.strictEqual(e.oci_reference_scan(written[0], written[1], out), 0);
  scanned = fields(memory, out);
  assert.strictEqual(scanned.kind, 2);
  assert.strictEqual(span(ref, scanned, "registry"), "registry.example");
  assert.strictEqual(span(ref, scanned, "repo"), "ns/app");
  assert.strictEqual(span(ref, scanned, "digest"), digest);

  ref = `registry.example/ns/app:v1@${digest}`;
  written = put(memory, ref);
  memory.fill(0, out, out + 36);
  assert.strictEqual(e.oci_reference_scan(written[0], written[1], out), 0);
  scanned = fields(memory, out);
  assert.strictEqual(scanned.kind, 3);
  assert.strictEqual(span(ref, scanned, "repo"), "ns/app");
  assert.strictEqual(span(ref, scanned, "tag"), "v1");
  assert.strictEqual(span(ref, scanned, "digest"), digest);

  ref = "Repo/Bad:latest";
  written = put(memory, ref);
  assert.strictEqual(e.oci_reference_scan(written[0], written[1], out), 1);
  ref = "alpine:bad tag";
  written = put(memory, ref);
  assert.strictEqual(e.oci_reference_scan(written[0], written[1], out), 1);

  written = put(memory, "application/vnd.oci.image.layer.v1.tar");
  assert.strictEqual(e.oci_media_layer_compression(written[0], written[1]), 0);
  written = put(memory, "application/vnd.oci.image.layer.v1.tar+gzip");
  assert.strictEqual(e.oci_media_layer_compression(written[0], written[1]), 1);
  written = put(memory, "application/vnd.oci.image.layer.v1.tar+zstd");
  assert.strictEqual(e.oci_media_layer_compression(written[0], written[1]), 2);
  written = put(memory, "application/vnd.docker.image.rootfs.diff.tar.gzip");
  assert.strictEqual(e.oci_media_layer_compression(written[0], written[1]), 1);
  written = put(memory, "application/vnd.example.layer");
  assert.strictEqual(e.oci_media_layer_compression(written[0], written[1]), 3);

  console.log(
    JSON.stringify({
      unit: "oci-reference",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      cases: [
        "valid_digest",
        "invalid_digest_uppercase_algorithm",
        "invalid_digest_algorithm",
        "invalid_digest_hex",
        "tag_ref",
        "registry_repo_tag_spans",
        "digest_ref",
        "tag_digest_ref",
        "invalid_repo",
        "invalid_tag",
        "media_uncompressed",
        "media_gzip",
        "media_zstd",
        "media_unknown",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
