#!/usr/bin/env node
"use strict";

const assert = require("assert");
const crypto = require("crypto");
const path = require("path");

const { simulateWatAppIdentityRelay } = require("./local-identity-relay-v0.js");
const { runTrustedWatExportMemory } = require("./local-memory-sim.js");

const port = path.resolve(__dirname, "..");
const root = path.resolve(port, "../../..");
const torCellCodecWat = path.join(root, "standards/build/wasm/app-primitives/tor-cell-codec/tor-cell-codec.wat");
const localTorCellRecordWat = path.join(port, "tests/local-tor-cell-record-v0.wat");
const localTorDeliveryProofWat = path.join(port, "tests/local-tor-delivery-proof-v0.wat");

const defaultCircuitIdentities = {
  guard: "guard-local-memory-v0",
  middle: "middle-local-memory-v0",
  rendezvous: "rendezvous-local-memory-v0",
};

const torCommands = {
  CREATE2: 0x0a,
  CREATED2: 0x0b,
  RELAY: 0x03,
};

const relayCommands = {
  EXTEND2: 0x0e,
  EXTENDED2: 0x0f,
  BEGIN: 0x01,
  DATA: 0x02,
  END: 0x03,
};

const handshakeTypes = {
  ntor: 2,
};

const torCellCodec = {
  bodyPtr: 8192,
  cellPtr: 4096,
  fixedLen: 514,
  linkProtocol: 4,
  payloadOffset: 5,
  relayCommandOffset: 0,
  relayStreamIdOffset: 3,
  relayLengthOffset: 9,
};

function sha256(parts) {
  const hash = crypto.createHash("sha256");
  for (const part of parts) hash.update(String(part));
  return hash.digest("hex");
}

function sha256Buffer(buffer) {
  return crypto.createHash("sha256").update(buffer).digest("hex");
}

function sha256Buffers(parts) {
  const hash = crypto.createHash("sha256");
  for (const part of parts) hash.update(part);
  return hash.digest("hex");
}

function hexBytes(value, label) {
  assert.match(value, /^[0-9a-f]{64}$/i, `${label} must be a 32-byte hex value`);
  return Buffer.from(value, "hex");
}

function optionalHexBytes(value, label) {
  if (value === undefined || value === null || value === "") return Buffer.alloc(32, 0);
  return hexBytes(value, label);
}

function asciiIdentityBytes(value, label) {
  assert.match(value, /^[0-9a-f]{16}$/i, `${label} must be an 8-byte hex identity string`);
  return Buffer.from(value, "ascii");
}

function circuitReceipt({ appId, sourceEventHash, identity, phase, amount, cellHash }) {
  return sha256([
    "edgerun.tor-circuit.receipt.v0",
    appId,
    sourceEventHash,
    identity,
    phase,
    ":",
    amount,
    ":",
    cellHash,
  ]);
}

function proofCellHash(cells, sequence) {
  const cell = cells.find((candidate) => candidate.sequence === sequence);
  assert(cell, `Tor delivery proof missing cell sequence ${sequence}`);
  return cell.cellHash;
}

function torDeliveryProofFields({ app, commit, route, service, cells, receipts }) {
  return {
    version: "edgerun.local-tor-delivery.v0",
    appId: app.appId,
    commitEventHash: commit.eventHash,
    sourceEventHash: route.sourceEventHash,
    routeTargetIdentity: route.targetIdentity,
    hiddenServiceIdentity: service.serviceIdentity,
    payloadBytes: route.payloadBytes,
    payloadSealed: true,
    relayTransitReceipt: route.transitReceipt,
    relayDeliveryReceipt: route.deliveryReceipt,
    hiddenServiceRegistrationReceipt: service.registrationReceipt,
    hsdirFetchRequestHash: service.hsdirFetchRequestHash,
    hsdirPublishHeaderHash: service.hsdirPublishHeaderHash,
    descriptorArmorHash: service.descriptorArmorHash,
    contactFrameHash: service.contactFrameHash,
    messageFrameHash: service.messageFrameHash,
    watContactStateHash: service.watContactStateHash,
    watMessageStateHash: service.watMessageStateHash,
    create2CellHash: proofCellHash(cells, 0),
    created2CellHash: proofCellHash(cells, 1),
    extend2CellHash: proofCellHash(cells, 2),
    extended2CellHash: proofCellHash(cells, 3),
    beginCellHash: proofCellHash(cells, 4),
    dataCellHash: proofCellHash(cells, 5),
    endCellHash: proofCellHash(cells, 6),
    circuitReceiptHash: sha256Buffers((receipts || []).map((receipt) => hexBytes(receipt.id, "circuit receipt id"))),
  };
}

