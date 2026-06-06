"use strict";

const assert = require("assert");
const crypto = require("crypto");
const fs = require("fs");

const abi = require("../abi/app-abi-v0.js");

function wasmHash(wasmPath) {
  return crypto.createHash("sha256").update(fs.readFileSync(wasmPath)).digest("hex");
}

function assertTransitionIdentityPolicy(testName, canonicalAppId, claimedAppId) {
  assert(Buffer.isBuffer(canonicalAppId), `${testName} canonical app id must be bytes`);
  assert(Buffer.isBuffer(claimedAppId), `${testName} claimed app id must be bytes`);
  assert.strictEqual(canonicalAppId.length, 32, `${testName} canonical app id must be 32 bytes`);
  assert.strictEqual(claimedAppId.length, 32, `${testName} claimed app id must be 32 bytes`);
  assert(
    crypto.timingSafeEqual(canonicalAppId, claimedAppId),
    `${testName} transition app id claim must match canonical wasm hash`,
  );
}

function eventHash({ appId, clock, previousHash, transition }) {
  const hash = crypto.createHash("sha256");
  hash.update("edgerun.event.v0");
  hash.update(appId);
  const header = Buffer.alloc(8 + abi.transitionHeaderSize);
  header.writeBigUInt64LE(BigInt(clock), 0);
  for (const [name, offset] of abi.transitionHeaderFields) {
    header.writeUInt32LE(transition[name] ?? 0, 8 + offset);
  }
  hash.update(header);
  hash.update(previousHash);
  return hash.digest();
}

function decodeHeader(memory, ptr, len, fields) {
  assert(ptr >= 0, "decoded header pointer must be non-negative");
  assert(ptr + len <= memory.length, "decoded header must fit in dumped memory");
  const decoded = { ptr, len };
  for (const [name, offset] of fields) {
    decoded[name] = memory.readUInt32LE(ptr + offset);
  }
  return decoded;
}

function transitionIntentRecordCount(transition) {
  return (
    (transition.storageIntentCount ?? 0) +
    (transition.routeIntentCount ?? 0) +
    (transition.hiddenServiceIntentCount ?? 0) +
    (transition.sealedObjectIntentCount ?? 0) +
    (transition.childSpawnIntentCount ?? 0) +
    (transition.dependencyCallIntentCount ?? 0)
  );
}

function decodeIntentRecords(memory, transition) {
  const count = transitionIntentRecordCount(transition);
  const ptr = transition.ptr + abi.transitionHeaderSize;
  const len = count * abi.intentRecordSize;
  assert(ptr + len <= memory.length, "decoded intent records must fit in dumped memory");
  const records = [];
  for (let index = 0; index < count; index += 1) {
    const recordPtr = ptr + index * abi.intentRecordSize;
    records.push({
      ...decodeHeader(memory, recordPtr, abi.intentRecordSize, abi.intentRecordFields),
      index,
    });
  }
  return records;
}

function decodeTransition(memory, ptr) {
  const transition = decodeHeader(memory, ptr, abi.transitionHeaderSize, abi.transitionHeaderFields);
  transition.intentRecords = decodeIntentRecords(memory, transition);
  return transition;
}

function decodeRender(memory, ptr) {
  return decodeHeader(memory, ptr, abi.renderHeaderSize, abi.renderHeaderFields);
}

