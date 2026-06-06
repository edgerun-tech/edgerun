#!/usr/bin/env node
"use strict";

const assert = require("assert");
const crypto = require("crypto");
const path = require("path");

const abi = require("../abi/app-abi-v0.js");
const policy = require("../policy/app-policy-v0.js");
const { simulateWatApp } = require("./local-memory-sim.js");

const port = path.resolve(__dirname, "..");

function sha256(parts) {
  const hash = crypto.createHash("sha256");
  for (const part of parts) {
    if (Buffer.isBuffer(part)) {
      hash.update(part);
    } else {
      hash.update(String(part));
    }
  }
  return hash.digest();
}

function hex(buffer) {
  return Buffer.from(buffer).toString("hex");
}

function u64le(value) {
  const out = Buffer.alloc(8);
  out.writeBigUInt64LE(BigInt(value), 0);
  return out;
}

function receiptId({ appId, clock, eventHash, kind, ordinal, amount }) {
  return hex(
    sha256([
      "edgerun.receipt.v0",
      appId,
      u64le(clock),
      eventHash,
      kind,
      ":",
      ordinal,
      ":",
      amount,
    ]),
  );
}

function stateRoot({ appId, previousStateRoot, eventHash, index }) {
  return hex(
    sha256([
      "edgerun.state-root.v0",
      appId,
      previousStateRoot,
      eventHash,
      ":",
      index,
    ]),
  );
}

function assertCommitReplay({ appId, eventLog, transitions }) {
  const appIdBytes = Buffer.isBuffer(appId) ? appId : Buffer.from(appId, "hex");
  let clock = 0;
  let previousHash = Buffer.alloc(32, 0);
  let previousStateRoot = "00".repeat(32);
  assert.strictEqual(eventLog.length, transitions.length, "commit replay transition count mismatch");
  for (let index = 0; index < transitions.length; index += 1) {
    const item = transitions[index];
    const transition = item.transition || item;
    const event = eventLog[index];
    clock += transition.clockDelta;
    const eventHash = policy.eventHash({ appId: appIdBytes, clock, previousHash, transition });
    const eventHashHex = hex(eventHash);
    const nextStateRoot = stateRoot({ appId: appIdBytes, previousStateRoot, eventHash, index });
    assert.strictEqual(event.index, index, "commit replay index mismatch");
    assert.strictEqual(event.clock, clock, "commit replay clock mismatch");
    assert.strictEqual(event.previousHash, hex(previousHash), "commit replay previous hash mismatch");
    assert.strictEqual(event.eventHash, eventHashHex, "commit replay event hash mismatch");
    assert.strictEqual(event.previousStateRoot, previousStateRoot, "commit replay previous state root mismatch");
    assert.strictEqual(event.nextStateRoot, nextStateRoot, "commit replay next state root mismatch");
    previousHash = eventHash;
    previousStateRoot = nextStateRoot;
  }
  return {
    clock,
    eventHash: hex(previousHash),
    stateRoot: previousStateRoot,
  };
}

function receiptKindsForTransition(transition) {
  const receipts = [{ kind: "compute", amount: transition.fuelUsed }];
  if (transition.storageIntentCount > 0) {
    receipts.push({ kind: "storage", amount: transition.storageBytesRequested });
  }
  if (transition.sealedObjectIntentCount > 0) {
    receipts.push({ kind: "sealed", amount: transition.sealedObjectIntentCount });
  }
  if (transition.routeIntentCount > 0) {
    receipts.push({ kind: "route", amount: transition.routeBytesRequested });
  }
  if (transition.childSpawnIntentCount > 0) {
    receipts.push({
      kind: "child",
      amount: transition.childMemoryBytes + transition.childStorageBytes,
    });
  }
  return receipts;
}

function intentRecordsOfKind(transition, kind) {
  return (transition.intentRecords || []).filter((record) => record.kind === kind);
}

