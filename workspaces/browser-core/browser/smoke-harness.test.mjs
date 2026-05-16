import assert from "node:assert/strict";
import test from "node:test";
import { runBrowserCoreSmokeHarness } from "./smoke-harness.mjs";

test("smoke harness reads Rust fixture bytes and drives the host adapter", async () => {
  const fake = fakeSmokeExports();
  const store = new MemoryByteStore();
  const result = await runBrowserCoreSmokeHarness(fake.instance, {
    store,
  });

  assert.equal(result.abiVersion, 1);
  assert.deepEqual([...result.firstRun], [...bytes("first-run-projection-result")]);
  assert.deepEqual([...result.firstRunProjection], [...bytes("projection-bundle")]);
  assert.deepEqual([...result.storage], [...bytes("storage-result")]);
  assert.deepEqual([...(await store.getRecord("smoke:session"))], [...bytes("session-result")]);
  assert.deepEqual([...(await store.getPackage("smoke:package:manifest"))], [...bytes("manifest")]);
  assert.equal(fake.calls.opened, 1);
  assert.equal(fake.calls.closed, 1);
  assert.deepEqual(fake.calls.input[0], {
    profile: 1,
    runtime: "runtime-id",
    previous: 0,
    firstEventSeq: 0n,
    eventTime: 10n,
    retrievalCost: 5n,
    sourceAdmission: 3,
    decision: 2,
    userSignature: "user",
  });
  assert.deepEqual(fake.calls.coreProjection[0], ["manifest", "graph", "signature", "input-record"]);
  assert.deepEqual(fake.calls.hostInputs, [
    "projection-bundle",
    "grant",
    "storage-binding",
    "session",
    "storage-request",
    "storage-invocation",
  ]);
});

test("smoke harness uses the selected first-run decision", async () => {
  const fake = fakeSmokeExports();
  await runBrowserCoreSmokeHarness(fake.instance, {
    store: new MemoryByteStore(),
    decision: "run-once",
  });

  assert.equal(fake.calls.input[0].decision, 1);
});

test("smoke harness accepts selected package bytes", async () => {
  const fake = fakeSmokeExports();
  await runBrowserCoreSmokeHarness(fake.instance, {
    store: new MemoryByteStore(),
    packageBytes: {
      manifestBytes: bytes("selected-manifest"),
      graphBytes: bytes("selected-graph"),
      developerSignatureBytes: bytes("selected-signature"),
    },
  });

  assert.deepEqual(fake.calls.coreProjection[0], [
    "selected-manifest",
    "selected-graph",
    "selected-signature",
    "input-record",
  ]);
});

test("smoke harness stores retrieval result through Rust retrieval record builder", async () => {
  const fake = fakeSmokeExports();
  const store = new MemoryByteStore();
  await runBrowserCoreSmokeHarness(fake.instance, {
    store,
    packageSelection: { packageKey: "apps/selected" },
    retrievalResult: {
      manifestBytes: bytes("retrieved-manifest"),
      graphBytes: bytes("retrieved-graph"),
      developerSignatureBytes: bytes("retrieved-signature"),
      retrievalCost: 8,
      retrievedAt: 14,
      sourceAdmissionHash: new Uint8Array(32).fill(4),
      retrievalEvidenceBytes: bytes("retrieval-evidence"),
      proofBytes: bytes("proof"),
    },
  });

  assert.deepEqual(fake.calls.coreRetrieval[0], {
    manifest: "retrieved-manifest",
    graph: "retrieved-graph",
    signature: "retrieved-signature",
    packageKey: "apps/selected",
    retrievalCost: 8n,
    retrievedAt: 14n,
    sourceAdmission: 4,
    retrievalEvidence: "retrieval-evidence",
    proof: "proof",
  });
  assert.deepEqual(
    [...(await store.getRecord("apps/selected:retrieval"))],
    [...bytes("retrieval-record")],
  );
  assert.deepEqual(fake.calls.coreProjection[0], [
    "retrieved-manifest",
    "retrieved-graph",
    "retrieved-signature",
    "input-record",
  ]);
});

test("smoke harness builds admitted retrieval evidence when it is not supplied", async () => {
  const fake = fakeSmokeExports();
  const store = new MemoryByteStore();
  await runBrowserCoreSmokeHarness(fake.instance, {
    store,
    packageSelection: { packageKey: "apps/admitted" },
    retrievalResult: {
      manifestBytes: bytes("retrieved-manifest"),
      graphBytes: bytes("retrieved-graph"),
      developerSignatureBytes: bytes("retrieved-signature"),
      retrievalCost: 9,
      retrievedAt: 15,
      proofBytes: bytes("proof"),
    },
  });

  assert.equal(fake.calls.coreEvidence.length, 2);
  assert.equal(fake.calls.coreEvidence[0].packageKey, "apps/admitted");
  assert.deepEqual(
    [...(await store.getRecord("apps/admitted:proof"))],
    [...bytes("proof")],
  );
  assert.deepEqual(
    [...(await store.getRecord("apps/admitted:source-admission"))],
    [...new Uint8Array(32).fill(9)],
  );
  assert.equal(fake.calls.coreRetrieval[0].sourceAdmission, 9);
  assert.equal(fake.calls.coreRetrieval[0].retrievalEvidence, "retrieval-evidence-record");
});