function assertTransitionPolicy(testName, transition, manifest) {
  const memoryGrantBytes = Number(manifest["memory.min"]);
  const storageGrantBytes = Number(manifest["storage.min"]);
  const sealedObjectIntentCount = transition.sealedObjectIntentCount ?? 0;
  const sealedPrincipalMask = transition.sealedPrincipalMask ?? 0;
  const plaintextPrivateBytes = transition.plaintextPrivateBytes ?? 0;
  const childSpawnIntentCount = transition.childSpawnIntentCount ?? 0;
  const childMemoryBytes = transition.childMemoryBytes ?? 0;
  const childStorageBytes = transition.childStorageBytes ?? 0;
  const childInspectHandleCount = transition.childInspectHandleCount ?? 0;
  const routeIdentityIntentCount = transition.routeIdentityIntentCount ?? 0;
  const rawPortOpenCount = transition.rawPortOpenCount ?? 0;
  const dnsLookupCount = transition.dnsLookupCount ?? 0;
  const eventAppendIntentCount = transition.eventAppendIntentCount ?? 0;
  const immediateStorageIoCount = transition.immediateStorageIoCount ?? 0;
  const cacheWriteIntentCount = transition.cacheWriteIntentCount ?? 0;
  const durableResultIntentCount = transition.durableResultIntentCount ?? 0;
  const receiptIntentCount = transition.receiptIntentCount ?? 0;
  const receiptRequiredMask = transition.receiptRequiredMask ?? 0;
  const unsignedReceiptCount = transition.unsignedReceiptCount ?? 0;
  const fuelUsed = transition.fuelUsed ?? 0;
  const fuelLimit = transition.fuelLimit ?? 0;
  const unmeteredLoopCount = transition.unmeteredLoopCount ?? 0;
  const previousStateRootCount = transition.previousStateRootCount ?? 0;
  const nextStateRootCount = transition.nextStateRootCount ?? 0;
  const stateRootOverrideCount = transition.stateRootOverrideCount ?? 0;
  const emittedMessageCount = transition.emittedMessageCount ?? 0;
  const emittedMessageIdentityTargetCount = transition.emittedMessageIdentityTargetCount ?? 0;
  const emittedSealedMessageCount = transition.emittedSealedMessageCount ?? 0;
  const plaintextMessageBytes = transition.plaintextMessageBytes ?? 0;
  const contributionReceiptCount = transition.contributionReceiptCount ?? 0;
  const unpaidResourceUseCount = transition.unpaidResourceUseCount ?? 0;
  const resourceBudgetOverrunCount = transition.resourceBudgetOverrunCount ?? 0;
  const migrationIntentCount = transition.migrationIntentCount ?? 0;
  const userSignedMigrationCount = transition.userSignedMigrationCount ?? 0;
  const developerToReleaseMigrationCount = transition.developerToReleaseMigrationCount ?? 0;
  const hiddenServiceIntentCount = transition.hiddenServiceIntentCount ?? 0;
  const rawListenPortCount = transition.rawListenPortCount ?? 0;
  const clearnetIngressCount = transition.clearnetIngressCount ?? 0;
  const tlsIntentCount = transition.tlsIntentCount ?? 0;
  const tlsIdentityRouteCount = transition.tlsIdentityRouteCount ?? 0;
  const rawTlsSocketCount = transition.rawTlsSocketCount ?? 0;
  const tlsPlaintextKeyExportCount = transition.tlsPlaintextKeyExportCount ?? 0;
  const dependencyCallIntentCount = transition.dependencyCallIntentCount ?? 0;
  const dependencyPinnedIdentityCount = transition.dependencyPinnedIdentityCount ?? 0;
  const dynamicDependencyCallCount = transition.dynamicDependencyCallCount ?? 0;
  const dependencyReceiptCount = transition.dependencyReceiptCount ?? 0;
  const objectRequirementCount = transition.objectRequirementCount ?? 0;
  const publicObjectIntentCount = transition.publicObjectIntentCount ?? 0;
  const integrityObjectIntentCount = transition.integrityObjectIntentCount ?? 0;
  const objectWithoutRequirementCount = transition.objectWithoutRequirementCount ?? 0;
  assert.strictEqual(transition.magic, abi.magic.transition, `${testName} transition magic mismatch`);
  assert.strictEqual(transition.clockDelta, 1, `${testName} transition must tick exactly once`);
  assert.strictEqual(previousStateRootCount, 1, `${testName} transition must bind exactly one previous state root`);
  assert.strictEqual(nextStateRootCount, 1, `${testName} transition must emit exactly one next state root`);
  assert.strictEqual(stateRootOverrideCount, 0, `${testName} transition must not override state root continuity`);
  assert(fuelLimit > 0, `${testName} transition must declare a positive fuel limit`);
  assert(fuelUsed > 0, `${testName} transition must report positive fuel use`);
  assert(fuelUsed <= fuelLimit, `${testName} transition fuel use must stay inside its limit`);
  assert.strictEqual(unmeteredLoopCount, 0, `${testName} transition must not contain unmetered loops`);
  let expectedContributionReceipts = 1;
  if (transition.storageIntentCount > 0) expectedContributionReceipts += 1;
  if (transition.routeIntentCount > 0) expectedContributionReceipts += 1;
  if (childSpawnIntentCount > 0) expectedContributionReceipts += 1;
  assert(
    contributionReceiptCount >= expectedContributionReceipts,
    `${testName} transition must carry contribution receipts for compute, storage, route, and child work`,
  );
  assert.strictEqual(unpaidResourceUseCount, 0, `${testName} transition must not use unpaid resources`);
  assert.strictEqual(resourceBudgetOverrunCount, 0, `${testName} transition must not overrun resource budgets`);
  assert.strictEqual(rawListenPortCount, 0, `${testName} transitions must not listen on raw ports`);
  assert.strictEqual(clearnetIngressCount, 0, `${testName} transitions must not expose clearnet ingress`);
  if (migrationIntentCount > 0) {
    assert(userSignedMigrationCount >= migrationIntentCount, `${testName} app identity migration requires explicit user-signed migration`);
    assert(userSignedMigrationCount >= developerToReleaseMigrationCount, `${testName} developer-to-release migration requires explicit user signature`);
  } else {
    assert.strictEqual(userSignedMigrationCount, 0, `${testName} user-signed migration count must be zero without migration intents`);
    assert.strictEqual(developerToReleaseMigrationCount, 0, `${testName} developer-to-release migration count must be zero without migration intents`);
  }
  assert(transition.ptr >= 0, `${testName} transition pointer must be non-negative`);
  assert(transition.len > 0, `${testName} transition length must be positive`);
  assert(transition.ptr + transition.len <= memoryGrantBytes, `${testName} transition buffer must stay inside memory grant`);
  assert(transition.storageIntentCount > 0, `${testName} transition must queue storage intents`);
  assert.strictEqual(transition.directStorageWriteCount, 0, `${testName} transition must not perform direct storage writes`);
  assert.strictEqual(immediateStorageIoCount, 0, `${testName} transition must not claim immediate storage IO`);
  assert.strictEqual(cacheWriteIntentCount, 0, `${testName} transition must not use hidden cache writes as storage`);
  assert(eventAppendIntentCount > 0, `${testName} storage intents must append hashed events`);
  assert(durableResultIntentCount > 0, `${testName} storage intents must produce durable results`);
  assert(transition.storageBytesRequested <= storageGrantBytes, `${testName} transition storage intents must stay inside storage grant`);
  const capabilities = manifest.capabilities || [];
  assert(capabilities.includes("storage.intent"), `${testName} transition storage intents require storage.intent capability`);
  assert.strictEqual(transition.directNetworkWriteCount, 0, `${testName} transition must not perform direct network writes`);
  if (transition.routeIntentCount > 0) {
    assert(capabilities.includes("route.intent"), `${testName} route intents require route.intent capability`);
    assert(transition.routeBytesRequested > 0, `${testName} route intents must declare route bytes`);
    assert(routeIdentityIntentCount > 0, `${testName} route intents must target identities or hidden services`);
  } else {
    assert.strictEqual(routeIdentityIntentCount, 0, `${testName} route identity intents must be zero without route intents`);
  }
  assert.strictEqual(rawPortOpenCount, 0, `${testName} transitions must not open raw ports`);
  assert.strictEqual(dnsLookupCount, 0, `${testName} transitions must not request DNS lookups`);
  if (hiddenServiceIntentCount > 0) {
    assert(capabilities.includes("route.intent"), `${testName} hidden service intents require route.intent capability`);
  }
  assert.strictEqual(rawTlsSocketCount, 0, `${testName} transitions must not open raw TLS sockets`);
  assert.strictEqual(tlsPlaintextKeyExportCount, 0, `${testName} transitions must not export TLS private key material`);
  if (tlsIntentCount > 0) {
    assert(capabilities.includes("tls.intent"), `${testName} TLS intents require tls.intent capability`);
    assert(capabilities.includes("route.intent"), `${testName} TLS intents require route.intent capability`);
    assert(tlsIdentityRouteCount >= tlsIntentCount, `${testName} TLS intents must be bound to identity-routed transport`);
  } else {
    assert.strictEqual(tlsIdentityRouteCount, 0, `${testName} TLS identity route count must be zero without TLS intents`);
  }
  assert.strictEqual(dynamicDependencyCallCount, 0, `${testName} dependency calls must not use dynamic dependency resolution`);
  if (dependencyCallIntentCount > 0) {
    assert(capabilities.includes("dependency.use"), `${testName} dependency calls require dependency.use capability`);
    assert((manifest.dependencies || []).length > 0, `${testName} dependency calls require pinned manifest dependencies`);
    assert(dependencyPinnedIdentityCount >= dependencyCallIntentCount, `${testName} dependency calls must target pinned executable identities`);
    assert(dependencyReceiptCount >= dependencyCallIntentCount, `${testName} dependency calls must emit dependency receipts`);
  } else {
    assert.strictEqual(dependencyPinnedIdentityCount, 0, `${testName} dependency pinned identity count must be zero without dependency calls`);
    assert.strictEqual(dependencyReceiptCount, 0, `${testName} dependency receipt count must be zero without dependency calls`);
  }
  assert.strictEqual(plaintextMessageBytes, 0, `${testName} emitted app messages must not contain plaintext private bytes`);
  if (emittedMessageCount > 0) {
    assert(emittedMessageIdentityTargetCount >= emittedMessageCount, `${testName} emitted app messages must target identities or hidden services`);
    assert(emittedSealedMessageCount >= emittedMessageCount, `${testName} emitted app messages must be sealed`);
  } else {
    assert.strictEqual(emittedMessageIdentityTargetCount, 0, `${testName} emitted message identity targets must be zero without messages`);
    assert.strictEqual(emittedSealedMessageCount, 0, `${testName} sealed emitted messages must be zero without messages`);
  }
  assert.strictEqual(plaintextPrivateBytes, 0, `${testName} transition must not expose private plaintext bytes to the app`);
  assert.strictEqual(objectWithoutRequirementCount, 0, `${testName} object intents must not omit data requirements`);
  const objectIntentCount = sealedObjectIntentCount + publicObjectIntentCount + integrityObjectIntentCount;
  if (objectIntentCount > 0) {
    assert(objectRequirementCount >= objectIntentCount, `${testName} object intents must carry data requirement records`);
  } else {
    assert.strictEqual(objectRequirementCount, 0, `${testName} object requirement count must be zero without object intents`);
  }
  if (sealedObjectIntentCount > 0) {
    assert(capabilities.includes("sealed.intent"), `${testName} sealed object intents require sealed.intent capability`);
    assert(sealedPrincipalMask > 0 && (sealedPrincipalMask & ~0b111) === 0, `${testName} sealed principal mask must be a non-empty subset of device/app/user`);
  } else {
    assert.strictEqual(sealedPrincipalMask, 0, `${testName} sealed principal mask must be zero without sealed intents`);
  }
  if (childSpawnIntentCount > 0) {
    assert(capabilities.includes("child.spawn"), `${testName} child spawn intents require child.spawn capability`);
    assert(childMemoryBytes > 0, `${testName} child spawn must move memory out of parent grant`);
    assert(childMemoryBytes < memoryGrantBytes, `${testName} child memory must be moved from available parent memory`);
    assert(childStorageBytes > 0, `${testName} child spawn must move storage out of parent grant`);
    assert(childStorageBytes + transition.storageBytesRequested <= storageGrantBytes, `${testName} child storage plus parent storage intents must stay inside parent storage grant`);
    assert.strictEqual(childInspectHandleCount, 0, `${testName} child handles must not expose child memory or storage inspection`);
  } else {
    assert.strictEqual(childMemoryBytes, 0, `${testName} child memory must be zero without child spawn`);
    assert.strictEqual(childStorageBytes, 0, `${testName} child storage must be zero without child spawn`);
    assert.strictEqual(childInspectHandleCount, 0, `${testName} child inspect handles must be zero without child spawn`);
  }
  let expectedReceiptMask = 0;
  let expectedReceiptCount = 0;
  if (transition.storageIntentCount > 0) {
    expectedReceiptMask |= abi.receiptMasks.storage;
    expectedReceiptCount += 1;
  }
  if (sealedObjectIntentCount > 0) {
    expectedReceiptMask |= abi.receiptMasks.sealed;
    expectedReceiptCount += 1;
  }
  if (transition.routeIntentCount > 0) {
    expectedReceiptMask |= abi.receiptMasks.route;
    expectedReceiptCount += 1;
  }
  if (childSpawnIntentCount > 0) {
    expectedReceiptMask |= abi.receiptMasks.child;
    expectedReceiptCount += 1;
  }
  assert(receiptIntentCount >= expectedReceiptCount, `${testName} transition must emit receipts for every authority-producing intent`);
  assert.strictEqual(receiptRequiredMask & expectedReceiptMask, expectedReceiptMask, `${testName} receipt mask must cover storage, sealed, route, and child work`);
  assert.strictEqual(unsignedReceiptCount, 0, `${testName} receipts must be signed or kernel-bound before commit`);
}