function torDeliveryProofFieldBytes(fields) {
  return Buffer.concat([
    hexBytes(fields.appId, "app id"),
    hexBytes(fields.commitEventHash, "commit event hash"),
    hexBytes(fields.sourceEventHash, "source event hash"),
    hexBytes(fields.relayTransitReceipt, "relay transit receipt"),
    hexBytes(fields.relayDeliveryReceipt, "relay delivery receipt"),
    hexBytes(fields.hiddenServiceRegistrationReceipt, "hidden service registration receipt"),
    hexBytes(fields.hsdirFetchRequestHash, "hsdir fetch request hash"),
    hexBytes(fields.hsdirPublishHeaderHash, "hsdir publish header hash"),
    hexBytes(fields.descriptorArmorHash, "descriptor armor hash"),
    hexBytes(fields.contactFrameHash, "contact frame hash"),
    hexBytes(fields.messageFrameHash, "message frame hash"),
    hexBytes(fields.watContactStateHash, "WAT contact state hash"),
    hexBytes(fields.watMessageStateHash, "WAT message state hash"),
    hexBytes(fields.create2CellHash, "CREATE2 cell hash"),
    hexBytes(fields.created2CellHash, "CREATED2 cell hash"),
    hexBytes(fields.extend2CellHash, "EXTEND2 cell hash"),
    hexBytes(fields.extended2CellHash, "EXTENDED2 cell hash"),
    hexBytes(fields.beginCellHash, "BEGIN cell hash"),
    hexBytes(fields.dataCellHash, "DATA cell hash"),
    hexBytes(fields.endCellHash, "END cell hash"),
    hexBytes(fields.circuitReceiptHash, "circuit receipt hash"),
  ]);
}

function buildTorDeliveryProofRecord(fields) {
  const outPtr = 4096;
  const fieldsPtr = 8192;
  const routeIdentityPtr = 12288;
  const hiddenServiceIdentityPtr = 12304;
  const recordLen = 728;
  const memory = runTrustedWatExportMemory({
    watPath: localTorDeliveryProofWat,
    exportName: "er_local_tor_delivery_proof_write",
    args: [
      outPtr,
      fieldsPtr,
      routeIdentityPtr,
      hiddenServiceIdentityPtr,
      fields.payloadBytes,
      fields.payloadSealed ? 1 : 0,
    ],
    expected: recordLen,
    memoryWrites: [
      { offset: fieldsPtr, bytes: [...torDeliveryProofFieldBytes(fields)] },
      { offset: routeIdentityPtr, bytes: [...asciiIdentityBytes(fields.routeTargetIdentity, "route target identity")] },
      { offset: hiddenServiceIdentityPtr, bytes: [...asciiIdentityBytes(fields.hiddenServiceIdentity, "hidden service identity")] },
    ],
  });
  const record = memory.subarray(outPtr, outPtr + recordLen);
  assert.strictEqual(record.readUInt32LE(0), 0x45524450, "local Tor delivery proof magic mismatch");
  assert.strictEqual(record.readUInt32LE(4), 0, "local Tor delivery proof version mismatch");
  assert.strictEqual(record.readUInt32LE(8), fields.payloadBytes, "local Tor delivery proof payload bytes mismatch");
  assert.strictEqual(record.readUInt32LE(12), fields.payloadSealed ? 1 : 0, "local Tor delivery proof sealed flag mismatch");
  assert.strictEqual(record.readUInt32LE(16), 16, "local Tor delivery proof route identity length mismatch");
  assert.strictEqual(record.readUInt32LE(20), 16, "local Tor delivery proof service identity length mismatch");
  assert.strictEqual(record.subarray(24, 40).toString("ascii"), fields.routeTargetIdentity, "local Tor delivery proof route identity mismatch");
  assert.strictEqual(record.subarray(40, 56).toString("ascii"), fields.hiddenServiceIdentity, "local Tor delivery proof service identity mismatch");
  return record;
}