function assertIntentRecords(transition) {
  assert(Array.isArray(transition.intentRecords), "committed transition must include decoded intent records");
  const expectedCount = policy.transitionIntentRecordCount(transition);
  assert.strictEqual(transition.intentRecords.length, expectedCount, "intent record count must match transition header counts");

  const storage = intentRecordsOfKind(transition, abi.intentKinds.storageAppend);
  const route = intentRecordsOfKind(transition, abi.intentKinds.routeSendIdentity);
  const hiddenService = intentRecordsOfKind(transition, abi.intentKinds.routeCreateHiddenService);
  const sealed = intentRecordsOfKind(transition, abi.intentKinds.sealedStore);
  const child = intentRecordsOfKind(transition, abi.intentKinds.childSpawn);
  const dependency = intentRecordsOfKind(transition, abi.intentKinds.dependencyCall);

  assert.strictEqual(storage.length, transition.storageIntentCount, "storage intent records must match storage intent count");
  assert.strictEqual(route.length, transition.routeIntentCount, "route intent records must match route intent count");
  assert.strictEqual(hiddenService.length, transition.hiddenServiceIntentCount, "hidden service intent records must match hidden service intent count");
  assert.strictEqual(sealed.length, transition.sealedObjectIntentCount, "sealed intent records must match sealed object intent count");
  assert.strictEqual(child.length, transition.childSpawnIntentCount, "child intent records must match child spawn intent count");
  assert.strictEqual(dependency.length, transition.dependencyCallIntentCount, "dependency intent records must match dependency call intent count");

  const storageBytes = storage.reduce((sum, record) => sum + record.amount, 0);
  assert.strictEqual(storageBytes, transition.storageBytesRequested, "storage intent records must sum to requested storage bytes");
  for (const record of storage) {
    assert(record.flags & abi.intentFlags.storageEventLog, "storage intent must queue an event-log append");
    assert(record.flags & abi.intentFlags.storageDurableResult, "storage intent must queue a durable result");
    assert.strictEqual(record.aux0, transition.eventAppendIntentCount, "storage intent event count mismatch");
    assert.strictEqual(record.aux1, transition.durableResultIntentCount, "storage intent durable result count mismatch");
  }

  const routeBytes = route.reduce((sum, record) => sum + record.amount, 0);
  assert.strictEqual(routeBytes, transition.routeBytesRequested, "route intent records must sum to requested route bytes");
  for (const record of route) {
    assert(record.flags & abi.intentFlags.routeIdentityTarget, "route intent must target an identity");
    assert(record.targetLo !== 0 || record.targetHi !== 0, "route intent must carry target identity material");
  }

  for (const record of hiddenService) {
    assert(record.flags & abi.intentFlags.routeIdentityTarget, "hidden service intent must be identity-routed");
    assert.strictEqual(record.amount, 0, "hidden service intent must not encode raw port or payload bytes");
    assert.strictEqual(record.aux0, 0, "hidden service intent must not encode raw host or port material");
    assert.strictEqual(record.aux1, 0, "hidden service intent must not encode raw host or port material");
    assert(record.targetLo !== 0 || record.targetHi !== 0, "hidden service intent must carry service identity material");
  }

  for (const record of sealed) {
    assert(record.flags & abi.intentFlags.sealedHasRequirements, "sealed intent must carry data requirements");
    assert.strictEqual(record.principalMask, transition.sealedPrincipalMask, "sealed intent principal mask mismatch");
    assert(record.aux0 > 0, "sealed intent must declare requirement count");
  }

  for (const record of child) {
    assert.strictEqual(record.amount, transition.childMemoryBytes, "child intent memory amount mismatch");
    assert.strictEqual(record.aux0, transition.childStorageBytes, "child intent storage amount mismatch");
    assert(record.targetLo !== 0 || record.targetHi !== 0, "child intent must carry target app identity material");
  }

  for (const record of dependency) {
    assert(record.targetLo !== 0 || record.targetHi !== 0, "dependency intent must carry pinned executable identity material");
  }
}

function requiredReceiptMaskForTransition(transition) {
  let mask = 0;
  if (transition.storageIntentCount > 0) mask |= abi.receiptMasks.storage;
  if (transition.sealedObjectIntentCount > 0) mask |= abi.receiptMasks.sealed;
  if (transition.routeIntentCount > 0) mask |= abi.receiptMasks.route;
  if (transition.childSpawnIntentCount > 0) mask |= abi.receiptMasks.child;
  return mask;
}

function queuedWritesForTransition({ index, clock, eventHash, transition }) {
  const writes = [];
  const storage = intentRecordsOfKind(transition, abi.intentKinds.storageAppend);
  for (let i = 0; i < storage.length; i += 1) {
    const record = storage[i];
    writes.push({
      kind: "event-log",
      index,
      clock,
      hash: hex(eventHash),
      target: `${record.targetHi.toString(16)}${record.targetLo.toString(16)}`,
      scheduledBy: "kernel",
    });
    writes.push({
      kind: "durable-result",
      index,
      bytes: record.amount,
      target: `${record.targetHi.toString(16)}${record.targetLo.toString(16)}`,
      scheduledBy: "kernel",
    });
  }
  return writes;
}

function assertGrantInvariant(name, used, grant) {
  assert(used <= grant, `${name} used ${used} bytes exceeds grant ${grant} bytes`);
}

