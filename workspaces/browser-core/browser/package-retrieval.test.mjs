import assert from "node:assert/strict";
import test from "node:test";
import {
  calculateBrowserPackageRetrievalPolicyCost,
  createBrowserPackageRetrievalAdmissionHash,
  createBrowserPackageRetrievalEvidence,
  createBrowserPackageRetrievalPolicySchedule,
  createBrowserPackageRetrievalRecord,
  createBrowserPackageRetrievalWorkAdmission,
} from "./package-retrieval.mjs";

test("package retrieval adapter asks Rust to serialize verified retrieval record", () => {
  const fake = fakeCoreExports();
  const bytes = createBrowserPackageRetrievalRecord(
    fake.exports,
    {
      manifestBytes: textBytes("manifest"),
      graphBytes: textBytes("graph"),
      developerSignatureBytes: textBytes("signature"),
    },
    {
      packageKey: "apps/mail",
      retrievalCost: 7,
      retrievedAt: 12,
      sourceAdmissionHash: new Uint8Array(32).fill(3),
      retrievalEvidenceBytes: textBytes("retrieval"),
      proofBytes: textBytes("proof"),
    },
  );

  assert.deepEqual([...bytes], [...textBytes("retrieval-record")]);
  assert.deepEqual(fake.calls.retrieval[0], {
    manifest: "manifest",
    graph: "graph",
    signature: "signature",
    packageKey: "apps/mail",
    retrievalCost: 7n,
    retrievedAt: 12n,
    sourceAdmission: 3,
    retrievalEvidence: "retrieval",
    proof: "proof",
  });
});

test("package retrieval adapter asks Rust to serialize admitted retrieval evidence", () => {
  const fake = fakeCoreExports();
  const packageBytes = {
    manifestBytes: textBytes("manifest"),
    graphBytes: textBytes("graph"),
    developerSignatureBytes: textBytes("signature"),
  };

  const evidence = createBrowserPackageRetrievalEvidence(fake.exports, packageBytes, {
    packageKey: "apps/mail",
    retrievalCost: 7,
    requestedAt: 12,
    policyScheduleBytes: textBytes("schedule"),
  });
  const admissionHash = createBrowserPackageRetrievalAdmissionHash(fake.exports, packageBytes, {
    packageKey: "apps/mail",
    retrievalCost: 7,
    requestedAt: 12,
    policyScheduleBytes: textBytes("schedule"),
  });
  const admissionBytes = createBrowserPackageRetrievalWorkAdmission(fake.exports, packageBytes, {
    packageKey: "apps/mail",
    retrievalCost: 7,
    requestedAt: 12,
    policyScheduleBytes: textBytes("schedule"),
  });

  assert.deepEqual([...evidence], [...textBytes("retrieval-evidence-record")]);
  assert.deepEqual([...admissionHash], [...new Uint8Array(32).fill(9)]);
  assert.deepEqual([...admissionBytes], [...textBytes("work-admission-packet")]);
  assert.equal(fake.calls.evidence.length, 3);
  assert.equal(fake.calls.evidence[0].packageKey, "apps/mail");
  assert.equal(fake.calls.evidence[0].retrievalCost, 7n);
  assert.equal(fake.calls.evidence[0].policySchedule, "schedule");
});

test("package retrieval adapter asks Rust to serialize and apply policy schedules", () => {
  const fake = fakeCoreExports();
  const packageBytes = {
    manifestBytes: textBytes("manifest"),
    graphBytes: textBytes("graph"),
    developerSignatureBytes: textBytes("signature"),
  };

  const schedule = createBrowserPackageRetrievalPolicySchedule(fake.exports, {
    baseCost: 10,
    costPerByte: 2,
    minCost: 10,
    maxCost: 100,
    validFrom: 1,
    validUntil: 1000,
  });
  const cost = calculateBrowserPackageRetrievalPolicyCost(
    fake.exports,
    packageBytes,
    schedule,
    { requestedAt: 12 },
  );

  assert.deepEqual([...schedule], [...textBytes("retrieval-policy-schedule")]);
  assert.equal(cost, 42n);
  assert.deepEqual(fake.calls.schedule[0], {
    baseCost: 10n,
    costPerByte: 2n,
    minCost: 10n,
    maxCost: 100n,
    validFrom: 1n,
    validUntil: 1000n,
  });
  assert.deepEqual(fake.calls.policyCost[0], {
    schedule: "retrieval-policy-schedule",
    manifestLen: 8n,
    graphLen: 5n,
    signatureLen: 9n,
    requestedAt: 12n,
  });
});

