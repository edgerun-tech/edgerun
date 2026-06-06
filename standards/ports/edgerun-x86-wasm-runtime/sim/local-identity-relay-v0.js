#!/usr/bin/env node
"use strict";

const assert = require("assert");
const crypto = require("crypto");
const path = require("path");

const abi = require("../abi/app-abi-v0.js");
const {
  assertCommitReplay,
  simulateWatAppCommit,
} = require("./local-kernel-commit-v0.js");
const {
  runTrustedWatCallSequenceMemory,
  runTrustedWatExportMemory,
} = require("./local-memory-sim.js");

const port = path.resolve(__dirname, "..");
const root = path.resolve(port, "../../..");
const torLibraryWat = path.join(root, "standards/build/wasm/app-primitives/tor-wat/tor-library.wat");

const hsWat = {
  out: 4096,
  data: 8192,
  identity: 12288,
  name: 12400,
  body: 12500,
  contacts: 32768,
  contactSize: 68,
  contactNameLenOffset: 32,
  contactIdentityOffset: 36,
  messages: 33856,
  messageSize: 420,
  messageBodyOffset: 32,
  messageBodyLenOffset: 416,
};

function sha256(parts) {
  const hash = crypto.createHash("sha256");
  for (const part of parts) {
    if (Buffer.isBuffer(part)) {
      hash.update(part);
    } else {
      hash.update(String(part));
    }
  }
  return hash.digest("hex");
}

function identityFromRecord(record) {
  assert(record.targetLo !== 0 || record.targetHi !== 0, "route record must carry identity material");
  return `${record.targetHi.toString(16).padStart(8, "0")}${record.targetLo.toString(16).padStart(8, "0")}`;
}

function routeRecordsFromEvent(event) {
  return (event.intents || []).filter((record) => record.kind === abi.intentKinds.routeSendIdentity);
}

function hiddenServiceRecordsFromEvent(event) {
  return (event.intents || []).filter((record) => record.kind === abi.intentKinds.routeCreateHiddenService);
}

function assertRouteRecord({ event, record }) {
  assert.strictEqual(record.kind, abi.intentKinds.routeSendIdentity, "relay can only accept identity-route records");
  assert(record.flags & abi.intentFlags.routeIdentityTarget, "relay route record must target identity");
  assert(record.amount > 0, "relay route record must declare payload bytes");
  assert.strictEqual(record.amount, event.routeQueuedBytes, "relay route bytes must match committed event route bytes");
  assert.strictEqual(record.aux0, 1, "relay route record must declare sealed payload");
  assert.strictEqual(record.aux1, 1, "relay route record must declare delivery receipt request");
  assert.strictEqual(event.transitionKind, 2, "relay routes must come from committed action/message transitions");
  assert.strictEqual(event.routeQueuedBytes > 0, true, "relay event must carry route bytes");
}

function assertHiddenServiceRecord({ record }) {
  assert.strictEqual(record.kind, abi.intentKinds.routeCreateHiddenService, "relay can only register hidden-service records");
  assert(record.flags & abi.intentFlags.routeIdentityTarget, "hidden service record must be identity-routed");
  assert.strictEqual(record.amount, 0, "hidden service record must not encode raw port or payload bytes");
  assert.strictEqual(record.aux0, 0, "hidden service record must not encode raw host or port material");
  assert.strictEqual(record.aux1, 0, "hidden service record must not encode raw host or port material");
  assert(record.targetLo !== 0 || record.targetHi !== 0, "hidden service record must carry service identity material");
}

function relayReceipt({ appId, eventHash, targetIdentity, phase, amount }) {
  return sha256([
    "edgerun.identity-relay.receipt.v0",
    appId,
    eventHash,
    targetIdentity,
    phase,
    ":",
    amount,
  ]);
}

function bufferHash(buffer) {
  return crypto.createHash("sha256").update(buffer).digest("hex");
}

function bytes(value) {
  return [...Buffer.from(value, "utf8")];
}

function identityBytes(identity) {
  return [...crypto.createHash("sha256").update(`edgerun.hs.identity.v0:${identity}`).digest()];
}

function runHsExport({ exportName, args, expected, memoryWrites = [] }) {
  return runTrustedWatExportMemory({
    watPath: torLibraryWat,
    exportName,
    args,
    expected,
    memoryBytes: 262144,
    memoryWrites,
  });
}

function sliceOutput(memory, len) {
  return Buffer.from(memory.subarray(hsWat.out, hsWat.out + len));
}