function assertRenderPolicy(testName, render, manifest) {
  const memoryGrantBytes = Number(manifest["memory.min"]);
  const sealedInputRequestCount = render.sealedInputRequestCount ?? 0;
  const rawInputCaptureCount = render.rawInputCaptureCount ?? 0;
  const revealWithoutUserActionCount = render.revealWithoutUserActionCount ?? 0;
  const uiActionIntentCount = render.uiActionIntentCount ?? 0;
  const renderDirectStateMutationCount = render.renderDirectStateMutationCount ?? 0;
  const renderDirectStorageMutationCount = render.renderDirectStorageMutationCount ?? 0;
  const renderDirectNetworkMutationCount = render.renderDirectNetworkMutationCount ?? 0;
  assert.strictEqual(render.magic, abi.magic.render, `${testName} render magic mismatch`);
  assert(render.ptr >= 0, `${testName} render pointer must be non-negative`);
  assert(render.len > 0, `${testName} render length must be positive`);
  assert(render.ptr + render.len <= memoryGrantBytes, `${testName} render buffer must stay inside memory grant`);
  assert.strictEqual(render.plaintextPrivateBytes ?? 0, 0, `${testName} render must not expose private plaintext bytes to the app`);
  assert(sealedInputRequestCount > 0, `${testName} render must request sealed input through host UI`);
  assert(sealedInputRequestCount + uiActionIntentCount > 0, `${testName} render must expose host-mediated UI intents`);
  assert.strictEqual(rawInputCaptureCount, 0, `${testName} render must not capture raw user input`);
  assert.strictEqual(revealWithoutUserActionCount, 0, `${testName} render must not reveal private data without user action`);
  assert.strictEqual(renderDirectStateMutationCount, 0, `${testName} render must not directly mutate app state`);
  assert.strictEqual(renderDirectStorageMutationCount, 0, `${testName} render must not directly mutate storage`);
  assert.strictEqual(renderDirectNetworkMutationCount, 0, `${testName} render must not directly mutate network state`);
}

