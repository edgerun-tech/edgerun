import assert from "node:assert/strict";
import test from "node:test";
import {
  BROWSER_HOST_FFI_STATUS_NULL_POINTER,
  EdgeRunBrowserHost,
} from "./host-adapter.mjs";

test("browser host adapter copies wire bytes through the raw WASM ABI", () => {
  const fake = fakeHostExports();
  const host = new EdgeRunBrowserHost(fake.exports).open(new Uint8Array(32).fill(7));

  assert.deepEqual(
    [...host.recordFirstRun(bytes("projection"), bytes("decision"), {
      cacheBytes: bytes("cache"),
      time: 11,
    })],
    [...bytes("first-run-ok")],
  );
  assert.deepEqual(
    [...host.recordFirstRunProjection(bytes("projection-bundle"), { time: 12 })],
    [...bytes("first-run-projection-ok")],
  );
  assert.deepEqual([...host.grant(bytes("grant"), { time: 12 })], [...bytes("grant-ok")]);
  assert.deepEqual([...host.bindStorage(bytes("binding"), { time: 13 })], [...bytes("binding-ok")]);
  assert.deepEqual([...host.openSession(bytes("session"), { time: 14 })], [...bytes("session-ok")]);
  assert.deepEqual(
    [...host.invokeStorage(bytes("request"), bytes("context"))],
    [...bytes("storage-ok")],
  );

  host.close();

  assert.equal(fake.calls.newHost.length, 1);
  assert.equal(fake.calls.drop, 1);
  assert.deepEqual(fake.calls.firstRun[0].inputs, ["projection", "decision", "cache"]);
  assert.deepEqual(fake.calls.firstRun[0].flags, { hasCache: 1, time: 11n });
  assert.deepEqual(fake.calls.firstRunProjection[0], {
    handle: 4096,
    input: "projection-bundle",
    time: 12n,
  });
  assert.deepEqual(fake.calls.storage[0], ["request", "context"]);
  assert.equal(fake.freedOutputs.size, 6);
});

test("browser host adapter rejects non-ok FFI status", () => {
  const fake = fakeHostExports({
    grantStatus: BROWSER_HOST_FFI_STATUS_NULL_POINTER,
  });
  const host = new EdgeRunBrowserHost(fake.exports).open(new Uint8Array(32).fill(9));

  assert.throws(
    () => host.grant(bytes("grant")),
    /edgerun browser host FFI failed with status 1/,
  );

  host.close();
});

function fakeHostExports(options = {}) {
  const memory = new WebAssembly.Memory({ initial: 1 });
  let bump = 16;
  let nextHandle = 0x1000;
  const outputPointers = new Set();
  const freedOutputs = new Set();
  const calls = {
    newHost: [],
    firstRun: [],
    firstRunProjection: [],
    storage: [],
    drop: 0,
  };

  const alloc = (len) => {
    if (len === 0) {
      return 0;
    }
    const ptr = bump;
    bump += len + 8;
    return ptr;
  };
  const free = (ptr) => {
    if (outputPointers.has(ptr)) {
      freedOutputs.add(ptr);
    }
  };
  const read = (ptr, len) => decoder.decode(new Uint8Array(memory.buffer, ptr, len));
  const writeOutput = (outPtr, outLen, text) => {
    const payload = bytes(text);
    const payloadPtr = alloc(payload.byteLength);
    outputPointers.add(payloadPtr);
    new Uint8Array(memory.buffer, payloadPtr, payload.byteLength).set(payload);
    const view = new DataView(memory.buffer);
    view.setUint32(outPtr, payloadPtr, true);
    view.setUint32(outLen, payload.byteLength, true);
    return 0;
  };

  return {
    calls,
    freedOutputs,
    exports: {
      memory,
      edgerun_browser_host_abi_version: () => 1,
      edgerun_browser_host_status_ok: () => 0,
      edgerun_browser_host_alloc: alloc,
      edgerun_browser_host_free: free,
      edgerun_browser_host_new: (runtimePtr, runtimeLen) => {
        calls.newHost.push([...new Uint8Array(memory.buffer, runtimePtr, runtimeLen)]);
        return nextHandle++;
      },
      edgerun_browser_host_drop: () => {
        calls.drop += 1;
      },
      edgerun_browser_host_record_first_run: (
        handle,
        projectionPtr,
        projectionLen,
        decisionPtr,
        decisionLen,
        cachePtr,
        cacheLen,
        hasCache,
        time,
        outPtr,
        outLen,
      ) => {
        calls.firstRun.push({
          handle,
          inputs: [
            read(projectionPtr, projectionLen),
            read(decisionPtr, decisionLen),
            read(cachePtr, cacheLen),
          ],
          flags: { hasCache, time },
        });
        return writeOutput(outPtr, outLen, "first-run-ok");
      },
      edgerun_browser_host_record_first_run_projection: (
        handle,
        projectionPtr,
        projectionLen,
        time,
        outPtr,
        outLen,
      ) => {
        calls.firstRunProjection.push({
          handle,
          input: read(projectionPtr, projectionLen),
          time,
        });
        return writeOutput(outPtr, outLen, "first-run-projection-ok");
      },
      edgerun_browser_host_grant: (handle, grantPtr, grantLen, time, outPtr, outLen) => {
        if (options.grantStatus != null) {
          return options.grantStatus;
        }
        assert.equal(read(grantPtr, grantLen), "grant");
        assert.equal(time > 0n, true);
        return writeOutput(outPtr, outLen, "grant-ok");
      },
      edgerun_browser_host_bind_storage: (handle, bindingPtr, bindingLen, time, outPtr, outLen) => {
        assert.equal(read(bindingPtr, bindingLen), "binding");
        return writeOutput(outPtr, outLen, "binding-ok");
      },
      edgerun_browser_host_open_session: (handle, sessionPtr, sessionLen, time, outPtr, outLen) => {
        assert.equal(read(sessionPtr, sessionLen), "session");
        return writeOutput(outPtr, outLen, "session-ok");
      },
      edgerun_browser_host_invoke_storage: (
        handle,
        requestPtr,
        requestLen,
        contextPtr,
        contextLen,
        outPtr,
        outLen,
      ) => {
        calls.storage.push([read(requestPtr, requestLen), read(contextPtr, contextLen)]);
        return writeOutput(outPtr, outLen, "storage-ok");
      },
    },
  };
}

const encoder = new TextEncoder();
const decoder = new TextDecoder();

function bytes(text) {
  return encoder.encode(text);
}