function buildHiddenServiceArtifacts({ appId, eventHash, serviceIdentity, routeTargetIdentity, payloadBytes }) {
  const onion = bytes(serviceIdentity);
  const fetchLen = 14 + onion.length + 41;
  const fetchMemory = runHsExport({
    exportName: "er_tor_hsdir_build_fetch_request",
    args: [hsWat.out, hsWat.data, onion.length],
    expected: fetchLen,
    memoryWrites: [{ offset: hsWat.data, bytes: onion }],
  });
  const fetchRequest = sliceOutput(fetchMemory, fetchLen);

  const descriptor = Buffer.from(
    `edgerun-hs-v0\napp=${appId}\nevent=${eventHash}\nservice=${serviceIdentity}\n`,
    "utf8",
  );
  const armoredLen = 24 + Math.ceil(descriptor.length / 3) * 4 + 1 + 21;
  const armorMemory = runHsExport({
    exportName: "er_tor_hs_desc_armor_message",
    args: [hsWat.out, hsWat.data, descriptor.length],
    expected: armoredLen,
    memoryWrites: [{ offset: hsWat.data, bytes: [...descriptor] }],
  });
  const armoredDescriptor = sliceOutput(armorMemory, armoredLen);

  const publishLen = 55 + String(armoredLen).length + 17;
  const publishMemory = runHsExport({
    exportName: "er_tor_hsdir_build_publish_header",
    args: [hsWat.out, armoredLen],
    expected: publishLen,
  });
  const publishHeader = sliceOutput(publishMemory, publishLen);

  const serviceIdBytes = identityBytes(serviceIdentity);
  const contactName = bytes("local-service");
  const contactLen = 41 + contactName.length;
  const contactMemory = runHsExport({
    exportName: "er_tor_hs_app_build_contact_put",
    args: [hsWat.out, hsWat.name, contactName.length, hsWat.identity, 32],
    expected: contactLen,
    memoryWrites: [
      { offset: hsWat.name, bytes: contactName },
      { offset: hsWat.identity, bytes: serviceIdBytes },
    ],
  });
  const contactFrame = sliceOutput(contactMemory, contactLen);

  const body = Buffer.from(
    `sealed-route:${routeTargetIdentity}:bytes=${payloadBytes}`,
    "utf8",
  );
  const messageLen = 42 + body.length;
  const messageMemory = runHsExport({
    exportName: "er_tor_hs_app_build_message_put",
    args: [hsWat.out, hsWat.identity, 32, hsWat.body, body.length],
    expected: messageLen,
    memoryWrites: [
      { offset: hsWat.identity, bytes: serviceIdBytes },
      { offset: hsWat.body, bytes: [...body] },
    ],
  });
  const messageFrame = sliceOutput(messageMemory, messageLen);

  const stateMemory = runTrustedWatCallSequenceMemory({
    watPath: torLibraryWat,
    memoryBytes: 262144,
    calls: [
      { exportName: "er_tor_hs_app_init", args: [], expected: 0 },
      {
        exportName: "er_tor_hs_app_handle_frame",
        args: [hsWat.data, contactLen],
        expected: 0,
        memoryWrites: [{ offset: hsWat.data, bytes: [...contactFrame] }],
      },
      { exportName: "er_tor_hs_app_contact_count", args: [], expected: 1 },
      { exportName: "er_tor_hs_app_contact_ptr", args: [0], expected: hsWat.contacts },
      {
        exportName: "er_tor_hs_app_handle_frame",
        args: [hsWat.data, messageLen],
        expected: 0,
        memoryWrites: [{ offset: hsWat.data, bytes: [...messageFrame] }],
      },
      { exportName: "er_tor_hs_app_message_count", args: [], expected: 1 },
      { exportName: "er_tor_hs_app_message_ptr", args: [0], expected: hsWat.messages },
    ],
  });
  const contactRecord = Buffer.from(stateMemory.subarray(hsWat.contacts, hsWat.contacts + hsWat.contactSize));
  const messageRecord = Buffer.from(stateMemory.subarray(hsWat.messages, hsWat.messages + hsWat.messageSize));
  const storedNameLen = stateMemory.readUInt32LE(hsWat.contacts + hsWat.contactNameLenOffset);
  const storedName = Buffer.from(stateMemory.subarray(hsWat.contacts, hsWat.contacts + storedNameLen));
  const storedIdentity = Buffer.from(
    stateMemory.subarray(
      hsWat.contacts + hsWat.contactIdentityOffset,
      hsWat.contacts + hsWat.contactIdentityOffset + 32,
    ),
  );
  const storedMessageIdentity = Buffer.from(stateMemory.subarray(hsWat.messages, hsWat.messages + 32));
  const storedBodyLen = stateMemory.readUInt32LE(hsWat.messages + hsWat.messageBodyLenOffset);
  const storedBody = Buffer.from(
    stateMemory.subarray(
      hsWat.messages + hsWat.messageBodyOffset,
      hsWat.messages + hsWat.messageBodyOffset + storedBodyLen,
    ),
  );
  assert.strictEqual(storedName.toString("utf8"), "local-service", "WAT contact state name mismatch");
  assert.deepStrictEqual([...storedIdentity], serviceIdBytes, "WAT contact state identity mismatch");
  assert.deepStrictEqual([...storedMessageIdentity], serviceIdBytes, "WAT message state identity mismatch");
  assert.strictEqual(storedBody.toString("utf8"), body.toString("utf8"), "WAT message state body mismatch");

  return {
    hsdirFetchRequestBytes: fetchLen,
    hsdirFetchRequestHash: bufferHash(fetchRequest),
    hsdirPublishHeaderBytes: publishLen,
    hsdirPublishHeaderHash: bufferHash(publishHeader),
    descriptorArmorBytes: armoredLen,
    descriptorArmorHash: bufferHash(armoredDescriptor),
    contactFrameBytes: contactLen,
    contactFrameHash: bufferHash(contactFrame),
    messageFrameBytes: messageLen,
    messageFrameHash: bufferHash(messageFrame),
    messageFrameAccepted: true,
    watContactCount: 1,
    watMessageCount: 1,
    watContactStateBytes: hsWat.contactSize,
    watContactStateHash: bufferHash(contactRecord),
    watMessageStateBytes: hsWat.messageSize,
    watMessageStateHash: bufferHash(messageRecord),
  };
}

