import assert from "node:assert/strict";
import test from "node:test";
import {
  retrieveAndPersistNetworkAppPackage,
} from "./network-retrieval.mjs";
import {
  createBrowserRelayNode,
  createWasmBrowserAdmissionNode,
} from "./relay-node.mjs";

test("network retrieval fetches package bytes and persists admitted records", async () => {
  const fake = fakeCoreHost();
  const store = new MemoryPackageStore();
  const fetched = [];

  const result = await retrieveAndPersistNetworkAppPackage(
    fake.host,
    store,
    { packageKey: "apps/mail" },
    {
      manifestUrl: "/manifest.rkyv",
      graphUrl: "/graph.rkyv",
      developerSignatureUrl: "/signature.rkyv",
      retrievedAt: 20,
      fetchBytes: async (url) => {
        fetched.push(url);
        return textBytes(url);
      },
    },
  );

  assert.deepEqual(fetched, ["/manifest.rkyv", "/graph.rkyv", "/signature.rkyv"]);
  assert.equal(fake.calls.evidence[0].retrievalCost, 40n);
  assert.equal(fake.calls.evidence[0].requestedAt, 20n);
  assert.equal(fake.calls.evidence[0].policySchedule, "default-schedule");
  assert.equal(fake.calls.schedule.length, 1);
  assert.equal(fake.calls.policyCost.length, 1);
  assert.deepEqual([...result.recordBytes], [...textBytes("retrieval-record")]);
  assert.deepEqual([...(await store.getPackage("apps/mail:manifest"))], [
    ...textBytes("/manifest.rkyv"),
  ]);
  assert.deepEqual([...(await store.getRecord("apps/mail:retrieval"))], [
    ...textBytes("retrieval-record"),
  ]);
  assert.deepEqual([...(await store.getRecord("apps/mail:retrieval-policy-schedule"))], [
    ...textBytes("default-schedule"),
  ]);
});

test("network retrieval accepts an explicit retrieval cost", async () => {
  const fake = fakeCoreHost();
  const store = new MemoryPackageStore();

  await retrieveAndPersistNetworkAppPackage(
    fake.host,
    store,
    { packageKey: "apps/mail" },
    {
      manifestUrl: "manifest",
      graphUrl: "graph",
      signatureUrl: "signature",
      retrievalCost: 99,
      retrievedAt: 20,
      fetchBytes: async (url) => textBytes(url),
    },
  );

  assert.equal(fake.calls.evidence[0].retrievalCost, 99n);
  assert.equal(fake.calls.schedule.length, 1);
  assert.equal(fake.calls.policyCost.length, 0);
});

test("network retrieval accepts caller-provided policy schedule bytes", async () => {
  const fake = fakeCoreHost();
  const store = new MemoryPackageStore();

  await retrieveAndPersistNetworkAppPackage(
    fake.host,
    store,
    { packageKey: "apps/mail" },
    {
      manifestUrl: "manifest",
      graphUrl: "graph",
      signatureUrl: "signature",
      policyScheduleBytes: textBytes("caller-schedule"),
      retrievedAt: 20,
      fetchBytes: async (url) => textBytes(url),
    },
  );

  assert.equal(fake.calls.schedule.length, 0);
  assert.equal(fake.calls.policyCost[0].schedule, "caller-schedule");
  assert.equal(fake.calls.evidence[0].retrievalCost, 40n);
});

test("network retrieval uses js relay controlled by wasm admission", async () => {
  const fake = fakeCoreHost();
  const store = new MemoryPackageStore();
  const admissionNode = createWasmBrowserAdmissionNode(fake.host.exports, {
    nodeId: "admission:browser",
  });
  const relayNode = createBrowserRelayNode({
    nodeId: "relay:browser",
    controllingAdmissionNodeId: "admission:browser",
    admissionNode,
  });

  const result = await retrieveAndPersistNetworkAppPackage(
    fake.host,
    store,
    { packageKey: "apps/mail" },
    {
      manifestUrl: "manifest",
      graphUrl: "graph",
      signatureUrl: "signature",
      policyScheduleBytes: textBytes("caller-schedule"),
      retrievedAt: 20,
      fetchBytes: async (url) => textBytes(url),
      proofBytes: textBytes("relay-proof"),
      browserBoundary: {
        boundaryId: "boundary:browser",
        admission: { nodeId: "admission:browser" },
        relay: relayNode,
      },
    },
  );

  assert.equal(fake.calls.evidence.length, 3);
  assert.equal(fake.calls.evidence[0].packageKey, "apps/mail");
  assert.equal(fake.calls.evidence[0].retrievalCost, 40n);
  assert.equal(fake.calls.evidence[0].policySchedule, "caller-schedule");
  assert.deepEqual([...result.retrievalEvidenceBytes], [...textBytes("retrieval-evidence")]);
  assert.deepEqual([...result.browserAdmissionHash], [...new Uint8Array(32).fill(9)]);
  assert.deepEqual([...(await store.getRecord("apps/mail:retrieval"))], [
    ...textBytes("retrieval-record"),
  ]);
  assert.deepEqual([...(await store.getRecord("apps/mail:browser-admission"))], [
    ...new Uint8Array(32).fill(9),
  ]);
  assert.deepEqual([...(await store.getRecord("apps/mail:browser-work-admission"))], [
    ...textBytes("work-admission-packet"),
  ]);
  assert.deepEqual([...(await store.getRecord("apps/mail:proof"))], [
    ...textBytes("relay-proof"),
  ]);
});

