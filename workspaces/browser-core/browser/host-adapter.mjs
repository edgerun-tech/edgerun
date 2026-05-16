export const BROWSER_HOST_FFI_STATUS_OK = 0;
export const BROWSER_HOST_FFI_STATUS_NULL_POINTER = 1;
export const BROWSER_HOST_FFI_STATUS_INVALID_LENGTH = 2;
export const WASM32_USIZE_BYTES = 4;

export async function loadEdgeRunBrowserHostWasm(source, imports = {}) {
  const instance = await instantiateWasm(source, imports);
  return new EdgeRunBrowserHost(instance.exports);
}

export class EdgeRunBrowserHost {
  constructor(exports) {
    this.exports = exports;
    this.memory = exports.memory;
    if (!(this.memory instanceof WebAssembly.Memory)) {
      throw new TypeError("edgerun browser host WASM must export memory");
    }
    for (const name of REQUIRED_EXPORTS) {
      if (typeof exports[name] !== "function") {
        throw new TypeError(`edgerun browser host WASM missing export: ${name}`);
      }
    }
    this.handle = 0;
  }

  abiVersion() {
    return this.exports.edgerun_browser_host_abi_version();
  }

  statusOk() {
    return this.exports.edgerun_browser_host_status_ok();
  }

  open(runtimeIdBytes) {
    if (this.handle !== 0) {
      throw new Error("edgerun browser host is already open");
    }
    const runtimeId = this.copyInput(runtimeIdBytes);
    try {
      this.handle = this.exports.edgerun_browser_host_new(runtimeId.ptr, runtimeId.len);
      if (this.handle === 0) {
        throw new Error("edgerun browser host rejected runtime id");
      }
      return this;
    } finally {
      runtimeId.free();
    }
  }

  close() {
    if (this.handle !== 0) {
      this.exports.edgerun_browser_host_drop(this.handle);
      this.handle = 0;
    }
  }

  recordFirstRun(runtimeProjectionBytes, decisionBytes, options = {}) {
    const cacheBytes = options.cacheBytes ?? null;
    const runtimeProjection = this.copyInput(runtimeProjectionBytes);
    const decision = this.copyInput(decisionBytes);
    const cache = cacheBytes == null ? emptyInput() : this.copyInput(cacheBytes);
    try {
      return this.withOutput((outPtr, outLen) =>
        this.exports.edgerun_browser_host_record_first_run(
          this.requireHandle(),
          runtimeProjection.ptr,
          runtimeProjection.len,
          decision.ptr,
          decision.len,
          cache.ptr,
          cache.len,
          cacheBytes == null ? 0 : 1,
          timestamp(options.time),
          outPtr,
          outLen,
        ),
      );
    } finally {
      runtimeProjection.free();
      decision.free();
      cache.free();
    }
  }

  recordFirstRunProjection(projectionBytes, options = {}) {
    const projection = this.copyInput(projectionBytes);
    try {
      return this.withOutput((outPtr, outLen) =>
        this.exports.edgerun_browser_host_record_first_run_projection(
          this.requireHandle(),
          projection.ptr,
          projection.len,
          timestamp(options.time),
          outPtr,
          outLen,
        ),
      );
    } finally {
      projection.free();
    }
  }

  grant(grantBytes, options = {}) {
    const grant = this.copyInput(grantBytes);
    try {
      return this.withOutput((outPtr, outLen) =>
        this.exports.edgerun_browser_host_grant(
          this.requireHandle(),
          grant.ptr,
          grant.len,
          timestamp(options.time),
          outPtr,
          outLen,
        ),
      );
    } finally {
      grant.free();
    }
  }

  bindStorage(bindingBytes, options = {}) {
    const binding = this.copyInput(bindingBytes);
    try {
      return this.withOutput((outPtr, outLen) =>
        this.exports.edgerun_browser_host_bind_storage(
          this.requireHandle(),
          binding.ptr,
          binding.len,
          timestamp(options.time),
          outPtr,
          outLen,
        ),
      );
    } finally {
      binding.free();
    }
  }

  openSession(sessionBytes, options = {}) {
    const session = this.copyInput(sessionBytes);
    try {
      return this.withOutput((outPtr, outLen) =>
        this.exports.edgerun_browser_host_open_session(
          this.requireHandle(),
          session.ptr,
          session.len,
          timestamp(options.time),
          outPtr,
          outLen,
        ),
      );
    } finally {
      session.free();
    }
  }

  invokeStorage(requestBytes, contextBytes) {
    const request = this.copyInput(requestBytes);
    const context = this.copyInput(contextBytes);
    try {
      return this.withOutput((outPtr, outLen) =>
        this.exports.edgerun_browser_host_invoke_storage(
          this.requireHandle(),
          request.ptr,
          request.len,
          context.ptr,
          context.len,
          outPtr,
          outLen,
        ),
      );
    } finally {
      request.free();
      context.free();
    }
  }