function relayCommittedRoutes({ app, commit, validate = true } = {}) {
  assert(app, "relayCommittedRoutes requires app");
  assert(commit, "relayCommittedRoutes requires commit");
  if (validate) {
    assertCommitReplay({
      appId: app.appId,
      eventLog: commit.eventLog,
      transitions: app.transitions,
    });
  }

  const accepted = [];
  const hiddenServices = [];
  for (const event of commit.eventLog) {
    const routeRecords = routeRecordsFromEvent(event);
    const hiddenServiceRecords = hiddenServiceRecordsFromEvent(event);
    if (event.routeQueuedBytes === 0) {
      assert.strictEqual(routeRecords.length, 0, "event without route bytes must not carry route records");
    } else {
      assert.strictEqual(routeRecords.length, 1, "v0 relay expects one identity route record per routed event");
      const record = routeRecords[0];
      assertRouteRecord({ event, record });
      const targetIdentity = identityFromRecord(record);
      accepted.push({
        appId: event.appId,
        sourceEventIndex: event.index,
        sourceEventHash: event.eventHash,
        targetIdentity,
        payloadBytes: record.amount,
        payloadSealed: true,
        accepted: true,
        transport: "identity-memory-relay-v0",
        transitReceipt: relayReceipt({
          appId: event.appId,
          eventHash: event.eventHash,
          targetIdentity,
          phase: "transit",
          amount: record.amount,
        }),
        deliveryReceipt: relayReceipt({
          appId: event.appId,
          eventHash: event.eventHash,
          targetIdentity,
          phase: "delivery",
          amount: record.amount,
        }),
      });
    }

    for (const record of hiddenServiceRecords) {
      assertHiddenServiceRecord({ record });
      const serviceIdentity = identityFromRecord(record);
      const matchingRoute = accepted.find((route) => route.sourceEventHash === event.eventHash);
      const artifacts = buildHiddenServiceArtifacts({
        appId: event.appId,
        eventHash: event.eventHash,
        serviceIdentity,
        routeTargetIdentity: matchingRoute?.targetIdentity || serviceIdentity,
        payloadBytes: matchingRoute?.payloadBytes || 0,
      });
      hiddenServices.push({
        appId: event.appId,
        sourceEventIndex: event.index,
        sourceEventHash: event.eventHash,
        serviceIdentity,
        registered: true,
        ingress: "hidden-service-identity-v0",
        rawListenPort: null,
        registrationReceipt: relayReceipt({
          appId: event.appId,
          eventHash: event.eventHash,
          targetIdentity: serviceIdentity,
          phase: "hidden-service",
          amount: 0,
        }),
        ...artifacts,
      });
    }
  }

  return {
    appId: commit.appId,
    sourceEventHash: commit.eventHash,
    routeCount: accepted.length,
    hiddenServiceCount: hiddenServices.length,
    accepted,
    hiddenServices,
    receipts: [
      ...accepted.flatMap((route) => [
        { kind: "transit", id: route.transitReceipt, sourceEventHash: route.sourceEventHash },
        { kind: "delivery", id: route.deliveryReceipt, sourceEventHash: route.sourceEventHash },
      ]),
      ...hiddenServices.map((service) => ({
        kind: "hidden_service",
        id: service.registrationReceipt,
        sourceEventHash: service.sourceEventHash,
      })),
    ],
  };
}

function simulateWatAppIdentityRelay(options = {}) {
  const result = simulateWatAppCommit(options);
  return {
    ...result,
    relay: relayCommittedRoutes(result),
  };
}

if (require.main === module) {
  const watPath = process.argv[2] || path.join(port, "tests/app-abi-v0.wat");
  const result = simulateWatAppIdentityRelay({ watPath });
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
}

module.exports = {
  assertHiddenServiceRecord,
  assertRouteRecord,
  buildHiddenServiceArtifacts,
  hiddenServiceRecordsFromEvent,
  identityFromRecord,
  relayCommittedRoutes,
  routeRecordsFromEvent,
  simulateWatAppIdentityRelay,
};