class MemoryByteStore {
  constructor() {
    this.records = new Map();
    this.packages = new Map();
  }

  async putRecord(key, bytes) {
    this.records.set(key, new Uint8Array(bytes));
  }

  async getRecord(key) {
    return this.records.get(key) ?? null;
  }

  async putPackage(key, bytes) {
    this.packages.set(key, new Uint8Array(bytes));
  }

  async getPackage(key) {
    return this.packages.get(key) ?? null;
  }
}

function fakeSmokeExports() {
  const memory = new WebAssembly.Memory({ initial: 1 });
  let bump = 16;
  let nextHandle = 0x2000;
  const coreOutputs = new Set();
  const calls = {
    input: [],
    coreEvidence: [],
    coreRetrieval: [],
    coreProjection: [],
    hostInputs: [],
    opened: 0,
    closed: 0,
  };

  const alloc = (len) => {
    if (len === 0) {
      return 0;
    }
    const ptr = bump;
    bump += len + 8;
    return ptr;
  };
  const read = (ptr, len) => decoder.decode(new Uint8Array(memory.buffer, ptr, len));
  const writeOutput = (outPtr, outLen, text, trackCoreOutput = false) => {
    const payload = bytes(text);
    const payloadPtr = alloc(payload.byteLength);
    if (trackCoreOutput) {
      coreOutputs.add(payloadPtr);
    }
    new Uint8Array(memory.buffer, payloadPtr, payload.byteLength).set(payload);
    const view = new DataView(memory.buffer);
    view.setUint32(outPtr, payloadPtr, true);
    view.setUint32(outLen, payload.byteLength, true);
    return 0;
  };
  const coreFixture = (text) => (outPtr, outLen) => writeOutput(outPtr, outLen, text, true);
  const hostResult = (inputText, resultText) => (...args) => {
    const inputPtr = args[1];
    const inputLen = args[2];
    const outPtr = args.at(-2);
    const outLen = args.at(-1);
    calls.hostInputs.push(read(inputPtr, inputLen));
    return writeOutput(outPtr, outLen, resultText);
  };

  return {
    calls,
    instance: {
      exports: {
        memory,
        edgerun_browser_host_abi_version: () => 1,
        edgerun_browser_host_status_ok: () => 0,
        edgerun_browser_host_alloc: alloc,
        edgerun_browser_host_free: () => {},
        edgerun_browser_host_new: (ptr, len) => {
          assert.equal(read(ptr, len), "runtime-id");
          calls.opened += 1;
          return nextHandle++;
        },
        edgerun_browser_host_drop: () => {
          calls.closed += 1;
        },
        edgerun_browser_host_record_first_run: (
          handle,
          projectionPtr,
          projectionLen,
          decisionPtr,
          decisionLen,
          cachePtr,
          cacheLen,
          hasCache,
          time,
          outPtr,
          outLen,
        ) => {
          calls.hostInputs.push(read(projectionPtr, projectionLen));
          calls.hostInputs.push(read(decisionPtr, decisionLen));
          calls.hostInputs.push(read(cachePtr, cacheLen));
          assert.equal(hasCache, 1);
          assert.equal(time, 11);
          return writeOutput(outPtr, outLen, "first-run-result");
        },
        edgerun_browser_host_record_first_run_projection: hostResult(
          "projection-bundle",
          "first-run-projection-result",
        ),
        edgerun_browser_host_grant: hostResult("grant", "grant-result"),
        edgerun_browser_host_bind_storage: hostResult(
          "storage-binding",
          "binding-result",
        ),
        edgerun_browser_host_open_session: hostResult("session", "session-result"),
        edgerun_browser_host_invoke_storage: (
          handle,
          requestPtr,
          requestLen,
          invocationPtr,
          invocationLen,
          outPtr,
          outLen,
        ) => {
          calls.hostInputs.push(read(requestPtr, requestLen));
          calls.hostInputs.push(read(invocationPtr, invocationLen));
          return writeOutput(outPtr, outLen, "storage-result");
        },
        edgerun_browser_core_free: (ptr) => {
          assert.equal(coreOutputs.has(ptr), true);
        },
        edgerun_browser_core_app_run_decision_run_once: () => 1,
        edgerun_browser_core_app_run_decision_verify_and_cache: () => 2,
        edgerun_browser_core_app_run_decision_cancel: () => 3,
        edgerun_browser_core_smoke_runtime_id: coreFixture("runtime-id"),
        edgerun_browser_core_smoke_app_manifest: coreFixture("manifest"),
        edgerun_browser_core_smoke_app_graph: coreFixture("graph"),
        edgerun_browser_core_smoke_developer_signature: coreFixture("signature"),
        edgerun_browser_core_first_run_input: (
          profilePtr,
          profileLen,
          runtimePtr,
          runtimeLen,
          previousPtr,
          previousLen,
          firstEventSeq,
          eventTime,
          retrievalCost,
          sourceAdmissionPtr,
          sourceAdmissionLen,
          decision,
          userSignaturePtr,
          userSignatureLen,
          outPtr,
          outLen,
        ) => {
          calls.input.push({
            profile: new Uint8Array(memory.buffer, profilePtr, profileLen)[0],
            runtime: read(runtimePtr, runtimeLen),
            previous: new Uint8Array(memory.buffer, previousPtr, previousLen)[0],
            firstEventSeq,
            eventTime,
            retrievalCost,
            sourceAdmission: new Uint8Array(
              memory.buffer,
              sourceAdmissionPtr,
              sourceAdmissionLen,
            )[0],
            decision,
            userSignature: read(userSignaturePtr, userSignatureLen),
          });
          return writeOutput(outPtr, outLen, "input-record", true);
        },
        edgerun_browser_core_package_retrieval: (
          manifestPtr,
          manifestLen,
          graphPtr,
          graphLen,
          signaturePtr,
          signatureLen,
          packageKeyPtr,
          packageKeyLen,
          retrievalCost,
          retrievedAt,
          sourceAdmissionPtr,
          sourceAdmissionLen,
          retrievalEvidencePtr,
          retrievalEvidenceLen,
          proofPtr,
          proofLen,
          outPtr,
          outLen,
        ) => {
          calls.coreRetrieval.push({
            manifest: read(manifestPtr, manifestLen),
            graph: read(graphPtr, graphLen),
            signature: read(signaturePtr, signatureLen),
            packageKey: read(packageKeyPtr, packageKeyLen),
            retrievalCost,
            retrievedAt,
            sourceAdmission: new Uint8Array(
              memory.buffer,
              sourceAdmissionPtr,
              sourceAdmissionLen,
            )[0],
            retrievalEvidence: read(retrievalEvidencePtr, retrievalEvidenceLen),
            proof: read(proofPtr, proofLen),
          });
          return writeOutput(outPtr, outLen, "retrieval-record", true);
        },
        edgerun_browser_core_package_retrieval_evidence: (...args) => {
          const outPtr = args.at(-2);
          const outLen = args.at(-1);
          calls.coreEvidence.push({
            manifest: read(args[0], args[1]),
            graph: read(args[2], args[3]),
            signature: read(args[4], args[5]),
            packageKey: read(args[6], args[7]),
            retrievalCost: args[8],
            requestedAt: args[9],
          });
          return writeOutput(outPtr, outLen, "retrieval-evidence-record", true);
        },
        edgerun_browser_core_package_retrieval_admission_hash: (...args) => {
          const outPtr = args.at(-2);
          const outLen = args.at(-1);
          calls.coreEvidence.push({
            manifest: read(args[0], args[1]),
            graph: read(args[2], args[3]),
            signature: read(args[4], args[5]),
            packageKey: read(args[6], args[7]),
            retrievalCost: args[8],
            requestedAt: args[9],
          });
          const payload = new Uint8Array(32).fill(9);
          const payloadPtr = alloc(payload.byteLength);
          coreOutputs.add(payloadPtr);
          new Uint8Array(memory.buffer, payloadPtr, payload.byteLength).set(payload);
          const view = new DataView(memory.buffer);
          view.setUint32(outPtr, payloadPtr, true);
          view.setUint32(outLen, payload.byteLength, true);
          return 0;
        },
        edgerun_browser_core_first_run_projection: (
          manifestPtr,
          manifestLen,
          graphPtr,
          graphLen,
          signaturePtr,
          signatureLen,
          inputPtr,
          inputLen,
          outPtr,
          outLen,
        ) => {
          calls.coreProjection.push([
            read(manifestPtr, manifestLen),
            read(graphPtr, graphLen),
            read(signaturePtr, signatureLen),
            read(inputPtr, inputLen),
          ]);
          return writeOutput(outPtr, outLen, "projection-bundle", true);
        },
        edgerun_browser_core_smoke_first_run_projection: coreFixture("projection-bundle"),
        edgerun_browser_core_smoke_runtime_projection: coreFixture("runtime-projection"),
        edgerun_browser_core_smoke_decision: coreFixture("decision"),
        edgerun_browser_core_smoke_cache: coreFixture("cache"),
        edgerun_browser_core_smoke_grant: coreFixture("grant"),
        edgerun_browser_core_smoke_storage_binding: coreFixture("storage-binding"),
        edgerun_browser_core_smoke_session: coreFixture("session"),
        edgerun_browser_core_smoke_storage_request: coreFixture("storage-request"),
        edgerun_browser_core_smoke_storage_invocation: coreFixture("storage-invocation"),
      },
    },
  };
}

const encoder = new TextEncoder();
const decoder = new TextDecoder();

function bytes(text) {
  return encoder.encode(text);
}