function fakeCoreExports() {
  const memory = new WebAssembly.Memory({ initial: 1 });
  let bump = 16;
  const coreOutputs = new Set();
  const calls = { evidence: [], policyCost: [], retrieval: [], schedule: [] };

  const alloc = (len) => {
    if (len === 0) {
      return 0;
    }
    const ptr = bump;
    bump += len + 8;
    return ptr;
  };
  const read = (ptr, len) => decoder.decode(new Uint8Array(memory.buffer, ptr, len));
  const writeOutput = (outPtr, outLen, text) => {
    const payload = textBytes(text);
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
    exports: {
      memory,
      edgerun_browser_host_alloc: alloc,
      edgerun_browser_host_free: () => {},
      edgerun_browser_core_free: (ptr) => assert.equal(coreOutputs.has(ptr), true),
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
        calls.retrieval.push({
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
        return writeOutput(outPtr, outLen, "retrieval-record");
      },
      edgerun_browser_core_package_retrieval_evidence: (...args) => {
        const outPtr = args.at(-2);
        const outLen = args.at(-1);
        pushEvidenceCall(args);
        return writeOutput(outPtr, outLen, "retrieval-evidence-record");
      },
      edgerun_browser_core_package_retrieval_admission_hash: (...args) => {
        const outPtr = args.at(-2);
        const outLen = args.at(-1);
        pushEvidenceCall(args);
        const payload = new Uint8Array(32).fill(9);
        const payloadPtr = alloc(payload.byteLength);
        coreOutputs.add(payloadPtr);
        new Uint8Array(memory.buffer, payloadPtr, payload.byteLength).set(payload);
        const view = new DataView(memory.buffer);
        view.setUint32(outPtr, payloadPtr, true);
        view.setUint32(outLen, payload.byteLength, true);
        return 0;
      },
      edgerun_browser_core_package_retrieval_work_admission: (...args) => {
        const outPtr = args.at(-2);
        const outLen = args.at(-1);
        pushEvidenceCall(args);
        return writeOutput(outPtr, outLen, "work-admission-packet");
      },
      edgerun_browser_core_package_retrieval_policy_schedule: (...args) => {
        const outPtr = args.at(-2);
        const outLen = args.at(-1);
        calls.schedule.push({
          baseCost: args[0],
          costPerByte: args[1],
          minCost: args[2],
          maxCost: args[3],
          validFrom: args[4],
          validUntil: args[5],
        });
        return writeOutput(outPtr, outLen, "retrieval-policy-schedule");
      },
      edgerun_browser_core_package_retrieval_policy_cost: (...args) => {
        const outPtr = args.at(-2);
        const outLen = args.at(-1);
        calls.policyCost.push({
          schedule: read(args[0], args[1]),
          manifestLen: args[2],
          graphLen: args[3],
          signatureLen: args[4],
          requestedAt: args[5],
        });
        const payload = new Uint8Array(8);
        new DataView(payload.buffer).setBigUint64(0, 42n, true);
        const payloadPtr = alloc(payload.byteLength);
        coreOutputs.add(payloadPtr);
        new Uint8Array(memory.buffer, payloadPtr, payload.byteLength).set(payload);
        const view = new DataView(memory.buffer);
        view.setUint32(outPtr, payloadPtr, true);
        view.setUint32(outLen, payload.byteLength, true);
        return 0;
      },
    },
  };

  function pushEvidenceCall(args) {
    calls.evidence.push({
      manifest: read(args[0], args[1]),
      graph: read(args[2], args[3]),
      signature: read(args[4], args[5]),
      packageKey: read(args[6], args[7]),
      retrievalCost: args[8],
      requestedAt: args[9],
      policySchedule: read(args[10], args[11]),
    });
  }
}

function textBytes(text) {
  return encoder.encode(text);
}

const encoder = new TextEncoder();
const decoder = new TextDecoder();
