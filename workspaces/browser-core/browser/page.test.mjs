import assert from "node:assert/strict";
import test from "node:test";
import {
  bindBrowserCorePage,
  bindBrowserCoreSmokePage,
  renderSmokeResult,
  runBrowserCorePage,
  runBrowserCoreSmokePage,
} from "./page.mjs";

test("smoke page runs harness and renders returned wire byte lengths", async () => {
  const documentRef = fakeDocument();
  const store = {};
  const packageBytes = {
    manifestBytes: new Uint8Array([1]),
    graphBytes: new Uint8Array([2]),
    developerSignatureBytes: new Uint8Array([3]),
  };
  const result = await runBrowserCoreSmokePage({
    document: documentRef,
    store,
    packageBytes,
    wasmUrl: "browser_core_slice.wasm",
    runHarness: async (wasmUrl, options) => {
      assert.equal(wasmUrl, "browser_core_slice.wasm");
      assert.equal(options.store, store);
      assert.equal(options.decision, "verify-cache");
      assert.equal(options.packageBytes, packageBytes);
      return smokeResult();
    },
  });

  assert.equal(result.abiVersion, 1);
  assert.equal(documentRef.status.textContent, "Passed");
  assert.equal(documentRef.button.disabled, false);
  assert.deepEqual(documentRef.resultRows(), [
    ["ABI", "1"],
    ["First-run projection", "2 bytes"],
    ["First run", "3 bytes"],
    ["Grant", "4 bytes"],
    ["Binding", "5 bytes"],
    ["Session", "6 bytes"],
    ["Storage", "7 bytes"],
  ]);
});

test("browser page exports non-smoke aliases", async () => {
  const documentRef = fakeDocument();
  let runs = 0;
  const result = await runBrowserCorePage({
    document: documentRef,
    store: {},
    runHarness: async () => {
      runs += 1;
      return smokeResult();
    },
  });

  assert.equal(result.abiVersion, 1);
  assert.equal(runs, 1);

  bindBrowserCorePage({
    autorun: false,
    document: documentRef,
    store: {},
    runHarness: async () => smokeResult(),
  });
});

test("smoke page renders harness failures", async () => {
  const documentRef = fakeDocument();

  await assert.rejects(
    () =>
      runBrowserCoreSmokePage({
        document: documentRef,
        store: {},
        runHarness: async () => {
          throw new Error("boom");
        },
      }),
    /boom/,
  );

  assert.equal(documentRef.status.textContent, "Failed");
  assert.deepEqual(documentRef.resultRows(), [["Error", "boom"]]);
});

test("smoke page passes the selected first-run decision to the harness", async () => {
  const documentRef = fakeDocument();
  documentRef.decision.value = "cancel";
  const result = await runBrowserCoreSmokePage({
    document: documentRef,
    store: {},
    runHarness: async (wasmUrl, options) => {
      assert.equal(options.decision, "cancel");
      return smokeResult();
    },
  });

  assert.equal(result.abiVersion, 1);
});

test("smoke page loads the selected network app package from the byte store", async () => {
  const documentRef = fakeDocument();
  documentRef.package.value = "apps:mail";
  const store = new MemoryByteStore([
    ["apps:mail:manifest", new Uint8Array([1])],
    ["apps:mail:graph", new Uint8Array([2])],
    ["apps:mail:signature", new Uint8Array([3])],
  ]);

  await runBrowserCoreSmokePage({
    document: documentRef,
    store,
    runHarness: async (wasmUrl, options) => {
      assert.deepEqual([...options.packageBytes.manifestBytes], [1]);
      assert.deepEqual([...options.packageBytes.graphBytes], [2]);
      assert.deepEqual([...options.packageBytes.developerSignatureBytes], [3]);
      return smokeResult();
    },
  });
});

test("smoke page forwards an admitted retrieval result to the harness", async () => {
  const documentRef = fakeDocument();
  documentRef.package.value = "apps:notes";
  const store = new MemoryByteStore();
  const retrievalResult = {
    manifestBytes: new Uint8Array([8]),
    graphBytes: new Uint8Array([9]),
    developerSignatureBytes: new Uint8Array([10]),
    recordBytes: new Uint8Array([11]),
    proofBytes: new Uint8Array([12]),
    sourceAdmissionHash: new Uint8Array(32).fill(13),
  };

  await runBrowserCoreSmokePage({
    document: documentRef,
    store,
    packageSelection: { packageKey: "apps:notes" },
    retrievalResult,
    runHarness: async (wasmUrl, options) => {
      assert.equal(options.packageBytes, undefined);
      assert.deepEqual(options.packageSelection, { packageKey: "apps:notes" });
      assert.equal(options.retrievalResult, retrievalResult);
      return smokeResult();
    },
  });
});

test("smoke page bind function wires the run button", async () => {
  const documentRef = fakeDocument();
  let runs = 0;
  bindBrowserCoreSmokePage({
    autorun: false,
    document: documentRef,
    store: {},
    runHarness: async () => {
      runs += 1;
      return smokeResult();
    },
  });

  documentRef.button.click();
  await Promise.resolve();
  await Promise.resolve();
  assert.equal(runs, 1);
});

test("result renderer tolerates absent result node", () => {
  renderSmokeResult(fakeDocument(), null, smokeResult());
});

function smokeResult() {
  return {
    abiVersion: 1,
    firstRunProjection: new Uint8Array(2),
    firstRun: new Uint8Array(3),
    grant: new Uint8Array(4),
    binding: new Uint8Array(5),
    session: new Uint8Array(6),
    storage: new Uint8Array(7),
  };
}

function fakeDocument() {
  const status = fakeNode("span");
  const results = fakeNode("div");
  const button = fakeNode("button");
  const decision = fakeNode("input");
  decision.value = "verify-cache";
  const packageNode = fakeNode("input");
  packageNode.value = null;
  return {
    status,
    results,
    button,
    decision,
    package: packageNode,
    createElement: fakeNode,
    querySelector(selector) {
      if (selector === "[data-edgerun-status]") {
        return status;
      }
      if (selector === "[data-edgerun-results]") {
        return results;
      }
      if (selector === "[data-edgerun-run]") {
        return button;
      }
      if (selector === "[data-edgerun-decision]:checked") {
        return decision;
      }
      if (selector === "[data-edgerun-package]:checked") {
        return packageNode.value == null ? null : packageNode;
      }
      return null;
    },
    resultRows() {
      return results.children.map((row) =>
        row.children.map((child) => child.textContent),
      );
    },
  };
}

class MemoryByteStore {
  constructor(packages = []) {
    this.packages = new Map(packages);
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

function fakeNode(tagName) {
  const listeners = new Map();
  return {
    tagName,
    className: "",
    textContent: "",
    disabled: false,
    children: [],
    append(...nodes) {
      this.children.push(...nodes);
    },
    replaceChildren(...nodes) {
      this.children = nodes;
    },
    addEventListener(type, listener) {
      listeners.set(type, listener);
    },
    click() {
      listeners.get("click")?.();
    },
  };
}