function readLebU32(buffer, offset) {
  let result = 0;
  let shift = 0;
  let pos = offset;
  while (pos < buffer.length) {
    const byte = buffer[pos++];
    result |= (byte & 0x7f) << shift;
    if ((byte & 0x80) === 0) {
      return { value: result >>> 0, next: pos };
    }
    shift += 7;
    if (shift > 35) {
      throw new Error("invalid u32 LEB");
    }
  }
  throw new Error("truncated u32 LEB");
}

function customSections(wasmPath) {
  const wasm = fs.readFileSync(wasmPath);
  if (
    wasm.length < 8 ||
    wasm[0] !== 0x00 ||
    wasm[1] !== 0x61 ||
    wasm[2] !== 0x73 ||
    wasm[3] !== 0x6d ||
    wasm[4] !== 0x01 ||
    wasm[5] !== 0x00 ||
    wasm[6] !== 0x00 ||
    wasm[7] !== 0x00
  ) {
    throw new Error(`${wasmPath} is not a wasm v1 module`);
  }
  const sections = [];
  let offset = 8;
  while (offset < wasm.length) {
    const id = wasm[offset++];
    const sizeLeb = readLebU32(wasm, offset);
    const payloadStart = sizeLeb.next;
    const payloadEnd = payloadStart + sizeLeb.value;
    if (payloadEnd > wasm.length) {
      throw new Error(`${wasmPath} has a truncated section`);
    }
    if (id === 0) {
      const nameLeb = readLebU32(wasm, payloadStart);
      const nameStart = nameLeb.next;
      const nameEnd = nameStart + nameLeb.value;
      if (nameEnd > payloadEnd) {
        throw new Error(`${wasmPath} has a truncated custom section name`);
      }
      sections.push({
        name: wasm.subarray(nameStart, nameEnd).toString("utf8"),
        payload: wasm.subarray(nameEnd, payloadEnd).toString("utf8"),
      });
    }
    offset = payloadEnd;
  }
  return sections;
}

