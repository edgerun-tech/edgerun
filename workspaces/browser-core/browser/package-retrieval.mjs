import {
  BROWSER_HOST_FFI_STATUS_OK,
  WASM32_USIZE_BYTES,
} from "./host-adapter.mjs";

export function createBrowserPackageRetrievalRecord(exports, packageBytes, input) {
  const memory = coreMemory(exports);
  requireCoreExports(exports, REQUIRED_RETRIEVAL_EXPORTS);

  const manifest = copyInput(exports, memory, packageBytes.manifestBytes);
  const graph = copyInput(exports, memory, packageBytes.graphBytes);
  const signature = copyInput(exports, memory, packageBytes.developerSignatureBytes);
  const packageKey = copyInput(exports, memory, new TextEncoder().encode(input.packageKey));
  const sourceAdmission = copyInput(
    exports,
    memory,
    input.sourceAdmissionHash ?? new Uint8Array(32),
  );
  const retrievalEvidence = copyInput(
    exports,
    memory,
    input.retrievalEvidenceBytes ?? input.retrievalRecordBytes ?? new Uint8Array(),
  );
  const proof = copyInput(exports, memory, input.proofBytes ?? new Uint8Array());
  try {
    return withCoreOutput(exports, memory, (outPtr, outLen) =>
      exports.edgerun_browser_core_package_retrieval(
        manifest.ptr,
        manifest.len,
        graph.ptr,
        graph.len,
        signature.ptr,
        signature.len,
        packageKey.ptr,
        packageKey.len,
        cost(input.retrievalCost),
        timestamp(input.retrievedAt),
        sourceAdmission.ptr,
        sourceAdmission.len,
        retrievalEvidence.ptr,
        retrievalEvidence.len,
        proof.ptr,
        proof.len,
        outPtr,
        outLen,
      ),
    );
  } finally {
    manifest.free();
    graph.free();
    signature.free();
    packageKey.free();
    sourceAdmission.free();
    retrievalEvidence.free();
    proof.free();
  }
}

export function createBrowserPackageRetrievalEvidence(exports, packageBytes, input) {
  return createAdmittedRetrievalOutput(
    exports,
    packageBytes,
    input,
    "edgerun_browser_core_package_retrieval_evidence",
  );
}

export function createBrowserPackageRetrievalAdmissionHash(exports, packageBytes, input) {
  return createAdmittedRetrievalOutput(
    exports,
    packageBytes,
    input,
    "edgerun_browser_core_package_retrieval_admission_hash",
  );
}

export function createBrowserPackageRetrievalWorkAdmission(exports, packageBytes, input) {
  return createAdmittedRetrievalOutput(
    exports,
    packageBytes,
    input,
    "edgerun_browser_core_package_retrieval_work_admission",
  );
}

export function createBrowserPackageRetrievalPolicySchedule(exports, schedule = {}) {
  const memory = coreMemory(exports);
  requireCoreExports(exports, [
    "edgerun_browser_host_alloc",
    "edgerun_browser_host_free",
    "edgerun_browser_core_free",
    "edgerun_browser_core_package_retrieval_policy_schedule",
  ]);

  return withCoreOutput(exports, memory, (outPtr, outLen) =>
    exports.edgerun_browser_core_package_retrieval_policy_schedule(
      cost(schedule.baseCost),
      cost(schedule.costPerByte),
      cost(schedule.minCost),
      cost(schedule.maxCost),
      timestamp(schedule.validFrom ?? 0),
      timestamp(schedule.validUntil ?? Number.MAX_SAFE_INTEGER),
      outPtr,
      outLen,
    ),
  );
}

export function calculateBrowserPackageRetrievalPolicyCost(
  exports,
  packageBytes,
  scheduleBytes,
  input = {},
) {
  const memory = coreMemory(exports);
  requireCoreExports(exports, [
    "edgerun_browser_host_alloc",
    "edgerun_browser_host_free",
    "edgerun_browser_core_free",
    "edgerun_browser_core_package_retrieval_policy_cost",
  ]);

  const schedule = copyInput(exports, memory, scheduleBytes);
  try {
    const output = withCoreOutput(exports, memory, (outPtr, outLen) =>
      exports.edgerun_browser_core_package_retrieval_policy_cost(
        schedule.ptr,
        schedule.len,
        cost(byteLength(packageBytes.manifestBytes)),
        cost(byteLength(packageBytes.graphBytes)),
        cost(byteLength(packageBytes.developerSignatureBytes)),
        timestamp(input.requestedAt ?? input.retrievedAt),
        outPtr,
        outLen,
      ),
    );
    if (output.byteLength !== 8) {
      throw new Error("edgerun browser core returned invalid retrieval policy cost");
    }
    return new DataView(output.buffer, output.byteOffset, output.byteLength).getBigUint64(
      0,
      true,
    );
  } finally {
    schedule.free();
  }
}

