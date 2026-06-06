"use strict";

const manifestAbi = "edgerun.app.v0";

const magic = {
  transition: 0x5452414e,
  render: 0x55490000,
};

const transitionHeaderSize = 232;
const renderHeaderSize = 40;
const intentRecordSize = 32;

const transitionHeaderFields = [
  ["magic", 0],
  ["kind", 4],
  ["inputLen", 8],
  ["clockDelta", 12],
  ["storageIntentCount", 16],
  ["directStorageWriteCount", 20],
  ["storageBytesRequested", 24],
  ["routeIntentCount", 28],
  ["directNetworkWriteCount", 32],
  ["routeBytesRequested", 36],
  ["sealedObjectIntentCount", 40],
  ["sealedPrincipalMask", 44],
  ["plaintextPrivateBytes", 48],
  ["childSpawnIntentCount", 52],
  ["childMemoryBytes", 56],
  ["childStorageBytes", 60],
  ["childInspectHandleCount", 64],
  ["routeIdentityIntentCount", 68],
  ["rawPortOpenCount", 72],
  ["dnsLookupCount", 76],
  ["eventAppendIntentCount", 80],
  ["immediateStorageIoCount", 84],
  ["cacheWriteIntentCount", 88],
  ["durableResultIntentCount", 92],
  ["receiptIntentCount", 96],
  ["receiptRequiredMask", 100],
  ["unsignedReceiptCount", 104],
  ["fuelUsed", 108],
  ["fuelLimit", 112],
  ["unmeteredLoopCount", 116],
  ["previousStateRootCount", 120],
  ["nextStateRootCount", 124],
  ["stateRootOverrideCount", 128],
  ["emittedMessageCount", 132],
  ["emittedMessageIdentityTargetCount", 136],
  ["emittedSealedMessageCount", 140],
  ["plaintextMessageBytes", 144],
  ["contributionReceiptCount", 148],
  ["unpaidResourceUseCount", 152],
  ["resourceBudgetOverrunCount", 156],
  ["migrationIntentCount", 160],
  ["userSignedMigrationCount", 164],
  ["developerToReleaseMigrationCount", 168],
  ["hiddenServiceIntentCount", 172],
  ["rawListenPortCount", 176],
  ["clearnetIngressCount", 180],
  ["tlsIntentCount", 184],
  ["tlsIdentityRouteCount", 188],
  ["rawTlsSocketCount", 192],
  ["tlsPlaintextKeyExportCount", 196],
  ["dependencyCallIntentCount", 200],
  ["dependencyPinnedIdentityCount", 204],
  ["dynamicDependencyCallCount", 208],
  ["dependencyReceiptCount", 212],
  ["objectRequirementCount", 216],
  ["publicObjectIntentCount", 220],
  ["integrityObjectIntentCount", 224],
  ["objectWithoutRequirementCount", 228],
];

const renderHeaderFields = [
  ["magic", 0],
  ["sealedRefCount", 4],
  ["plaintextPrivateBytes", 8],
  ["sealedInputRequestCount", 12],
  ["rawInputCaptureCount", 16],
  ["revealWithoutUserActionCount", 20],
  ["uiActionIntentCount", 24],
  ["renderDirectStateMutationCount", 28],
  ["renderDirectStorageMutationCount", 32],
  ["renderDirectNetworkMutationCount", 36],
];

const intentRecordFields = [
  ["kind", 0],
  ["flags", 4],
  ["amount", 8],
  ["principalMask", 12],
  ["aux0", 16],
  ["aux1", 20],
  ["targetLo", 24],
  ["targetHi", 28],
];

const intentKinds = {
  storageAppend: 1,
  routeSendIdentity: 2,
  routeCreateHiddenService: 3,
  sealedStore: 4,
  childSpawn: 5,
  dependencyCall: 6,
};

const intentFlags = {
  storageEventLog: 1,
  storageDurableResult: 2,
  routeIdentityTarget: 1,
  sealedHasRequirements: 1,
};

const capabilities = [
  "transition.emit",
  "storage.intent",
  "route.intent",
  "tls.intent",
  "sealed.intent",
  "child.spawn",
  "dependency.use",
  "render.emit",
];

const requiredAbiCapabilities = [
  "transition.emit",
  "storage.intent",
  "render.emit",
];

const receiptMasks = {
  storage: 1,
  sealed: 2,
  route: 4,
  child: 8,
};

const manifestRequiredFields = {
  abi: manifestAbi,
  recursion: "false",
  "memory.static": "true",
  "storage.direct_access": "false",
};

module.exports = {
  capabilities,
  intentFlags,
  intentKinds,
  intentRecordFields,
  intentRecordSize,
  magic,
  manifestAbi,
  manifestRequiredFields,
  receiptMasks,
  renderHeaderFields,
  renderHeaderSize,
  requiredAbiCapabilities,
  transitionHeaderFields,
  transitionHeaderSize,
};
