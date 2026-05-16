import {
  calculateBrowserPackageRetrievalPolicyCost,
  createBrowserPackageRetrievalPolicySchedule,
} from "./package-retrieval.mjs";
import { persistAdmittedNetworkAppRetrieval } from "./retrieval-flow.mjs";

export async function retrieveAndPersistNetworkAppPackage(
  host,
  store,
  selection = {},
  source = {},
) {
  const fetchBytes = source.fetchBytes ?? defaultFetchBytes(source.fetch);
  const packageBytes = {
    manifestBytes: await fetchBytes(requiredSource(source.manifestUrl, "manifestUrl")),
    graphBytes: await fetchBytes(requiredSource(source.graphUrl, "graphUrl")),
    developerSignatureBytes: await fetchBytes(
      requiredSource(source.developerSignatureUrl ?? source.signatureUrl, "developerSignatureUrl"),
    ),
  };
  const retrievedAt = source.retrievedAt ?? Date.now();
  const policyScheduleBytes = retrievalPolicyScheduleBytes(host, source, retrievedAt);
  const retrievalCost =
    source.retrievalCost ??
    retrievalCostFromPolicySchedule(host, packageBytes, policyScheduleBytes, retrievedAt);
  const admission = await admitBrowserPackageRetrieval(source, {
    packageKey: selection.packageKey ?? selection.key,
    packageBytes,
    retrievalCost,
    retrievedAt,
    policyScheduleBytes,
  });

  return persistAdmittedNetworkAppRetrieval(host, store, selection, {
    ...packageBytes,
    retrievalCost,
    retrievedAt,
    policyScheduleBytes,
    retrievalEvidenceBytes: admission.retrievalEvidenceBytes,
    browserAdmissionHash: admission.browserAdmissionHash,
    sourceAdmissionHash: admission.browserAdmissionHash,
    browserWorkAdmissionBytes: admission.browserWorkAdmissionBytes,
    workAdmissionBytes: admission.browserWorkAdmissionBytes,
    proofBytes: admission.proofBytes ?? source.proofBytes,
  });
}

function defaultFetchBytes(fetchImpl = globalThis.fetch) {
  if (typeof fetchImpl !== "function") {
    throw new TypeError("network app package retrieval requires fetch");
  }
  return async (url) => {
    const response = await fetchImpl(url);
    if (!response?.ok) {
      throw new Error(`network app package retrieval failed: ${url}`);
    }
    return new Uint8Array(await response.arrayBuffer());
  };
}

function requiredSource(value, name) {
  if (!value) {
    throw new Error(`network app package retrieval source is missing ${name}`);
  }
  return value;
}

function retrievalCostFromPolicySchedule(host, packageBytes, policyScheduleBytes, retrievedAt) {
  return calculateBrowserPackageRetrievalPolicyCost(
    host.exports,
    packageBytes,
    policyScheduleBytes,
    { requestedAt: retrievedAt },
  );
}

function retrievalPolicyScheduleBytes(host, source, retrievedAt) {
  return (
    source.policyScheduleBytes ??
    createBrowserPackageRetrievalPolicySchedule(
      host.exports,
      source.policySchedule ?? defaultPolicySchedule(retrievedAt),
    )
  );
}

async function admitBrowserPackageRetrieval(source, input) {
  if (source.browserBoundary != null) {
    return normalizeAdmission(
      await admitBrowserBoundaryPackageRetrieval(source.browserBoundary, input),
    );
  }
  return {};
}

async function admitBrowserBoundaryPackageRetrieval(boundary, input) {
  const admission = boundary.admission ?? {};
  const relay = boundary.relay ?? {};
  const boundaryId = requiredBoundaryValue(
    boundary.boundaryId ?? boundary.id,
    "browser boundary id is required",
  );
  const admissionNodeId = requiredBoundaryValue(
    admission.nodeId ?? boundary.admissionNodeId,
    "browser boundary admission node id is required",
  );
  const forwardAdmissionRequest =
    relay.forwardAdmissionRequest ?? relay.forwardToAdmission;
  if (typeof forwardAdmissionRequest !== "function") {
    throw new TypeError("browser boundary admission requires a relay node");
  }
  const relayAdmissionNodeId =
    relay.controllingAdmissionNodeId ?? relay.admissionNodeId ?? boundary.admissionNodeId;
  if (!sameBoundaryValue(admissionNodeId, relayAdmissionNodeId)) {
    throw new Error("browser boundary relay must be controlled by its admission node");
  }

  return forwardAdmissionRequest({
    kind: "browser-package-retrieval-admission",
    boundaryId,
    controllingAdmissionNodeId: admissionNodeId,
    relayNodeId: relay.nodeId ?? boundary.relayNodeId,
    request: input,
  });
}

function normalizeAdmission(admission) {
  return {
    retrievalEvidenceBytes: admission.retrievalEvidenceBytes,
    browserAdmissionHash:
      admission.browserAdmissionHash ?? admission.sourceAdmissionHash ?? admission.admissionHash,
    browserWorkAdmissionBytes:
      admission.browserWorkAdmissionBytes ??
      admission.workAdmissionBytes ??
      admission.admissionBytes,
    proofBytes: admission.proofBytes,
  };
}

function requiredBoundaryValue(value, message) {
  if (value == null || value === "") {
    throw new Error(message);
  }
  return value;
}

function sameBoundaryValue(left, right) {
  if (left instanceof Uint8Array || right instanceof Uint8Array) {
    const leftBytes = asUint8Array(left);
    const rightBytes = asUint8Array(right);
    return (
      leftBytes.byteLength === rightBytes.byteLength &&
      leftBytes.every((value, index) => value === rightBytes[index])
    );
  }
  return left === right;
}

function asUint8Array(bytes) {
  if (bytes instanceof Uint8Array) {
    return bytes;
  }
  if (bytes instanceof ArrayBuffer) {
    return new Uint8Array(bytes);
  }
  if (ArrayBuffer.isView(bytes)) {
    return new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  }
  return new TextEncoder().encode(String(bytes));
}

function defaultPolicySchedule(retrievedAt) {
  return {
    baseCost: 0,
    costPerByte: 1,
    minCost: 0,
    maxCost: 0,
    validFrom: 0,
    validUntil: Math.max(Number.MAX_SAFE_INTEGER, Number(retrievedAt ?? 0)),
  };
}
