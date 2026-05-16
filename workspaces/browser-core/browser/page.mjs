import { EdgeRunBrowserByteStore } from "./host-adapter.mjs";
import {
  loadSelectedNetworkAppPackage,
  selectedNetworkAppPackageKey,
} from "./package-source.mjs";
import { runBrowserCoreSmokeHarness } from "./smoke-harness.mjs";

export async function runBrowserCoreSmokePage(options = {}) {
  const documentRef = options.document ?? globalThis.document;
  if (!documentRef) {
    throw new Error("document is not available");
  }

  const statusNode = documentRef.querySelector("[data-edgerun-status]");
  const resultNode = documentRef.querySelector("[data-edgerun-results]");
  const runButton = documentRef.querySelector("[data-edgerun-run]");
  const wasmUrl = options.wasmUrl ?? "dist/browser_core_slice.wasm";
  const runHarness = options.runHarness ?? runBrowserCoreSmokeHarness;

  setText(statusNode, "Running");
  setDisabled(runButton, true);
  clearNode(resultNode);

  try {
    const store =
      options.store ??
      (await EdgeRunBrowserByteStore.open({
        name: options.dbName ?? "edgerun-browser-core-smoke",
      }));
    const packageBytes =
      options.packageBytes ??
      (options.retrievalResult
        ? undefined
        : await selectedPackageBytes(store, documentRef, options.packageSelection));
    const result = await runHarness(wasmUrl, {
      imports: options.imports ?? {},
      store,
      decision: options.decision ?? selectedDecision(documentRef),
      packageBytes,
      packageSelection: options.packageSelection,
      retrievalResult: options.retrievalResult,
    });
    renderSmokeResult(documentRef, resultNode, result);
    setText(statusNode, "Passed");
    return result;
  } catch (error) {
    setText(statusNode, "Failed");
    renderError(documentRef, resultNode, error);
    throw error;
  } finally {
    setDisabled(runButton, false);
  }
}

export const runBrowserCorePage = runBrowserCoreSmokePage;

export function bindBrowserCoreSmokePage(options = {}) {
  const documentRef = options.document ?? globalThis.document;
  const runButton = documentRef.querySelector("[data-edgerun-run]");
  const run = () => runBrowserCoreSmokePage(options).catch(() => {});
  if (runButton) {
    runButton.addEventListener("click", run);
  }
  if (options.autorun ?? true) {
    run();
  }
  return run;
}

export const bindBrowserCorePage = bindBrowserCoreSmokePage;

export function renderSmokeResult(documentRef, resultNode, result) {
  clearNode(resultNode);
  if (!resultNode) {
    return;
  }
  const rows = [
    ["ABI", String(result.abiVersion)],
    ["First-run projection", byteCount(result.firstRunProjection)],
    ["First run", byteCount(result.firstRun)],
    ["Grant", byteCount(result.grant)],
    ["Binding", byteCount(result.binding)],
    ["Session", byteCount(result.session)],
    ["Storage", byteCount(result.storage)],
  ];
  for (const [label, value] of rows) {
    const row = documentRef.createElement("div");
    row.className = "result-row";
    const labelNode = documentRef.createElement("span");
    labelNode.textContent = label;
    const valueNode = documentRef.createElement("strong");
    valueNode.textContent = value;
    row.append(labelNode, valueNode);
    resultNode.append(row);
  }
}

function renderError(documentRef, resultNode, error) {
  clearNode(resultNode);
  if (!resultNode) {
    return;
  }
  const row = documentRef.createElement("div");
  row.className = "result-row error";
  const labelNode = documentRef.createElement("span");
  labelNode.textContent = "Error";
  const valueNode = documentRef.createElement("strong");
  valueNode.textContent = error instanceof Error ? error.message : String(error);
  row.append(labelNode, valueNode);
  resultNode.append(row);
}

function byteCount(bytes) {
  return `${bytes?.byteLength ?? 0} bytes`;
}

function clearNode(node) {
  if (node) {
    node.replaceChildren();
  }
}

function setText(node, text) {
  if (node) {
    node.textContent = text;
  }
}

function setDisabled(node, disabled) {
  if (node) {
    node.disabled = disabled;
  }
}

function selectedDecision(documentRef) {
  const selected = documentRef.querySelector("[data-edgerun-decision]:checked");
  return selected?.value ?? "verify-cache";
}

async function selectedPackageBytes(store, documentRef, explicitSelection) {
  const packageKey =
    explicitSelection?.packageKey ??
    explicitSelection?.key ??
    selectedNetworkAppPackageKey(documentRef);
  if (!explicitSelection && !packageKey) {
    return undefined;
  }
  return loadSelectedNetworkAppPackage(store, {
    ...explicitSelection,
    packageKey,
  });
}