function commitSimulation({
  appId,
  manifest,
  transitions,
  memoryGrantBytes = Number(manifest["memory.min"]),
  storageGrantBytes = Number(manifest["storage.min"]),
  validate = true,
} = {}) {
  assert(appId, "commitSimulation requires appId");
  assert(manifest, "commitSimulation requires manifest");
  assert(Array.isArray(transitions), "commitSimulation requires transitions");

  const appIdBytes = Buffer.isBuffer(appId) ? appId : Buffer.from(appId, "hex");
  assert.strictEqual(appIdBytes.length, 32, "commitSimulation appId must be 32 bytes");
  assert(memoryGrantBytes >= Number(manifest["memory.min"]), "memory grant must cover manifest memory.min");
  assert(storageGrantBytes >= Number(manifest["storage.min"]), "storage grant must cover manifest storage.min");

  let clock = 0;
  let previousHash = Buffer.alloc(32, 0);
  let previousStateRoot = "00".repeat(32);
  let storageUsedBytes = 0;
  let childMemoryMovedBytes = 0;
  let childStorageMovedBytes = 0;
  const eventLog = [];
  const queuedWrites = [];

  for (let index = 0; index < transitions.length; index += 1) {
    const item = transitions[index];
    const transition = item.transition || item;
    if (validate) {
      policy.assertTransitionIdentityPolicy("local-kernel-commit", appIdBytes, appIdBytes);
      policy.assertTransitionPolicy("local-kernel-commit", transition, manifest);
      assertIntentRecords(transition);
    }

    const nextClock = clock + transition.clockDelta;
    const eventHash = policy.eventHash({
      appId: appIdBytes,
      clock: nextClock,
      previousHash,
      transition,
    });
    const eventHashHex = hex(eventHash);
    if (item.hash) {
      assert.strictEqual(item.hash, eventHashHex, "transition hash must match kernel recompute");
    }

    storageUsedBytes += transition.storageBytesRequested;
    childMemoryMovedBytes += transition.childMemoryBytes;
    childStorageMovedBytes += transition.childStorageBytes;
    assertGrantInvariant("storage", storageUsedBytes + childStorageMovedBytes, storageGrantBytes);
    assertGrantInvariant("child memory", childMemoryMovedBytes, memoryGrantBytes);

    const expectedMask = requiredReceiptMaskForTransition(transition);
    assert.strictEqual(
      transition.receiptRequiredMask & expectedMask,
      expectedMask,
      "transition receipt mask must cover committed authority",
    );

    const receipts = receiptKindsForTransition(transition).map((receipt, ordinal) => ({
      ...receipt,
      id: receiptId({
        appId: appIdBytes,
        clock: nextClock,
        eventHash,
        kind: receipt.kind,
        ordinal,
        amount: receipt.amount,
      }),
      signedBy: "local-device-kernel",
    }));
    assert(
      receipts.length <= transition.contributionReceiptCount + transition.receiptIntentCount,
      "kernel receipts must be representable by transition receipt declarations",
    );

    const writes = queuedWritesForTransition({ index, clock: nextClock, eventHash, transition });
    queuedWrites.push(...writes);
    const nextStateRoot = stateRoot({
      appId: appIdBytes,
      previousStateRoot,
      eventHash,
      index,
    });
    eventLog.push({
      index,
      appId: hex(appIdBytes),
      clock: nextClock,
      previousHash: hex(previousHash),
      eventHash: eventHashHex,
      previousStateRoot,
      nextStateRoot,
      transitionKind: transition.kind,
      storageQueuedBytes: transition.storageBytesRequested,
      routeQueuedBytes: transition.routeBytesRequested,
      sealedObjectCount: transition.sealedObjectIntentCount,
      childSpawnCount: transition.childSpawnIntentCount,
      intents: transition.intentRecords,
      childMemoryMovedBytes: transition.childMemoryBytes,
      childStorageMovedBytes: transition.childStorageBytes,
      receiptRequiredMask: transition.receiptRequiredMask,
      receipts,
      queuedWrites: writes,
    });

    clock = nextClock;
    previousHash = eventHash;
    previousStateRoot = nextStateRoot;
  }

  assertCommitReplay({ appId: appIdBytes, eventLog, transitions });

  return {
    appId: hex(appIdBytes),
    clock,
    eventHash: hex(previousHash),
    stateRoot: previousStateRoot,
    grants: {
      memoryGrantBytes,
      storageGrantBytes,
      childMemoryMovedBytes,
      childStorageMovedBytes,
      storageUsedBytes,
      memoryRemainingBytes: memoryGrantBytes - childMemoryMovedBytes,
      storageRemainingBytes: storageGrantBytes - storageUsedBytes - childStorageMovedBytes,
    },
    eventLog,
    queuedWrites,
  };
}

function simulateWatAppCommit(options = {}) {
  const app = simulateWatApp(options);
  return {
    app,
    commit: commitSimulation({
      appId: app.appId,
      manifest: app.manifest,
      transitions: app.transitions,
    }),
  };
}

if (require.main === module) {
  const watPath = process.argv[2] || path.join(port, "tests/app-abi-v0.wat");
  const result = simulateWatAppCommit({ watPath });
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
}

module.exports = {
  assertCommitReplay,
  assertIntentRecords,
  commitSimulation,
  receiptKindsForTransition,
  requiredReceiptMaskForTransition,
  simulateWatAppCommit,
};