function torDeliveryProofHash(fields) {
  return sha256Buffer(buildTorDeliveryProofRecord(fields));
}

function buildTorDeliveryProof(input) {
  const fields = torDeliveryProofFields(input);
  const canonicalRecord = buildTorDeliveryProofRecord(fields);
  const proofHash = sha256Buffer(canonicalRecord);
  return {
    ...fields,
    canonicalRecordBytes: canonicalRecord.length,
    proofHash,
    receipt: sha256Buffers([
      Buffer.from("edgerun.local-tor-delivery.receipt.v0", "utf8"),
      hexBytes(fields.appId, "receipt app id"),
      hexBytes(fields.sourceEventHash, "receipt source event hash"),
      asciiIdentityBytes(fields.hiddenServiceIdentity, "receipt hidden service identity"),
      hexBytes(proofHash, "receipt proof hash"),
    ]),
  };
}

function assertTorDeliveryProof({ app, commit, relay, tor }) {
  assert(tor?.deliveryProof, "Tor delivery proof missing");
  const route = relay.accepted[0];
  const service = relay.hiddenServices[0];
  const expected = buildTorDeliveryProof({
    app,
    commit,
    route,
    service,
    cells: tor.cells,
    receipts: tor.receipts,
  });
  assert.deepStrictEqual(tor.deliveryProof, expected, "Tor delivery proof mismatch");
}

function receiptKinds(relay) {
  return new Set((relay.receipts || []).map((receipt) => receipt.kind));
}

function assertRelayReadyForTor({ relay }) {
  assert(relay, "Tor circuit requires relay output");
  assert.strictEqual(relay.routeCount, 1, "Tor circuit v0 expects one accepted route");
  assert.strictEqual(relay.hiddenServiceCount, 1, "Tor circuit v0 expects one registered hidden service");
  const kinds = receiptKinds(relay);
  assert(kinds.has("transit"), "Tor circuit requires identity relay transit receipt");
  assert(kinds.has("delivery"), "Tor circuit requires identity relay delivery receipt");
  assert(kinds.has("hidden_service"), "Tor circuit requires hidden-service registration receipt");
  const route = relay.accepted[0];
  const service = relay.hiddenServices[0];
  assert(route.payloadSealed, "Tor circuit requires sealed route payload");
  assert(route.targetIdentity, "Tor circuit route must target identity");
  assert(service.registered, "Tor circuit hidden service must be registered");
  assert(service.serviceIdentity, "Tor circuit hidden service must have identity");
  assert.strictEqual(service.rawListenPort, null, "Tor circuit must not expose raw listen port");
}

function payloadHash({ appId, sourceEventHash, targetIdentity, serviceIdentity, payloadBytes }) {
  return sha256([
    "edgerun.tor-cell.payload.v0",
    appId,
    sourceEventHash,
    targetIdentity,
    serviceIdentity,
    ":",
    payloadBytes,
  ]);
}

function torCellRecordHashBytes(cell) {
  return Buffer.concat([
    hexBytes(cell.appId, "cell app id"),
    hexBytes(cell.sourceEventHash, "cell source event hash"),
    hexBytes(cell.payloadHash, "cell payload hash"),
    optionalHexBytes(cell.onionKeyIdentity, "cell onion key identity"),
    optionalHexBytes(cell.clientEphemeralHash, "cell client ephemeral hash"),
    optionalHexBytes(cell.serverEphemeralHash, "cell server ephemeral hash"),
    optionalHexBytes(cell.handshakeTranscriptHash, "cell handshake transcript hash"),
  ]);
}

