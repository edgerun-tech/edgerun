import {
  BROWSER_HOST_FFI_STATUS_OK,
  WASM32_USIZE_BYTES,
} from "./host-adapter.mjs";

export const APP_RUN_DECISION_RUN_ONCE = 1;
export const APP_RUN_DECISION_VERIFY_AND_CACHE = 2;
export const APP_RUN_DECISION_CANCEL = 3;

export function createBrowserAppFirstRunInput(exports, input) {
  const memory = coreMemory(exports);
  requireCoreExports(exports, REQUIRED_INPUT_EXPORTS);
  const profileId = copyInput(exports, memory, input.profileId);
  const runtimeId = copyInput(exports, memory, input.runtimeId);
  const previousEvent = copyInput(
    exports,
    memory,
    input.previousEventSha256 ?? new Uint8Array(32),
  );
  const sourceAdmission = copyInput(
    exports,
    memory,
    input.sourceAdmissionHash ?? new Uint8Array(32),
  );
  const userSignature = copyInput(exports, memory, input.userSignature ?? new Uint8Array());
  try {
    return withCoreOutput(exports, memory, (outPtr, outLen) =>
      exports.edgerun_browser_core_first_run_input(
        profileId.ptr,
        profileId.len,
        runtimeId.ptr,
        runtimeId.len,
        previousEvent.ptr,
        previousEvent.len,
        sequence(input.firstEventSeq),
        timestamp(input.eventTime),
        cost(input.retrievalCost),
        sourceAdmission.ptr,
        sourceAdmission.len,
        decision(input.decision),
        userSignature.ptr,
        userSignature.len,
        outPtr,
        outLen,
      ),
    );
  } finally {
    profileId.free();
    runtimeId.free();
    previousEvent.free();
    sourceAdmission.free();
    userSignature.free();
  }
}

export function projectBrowserAppFirstRun(exports, packageBytes, inputBytes) {
  const memory = coreMemory(exports);
  requireCoreExports(exports, REQUIRED_PROJECTION_EXPORTS);

  const manifest = copyInput(exports, memory, packageBytes.manifestBytes);
  const graph = copyInput(exports, memory, packageBytes.graphBytes);
  const signature = copyInput(exports, memory, packageBytes.developerSignatureBytes);
  const input = copyInput(exports, memory, inputBytes);
  try {
    return withCoreOutput(exports, memory, (outPtr, outLen) =>
      exports.edgerun_browser_core_first_run_projection(
        manifest.ptr,
        manifest.len,
        graph.ptr,
        graph.len,
        signature.ptr,
        signature.len,
        input.ptr,
        input.len,
        outPtr,
        outLen,
      ),
    );
  } finally {
    manifest.free();
    graph.free();
    signature.free();
    input.free();
  }
}

export async function runBrowserAppFirstRun(host, packageBytes, inputBytes, options = {}) {
  const resolvedInputBytes =
    inputBytes == null || isFirstRunInputOptions(inputBytes)
      ? createBrowserAppFirstRunInput(host.exports, inputBytes ?? options.input)
      : inputBytes;
  const projectionBytes = projectBrowserAppFirstRun(host.exports, packageBytes, resolvedInputBytes);
  if (options.store) {
    const key = options.packageKey ?? "first-run:package";
    await options.store.putPackage(`${key}:manifest`, packageBytes.manifestBytes);
    await options.store.putPackage(`${key}:graph`, packageBytes.graphBytes);
    await options.store.putPackage(`${key}:signature`, packageBytes.developerSignatureBytes);
    await options.store.putRecord(`${key}:input`, resolvedInputBytes);
    await options.store.putRecord(`${key}:projection`, projectionBytes);
  }
  const resultBytes = host.recordFirstRunProjection(projectionBytes, { time: options.time });
  return { projectionBytes, resultBytes };
}

const REQUIRED_INPUT_EXPORTS = [
  "edgerun_browser_host_alloc",
  "edgerun_browser_host_free",
  "edgerun_browser_core_free",
  "edgerun_browser_core_first_run_input",
];

const REQUIRED_PROJECTION_EXPORTS = [
  "edgerun_browser_host_alloc",
  "edgerun_browser_host_free",
  "edgerun_browser_core_free",
  "edgerun_browser_core_first_run_projection",
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

function isFirstRunInputOptions(value) {
  return !(value instanceof Uint8Array || value instanceof ArrayBuffer || ArrayBuffer.isView(value));
}

function sequence(value) {
  return BigInt(value == null ? 0 : value);
}

function timestamp(value) {
  return BigInt(value == null ? Date.now() : value);
}

function cost(value) {
  return BigInt(value == null ? 0 : value);
}

function decision(value) {
  return value == null ? APP_RUN_DECISION_VERIFY_AND_CACHE : value;
}
