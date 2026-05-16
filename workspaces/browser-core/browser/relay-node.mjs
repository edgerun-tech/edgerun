import {
  createBrowserPackageRetrievalAdmissionHash,
  createBrowserPackageRetrievalEvidence,
  createBrowserPackageRetrievalWorkAdmission,
} from "./package-retrieval.mjs";

export function createWasmBrowserAdmissionNode(exports, options = {}) {
  const nodeId = requiredNodeId(options.nodeId, "browser admission node id is required");
  return {
    nodeId,
    async admitPackageRetrieval(request) {
      const packageBytes = request.packageBytes;
      const input = {
        packageKey: request.packageKey,
        retrievalCost: request.retrievalCost,
        retrievedAt: request.retrievedAt,
        requestedAt: request.requestedAt ?? request.retrievedAt,
        policyScheduleBytes: request.policyScheduleBytes,
      };
      return {
        retrievalEvidenceBytes: createBrowserPackageRetrievalEvidence(
          exports,
          packageBytes,
          input,
        ),
        browserAdmissionHash: createBrowserPackageRetrievalAdmissionHash(
          exports,
          packageBytes,
          input,
        ),
        browserWorkAdmissionBytes: createBrowserPackageRetrievalWorkAdmission(
          exports,
          packageBytes,
          input,
        ),
      };
    },
  };
}

export function createBrowserRelayNode(options = {}) {
  const nodeId = requiredNodeId(options.nodeId, "browser relay node id is required");
  const controllingAdmissionNodeId = requiredNodeId(
    options.controllingAdmissionNodeId,
    "browser relay controlling admission node id is required",
  );
  const admissionNode = options.admissionNode;
  if (!admissionNode || typeof admissionNode.admitPackageRetrieval !== "function") {
    throw new TypeError("browser relay requires a controlling admission node");
  }
  if (!sameNodeId(controllingAdmissionNodeId, admissionNode.nodeId)) {
    throw new Error("browser relay controller must match admission node id");
  }

  return {
    nodeId,
    controllingAdmissionNodeId,
    async forwardAdmissionRequest(envelope) {
      if (envelope?.kind !== "browser-package-retrieval-admission") {
        throw new Error("browser relay can only forward browser package retrieval admission");
      }
      if (!sameNodeId(envelope.controllingAdmissionNodeId, controllingAdmissionNodeId)) {
        throw new Error("browser relay cannot forward to a different admission node");
      }
      return admissionNode.admitPackageRetrieval(envelope.request);
    },
  };
}

function requiredNodeId(value, message) {
  if (value == null || value === "") {
    throw new Error(message);
  }
  return value;
}

function sameNodeId(left, right) {
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