function buildTorCellRecord(cell) {
  const outPtr = 4096;
  const hashesPtr = 8192;
  const recordLen = 272;
  const memory = runTrustedWatExportMemory({
    watPath: localTorCellRecordWat,
    exportName: "er_local_tor_cell_record_write",
    args: [
      outPtr,
      hashesPtr,
      cell.circuitId,
      cell.sequence,
      cell.command,
      cell.relayCommand,
      cell.streamId,
      cell.length,
      cell.handshakeType ?? 0,
    ],
    expected: recordLen,
    memoryWrites: [{ offset: hashesPtr, bytes: [...torCellRecordHashBytes(cell)] }],
  });
  const record = memory.subarray(outPtr, outPtr + recordLen);
  assert.strictEqual(record.readUInt32LE(0), 0x45524352, "local Tor cell record magic mismatch");
  assert.strictEqual(record.readUInt32LE(4), 0, "local Tor cell record version mismatch");
  assert.strictEqual(record.readUInt32LE(8), cell.circuitId, "local Tor cell record circuit id mismatch");
  assert.strictEqual(record.readUInt32LE(12), cell.sequence, "local Tor cell record sequence mismatch");
  assert.strictEqual(record.readUInt32LE(16), cell.command, "local Tor cell record command mismatch");
  assert.strictEqual(record.readUInt32LE(20), cell.relayCommand, "local Tor cell record relay command mismatch");
  assert.strictEqual(record.readUInt32LE(24), cell.streamId, "local Tor cell record stream id mismatch");
  assert.strictEqual(record.readUInt32LE(28), cell.length, "local Tor cell record relay length mismatch");
  assert.strictEqual(record.readUInt32LE(32), cell.handshakeType ?? 0, "local Tor cell record handshake type mismatch");
  assert.strictEqual(record.readUInt32LE(36), 0, "local Tor cell record plaintext private byte count mismatch");
  assert.strictEqual(record.readUInt32LE(40), 0, "local Tor cell record private key export count mismatch");
  return record;
}

function cellHash(cell) {
  return sha256Buffer(buildTorCellRecord(cell));
}

function ntorPublicMetadata({ appId, sourceEventHash, guardIdentity }) {
  const onionKeyIdentity = sha256(["edgerun.ntor.onion-key.v0", guardIdentity]);
  const clientEphemeralHash = sha256(["edgerun.ntor.client-ephemeral.v0", appId, sourceEventHash, guardIdentity]);
  const serverEphemeralHash = sha256(["edgerun.ntor.server-ephemeral.v0", appId, sourceEventHash, guardIdentity]);
  const handshakeTranscriptHash = sha256([
    "edgerun.ntor.transcript.v0",
    appId,
    sourceEventHash,
    guardIdentity,
    onionKeyIdentity,
    clientEphemeralHash,
    serverEphemeralHash,
  ]);
  return {
    clientEphemeralHash,
    handshakeTranscriptHash,
    onionKeyIdentity,
    serverEphemeralHash,
  };
}

function makeCell({
  appId,
  sourceEventHash,
  circuitId,
  sequence,
  command,
  relayCommand = 0,
  streamId = 0,
  length = 0,
  payloadHash: cellPayloadHash,
  handshake = null,
}) {
  const cell = {
    appId,
    sourceEventHash,
    circuitId,
    sequence,
    command,
    relayCommand,
    streamId,
    digest: "0".repeat(40),
    length,
    payloadHash: cellPayloadHash,
    plaintextPrivateBytes: 0,
    ...(handshake || {}),
  };
  const canonicalRecord = buildTorCellRecord(cell);
  return {
    ...cell,
    canonicalRecordBytes: canonicalRecord.length,
    cellHash: sha256Buffer(canonicalRecord),
  };
}

function readUInt32BE(buffer, offset) {
  return buffer.readUInt32BE(offset);
}

function readUInt16BE(buffer, offset) {
  return buffer.readUInt16BE(offset);
}

function relayBody({ relayCommand, streamId = 0, length = 0 }) {
  const body = Buffer.alloc(11 + length, 0);
  body.writeUInt8(relayCommand, 0);
  body.writeUInt16BE(streamId, torCellCodec.relayStreamIdOffset);
  body.writeUInt16BE(length, torCellCodec.relayLengthOffset);
  return body;
}

function create2Body() {
  const body = Buffer.alloc(4, 0);
  body.writeUInt16BE(handshakeTypes.ntor, 0);
  body.writeUInt16BE(0, 2);
  return body;
}

function created2Body() {
  const body = Buffer.alloc(2, 0);
  body.writeUInt16BE(0, 0);
  return body;
}

function fixedCellBody({ command, relayCommand, streamId, length }) {
  if (command === torCommands.CREATE2) return create2Body();
  if (command === torCommands.CREATED2) return created2Body();
  if (command === torCommands.RELAY) return relayBody({ relayCommand, streamId, length });
  return Buffer.alloc(0);
}