  copyInput(bytes) {
    const input = asUint8Array(bytes);
    if (input.byteLength === 0) {
      return emptyInput();
    }
    const ptr = this.exports.edgerun_browser_host_alloc(input.byteLength);
    if (ptr === 0) {
      throw new Error("edgerun browser host allocation failed");
    }
    new Uint8Array(this.memory.buffer, ptr, input.byteLength).set(input);
    return {
      ptr,
      len: input.byteLength,
      free: () => this.exports.edgerun_browser_host_free(ptr, input.byteLength),
    };
  }

  withOutput(call, options = {}) {
    const freeOutput = options.freeOutput ?? this.exports.edgerun_browser_host_free;
    const slots = this.exports.edgerun_browser_host_alloc(WASM32_USIZE_BYTES * 2);
    if (slots === 0) {
      throw new Error("edgerun browser host output slot allocation failed");
    }
    try {
      const status = call(slots, slots + WASM32_USIZE_BYTES);
      if (status !== BROWSER_HOST_FFI_STATUS_OK) {
        throw new Error(`edgerun browser host FFI failed with status ${status}`);
      }
      const view = new DataView(this.memory.buffer);
      const outputPtr = view.getUint32(slots, true);
      const outputLen = view.getUint32(slots + WASM32_USIZE_BYTES, true);
      if (outputLen === 0) {
        return new Uint8Array();
      }
      const output = new Uint8Array(outputLen);
      output.set(new Uint8Array(this.memory.buffer, outputPtr, outputLen));
      freeOutput(outputPtr, outputLen);
      return output;
    } finally {
      this.exports.edgerun_browser_host_free(slots, WASM32_USIZE_BYTES * 2);
    }
  }

  requireHandle() {
    if (this.handle === 0) {
      throw new Error("edgerun browser host is not open");
    }
    return this.handle;
  }
}

export class EdgeRunBrowserByteStore {
  constructor(db) {
    this.db = db;
  }

  static async open(options = {}) {
    const indexedDB = options.indexedDB ?? globalThis.indexedDB;
    if (!indexedDB) {
      throw new Error("IndexedDB is not available");
    }
    const name = options.name ?? "edgerun-browser-core";
    const version = options.version ?? 1;
    const db = await openDatabase(indexedDB, name, version);
    return new EdgeRunBrowserByteStore(db);
  }

  putPackage(key, bytes) {
    return this.putBytes("packages", key, bytes);
  }

  getPackage(key) {
    return this.getBytes("packages", key);
  }

  putObject(key, bytes) {
    return this.putBytes("objects", key, bytes);
  }

  getObject(key) {
    return this.getBytes("objects", key);
  }

  putRecord(key, bytes) {
    return this.putBytes("records", key, bytes);
  }

  getRecord(key) {
    return this.getBytes("records", key);
  }

  putBytes(storeName, key, bytes) {
    return storeOperation(this.db, storeName, "readwrite", (store) => {
      store.put(asUint8Array(bytes), key);
    });
  }

  getBytes(storeName, key) {
    return storeOperation(this.db, storeName, "readonly", (store, resolve, reject) => {
      const request = store.get(key);
      request.onsuccess = () => {
        const value = request.result;
        resolve(value == null ? null : asUint8Array(value));
      };
      request.onerror = () => reject(request.error);
    });
  }

  deleteBytes(storeName, key) {
    return storeOperation(this.db, storeName, "readwrite", (store) => {
      store.delete(key);
    });
  }
}

const REQUIRED_EXPORTS = [
  "edgerun_browser_host_abi_version",
  "edgerun_browser_host_status_ok",
  "edgerun_browser_host_alloc",
  "edgerun_browser_host_free",
  "edgerun_browser_host_new",
  "edgerun_browser_host_drop",
  "edgerun_browser_host_record_first_run",
  "edgerun_browser_host_record_first_run_projection",
  "edgerun_browser_host_grant",
  "edgerun_browser_host_bind_storage",
  "edgerun_browser_host_open_session",
  "edgerun_browser_host_invoke_storage",
];

async function instantiateWasm(source, imports) {
  if (source?.exports) {
    return source;
  }
  if (source instanceof WebAssembly.Module) {
    return new WebAssembly.Instance(source, imports);
  }
  if (source instanceof Response) {
    const result = await WebAssembly.instantiateStreaming(source, imports);
    return result.instance;
  }
  if (source instanceof Uint8Array || source instanceof ArrayBuffer) {
    const result = await WebAssembly.instantiate(source, imports);
    return result.instance;
  }
  const response = await fetch(source);
  const result = await WebAssembly.instantiateStreaming(response, imports);
  return result.instance;
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

function openDatabase(indexedDB, name, version) {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(name, version);
    request.onupgradeneeded = () => {
      const db = request.result;
      for (const storeName of ["packages", "objects", "records"]) {
        if (!db.objectStoreNames.contains(storeName)) {
          db.createObjectStore(storeName);
        }
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

function storeOperation(db, storeName, mode, body) {
  return new Promise((resolve, reject) => {
    const tx = db.transaction(storeName, mode);
    const store = tx.objectStore(storeName);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
    body(store, resolve, reject);
  });
}
