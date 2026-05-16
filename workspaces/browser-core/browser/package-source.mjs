export async function loadSelectedNetworkAppPackage(store, selection = {}) {
  if (!store || typeof store.getPackage !== "function") {
    throw new TypeError("selected network app packages require a browser byte store");
  }
  const baseKey = selection.packageKey ?? selection.key;
  const manifestKey = selection.manifestKey ?? packagePartKey(baseKey, "manifest");
  const graphKey = selection.graphKey ?? packagePartKey(baseKey, "graph");
  const signatureKey =
    selection.developerSignatureKey ??
    selection.signatureKey ??
    packagePartKey(baseKey, "signature");

  const [manifestBytes, graphBytes, developerSignatureBytes] = await Promise.all([
    store.getPackage(manifestKey),
    store.getPackage(graphKey),
    store.getPackage(signatureKey),
  ]);

  return {
    manifestBytes: requireBytes(manifestBytes, manifestKey),
    graphBytes: requireBytes(graphBytes, graphKey),
    developerSignatureBytes: requireBytes(
      developerSignatureBytes,
      signatureKey,
    ),
  };
}

export async function storeRetrievedNetworkAppPackage(store, selection = {}, retrieval = {}) {
  if (!store || typeof store.putPackage !== "function") {
    throw new TypeError("retrieved network app packages require a browser byte store");
  }
  const baseKey = selection.packageKey ?? selection.key;
  const manifestKey = selection.manifestKey ?? packagePartKey(baseKey, "manifest");
  const graphKey = selection.graphKey ?? packagePartKey(baseKey, "graph");
  const signatureKey =
    selection.developerSignatureKey ??
    selection.signatureKey ??
    packagePartKey(baseKey, "signature");

  const manifestBytes = requireBytes(retrieval.manifestBytes, manifestKey);
  const graphBytes = requireBytes(retrieval.graphBytes, graphKey);
  const developerSignatureBytes = requireBytes(
    retrieval.developerSignatureBytes,
    signatureKey,
  );

  await Promise.all([
    store.putPackage(manifestKey, manifestBytes),
    store.putPackage(graphKey, graphBytes),
    store.putPackage(signatureKey, developerSignatureBytes),
    putOptionalRecord(store, packagePartKey(baseKey, "retrieval"), retrieval.recordBytes),
    putOptionalRecord(
      store,
      packagePartKey(baseKey, "retrieval-policy-schedule"),
      retrieval.policyScheduleBytes,
    ),
    putOptionalRecord(store, packagePartKey(baseKey, "proof"), retrieval.proofBytes),
    putOptionalRecord(
      store,
      packagePartKey(baseKey, "browser-admission"),
      retrieval.browserAdmissionHash ?? retrieval.sourceAdmissionHash,
    ),
    putOptionalRecord(
      store,
      packagePartKey(baseKey, "source-admission"),
      retrieval.sourceAdmissionHash ?? retrieval.browserAdmissionHash,
    ),
    putOptionalRecord(
      store,
      packagePartKey(baseKey, "browser-work-admission"),
      retrieval.browserWorkAdmissionBytes ?? retrieval.workAdmissionBytes,
    ),
    putOptionalRecord(
      store,
      packagePartKey(baseKey, "work-admission"),
      retrieval.workAdmissionBytes ?? retrieval.browserWorkAdmissionBytes,
    ),
  ]);

  return { manifestKey, graphKey, developerSignatureKey: signatureKey };
}

export async function loadNetworkAppRetrievalRecords(store, selection = {}) {
  if (!store || typeof store.getRecord !== "function") {
    throw new TypeError("network app retrieval records require a browser record store");
  }
  const baseKey = selection.packageKey ?? selection.key;
  return {
    recordBytes: await optionalRecord(store, packagePartKey(baseKey, "retrieval")),
    policyScheduleBytes: await optionalRecord(
      store,
      packagePartKey(baseKey, "retrieval-policy-schedule"),
    ),
    proofBytes: await optionalRecord(store, packagePartKey(baseKey, "proof")),
    browserAdmissionHash:
      (await optionalRecord(store, packagePartKey(baseKey, "browser-admission"))) ??
      (await optionalRecord(store, packagePartKey(baseKey, "source-admission"))),
    sourceAdmissionHash:
      (await optionalRecord(store, packagePartKey(baseKey, "source-admission"))) ??
      (await optionalRecord(store, packagePartKey(baseKey, "browser-admission"))),
    browserWorkAdmissionBytes:
      (await optionalRecord(store, packagePartKey(baseKey, "browser-work-admission"))) ??
      (await optionalRecord(store, packagePartKey(baseKey, "work-admission"))),
    workAdmissionBytes:
      (await optionalRecord(store, packagePartKey(baseKey, "work-admission"))) ??
      (await optionalRecord(store, packagePartKey(baseKey, "browser-work-admission"))),
  };
}

export function selectedNetworkAppPackageKey(documentRef) {
  const selected = documentRef?.querySelector?.("[data-edgerun-package]:checked");
  return selected?.value ?? null;
}

function packagePartKey(baseKey, part) {
  if (!baseKey) {
    throw new Error("selected network app package key is required");
  }
  return `${baseKey}:${part}`;
}

function requireBytes(bytes, key) {
  if (bytes == null) {
    throw new Error(`selected network app package bytes not found: ${key}`);
  }
  if (bytes instanceof Uint8Array) {
    return bytes;
  }
  if (bytes instanceof ArrayBuffer) {
    return new Uint8Array(bytes);
  }
  if (ArrayBuffer.isView(bytes)) {
    return new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  }
  throw new TypeError(`selected network app package bytes are invalid: ${key}`);
}

async function putOptionalRecord(store, key, bytes) {
  if (bytes == null) {
    return;
  }
  if (typeof store.putRecord !== "function") {
    throw new TypeError("retrieval evidence requires a browser record store");
  }
  await store.putRecord(key, requireBytes(bytes, key));
}

async function optionalRecord(store, key) {
  const bytes = await store.getRecord(key);
  return bytes == null ? null : requireBytes(bytes, key);
}