function buildCodecCellRecord({ sequence, circuitId, command, relayCommand = 0, streamId = 0, length = 0 }) {
  const body = fixedCellBody({ command, relayCommand, streamId, length });
  const memory = runTrustedWatExportMemory({
    watPath: torCellCodecWat,
    exportName: "tor_cell_build_fixed",
    args: [torCellCodec.linkProtocol, torCellCodec.cellPtr, circuitId, command, torCellCodec.bodyPtr, body.length],
    expected: torCellCodec.fixedLen,
    memoryWrites: body.length ? [{ offset: torCellCodec.bodyPtr, bytes: [...body] }] : [],
  });
  const payload = torCellCodec.cellPtr + torCellCodec.payloadOffset;
  const decodedCommand = memory.readUInt8(torCellCodec.cellPtr + 4);
  assert.strictEqual(decodedCommand, command, "real Tor codec command mismatch");
  const record = {
    sequence,
    circuitId: readUInt32BE(memory, torCellCodec.cellPtr),
    command: decodedCommand,
    relayCommand: 0,
    streamId: 0,
    length: 0,
    handshakeType: 0,
    plaintextPrivateBytes: 0,
    privateKeyExportCount: 0,
  };
  if (decodedCommand === torCommands.RELAY) {
    record.relayCommand = memory.readUInt8(payload + torCellCodec.relayCommandOffset);
    record.streamId = readUInt16BE(memory, payload + torCellCodec.relayStreamIdOffset);
    record.length = readUInt16BE(memory, payload + torCellCodec.relayLengthOffset);
    assert.strictEqual(record.relayCommand, relayCommand, "real Tor codec relay command mismatch");
    assert.strictEqual(record.streamId, streamId, "real Tor codec stream id mismatch");
    assert.strictEqual(record.length, length, "real Tor codec relay length mismatch");
  }
  if (decodedCommand === torCommands.CREATE2) {
    record.handshakeType = readUInt16BE(memory, payload);
    assert.strictEqual(record.handshakeType, handshakeTypes.ntor, "real Tor codec CREATE2 handshake type mismatch");
  }
  if (decodedCommand === torCommands.CREATED2) {
    record.handshakeType = handshakeTypes.ntor;
  }
  return record;
}

function readTorCellCodecRecords({ circuitId, payloadBytes }) {
  return [
    buildCodecCellRecord({ sequence: 0, circuitId, command: torCommands.CREATE2 }),
    buildCodecCellRecord({ sequence: 1, circuitId, command: torCommands.CREATED2 }),
    buildCodecCellRecord({ sequence: 2, circuitId, command: torCommands.RELAY, relayCommand: relayCommands.EXTEND2 }),
    buildCodecCellRecord({ sequence: 3, circuitId, command: torCommands.RELAY, relayCommand: relayCommands.EXTENDED2 }),
    buildCodecCellRecord({ sequence: 4, circuitId, command: torCommands.RELAY, relayCommand: relayCommands.BEGIN, streamId: 1 }),
    buildCodecCellRecord({
      sequence: 5,
      circuitId,
      command: torCommands.RELAY,
      relayCommand: relayCommands.DATA,
      streamId: 1,
      length: payloadBytes,
    }),
    buildCodecCellRecord({ sequence: 6, circuitId, command: torCommands.RELAY, relayCommand: relayCommands.END, streamId: 1 }),
  ];
}