test("network retrieval sends browser boundary admission through relay", async () => {
  const fake = fakeCoreHost();
  const store = new MemoryPackageStore();
  const relayRequests = [];

  const result = await retrieveAndPersistNetworkAppPackage(
    fake.host,
    store,
    { packageKey: "apps/mail" },
    {
      manifestUrl: "manifest",
      graphUrl: "graph",
      signatureUrl: "signature",
      retrievedAt: 20,
      fetchBytes: async (url) => textBytes(url),
      browserBoundary: {
        boundaryId: "boundary:browser-personal",
        admission: { nodeId: "browser-admission:personal" },
        relay: {
          nodeId: "relay:browser-local",
          controllingAdmissionNodeId: "browser-admission:personal",
          forwardAdmissionRequest: async (request) => {
            relayRequests.push(request);
            return {
              retrievalEvidenceBytes: textBytes("relay-admitted-evidence"),
              browserAdmissionHash: new Uint8Array(32).fill(8),
              browserWorkAdmissionBytes: textBytes("relay-signed-browser-work-admission"),
            };
          },
        },
      },
    },
  );

  assert.equal(relayRequests.length, 1);
  assert.equal(relayRequests[0].boundaryId, "boundary:browser-personal");
  assert.equal(relayRequests[0].controllingAdmissionNodeId, "browser-admission:personal");
  assert.equal(relayRequests[0].relayNodeId, "relay:browser-local");
  assert.equal(relayRequests[0].request.packageKey, "apps/mail");
  assert.equal(fake.calls.evidence.length, 0);
  assert.deepEqual([...result.browserAdmissionHash], [...new Uint8Array(32).fill(8)]);
  assert.deepEqual([...(await store.getRecord("apps/mail:browser-work-admission"))], [
    ...textBytes("relay-signed-browser-work-admission"),
  ]);
});

test("network retrieval rejects browser boundary admission without relay", async () => {
  const fake = fakeCoreHost();
  const store = new MemoryPackageStore();

  await assert.rejects(
    () =>
      retrieveAndPersistNetworkAppPackage(
        fake.host,
        store,
        { packageKey: "apps/mail" },
        {
          manifestUrl: "manifest",
          graphUrl: "graph",
          signatureUrl: "signature",
          retrievedAt: 20,
          fetchBytes: async (url) => textBytes(url),
          browserBoundary: {
            boundaryId: "boundary:browser-personal",
            admission: { nodeId: "browser-admission:personal" },
          },
        },
      ),
    /browser boundary admission requires a relay node/,
  );
});

test("network retrieval keeps same-tab browser boundaries separate", async () => {
  const fake = fakeCoreHost();
  const store = new MemoryPackageStore();
  const relayRequests = [];

  const boundary = (boundaryId, admissionNodeId, relayNodeId, hashByte) => ({
    boundaryId,
    admission: { nodeId: admissionNodeId },
    relay: {
      nodeId: relayNodeId,
      controllingAdmissionNodeId: admissionNodeId,
      forwardAdmissionRequest: async (request) => {
        relayRequests.push(request);
        return {
          retrievalEvidenceBytes: textBytes(`${boundaryId}:evidence`),
          browserAdmissionHash: new Uint8Array(32).fill(hashByte),
          browserWorkAdmissionBytes: textBytes(`${boundaryId}:admission`),
        };
      },
    },
  });

  await retrieveAndPersistNetworkAppPackage(
    fake.host,
    store,
    { packageKey: "apps/personal-mail" },
    {
      manifestUrl: "manifest-a",
      graphUrl: "graph-a",
      signatureUrl: "signature-a",
      retrievedAt: 20,
      fetchBytes: async (url) => textBytes(url),
      browserBoundary: boundary(
        "boundary:personal",
        "admission:personal",
        "relay:personal-local",
        8,
      ),
    },
  );
  await retrieveAndPersistNetworkAppPackage(
    fake.host,
    store,
    { packageKey: "apps/work-mail" },
    {
      manifestUrl: "manifest-b",
      graphUrl: "graph-b",
      signatureUrl: "signature-b",
      retrievedAt: 21,
      fetchBytes: async (url) => textBytes(url),
      browserBoundary: boundary(
        "boundary:work",
        "admission:work",
        "relay:work-vps",
        9,
      ),
    },
  );

  assert.deepEqual(relayRequests.map((request) => request.boundaryId), [
    "boundary:personal",
    "boundary:work",
  ]);
  assert.deepEqual(relayRequests.map((request) => request.controllingAdmissionNodeId), [
    "admission:personal",
    "admission:work",
  ]);
  assert.deepEqual([...(await store.getRecord("apps/personal-mail:browser-admission"))], [
    ...new Uint8Array(32).fill(8),
  ]);
  assert.deepEqual([...(await store.getRecord("apps/work-mail:browser-admission"))], [
    ...new Uint8Array(32).fill(9),
  ]);
});

function fakeCoreHost() {
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
        edgerun_browser_core_package_retrieval_work_admission: (...args) => {
          const outPtr = args.at(-2);
          const outLen = args.at(-1);
          pushEvidenceCall(args);
          return writeOutput(outPtr, outLen, textBytes("work-admission-packet"));
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
          return writeOutput(outPtr, outLen, textBytes("default-schedule"));
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
          new DataView(payload.buffer).setBigUint64(0, 40n, true);
          return writeOutput(outPtr, outLen, payload);
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
