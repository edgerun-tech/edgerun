import assert from "node:assert/strict";
import test from "node:test";
import { persistAdmittedNetworkAppRetrieval } from "./retrieval-flow.mjs";

test("retrieval flow builds admitted evidence and stores opaque records", async () => {
  const fake = fakeCoreHost();
  const store = new MemoryPackageStore();

  const result = await persistAdmittedNetworkAppRetrieval(
    fake.host,
    store,
    { packageKey: "apps/mail" },
    {
      manifestBytes: textBytes("manifest"),
      graphBytes: textBytes("graph"),
      developerSignatureBytes: textBytes("signature"),
      retrievalCost: 7,
      retrievedAt: 12,
      policyScheduleBytes: textBytes("schedule"),
      proofBytes: textBytes("proof"),
    },
  );

  assert.deepEqual([...result.retrievalEvidenceBytes], [...textBytes("retrieval-evidence")]);
  assert.deepEqual([...result.policyScheduleBytes], [...textBytes("schedule")]);
  assert.deepEqual([...result.browserAdmissionHash], [...new Uint8Array(32).fill(9)]);
  assert.deepEqual([...result.sourceAdmissionHash], [...new Uint8Array(32).fill(9)]);
  assert.deepEqual([...result.recordBytes], [...textBytes("retrieval-record")]);
  assert.equal(fake.calls.evidence.length, 2);
  assert.deepEqual(fake.calls.retrieval[0], {
    packageKey: "apps/mail",
    retrievalCost: 7n,
    retrievedAt: 12n,
    sourceAdmission: 9,
    retrievalEvidence: "retrieval-evidence",
    proof: "proof",
  });
  assert.deepEqual([...(await store.getPackage("apps/mail:manifest"))], [...textBytes("manifest")]);
  assert.deepEqual([...(await store.getRecord("apps/mail:retrieval"))], [...textBytes("retrieval-record")]);
  assert.deepEqual([...(await store.getRecord("apps/mail:retrieval-policy-schedule"))], [
    ...textBytes("schedule"),
  ]);
  assert.equal((await store.getRecord("apps/mail:source-admission")).byteLength, 32);
});

test("retrieval flow reuses caller-provided evidence", async () => {
  const fake = fakeCoreHost();
  const store = new MemoryPackageStore();

  await persistAdmittedNetworkAppRetrieval(
    fake.host,
    store,
    { packageKey: "apps/mail" },
    {
      manifestBytes: textBytes("manifest"),
      graphBytes: textBytes("graph"),
      developerSignatureBytes: textBytes("signature"),
      retrievalCost: 7,
      retrievedAt: 12,
      retrievalEvidenceBytes: textBytes("caller-evidence"),
      browserAdmissionHash: new Uint8Array(32).fill(4),
      browserWorkAdmissionBytes: textBytes("signed-browser-work-admission"),
      recordBytes: textBytes("caller-record"),
    },
  );

  assert.equal(fake.calls.evidence.length, 0);
  assert.equal(fake.calls.retrieval.length, 0);
  assert.deepEqual([...(await store.getRecord("apps/mail:retrieval"))], [...textBytes("caller-record")]);
  assert.deepEqual([...(await store.getRecord("apps/mail:browser-admission"))], [
    ...new Uint8Array(32).fill(4),
  ]);
  assert.deepEqual([...(await store.getRecord("apps/mail:source-admission"))], [
    ...new Uint8Array(32).fill(4),
  ]);
  assert.deepEqual([...(await store.getRecord("apps/mail:browser-work-admission"))], [
    ...textBytes("signed-browser-work-admission"),
  ]);
  assert.deepEqual([...(await store.getRecord("apps/mail:work-admission"))], [
    ...textBytes("signed-browser-work-admission"),
  ]);
});

function fakeCoreHost() {
  const memory = new WebAssembly.Memory({ initial: 1 });
  let bump = 16;
  const coreOutputs = new Set();
  const calls = { evidence: [], retrieval: [] };

  const alloc = (len) => {
    if (len === 0) {
      return 0;
    }
    const ptr = bump;
    bump += len + 8;
    return ptr;
  };
  const read = (ptr, len) => decoder.decode(new Uint8Array(memory.buffer, ptr, len));
  const writeOutput = (outPtr, outLen, payload) => {
    const payloadPtr = alloc(payload.byteLength);
    coreOutputs.add(payloadPtr);
    new Uint8Array(memory.buffer, payloadPtr, payload.byteLength).set(payload);
    const view = new DataView(memory.buffer);
    view.setUint32(outPtr, payloadPtr, true);
    view.setUint32(outLen, payload.byteLength, true);
    return 0;
  };

  return {
    calls,
    host: {
      exports: {
        memory,
        edgerun_browser_host_alloc: alloc,
        edgerun_browser_host_free: () => {},
        edgerun_browser_core_free: (ptr) => assert.equal(coreOutputs.has(ptr), true),
        edgerun_browser_core_package_retrieval: (...args) => {
          const outPtr = args.at(-2);
          const outLen = args.at(-1);
          calls.retrieval.push({
            packageKey: read(args[6], args[7]),
            retrievalCost: args[8],
            retrievedAt: args[9],
            sourceAdmission: new Uint8Array(memory.buffer, args[10], args[11])[0],
            retrievalEvidence: read(args[12], args[13]),
            proof: read(args[14], args[15]),
          });
          return writeOutput(outPtr, outLen, textBytes("retrieval-record"));
        },
        edgerun_browser_core_package_retrieval_evidence: (...args) => {
          const outPtr = args.at(-2);
          const outLen = args.at(-1);
          pushEvidenceCall(args);
          return writeOutput(outPtr, outLen, textBytes("retrieval-evidence"));
        },
        edgerun_browser_core_package_retrieval_admission_hash: (...args) => {
          const outPtr = args.at(-2);
          const outLen = args.at(-1);
          pushEvidenceCall(args);
          return writeOutput(outPtr, outLen, new Uint8Array(32).fill(9));
        },
      },
    },
  };

  function pushEvidenceCall(args) {
    calls.evidence.push({
      packageKey: read(args[6], args[7]),
      retrievalCost: args[8],
      requestedAt: args[9],
      policySchedule: read(args[10], args[11]),
    });
  }
}

class MemoryPackageStore {
  constructor() {
    this.packages = new Map();
    this.records = new Map();
  }

  async getPackage(key) {
    return this.packages.get(key) ?? null;
  }

  async putPackage(key, bytes) {
    this.packages.set(key, new Uint8Array(bytes));
  }

  async getRecord(key) {
    return this.records.get(key) ?? null;
  }

  async putRecord(key, bytes) {
    this.records.set(key, new Uint8Array(bytes));
  }
}

function textBytes(text) {
  return encoder.encode(text);
}

const encoder = new TextEncoder();
const decoder = new TextDecoder();