function buildTorCells({ app, commit, route, service }) {
  const circuitId = Number.parseInt(commit.eventHash.slice(0, 8), 16) || 1;
  const sealedPayloadHash = payloadHash({
    appId: app.appId,
    sourceEventHash: route.sourceEventHash,
    targetIdentity: route.targetIdentity,
    serviceIdentity: service.serviceIdentity,
    payloadBytes: route.payloadBytes,
  });
  const common = {
    appId: app.appId,
    sourceEventHash: route.sourceEventHash,
    circuitId,
    payloadHash: sealedPayloadHash,
  };
  const records = readTorCellCodecRecords({ circuitId, payloadBytes: route.payloadBytes });
  const ntor = ntorPublicMetadata({
    appId: app.appId,
    sourceEventHash: route.sourceEventHash,
    guardIdentity: defaultCircuitIdentities.guard,
  });
  const handshakeBySequence = new Map([
    [
      0,
      {
        handshakeType: handshakeTypes.ntor,
        onionKeyIdentity: ntor.onionKeyIdentity,
        clientEphemeralHash: ntor.clientEphemeralHash,
        handshakeTranscriptHash: ntor.handshakeTranscriptHash,
      },
    ],
    [
      1,
      {
        handshakeType: handshakeTypes.ntor,
        onionKeyIdentity: ntor.onionKeyIdentity,
        clientEphemeralHash: ntor.clientEphemeralHash,
        serverEphemeralHash: ntor.serverEphemeralHash,
        handshakeTranscriptHash: ntor.handshakeTranscriptHash,
      },
    ],
  ]);
  return records.map((record) => {
    assert.strictEqual(record.circuitId, circuitId, "Tor codec circuit id mismatch");
    assert.strictEqual(record.plaintextPrivateBytes, 0, "Tor codec path must not expose plaintext private bytes");
    assert.strictEqual(record.privateKeyExportCount, 0, "Tor codec path must not export private keys");
    const handshake = handshakeBySequence.get(record.sequence) || null;
    if (handshake) {
      assert.strictEqual(record.handshakeType, handshake.handshakeType, "Tor codec handshake type mismatch");
    } else {
      assert.strictEqual(record.handshakeType, 0, "Tor codec unexpected handshake type");
    }
    return makeCell({
      ...common,
      sequence: record.sequence,
      command: record.command,
      relayCommand: record.relayCommand,
      streamId: record.streamId,
      length: record.length,
      handshake,
    });
  });
}

function assertNoPrivateHandshakeMaterial(cell) {
  for (const field of ["clientPrivateKey", "serverPrivateKey", "privateScalar", "rawKeyExport"]) {
    assert(!Object.hasOwn(cell, field), `Tor cell must not export ${field}`);
  }
}

function assertNtorHandshake({ app, route, cells, identities = defaultCircuitIdentities }) {
  const create = cells[0];
  const created = cells[1];
  const expected = ntorPublicMetadata({
    appId: app.appId,
    sourceEventHash: route.sourceEventHash,
    guardIdentity: identities.guard,
  });
  assert.strictEqual(create.handshakeType, handshakeTypes.ntor, "CREATE2 handshake type must be ntor");
  assert.strictEqual(created.handshakeType, handshakeTypes.ntor, "CREATED2 handshake type must be ntor");
  assert.strictEqual(create.onionKeyIdentity, expected.onionKeyIdentity, "CREATE2 onion key identity mismatch");
  assert.strictEqual(created.onionKeyIdentity, expected.onionKeyIdentity, "CREATED2 onion key identity mismatch");
  assert.strictEqual(create.clientEphemeralHash, expected.clientEphemeralHash, "CREATE2 client ephemeral hash mismatch");
  assert.strictEqual(created.clientEphemeralHash, expected.clientEphemeralHash, "CREATED2 client ephemeral hash mismatch");
  assert.strictEqual(created.serverEphemeralHash, expected.serverEphemeralHash, "CREATED2 server ephemeral hash mismatch");
  assert.strictEqual(create.serverEphemeralHash, undefined, "CREATE2 must not carry server ephemeral hash");
  assert.strictEqual(create.handshakeTranscriptHash, expected.handshakeTranscriptHash, "CREATE2 transcript hash mismatch");
  assert.strictEqual(created.handshakeTranscriptHash, expected.handshakeTranscriptHash, "CREATED2 transcript hash mismatch");
  assertNoPrivateHandshakeMaterial(create);
  assertNoPrivateHandshakeMaterial(created);
}