function createAdmittedRetrievalOutput(exports, packageBytes, input, exportName) {
  const memory = coreMemory(exports);
  requireCoreExports(exports, [
    "edgerun_browser_host_alloc",
    "edgerun_browser_host_free",
    "edgerun_browser_core_free",
    exportName,
  ]);

  const manifest = copyInput(exports, memory, packageBytes.manifestBytes);
  const graph = copyInput(exports, memory, packageBytes.graphBytes);
  const signature = copyInput(exports, memory, packageBytes.developerSignatureBytes);
  const packageKey = copyInput(exports, memory, new TextEncoder().encode(input.packageKey));
  const policySchedule = copyInput(
    exports,
    memory,
    input.policyScheduleBytes ?? new Uint8Array(),
  );
  try {
    return withCoreOutput(exports, memory, (outPtr, outLen) =>
      exports[exportName](
        manifest.ptr,
        manifest.len,
        graph.ptr,
        graph.len,
        signature.ptr,
        signature.len,
        packageKey.ptr,
        packageKey.len,
        cost(input.retrievalCost),
        timestamp(input.retrievedAt ?? input.requestedAt),
        policySchedule.ptr,
        policySchedule.len,
        outPtr,
        outLen,
      ),
    );
  } finally {
    manifest.free();
    graph.free();
    signature.free();
    packageKey.free();
    policySchedule.free();
  }
}

const REQUIRED_RETRIEVAL_EXPORTS = [
  "edgerun_browser_host_alloc",
  "edgerun_browser_host_free",
  "edgerun_browser_core_free",
  "edgerun_browser_core_package_retrieval",
];

function coreMemory(exports) {
  const memory = exports.memory;
  if (!(memory instanceof WebAssembly.Memory)) {
    throw new TypeError("edgerun browser core WASM must export memory");
  }
  return memory;
}

function requireCoreExports(exports, names) {
  for (const name of names) {
    if (typeof exports[name] !== "function") {
      throw new TypeError(`edgerun browser core WASM missing export: ${name}`);
    }
  }
}

function copyInput(exports, memory, bytes) {
  const input = asUint8Array(bytes);
  if (input.byteLength === 0) {
    return emptyInput();
  }
  const ptr = exports.edgerun_browser_host_alloc(input.byteLength);
  if (ptr === 0) {
    throw new Error("edgerun browser core allocation failed");
  }
  new Uint8Array(memory.buffer, ptr, input.byteLength).set(input);
  return {
    ptr,
    len: input.byteLength,
    free: () => exports.edgerun_browser_host_free(ptr, input.byteLength),
  };
}

function withCoreOutput(exports, memory, call) {
  const slots = exports.edgerun_browser_host_alloc(WASM32_USIZE_BYTES * 2);
  if (slots === 0) {
    throw new Error("edgerun browser core output slot allocation failed");
  }
  try {
    const status = call(slots, slots + WASM32_USIZE_BYTES);
    if (status !== BROWSER_HOST_FFI_STATUS_OK) {
      throw new Error(`edgerun browser core FFI failed with status ${status}`);
    }
    const view = new DataView(memory.buffer);
    const outputPtr = view.getUint32(slots, true);
    const outputLen = view.getUint32(slots + WASM32_USIZE_BYTES, true);
    if (outputLen === 0) {
      return new Uint8Array();
    }
    const output = new Uint8Array(outputLen);
    output.set(new Uint8Array(memory.buffer, outputPtr, outputLen));
    exports.edgerun_browser_core_free(outputPtr, outputLen);
    return output;
  } finally {
    exports.edgerun_browser_host_free(slots, WASM32_USIZE_BYTES * 2);
  }
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
  throw new TypeError("expected bytes as Uint8Array, ArrayBuffer, or ArrayBufferView");
}

function emptyInput() {
  return {
    ptr: 0,
    len: 0,
    free: () => {},
  };
}

function timestamp(value) {
  return BigInt(value == null ? Date.now() : value);
}

function cost(value) {
  return BigInt(value == null ? 0 : value);
}

function byteLength(bytes) {
  return BigInt(asUint8Array(bytes).byteLength);
}
