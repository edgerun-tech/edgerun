import assert from "node:assert/strict";
import test from "node:test";
import { EdgeRunBrowserHost } from "./host-adapter.mjs";
import {
  APP_RUN_DECISION_RUN_ONCE,
  createBrowserAppFirstRunInput,
  projectBrowserAppFirstRun,
  runBrowserAppFirstRun,
} from "./first-run.mjs";

test("first-run adapter asks Rust to serialize the user decision input", () => {
  const fake = fakeCoreHostExports();
  const inputBytes = createBrowserAppFirstRunInput(fake.exports, {
    profileId: new Uint8Array(32).fill(1),
    runtimeId: new Uint8Array(32).fill(2),
    previousEventSha256: new Uint8Array(32).fill(3),
    firstEventSeq: 8,
    eventTime: 11,
    retrievalCost: 5,
    sourceAdmissionHash: new Uint8Array(32).fill(4),
    decision: APP_RUN_DECISION_RUN_ONCE,
    userSignature: bytes("user"),
  });

  assert.deepEqual([...inputBytes], [...bytes("input-record")]);
  assert.deepEqual(fake.calls.input[0], {
    profile: 1,
    runtime: 2,
    previous: 3,
    firstEventSeq: 8n,
    eventTime: 11n,
    retrievalCost: 5n,
    sourceAdmission: 4,
    decision: APP_RUN_DECISION_RUN_ONCE,
    userSignature: "user",
  });
});

test("first-run adapter projects package bytes through Rust and records the projection", async () => {
  const fake = fakeCoreHostExports();
  const host = new EdgeRunBrowserHost(fake.exports).open(new Uint8Array(32).fill(4));
  const store = new MemoryStore();

  const result = await runBrowserAppFirstRun(
    host,
    {
      manifestBytes: bytes("manifest"),
      graphBytes: bytes("graph"),
      developerSignatureBytes: bytes("signature"),
    },
    {
      profileId: new Uint8Array(32).fill(1),
      runtimeId: new Uint8Array(32).fill(2),
      decision: APP_RUN_DECISION_RUN_ONCE,
      userSignature: bytes("user"),
    },
    { store, packageKey: "pkg", time: 23 },
  );

  assert.deepEqual([...result.projectionBytes], [...bytes("projection-bundle")]);
  assert.deepEqual([...result.resultBytes], [...bytes("host-first-run-projection-ok")]);
  assert.deepEqual(fake.calls.project[0], ["manifest", "graph", "signature", "input-record"]);
  assert.deepEqual(fake.calls.recordProjection[0], {
    input: "projection-bundle",
    time: 23n,
  });
  assert.deepEqual([...(await store.getRecord("pkg:input"))], [...bytes("input-record")]);
  assert.deepEqual([...(await store.getRecord("pkg:projection"))], [...bytes("projection-bundle")]);

  host.close();
});

test("first-run projection reports Rust validation failures", () => {
  const fake = fakeCoreHostExports({ projectStatus: 2 });

  assert.throws(
    () =>
      projectBrowserAppFirstRun(
        fake.exports,
        {
          manifestBytes: bytes("manifest"),
          graphBytes: bytes("graph"),
          developerSignatureBytes: bytes("signature"),
        },
        bytes("first-run-input"),
      ),
    /edgerun browser core FFI failed with status 2/,
  );
});

function fakeCoreHostExports(options = {}) {
  const memory = new WebAssembly.Memory({ initial: 1 });
  let bump = 16;
  let nextHandle = 0x2000;
  const calls = {
    input: [],
    project: [],
    recordProjection: [],
  };

  const alloc = (len) => {
    if (len === 0) {
      return 0;
    }
    const ptr = bump;
    bump += len + 8;
    return ptr;
  };
  const read = (ptr, len) => decoder.decode(new Uint8Array(memory.buffer, ptr, len));
  const writeOutput = (outPtr, outLen, text) => {
    const payload = bytes(text);
    const payloadPtr = alloc(payload.byteLength);
    new Uint8Array(memory.buffer, payloadPtr, payload.byteLength).set(payload);
    const view = new DataView(memory.buffer);
    view.setUint32(outPtr, payloadPtr, true);
    view.setUint32(outLen, payload.byteLength, true);
    return 0;
  };

  return {
    calls,
    exports: {
      memory,
      edgerun_browser_host_abi_version: () => 1,
      edgerun_browser_host_status_ok: () => 0,
      edgerun_browser_host_alloc: alloc,
      edgerun_browser_host_free: () => {},
      edgerun_browser_core_free: () => {},
      edgerun_browser_host_new: () => nextHandle++,
      edgerun_browser_host_drop: () => {},
      edgerun_browser_host_record_first_run: () => {
        throw new Error("legacy first-run path should not be used");
      },
      edgerun_browser_host_record_first_run_projection: (
        handle,
        projectionPtr,
        projectionLen,
        time,
        outPtr,
        outLen,
      ) => {
        calls.recordProjection.push({
          input: read(projectionPtr, projectionLen),
          time,
        });
        return writeOutput(outPtr, outLen, "host-first-run-projection-ok");
      },
      edgerun_browser_host_grant: () => 0,
      edgerun_browser_host_bind_storage: () => 0,
      edgerun_browser_host_open_session: () => 0,
      edgerun_browser_host_invoke_storage: () => 0,
      edgerun_browser_core_first_run_input: (
        profilePtr,
        profileLen,
        runtimePtr,
        runtimeLen,
        previousPtr,
        previousLen,
        firstEventSeq,
        eventTime,
        retrievalCost,
        sourceAdmissionPtr,
        sourceAdmissionLen,
        decision,
        userSignaturePtr,
        userSignatureLen,
        outPtr,
        outLen,
      ) => {
        calls.input.push({
          profile: new Uint8Array(memory.buffer, profilePtr, profileLen)[0],
          runtime: new Uint8Array(memory.buffer, runtimePtr, runtimeLen)[0],
          previous: new Uint8Array(memory.buffer, previousPtr, previousLen)[0],
          firstEventSeq,
          eventTime,
          retrievalCost,
          sourceAdmission: new Uint8Array(
            memory.buffer,
            sourceAdmissionPtr,
            sourceAdmissionLen,
          )[0],
          decision,
          userSignature: read(userSignaturePtr, userSignatureLen),
        });
        return writeOutput(outPtr, outLen, "input-record");
      },
      edgerun_browser_core_first_run_projection: (
        manifestPtr,
        manifestLen,
        graphPtr,
        graphLen,
        signaturePtr,
        signatureLen,
        inputPtr,
        inputLen,
        outPtr,
        outLen,
      ) => {
        if (options.projectStatus != null) {
          return options.projectStatus;
        }
        calls.project.push([
          read(manifestPtr, manifestLen),
          read(graphPtr, graphLen),
          read(signaturePtr, signatureLen),
          read(inputPtr, inputLen),
        ]);
        return writeOutput(outPtr, outLen, "projection-bundle");
      },
    },
  };
}

class MemoryStore {
  constructor() {
    this.records = new Map();
  }

  async putPackage(key, value) {
    this.records.set(`package:${key}`, value);
  }

  async putRecord(key, value) {
    this.records.set(`record:${key}`, value);
  }

  async getRecord(key) {
    return this.records.get(`record:${key}`) ?? null;
  }
}

const encoder = new TextEncoder();
const decoder = new TextDecoder();

function bytes(text) {
  return encoder.encode(text);
}