function assertTorCells({ app, commit, route, cells }) {
  assert.strictEqual(cells.length, 7, "Tor circuit v0 must emit seven cell envelopes");
  const expected = [
    [torCommands.CREATE2, 0],
    [torCommands.CREATED2, 0],
    [torCommands.RELAY, relayCommands.EXTEND2],
    [torCommands.RELAY, relayCommands.EXTENDED2],
    [torCommands.RELAY, relayCommands.BEGIN],
    [torCommands.RELAY, relayCommands.DATA],
    [torCommands.RELAY, relayCommands.END],
  ];
  for (let index = 0; index < cells.length; index += 1) {
    const cell = cells[index];
    assert.strictEqual(cell.sequence, index, "Tor cell sequence mismatch");
    assert.strictEqual(cell.appId, app.appId, "Tor cell app id mismatch");
    assert.strictEqual(cell.sourceEventHash, route.sourceEventHash, "Tor cell source event hash mismatch");
    assert.strictEqual(cell.circuitId, cells[0].circuitId, "Tor cell circuit id continuity mismatch");
    assert.strictEqual(cell.command, expected[index][0], "Tor cell command sequence mismatch");
    assert.strictEqual(cell.relayCommand, expected[index][1], "Tor cell relay command sequence mismatch");
    assert.strictEqual(cell.plaintextPrivateBytes, 0, "Tor cell must not expose plaintext private bytes");
    assert.strictEqual(cell.canonicalRecordBytes, 272, "Tor cell must carry WAT-canonical record length");
    assert.strictEqual(cell.cellHash, cellHash(cell), "Tor cell hash mismatch");
    assert(!Object.hasOwn(cell, "plaintextPayload"), "Tor cell must not carry plaintext payload");
    assertNoPrivateHandshakeMaterial(cell);
  }
  assertNtorHandshake({ app, route, cells });
  assert.strictEqual(cells[5].length, route.payloadBytes, "Tor RELAY_DATA length must match route payload bytes");
  assert.strictEqual(cells[0].circuitId, Number.parseInt(commit.eventHash.slice(0, 8), 16) || 1, "Tor circuit id must bind to commit hash");
}

function buildLocalTorCircuit({
  app,
  commit,
  relay,
  identities = defaultCircuitIdentities,
  validate = true,
} = {}) {
  assert(app, "buildLocalTorCircuit requires app output");
  assert(commit, "buildLocalTorCircuit requires commit output");
  if (validate) {
    assertRelayReadyForTor({ relay });
  }

  const route = relay.accepted[0];
  const service = relay.hiddenServices[0];
  const cells = buildTorCells({ app, commit, route, service });
  assertTorCells({ app, commit, route, cells });
  const phases = [
    { kind: "guard_accepted", identity: identities.guard, amount: route.payloadBytes, cell: cells[0] },
    { kind: "middle_transit", identity: identities.middle, amount: route.payloadBytes, cell: cells[2] },
    { kind: "rendezvous_established", identity: identities.rendezvous, amount: 0, cell: cells[4] },
    { kind: "hidden_service_delivered", identity: service.serviceIdentity, amount: route.payloadBytes, cell: cells[5] },
  ];
  const receipts = phases.map((phase) => ({
    kind: phase.kind,
    identity: phase.identity,
    id: circuitReceipt({
      appId: app.appId,
      sourceEventHash: route.sourceEventHash,
      identity: phase.identity,
      phase: phase.kind,
      amount: phase.amount,
      cellHash: phase.cell.cellHash,
    }),
    sourceEventHash: route.sourceEventHash,
    cellHash: phase.cell.cellHash,
    handshakeTranscriptHash: cells[0].handshakeTranscriptHash,
  }));
  const deliveryProof = buildTorDeliveryProof({
    app,
    commit,
    route,
    service,
    cells,
    receipts,
  });

  return {
    appId: app.appId,
    sourceEventHash: commit.eventHash,
    routeTargetIdentity: route.targetIdentity,
    hiddenServiceIdentity: service.serviceIdentity,
    payloadBytes: route.payloadBytes,
    payloadSealed: true,
    circuit: {
      guardIdentity: identities.guard,
      middleIdentity: identities.middle,
      rendezvousIdentity: identities.rendezvous,
      hiddenServiceIdentity: service.serviceIdentity,
      rawSocket: false,
      dnsLookup: false,
      rawListenPort: null,
    },
    cells,
    receipts,
    deliveryProof,
    delivered: true,
  };
}

function simulateWatAppTorCircuit(options = {}) {
  const result = simulateWatAppIdentityRelay(options);
  return {
    ...result,
    tor: buildLocalTorCircuit(result),
  };
}

if (require.main === module) {
  const watPath = process.argv[2] || path.join(port, "tests/app-abi-v0.wat");
  const result = simulateWatAppTorCircuit({ watPath });
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
}

module.exports = {
  assertRelayReadyForTor,
  assertNtorHandshake,
  assertTorCells,
  assertTorDeliveryProof,
  buildTorDeliveryProof,
  buildTorDeliveryProofRecord,
  buildTorCellRecord,
  buildLocalTorCircuit,
  buildTorCells,
  handshakeTypes,
  ntorPublicMetadata,
  simulateWatAppTorCircuit,
  torCommands,
  relayCommands,
};
