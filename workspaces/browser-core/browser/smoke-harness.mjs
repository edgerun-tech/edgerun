import { loadEdgeRunBrowserHostWasm } from "./host-adapter.mjs";
import { runBrowserAppFirstRun } from "./first-run.mjs";
import { persistAdmittedNetworkAppRetrieval } from "./retrieval-flow.mjs";

export async function runBrowserCoreSmokeHarness(wasmSource, options = {}) {
  const host = await loadEdgeRunBrowserHostWasm(wasmSource, options.imports ?? {});
  const store = options.store ?? null;
  const fixture = smokeFixture(host);
  const retrievalResult = options.retrievalResult ?? null;
  const appPackage =
    retrievalResult == null
      ? options.packageBytes ?? options.package ?? fixture.package
      : retrievalPackageBytes(retrievalResult);

  if (store && retrievalResult) {
    await persistAdmittedNetworkAppRetrieval(
      host,
      store,
      options.packageSelection,
      retrievalResult,
    );
  }

  host.open(fixture.runtimeId);
  try {
    const firstRun = await runBrowserAppFirstRun(
      host,
      appPackage,
      {
        profileId: filledBytes(1),
        runtimeId: fixture.runtimeId,
        previousEventSha256: filledBytes(0),
        firstEventSeq: 0,
        eventTime: 10,
        retrievalCost: 5,
        sourceAdmissionHash: filledBytes(3),
        decision: resolveDecision(host, options.decision),
        userSignature: new TextEncoder().encode("user"),
      },
      {
        store,
        packageKey: "smoke:package",
        time: 11,
      },
    );
    const grant = host.grant(fixture.grant, { time: 12 });
    const binding = host.bindStorage(fixture.storageBinding, { time: 12 });
    const session = host.openSession(fixture.session, { time: 13 });
    const storage = host.invokeStorage(fixture.storageRequest, fixture.storageInvocation);

    if (store) {
      await store.putRecord("smoke:first-run", firstRun.resultBytes);
      await store.putRecord("smoke:first-run-projection", firstRun.projectionBytes);
      await store.putRecord("smoke:grant", grant);
      await store.putRecord("smoke:binding", binding);
      await store.putRecord("smoke:session", session);
      await store.putRecord("smoke:storage", storage);
    }

    return {
      abiVersion: host.abiVersion(),
      firstRun: firstRun.resultBytes,
      firstRunProjection: firstRun.projectionBytes,
      grant,
      binding,
      session,
      storage,
    };
  } finally {
    host.close();
  }
}

export function smokeFixture(host) {
  const exports = host.exports;
  return {
    runtimeId: readSmokeBytes(host, exports.edgerun_browser_core_smoke_runtime_id),
    package: {
      manifestBytes: readSmokeBytes(host, exports.edgerun_browser_core_smoke_app_manifest),
      graphBytes: readSmokeBytes(host, exports.edgerun_browser_core_smoke_app_graph),
      developerSignatureBytes: readSmokeBytes(
        host,
        exports.edgerun_browser_core_smoke_developer_signature,
      ),
    },
    grant: readSmokeBytes(host, exports.edgerun_browser_core_smoke_grant),
    storageBinding: readSmokeBytes(
      host,
      exports.edgerun_browser_core_smoke_storage_binding,
    ),
    session: readSmokeBytes(host, exports.edgerun_browser_core_smoke_session),
    storageRequest: readSmokeBytes(
      host,
      exports.edgerun_browser_core_smoke_storage_request,
    ),
    storageInvocation: readSmokeBytes(
      host,
      exports.edgerun_browser_core_smoke_storage_invocation,
    ),
  };
}

function filledBytes(value) {
  return new Uint8Array(32).fill(value);
}

function resolveDecision(host, decision) {
  if (decision == null || decision === "verify-cache") {
    return host.exports.edgerun_browser_core_app_run_decision_verify_and_cache();
  }
  if (decision === "run-once") {
    return host.exports.edgerun_browser_core_app_run_decision_run_once();
  }
  if (decision === "cancel") {
    return host.exports.edgerun_browser_core_app_run_decision_cancel();
  }
  return decision;
}

function retrievalPackageBytes(retrievalResult) {
  return {
    manifestBytes: retrievalResult.manifestBytes,
    graphBytes: retrievalResult.graphBytes,
    developerSignatureBytes: retrievalResult.developerSignatureBytes,
  };
}

function readSmokeBytes(host, exportFn) {
  if (typeof exportFn !== "function") {
    throw new TypeError("browser core WASM is missing a smoke fixture export");
  }
  return host.withOutput((outPtr, outLen) => exportFn(outPtr, outLen), {
    freeOutput: host.exports.edgerun_browser_core_free,
  });
}