function wasmModuleSummary(wasmPath) {
  const wasm = fs.readFileSync(wasmPath);
  const summary = { types: [], functionTypeIndices: [], memory: null, exports: [], dataCount: 0 };
  let offset = 8;
  while (offset < wasm.length) {
    const id = wasm[offset++];
    const sizeLeb = readLebU32(wasm, offset);
    const payloadStart = sizeLeb.next;
    const payloadEnd = payloadStart + sizeLeb.value;
    if (payloadEnd > wasm.length) {
      throw new Error(`${wasmPath} has a truncated section`);
    }
    if (id === 1) {
      let pos = payloadStart;
      const count = readLebU32(wasm, pos);
      pos = count.next;
      for (let i = 0; i < count.value; i += 1) {
        assert.strictEqual(wasm[pos++], 0x60, `${wasmPath} has unsupported type form`);
        const paramCount = readLebU32(wasm, pos);
        pos = paramCount.next;
        const params = [];
        for (let p = 0; p < paramCount.value; p += 1) params.push(wasm[pos++]);
        const resultCount = readLebU32(wasm, pos);
        pos = resultCount.next;
        const results = [];
        for (let r = 0; r < resultCount.value; r += 1) results.push(wasm[pos++]);
        summary.types.push({ params, results });
      }
    } else if (id === 2) {
      let pos = payloadStart;
      const count = readLebU32(wasm, pos);
      pos = count.next;
      for (let i = 0; i < count.value; i += 1) {
        const moduleLen = readLebU32(wasm, pos);
        pos = moduleLen.next + moduleLen.value;
        const nameLen = readLebU32(wasm, pos);
        pos = nameLen.next + nameLen.value;
        const kind = wasm[pos++];
        if (kind === 0x00) {
          const typeIndex = readLebU32(wasm, pos);
          pos = typeIndex.next;
          summary.functionTypeIndices.push(typeIndex.value);
        } else if (kind === 0x01 || kind === 0x02) {
          pos += kind === 0x01 ? 1 : 0;
          const flags = wasm[pos++];
          const min = readLebU32(wasm, pos);
          pos = min.next;
          if (flags & 0x01) {
            const max = readLebU32(wasm, pos);
            pos = max.next;
          }
        } else if (kind === 0x03) {
          pos += 2;
        } else {
          throw new Error(`${wasmPath} has unsupported import kind ${kind}`);
        }
      }
    } else if (id === 3) {
      let pos = payloadStart;
      const count = readLebU32(wasm, pos);
      pos = count.next;
      for (let i = 0; i < count.value; i += 1) {
        const typeIndex = readLebU32(wasm, pos);
        pos = typeIndex.next;
        summary.functionTypeIndices.push(typeIndex.value);
      }
    } else if (id === 5) {
      let pos = payloadStart;
      const count = readLebU32(wasm, pos);
      pos = count.next;
      assert.strictEqual(count.value, 1, `${wasmPath} must declare exactly one wasm memory`);
      assert(pos < payloadEnd, `${wasmPath} has a truncated memory limits field`);
      const flags = wasm[pos++];
      assert(flags === 0x00 || flags === 0x01, `${wasmPath} uses unsupported memory limit flags ${flags}`);
      const min = readLebU32(wasm, pos);
      pos = min.next;
      let max = null;
      if (flags === 0x01) {
        const maxLeb = readLebU32(wasm, pos);
        max = maxLeb.value;
        pos = maxLeb.next;
      }
      summary.memory = { minPages: min.value, minBytes: min.value * 65536, hasMax: max !== null, maxPages: max };
    } else if (id === 7) {
      let pos = payloadStart;
      const count = readLebU32(wasm, pos);
      pos = count.next;
      for (let i = 0; i < count.value; i += 1) {
        const nameLen = readLebU32(wasm, pos);
        const nameStart = nameLen.next;
        const nameEnd = nameStart + nameLen.value;
        pos = nameEnd;
        const kind = wasm[pos++];
        const index = readLebU32(wasm, pos);
        pos = index.next;
        summary.exports.push({ name: wasm.subarray(nameStart, nameEnd).toString("utf8"), kind, index: index.value });
      }
    } else if (id === 11) {
      const count = readLebU32(wasm, payloadStart);
      summary.dataCount = count.value;
    }
    offset = payloadEnd;
  }
  return summary;
}

