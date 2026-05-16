import assert from "node:assert/strict";
import test from "node:test";
import {
  createBrowserRelayNode,
  createWasmBrowserAdmissionNode,
} from "./relay-node.mjs";

test("js relay forwards admission requests to its controlling wasm admission node", async () => {
  const fake = fakeCoreExports();
  const admissionNode = createWasmBrowserAdmissionNode(fake.exports, {
    nodeId: "admission:browser",
  });
  const relayNode = createBrowserRelayNode({
    nodeId: "relay:browser",
    controllingAdmissionNodeId: "admission:browser",
    admissionNode,
  });

  const admitted = await relayNode.forwardAdmissionRequest({
    kind: "browser-package-retrieval-admission",
    boundaryId: "boundary:browser",
    controllingAdmissionNodeId: "admission:browser",
    relayNodeId: "relay:browser",
    request: {
      packageKey: "apps/mail",
      packageBytes: {
        manifestBytes: textBytes("manifest"),
        graphBytes: textBytes("graph"),
        developerSignatureBytes: textBytes("signature"),
      },
      retrievalCost: 7,
      retrievedAt: 12,
      policyScheduleBytes: textBytes("schedule"),
    },
  });

  assert.deepEqual([...admitted.retrievalEvidenceBytes], [...textBytes("retrieval-evidence")]);
  assert.deepEqual([...admitted.browserAdmissionHash], [...new Uint8Array(32).fill(9)]);
  assert.deepEqual([...admitted.browserWorkAdmissionBytes], [...textBytes("work-admission")]);
  assert.equal(fake.calls.evidence.length, 3);
  assert.equal(fake.calls.evidence[0].packageKey, "apps/mail");
});

test("js relay rejects admission requests for another controller", async () => {
  const fake = fakeCoreExports();
  const admissionNode = createWasmBrowserAdmissionNode(fake.exports, {
    nodeId: "admission:browser",
  });
  const relayNode = createBrowserRelayNode({
    nodeId: "relay:browser",
    controllingAdmissionNodeId: "admission:browser",
    admissionNode,
  });

  await assert.rejects(
    () =>
      relayNode.forwardAdmissionRequest({
        kind: "browser-package-retrieval-admission",
        controllingAdmissionNodeId: "admission:other",
        request: {},
      }),
    /browser relay cannot forward to a different admission node/,
  );
});

function fakeCoreExports() {
  const memory = new WebAssembly.Memory({ initial: 1 });
  let bump = 16;
  const coreOutputs = new Set();
  const calls = { evidence: [] };

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
    exports: {
      memory,
      edgerun_browser_host_alloc: alloc,
      edgerun_browser_host_free: () => {},
      edgerun_browser_core_free: (ptr) => assert.equal(coreOutputs.has(ptr), true),
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
      edgerun_browser_core_package_retrieval_work_admission: (...args) => {
        const outPtr = args.at(-2);
        const outLen = args.at(-1);
        pushEvidenceCall(args);
        return writeOutput(outPtr, outLen, textBytes("work-admission"));
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

function textBytes(text) {
  return encoder.encode(text);
}

const encoder = new TextEncoder();
const decoder = new TextDecoder();
