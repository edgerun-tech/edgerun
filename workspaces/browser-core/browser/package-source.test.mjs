import assert from "node:assert/strict";
import test from "node:test";
import {
  loadNetworkAppRetrievalRecords,
  loadSelectedNetworkAppPackage,
  selectedNetworkAppPackageKey,
  storeRetrievedNetworkAppPackage,
} from "./package-source.mjs";

test("selected network app package source loads opaque package bytes", async () => {
  const store = new MemoryPackageStore([
    ["apps:mail:manifest", new Uint8Array([1])],
    ["apps:mail:graph", new Uint8Array([2])],
    ["apps:mail:signature", new Uint8Array([3])],
  ]);

  const packageBytes = await loadSelectedNetworkAppPackage(store, {
    packageKey: "apps:mail",
  });

  assert.deepEqual([...packageBytes.manifestBytes], [1]);
  assert.deepEqual([...packageBytes.graphBytes], [2]);
  assert.deepEqual([...packageBytes.developerSignatureBytes], [3]);
});

test("selected network app package source accepts explicit part keys", async () => {
  const store = new MemoryPackageStore([
    ["manifest", new Uint8Array([4])],
    ["graph", new Uint8Array([5])],
    ["developer-signature", new Uint8Array([6])],
  ]);

  const packageBytes = await loadSelectedNetworkAppPackage(store, {
    manifestKey: "manifest",
    graphKey: "graph",
    developerSignatureKey: "developer-signature",
  });

  assert.deepEqual([...packageBytes.manifestBytes], [4]);
  assert.deepEqual([...packageBytes.graphBytes], [5]);
  assert.deepEqual([...packageBytes.developerSignatureBytes], [6]);
});

test("selected network app package source reports missing bytes", async () => {
  const store = new MemoryPackageStore();

  await assert.rejects(
    () => loadSelectedNetworkAppPackage(store, { packageKey: "apps:missing" }),
    /selected network app package bytes not found: apps:missing:manifest/,
  );
});

test("retrieved network app package source stores package bytes and evidence", async () => {
  const store = new MemoryPackageStore();

  const keys = await storeRetrievedNetworkAppPackage(
    store,
    { packageKey: "apps:notes" },
    {
      manifestBytes: new Uint8Array([1]),
      graphBytes: new Uint8Array([2]),
      developerSignatureBytes: new Uint8Array([3]),
      recordBytes: new Uint8Array([4]),
      policyScheduleBytes: new Uint8Array([14]),
      proofBytes: new Uint8Array([5]),
      browserAdmissionHash: new Uint8Array(32).fill(6),
      browserWorkAdmissionBytes: new Uint8Array([7]),
    },
  );

  assert.deepEqual(keys, {
    manifestKey: "apps:notes:manifest",
    graphKey: "apps:notes:graph",
    developerSignatureKey: "apps:notes:signature",
  });
  assert.deepEqual([...(await store.getPackage("apps:notes:manifest"))], [1]);
  assert.deepEqual([...(await store.getPackage("apps:notes:graph"))], [2]);
  assert.deepEqual([...(await store.getPackage("apps:notes:signature"))], [3]);
  assert.deepEqual([...(await store.getRecord("apps:notes:retrieval"))], [4]);
  assert.deepEqual([...(await store.getRecord("apps:notes:retrieval-policy-schedule"))], [14]);
  assert.deepEqual([...(await store.getRecord("apps:notes:proof"))], [5]);
  assert.equal((await store.getRecord("apps:notes:browser-admission")).byteLength, 32);
  assert.equal((await store.getRecord("apps:notes:source-admission")).byteLength, 32);
  assert.deepEqual([...(await store.getRecord("apps:notes:browser-work-admission"))], [7]);
  assert.deepEqual([...(await store.getRecord("apps:notes:work-admission"))], [7]);
});

test("network app retrieval records load from canonical keys", async () => {
  const store = new MemoryPackageStore();
  store.records.set("apps:notes:retrieval", new Uint8Array([1]));
  store.records.set("apps:notes:retrieval-policy-schedule", new Uint8Array([2]));
  store.records.set("apps:notes:proof", new Uint8Array([3]));
  store.records.set("apps:notes:browser-admission", new Uint8Array([4]));
  store.records.set("apps:notes:browser-work-admission", new Uint8Array([5]));

  const records = await loadNetworkAppRetrievalRecords(store, { packageKey: "apps:notes" });

  assert.deepEqual([...records.recordBytes], [1]);
  assert.deepEqual([...records.policyScheduleBytes], [2]);
  assert.deepEqual([...records.proofBytes], [3]);
  assert.deepEqual([...records.browserAdmissionHash], [4]);
  assert.deepEqual([...records.sourceAdmissionHash], [4]);
  assert.deepEqual([...records.browserWorkAdmissionBytes], [5]);
  assert.deepEqual([...records.workAdmissionBytes], [5]);
});

test("selected network app package key reads the checked browser selection", () => {
  const documentRef = {
    querySelector(selector) {
      assert.equal(selector, "[data-edgerun-package]:checked");
      return { value: "apps:mail" };
    },
  };

  assert.equal(selectedNetworkAppPackageKey(documentRef), "apps:mail");
});

class MemoryPackageStore {
  constructor(entries = []) {
    this.packages = new Map(entries);
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