function manifestPolicy(wasmPath) {
  const summary = wasmModuleSummary(wasmPath);
  const manifests = customSections(wasmPath).filter((section) => section.name === "er.manifest");
  assert.strictEqual(manifests.length, 1, `${wasmPath} must contain exactly one er.manifest`);
  const fields = Object.fromEntries(
    manifests[0].payload
      .split(";")
      .filter(Boolean)
      .map((field) => {
        const [key, value] = field.split("=");
        return [key, value];
      }),
  );
  assert.strictEqual(fields.abi, abi.manifestAbi, `${wasmPath} manifest ABI must be ${abi.manifestAbi}`);
  assert(fields.mode === "release" || fields.mode === "developer", `${wasmPath} manifest mode must be release or developer`);
  assert(/^[1-9][0-9]*$/.test(fields["memory.min"] || ""), `${wasmPath} missing memory.min`);
  assert(/^[1-9][0-9]*$/.test(fields["storage.min"] || ""), `${wasmPath} missing storage.min`);
  assert.strictEqual(fields.recursion, abi.manifestRequiredFields.recursion, `${wasmPath} manifest recursion must be false`);
  assert.strictEqual(fields["memory.static"], abi.manifestRequiredFields["memory.static"], `${wasmPath} manifest memory must be static`);
  assert.strictEqual(fields["storage.direct_access"], abi.manifestRequiredFields["storage.direct_access"], `${wasmPath} manifest storage direct access must be false`);
  const memoryMin = Number(fields["memory.min"]);
  const storageMin = Number(fields["storage.min"]);
  assert(Number.isSafeInteger(memoryMin), `${wasmPath} memory.min is not safely representable`);
  assert(Number.isSafeInteger(storageMin), `${wasmPath} storage.min is not safely representable`);
  assert.strictEqual(memoryMin % 65536, 0, `${wasmPath} memory.min must be wasm-page aligned`);
  assert.strictEqual(storageMin % 4096, 0, `${wasmPath} storage.min must be storage-page aligned`);
  assert(summary.memory, `${wasmPath} must declare one wasm memory`);
  assert.strictEqual(summary.memory.hasMax, false, `${wasmPath} wasm memory must not declare max`);
  assert.strictEqual(summary.memory.minBytes, memoryMin, `${wasmPath} manifest memory.min must match wasm memory min`);
  for (const exported of summary.exports) assert.strictEqual(exported.kind, 0x00, `${wasmPath} must only export functions`);
  assert.strictEqual(summary.dataCount, 0, `${wasmPath} must not contain data segments`);
  const allowedCapabilities = new Set(abi.capabilities);
  fields.capabilities = (fields.capabilities || "").split(",").filter(Boolean);
  for (const capability of fields.capabilities) {
    assert(allowedCapabilities.has(capability), `${wasmPath} declares unknown capability ${capability}`);
  }
  assert.strictEqual(fields.postinstall, undefined, `${wasmPath} must not declare postinstall hooks`);
  fields.dependencies = (fields.dependencies || "").split(",").filter(Boolean);
  for (const dependency of fields.dependencies) {
    assert(!/^https?:\/\//.test(dependency), `${wasmPath} dependencies must not be dynamic URLs`);
    assert(/^[a-z0-9_.-]+:[0-9a-f]{64}$/.test(dependency), `${wasmPath} dependencies must be name:sha256hex pinned executable identities`);
  }
  if (fields.dependencies.length > 0) {
    assert(fields.capabilities.includes("dependency.use"), `${wasmPath} pinned dependencies require dependency.use capability`);
  }
  return fields;
}

function assertManifest(wasmPath, testName) {
  assert.doesNotThrow(() => manifestPolicy(wasmPath), `${testName} has invalid er.manifest`);
}

function assertAppAbi(wasmPath, testName, manifest = manifestPolicy(wasmPath)) {
  const summary = wasmModuleSummary(wasmPath);
  const expected = new Map([
    ["er_init", { params: [0x7f], results: [0x7f] }],
    ["er_handle_message", { params: [0x7f, 0x7f, 0x7f], results: [0x7f] }],
    ["er_handle_action", { params: [0x7f, 0x7f, 0x7f], results: [0x7f] }],
    ["er_render", { params: [0x7f], results: [0x7f] }],
  ]);
  for (const [name, signature] of expected) {
    const exported = summary.exports.find((item) => item.name === name && item.kind === 0x00);
    assert(exported, `${testName} missing ABI export ${name}`);
    const typeIndex = summary.functionTypeIndices[exported.index];
    const actual = summary.types[typeIndex];
    assert(actual, `${testName} ABI export ${name} has missing type`);
    assert.deepStrictEqual(actual.params, signature.params, `${testName} ABI export ${name} params mismatch`);
    assert.deepStrictEqual(actual.results, signature.results, `${testName} ABI export ${name} results mismatch`);
  }
  for (const exported of summary.exports) {
    if (exported.name.startsWith("er_")) {
      assert(expected.has(exported.name), `${testName} has unsupported ABI export ${exported.name}`);
    }
  }
  for (const capability of abi.requiredAbiCapabilities) {
    assert(manifest.capabilities.includes(capability), `${testName} missing ABI capability ${capability}`);
  }
}

module.exports = {
  assertAppAbi,
  assertManifest,
  assertRenderPolicy,
  assertTransitionIdentityPolicy,
  assertTransitionPolicy,
  customSections,
  decodeIntentRecords,
  decodeRender,
  decodeTransition,
  eventHash,
  manifestPolicy,
  readLebU32,
  transitionIntentRecordCount,
  wasmHash,
  wasmModuleSummary,
};
