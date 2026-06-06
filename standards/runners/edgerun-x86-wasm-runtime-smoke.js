#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { spawnSync } = require("child_process");

const root = path.resolve(__dirname, "../..");
const port = path.join(root, "standards/ports/edgerun-x86-wasm-runtime");
const source = path.join(port, "source/kernel");
const outDir = fs.mkdtempSync(path.join(os.tmpdir(), "edgerun-x86-wasm-runtime-"));
const harnessMemoryBytes = 1114112;
const appAbiV0 = require(path.join(port, "abi/app-abi-v0.js"));
const { simulateWatApp } = require(path.join(port, "sim/local-memory-sim.js"));
const {
  assertCommitReplay,
  assertIntentRecords,
  commitSimulation,
  simulateWatAppCommit,
} = require(path.join(port, "sim/local-kernel-commit-v0.js"));
const {
  assertHiddenServiceRecord,
  relayCommittedRoutes,
  simulateWatAppIdentityRelay,
} = require(path.join(port, "sim/local-identity-relay-v0.js"));
const {
  assertTorDeliveryProof,
  buildTorDeliveryProofRecord,
  buildTorCellRecord,
  assertTorCells,
  buildLocalTorCircuit,
  relayCommands,
  simulateWatAppTorCircuit,
  torCommands,
} = require(path.join(port, "sim/local-tor-circuit-v0.js"));
const policy = require(path.join(port, "policy/app-policy-v0.js"));

const tests = [
  "test_recursion_valid",
  "test_recursion_invalid",
  "test_wasm_float",
  "test_wasm_jit_self",
];

const watTests = [
  {
    name: "app-runtime-invariants",
    wat: path.join(port, "tests/app-runtime-invariants.wat"),
    requiresManifest: true,
    cases: [
      ["release_memory_inspect_allowed", 0],
      ["developer_memory_inspect_allowed", 1],
      ["direct_storage_write_allowed", 0],
      ["recursion_allowed", 0],
      ["commit_tick_delta", 1],
    ],
  },
];

const watArgTests = [
  {
    name: "app-abi-v0",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    requiresManifest: true,
    requiresAppAbi: true,
    cases: [
      { exportName: "er_init", args: [64], expected: 64, memoryChecks: [[64, 0x494e4954]] },
      {
        exportName: "er_handle_message",
        args: [64, 640, 16],
        expected: 96,
        memoryChecks: [
          [68, 0x4d534700],
          [656, 16],
          [96, 0x5452414e],
          [100, 1],
          [104, 16],
          [108, 1],
          [112, 1],
          [116, 0],
          [120, 64],
          [124, 0],
          [128, 0],
          [132, 0],
          [136, 1],
          [140, 6],
          [144, 0],
          [148, 0],
          [152, 0],
          [156, 0],
          [160, 0],
          [164, 0],
          [168, 0],
          [172, 0],
          [176, 1],
          [180, 0],
          [184, 0],
          [188, 1],
          [192, 2],
          [196, 3],
          [200, 0],
          [204, 4096],
          [208, 1000000],
          [212, 0],
          [216, 1],
          [220, 1],
          [224, 0],
          [228, 0],
          [232, 0],
          [236, 0],
          [240, 0],
          [244, 2],
          [248, 0],
          [252, 0],
          [256, 0],
          [260, 0],
          [264, 0],
          [268, 0],
          [272, 0],
          [276, 0],
	          [280, 0],
	          [284, 0],
	          [288, 0],
	          [292, 0],
	          [296, 0],
	          [300, 0],
	          [304, 0],
	          [308, 0],
	          [312, 1],
	          [316, 0],
	          [320, 0],
	          [324, 0],
        ],
      },
      {
        exportName: "er_handle_action",
        args: [64, 768, 8],
        expected: 352,
        memoryChecks: [
          [72, 0x41435400],
          [784, 8],
          [352, 0x5452414e],
          [356, 2],
          [360, 8],
          [364, 1],
          [368, 1],
          [372, 0],
          [376, 32],
          [380, 1],
          [384, 0],
          [388, 24],
          [392, 0],
          [396, 0],
          [400, 0],
          [404, 1],
          [408, 16384],
          [412, 2048],
          [416, 0],
          [420, 1],
          [424, 0],
          [428, 0],
          [432, 1],
          [436, 0],
          [440, 0],
          [444, 1],
          [448, 4],
          [452, 15],
          [456, 0],
          [460, 8192],
          [464, 1000000],
          [468, 0],
          [472, 1],
          [476, 1],
          [480, 0],
          [484, 1],
          [488, 1],
          [492, 1],
          [496, 0],
          [500, 4],
          [504, 0],
          [508, 0],
          [512, 0],
          [516, 0],
          [520, 0],
          [524, 1],
          [528, 0],
          [532, 0],
	          [536, 1],
	          [540, 1],
	          [544, 0],
	          [548, 0],
	          [552, 0],
	          [556, 0],
	          [560, 0],
	          [564, 0],
	          [568, 0],
	          [572, 0],
	          [576, 0],
	          [580, 0],
        ],
      },
      {
        exportName: "er_render",
        args: [64],
        expected: 896,
        memoryChecks: [
          [896, 0x55490000],
          [900, 1],
          [904, 0],
	          [908, 1],
	          [912, 0],
	          [916, 0],
	          [920, 1],
	          [924, 0],
	          [928, 0],
	          [932, 0],
	        ],
      },
    ],
  },
  {
    name: "br-table-branch",
    wat: path.join(port, "tests/br-table-branch.wat"),
    requiresManifest: false,
    cases: [
      { exportName: "select_role_caps", args: [0], expected: 1 },
      { exportName: "select_role_caps", args: [1], expected: 2 },
      { exportName: "select_role_caps", args: [2], expected: 2 },
      { exportName: "select_role_caps", args: [6], expected: 32 },
      { exportName: "select_role_caps", args: [10], expected: 227 },
    ],
  },
  {
    name: "real-tor-library-wat",
    wat: path.join(root, "standards/build/wasm/app-primitives/tor-wat/tor-library.wat"),
    requiresManifest: false,
    trustedLoad: true,
    cases: [
      { exportName: "er_tor_set_role", args: [6], expected: 0 },
      { exportName: "er_tor_set_role_caps", args: [32], expected: 0 },
      { exportName: "er_tor_enable_role", args: [8], expected: 0 },
      { exportName: "er_tor_cell_get_circ_id", args: [49152], expected: 0 },
      { exportName: "er_tor_cell_get_cmd", args: [49152], expected: 0 },
      {
        exportName: "er_tor_build_relay_begin_ipv4",
        args: [49152, 123, 7, 1, 2, 3, 4, 80],
        expected: 0,
        memoryChecks: [[49152, 123]],
        memoryByteChecks: [
          [49156, 3],
          [49157, 1],
          [49160, 0],
          [49161, 7],
          [49166, 0],
          [49167, 11],
          [49168, 49],
          [49169, 46],
          [49170, 50],
          [49171, 46],
          [49172, 51],
          [49173, 46],
          [49174, 52],
          [49175, 58],
          [49176, 56],
          [49177, 48],
          [49178, 0],
        ],
      },
      {
        exportName: "er_tor_build_extend2_body",
        args: [50176, 0x01020304, 9001, 4096, 8192],
        expected: 119,
        memoryByteChecks: [
          [50176, 2],
          [50177, 0],
          [50178, 6],
          [50179, 4],
          [50180, 3],
          [50181, 2],
          [50182, 1],
          [50183, 35],
          [50184, 41],
          [50185, 2],
          [50186, 20],
          [50207, 0],
          [50208, 2],
          [50209, 0],
          [50210, 84],
        ],
      },
    ],
  },
  {
    name: "many-export-constant",
    wat: path.join(port, "tests/many-export-constant.wat"),
    requiresManifest: false,
    cases: [
      { exportName: "f0", args: [], expected: 0 },
      { exportName: "f20", args: [], expected: 20 },
      { exportName: "proto_abi_version", args: [], expected: 1 },
      { exportName: "proto_standard_id", args: [], expected: 300224 },
    ],
  },
  {
    name: "memory-export-constant",
    wat: path.join(port, "tests/memory-export-constant.wat"),
    requiresManifest: false,
    cases: [
      { exportName: "f0", args: [], expected: 0 },
      { exportName: "proto_abi_version", args: [], expected: 1 },
      { exportName: "proto_standard_id", args: [], expected: 300224 },
    ],
  },
  {
    name: "table-shape-constant",
    wat: path.join(port, "tests/table-shape-constant.wat"),
    requiresManifest: false,
    cases: [
      { exportName: "f0", args: [], expected: 0 },
      { exportName: "proto_standard_id", args: [], expected: 300224 },
    ],
  },
  {
    name: "global-shape-constant",
    wat: path.join(port, "tests/global-shape-constant.wat"),
    requiresManifest: false,
    trustedLoad: true,
    cases: [
      { exportName: "f0", args: [], expected: 0 },
      { exportName: "proto_standard_id", args: [], expected: 300224 },
    ],
  },
  {
    name: "data-shape-constant",
    wat: path.join(port, "tests/data-shape-constant.wat"),
    requiresManifest: false,
    trustedLoad: true,
    cases: [
      { exportName: "f0", args: [], expected: 0 },
      { exportName: "proto_standard_id", args: [], expected: 300224 },
    ],
  },
  {
    name: "data-load-constant",
    wat: path.join(port, "tests/data-load-constant.wat"),
    requiresManifest: false,
    trustedLoad: true,
    cases: [
      { exportName: "load_first", args: [], expected: 1 },
      { exportName: "load_second", args: [], expected: 0xfffffffe },
      { exportName: "load_third", args: [], expected: 0 },
    ],
  },
  {
    name: "command-classifier",
    wat: path.join(port, "tests/command-classifier.wat"),
    requiresManifest: false,
    trustedLoad: true,
    cases: [
      { exportName: "classify", args: [1], expected: 1 },
      { exportName: "classify", args: [3], expected: 1 },
      { exportName: "classify", args: [5], expected: 0xfffffffe },
      { exportName: "classify", args: [255], expected: 0xfffffffc },
    ],
  },
  {
    name: "command-validator",
    wat: path.join(port, "tests/command-validator.wat"),
    requiresManifest: false,
    trustedLoad: true,
    cases: [
      { exportName: "validate_stream", args: [3, 0], expected: 0xffffffff },
      { exportName: "validate_stream", args: [3, 7], expected: 0 },
      { exportName: "validate_stream", args: [5, 0], expected: 0 },
      { exportName: "validate_stream", args: [5, 7], expected: 0 },
    ],
  },
  {
    name: "codec-shape-constant",
    wat: path.join(port, "tests/codec-shape-constant.wat"),
    requiresManifest: false,
    trustedLoad: true,
    cases: [
      { exportName: "f0", args: [], expected: 0 },
      { exportName: "proto_abi_version", args: [], expected: 1 },
      { exportName: "proto_standard_id", args: [], expected: 300224 },
    ],
  },
  {
    name: "real-tor-cell-codec-wat",
    wat: path.join(root, "standards/build/wasm/app-primitives/tor-cell-codec/tor-cell-codec.wat"),
    requiresManifest: false,
    trustedLoad: true,
    cases: [
      { exportName: "proto_standard_id", args: [], expected: 300224 },
      { exportName: "proto_abi_version", args: [], expected: 1 },
      { exportName: "tor_cell_fixed_len", args: [4], expected: 514 },
      { exportName: "tor_cell_relay_data_max", args: [], expected: 498 },
      { exportName: "tor_cell_validate_command", args: [4, 3, 7, 0], expected: 0 },
      { exportName: "tor_cell_command_circ_requirement", args: [10], expected: 1 },
      { exportName: "tor_cell_is_variable_command", args: [7], expected: 1 },
      { exportName: "tor_cell_var_header_len", args: [3], expected: 5 },
      { exportName: "tor_cell_var_header_len", args: [4], expected: 7 },
      { exportName: "tor_cell_var_header_len", args: [6], expected: 0xfffffffc },
      { exportName: "tor_cell_fixed_len", args: [3], expected: 512 },
      { exportName: "tor_cell_fixed_len", args: [6], expected: 0xfffffffc },
      { exportName: "tor_cell_circ_id_len", args: [3], expected: 2 },
      { exportName: "tor_cell_circ_id_len", args: [4], expected: 4 },
      { exportName: "tor_cell_relay_header_len", args: [], expected: 11 },
      { exportName: "tor_cell_body_len", args: [], expected: 509 },
      { exportName: "tor_cell_relay_stream_requirement", args: [1], expected: 1 },
      { exportName: "tor_cell_relay_stream_requirement", args: [10], expected: 0 },
      {
        exportName: "tor_cell_build_fixed",
        args: [4, 4096, 123, 10, 8192, 0],
        expected: 514,
        memoryByteChecks: [
          [4096, 0],
          [4097, 0],
          [4098, 0],
          [4099, 123],
          [4100, 10],
        ],
      },
      {
        exportName: "tor_cell_build_destroy_body",
        args: [5000, 1],
        expected: 1,
        memoryByteChecks: [[5000, 1]],
      },
    ],
  },
];

const watIdentityTests = [
  {
    name: "app-identity-mode",
    releaseWat: path.join(port, "tests/app-identity-release.wat"),
    developerWat: path.join(port, "tests/app-identity-developer.wat"),
  },
];

const transitionLogTests = [
  {
    name: "app-clock-hash-chain",
    appWat: path.join(port, "tests/app-abi-v0.wat"),
    transitionCalls: [
      { exportName: "er_handle_message", args: [64, 640, 16], expectedPtr: 96 },
      { exportName: "er_handle_action", args: [64, 768, 8], expectedPtr: 352 },
    ],
    renderCall: { exportName: "er_render", args: [64], expectedPtr: 896 },
  },
];

const localMemorySimTests = [
  {
    name: "app-abi-v0-local-memory-sim",
    wat: path.join(port, "tests/app-abi-v0.wat"),
  },
];

const localKernelCommitTests = [
  {
    name: "app-abi-v0-local-kernel-commit",
    wat: path.join(port, "tests/app-abi-v0.wat"),
  },
];

const localIdentityRelayTests = [
  {
    name: "app-abi-v0-local-identity-relay",
    wat: path.join(port, "tests/app-abi-v0.wat"),
  },
];

const localTorCircuitTests = [
  {
    name: "app-abi-v0-local-tor-circuit",
    wat: path.join(port, "tests/app-abi-v0.wat"),
  },
];

const transitionIdentityRejectTests = [
  {
    name: "reject-transition-spoofed-app-id",
    appWat: path.join(port, "tests/app-abi-v0.wat"),
    claimedAppId: Buffer.alloc(32, 0xff),
  },
  {
    name: "reject-transition-release-claims-developer-id",
    appWat: path.join(port, "tests/app-abi-v0.wat"),
    claimedAppWat: path.join(port, "tests/app-identity-developer.wat"),
  },
];

const transitionPolicyRejectTests = [
  {
    name: "reject-transition-direct-storage",
    wat: path.join(port, "tests/reject-transition-direct-storage.wat"),
    transition: {
      ptr: 96,
      len: 40,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 1,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
    },
  },
  {
    name: "reject-transition-storage-quota",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 28,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 8192,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
    },
  },
  {
    name: "reject-transition-memory-bounds",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 65520,
      len: 28,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
    },
  },
  {
    name: "reject-transition-zero-clock",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 28,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 0,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
    },
  },
  {
    name: "reject-transition-double-clock",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 28,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 2,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
    },
  },
  {
    name: "reject-transition-direct-network",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 160,
      len: 40,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 0,
      directNetworkWriteCount: 1,
      routeBytesRequested: 0,
    },
  },
  {
    name: "reject-transition-missing-route-capability",
    wat: path.join(port, "tests/reject-abi-missing-route-intent-capability.wat"),
    transition: {
      ptr: 160,
      len: 40,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 1,
      directNetworkWriteCount: 0,
      routeBytesRequested: 24,
    },
  },
  {
    name: "reject-transition-plaintext-private",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 52,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 1,
    },
  },
  {
    name: "reject-transition-missing-sealed-capability",
    wat: path.join(port, "tests/reject-abi-missing-sealed-intent-capability.wat"),
    transition: {
      ptr: 96,
      len: 52,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
    },
  },
  {
    name: "reject-transition-invalid-principal-mask",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 52,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 8,
      plaintextPrivateBytes: 0,
    },
  },
  {
    name: "reject-transition-missing-child-spawn-capability",
    wat: path.join(port, "tests/reject-abi-missing-child-spawn-capability.wat"),
    transition: {
      ptr: 224,
      len: 80,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 1,
      directNetworkWriteCount: 0,
      routeBytesRequested: 24,
      sealedObjectIntentCount: 0,
      sealedPrincipalMask: 0,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 1,
      childMemoryBytes: 16384,
      childStorageBytes: 2048,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 1,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
    },
  },
  {
    name: "reject-transition-child-memory-quota",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 224,
      len: 80,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 1,
      directNetworkWriteCount: 0,
      routeBytesRequested: 24,
      sealedObjectIntentCount: 0,
      sealedPrincipalMask: 0,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 1,
      childMemoryBytes: 65536,
      childStorageBytes: 2048,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 1,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
    },
  },
  {
    name: "reject-transition-child-storage-quota",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 224,
      len: 80,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 1,
      directNetworkWriteCount: 0,
      routeBytesRequested: 24,
      sealedObjectIntentCount: 0,
      sealedPrincipalMask: 0,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 1,
      childMemoryBytes: 16384,
      childStorageBytes: 4096,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 1,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
    },
  },
  {
    name: "reject-transition-child-inspection",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 224,
      len: 80,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 1,
      directNetworkWriteCount: 0,
      routeBytesRequested: 24,
      sealedObjectIntentCount: 0,
      sealedPrincipalMask: 0,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 1,
      childMemoryBytes: 16384,
      childStorageBytes: 2048,
      childInspectHandleCount: 1,
      routeIdentityIntentCount: 1,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
    },
  },
  {
    name: "reject-transition-route-without-identity",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 224,
      len: 80,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 1,
      directNetworkWriteCount: 0,
      routeBytesRequested: 24,
      sealedObjectIntentCount: 0,
      sealedPrincipalMask: 0,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 1,
      childMemoryBytes: 16384,
      childStorageBytes: 2048,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
    },
  },
  {
    name: "reject-transition-raw-port",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 224,
      len: 80,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 1,
      directNetworkWriteCount: 0,
      routeBytesRequested: 24,
      sealedObjectIntentCount: 0,
      sealedPrincipalMask: 0,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 1,
      childMemoryBytes: 16384,
      childStorageBytes: 2048,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 1,
      rawPortOpenCount: 1,
      dnsLookupCount: 0,
    },
  },
  {
    name: "reject-transition-dns-lookup",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 224,
      len: 80,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 1,
      directNetworkWriteCount: 0,
      routeBytesRequested: 24,
      sealedObjectIntentCount: 0,
      sealedPrincipalMask: 0,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 1,
      childMemoryBytes: 16384,
      childStorageBytes: 2048,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 1,
      rawPortOpenCount: 0,
      dnsLookupCount: 1,
    },
  },
  {
    name: "reject-transition-immediate-storage-io",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 96,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 1,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
    },
  },
  {
    name: "reject-transition-cache-write",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 96,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 1,
      durableResultIntentCount: 1,
    },
  },
  {
    name: "reject-transition-storage-without-event",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 96,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 0,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
    },
  },
  {
    name: "reject-transition-storage-without-durable-result",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 96,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 0,
    },
  },
  {
    name: "reject-transition-missing-receipts",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 108,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 0,
      receiptRequiredMask: 0,
      unsignedReceiptCount: 0,
    },
  },
  {
    name: "reject-transition-incomplete-receipt-mask",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 224,
      len: 108,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 1,
      directNetworkWriteCount: 0,
      routeBytesRequested: 24,
      sealedObjectIntentCount: 0,
      sealedPrincipalMask: 0,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 1,
      childMemoryBytes: 16384,
      childStorageBytes: 2048,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 1,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 4,
      receiptRequiredMask: 7,
      unsignedReceiptCount: 0,
    },
  },
  {
    name: "reject-transition-unsigned-receipt",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 108,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 2,
      receiptRequiredMask: 3,
      unsignedReceiptCount: 1,
    },
  },
  {
    name: "reject-transition-zero-fuel",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 120,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 2,
      receiptRequiredMask: 3,
      unsignedReceiptCount: 0,
      fuelUsed: 0,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
    },
  },
  {
    name: "reject-transition-fuel-over-budget",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 120,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 2,
      receiptRequiredMask: 3,
      unsignedReceiptCount: 0,
      fuelUsed: 1000001,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
    },
  },
  {
    name: "reject-transition-zero-fuel-limit",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 120,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 2,
      receiptRequiredMask: 3,
      unsignedReceiptCount: 0,
      fuelUsed: 4096,
      fuelLimit: 0,
      unmeteredLoopCount: 0,
    },
  },
  {
    name: "reject-transition-unmetered-loop",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 120,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 2,
      receiptRequiredMask: 3,
      unsignedReceiptCount: 0,
      fuelUsed: 4096,
      fuelLimit: 1000000,
      unmeteredLoopCount: 1,
    },
  },
  {
    name: "reject-transition-missing-previous-state-root",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 132,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 2,
      receiptRequiredMask: 3,
      unsignedReceiptCount: 0,
      fuelUsed: 4096,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 0,
      nextStateRootCount: 1,
      stateRootOverrideCount: 0,
    },
  },
  {
    name: "reject-transition-missing-next-state-root",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 132,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 2,
      receiptRequiredMask: 3,
      unsignedReceiptCount: 0,
      fuelUsed: 4096,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 1,
      nextStateRootCount: 0,
      stateRootOverrideCount: 0,
    },
  },
  {
    name: "reject-transition-state-root-override",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 132,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 2,
      receiptRequiredMask: 3,
      unsignedReceiptCount: 0,
      fuelUsed: 4096,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 1,
      nextStateRootCount: 1,
      stateRootOverrideCount: 1,
    },
  },
  {
    name: "reject-transition-plaintext-message",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 352,
      len: 148,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 1,
      directNetworkWriteCount: 0,
      routeBytesRequested: 24,
      sealedObjectIntentCount: 0,
      sealedPrincipalMask: 0,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 1,
      childMemoryBytes: 16384,
      childStorageBytes: 2048,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 1,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 4,
      receiptRequiredMask: 15,
      unsignedReceiptCount: 0,
      fuelUsed: 8192,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 1,
      nextStateRootCount: 1,
      stateRootOverrideCount: 0,
      emittedMessageCount: 1,
      emittedMessageIdentityTargetCount: 1,
      emittedSealedMessageCount: 1,
      plaintextMessageBytes: 1,
    },
  },
  {
    name: "reject-transition-message-without-identity-target",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 352,
      len: 148,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 1,
      directNetworkWriteCount: 0,
      routeBytesRequested: 24,
      sealedObjectIntentCount: 0,
      sealedPrincipalMask: 0,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 1,
      childMemoryBytes: 16384,
      childStorageBytes: 2048,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 1,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 4,
      receiptRequiredMask: 15,
      unsignedReceiptCount: 0,
      fuelUsed: 8192,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 1,
      nextStateRootCount: 1,
      stateRootOverrideCount: 0,
      emittedMessageCount: 1,
      emittedMessageIdentityTargetCount: 0,
      emittedSealedMessageCount: 1,
      plaintextMessageBytes: 0,
    },
  },
  {
    name: "reject-transition-unsealed-message",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 352,
      len: 148,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 1,
      directNetworkWriteCount: 0,
      routeBytesRequested: 24,
      sealedObjectIntentCount: 0,
      sealedPrincipalMask: 0,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 1,
      childMemoryBytes: 16384,
      childStorageBytes: 2048,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 1,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 4,
      receiptRequiredMask: 15,
      unsignedReceiptCount: 0,
      fuelUsed: 8192,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 1,
      nextStateRootCount: 1,
      stateRootOverrideCount: 0,
      emittedMessageCount: 1,
      emittedMessageIdentityTargetCount: 1,
      emittedSealedMessageCount: 0,
      plaintextMessageBytes: 0,
    },
  },
  {
    name: "reject-transition-missing-contribution-receipts",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 352,
      len: 160,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 1,
      directNetworkWriteCount: 0,
      routeBytesRequested: 24,
      sealedObjectIntentCount: 0,
      sealedPrincipalMask: 0,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 1,
      childMemoryBytes: 16384,
      childStorageBytes: 2048,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 1,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 4,
      receiptRequiredMask: 15,
      unsignedReceiptCount: 0,
      fuelUsed: 8192,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 1,
      nextStateRootCount: 1,
      stateRootOverrideCount: 0,
      emittedMessageCount: 1,
      emittedMessageIdentityTargetCount: 1,
      emittedSealedMessageCount: 1,
      plaintextMessageBytes: 0,
      contributionReceiptCount: 3,
      unpaidResourceUseCount: 0,
      resourceBudgetOverrunCount: 0,
    },
  },
  {
    name: "reject-transition-unpaid-resource-use",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 160,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 2,
      receiptRequiredMask: 3,
      unsignedReceiptCount: 0,
      fuelUsed: 4096,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 1,
      nextStateRootCount: 1,
      stateRootOverrideCount: 0,
      emittedMessageCount: 0,
      emittedMessageIdentityTargetCount: 0,
      emittedSealedMessageCount: 0,
      plaintextMessageBytes: 0,
      contributionReceiptCount: 2,
      unpaidResourceUseCount: 1,
      resourceBudgetOverrunCount: 0,
    },
  },
  {
    name: "reject-transition-resource-budget-overrun",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 160,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 2,
      receiptRequiredMask: 3,
      unsignedReceiptCount: 0,
      fuelUsed: 4096,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 1,
      nextStateRootCount: 1,
      stateRootOverrideCount: 0,
      emittedMessageCount: 0,
      emittedMessageIdentityTargetCount: 0,
      emittedSealedMessageCount: 0,
      plaintextMessageBytes: 0,
      contributionReceiptCount: 2,
      unpaidResourceUseCount: 0,
      resourceBudgetOverrunCount: 1,
    },
  },
  {
    name: "reject-transition-unsigned-migration",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 172,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 2,
      receiptRequiredMask: 3,
      unsignedReceiptCount: 0,
      fuelUsed: 4096,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 1,
      nextStateRootCount: 1,
      stateRootOverrideCount: 0,
      emittedMessageCount: 0,
      emittedMessageIdentityTargetCount: 0,
      emittedSealedMessageCount: 0,
      plaintextMessageBytes: 0,
      contributionReceiptCount: 2,
      unpaidResourceUseCount: 0,
      resourceBudgetOverrunCount: 0,
      migrationIntentCount: 1,
      userSignedMigrationCount: 0,
      developerToReleaseMigrationCount: 0,
    },
  },
  {
    name: "reject-transition-developer-release-migration-without-user-signature",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 172,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 2,
      receiptRequiredMask: 3,
      unsignedReceiptCount: 0,
      fuelUsed: 4096,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 1,
      nextStateRootCount: 1,
      stateRootOverrideCount: 0,
      emittedMessageCount: 0,
      emittedMessageIdentityTargetCount: 0,
      emittedSealedMessageCount: 0,
      plaintextMessageBytes: 0,
      contributionReceiptCount: 2,
      unpaidResourceUseCount: 0,
      resourceBudgetOverrunCount: 0,
      migrationIntentCount: 1,
      userSignedMigrationCount: 0,
      developerToReleaseMigrationCount: 1,
    },
  },
  {
    name: "reject-transition-raw-listen-port",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 184,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 2,
      receiptRequiredMask: 3,
      unsignedReceiptCount: 0,
      fuelUsed: 4096,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 1,
      nextStateRootCount: 1,
      stateRootOverrideCount: 0,
      emittedMessageCount: 0,
      emittedMessageIdentityTargetCount: 0,
      emittedSealedMessageCount: 0,
      plaintextMessageBytes: 0,
      contributionReceiptCount: 2,
      unpaidResourceUseCount: 0,
      resourceBudgetOverrunCount: 0,
      migrationIntentCount: 0,
      userSignedMigrationCount: 0,
      developerToReleaseMigrationCount: 0,
      hiddenServiceIntentCount: 0,
      rawListenPortCount: 1,
      clearnetIngressCount: 0,
    },
  },
  {
    name: "reject-transition-clearnet-ingress",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 96,
      len: 184,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 1,
      sealedPrincipalMask: 6,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 2,
      receiptRequiredMask: 3,
      unsignedReceiptCount: 0,
      fuelUsed: 4096,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 1,
      nextStateRootCount: 1,
      stateRootOverrideCount: 0,
      emittedMessageCount: 0,
      emittedMessageIdentityTargetCount: 0,
      emittedSealedMessageCount: 0,
      plaintextMessageBytes: 0,
      contributionReceiptCount: 2,
      unpaidResourceUseCount: 0,
      resourceBudgetOverrunCount: 0,
      migrationIntentCount: 0,
      userSignedMigrationCount: 0,
      developerToReleaseMigrationCount: 0,
      hiddenServiceIntentCount: 0,
      rawListenPortCount: 0,
      clearnetIngressCount: 1,
    },
  },
  {
    name: "reject-transition-hidden-service-without-route-capability",
    wat: path.join(port, "tests/reject-abi-missing-route-intent-capability.wat"),
    transition: {
      ptr: 96,
      len: 184,
      magic: 0x5452414e,
      kind: 1,
      inputLen: 16,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 64,
      routeIntentCount: 0,
      directNetworkWriteCount: 0,
      routeBytesRequested: 0,
      sealedObjectIntentCount: 0,
      sealedPrincipalMask: 0,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 0,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 1,
      receiptRequiredMask: 1,
      unsignedReceiptCount: 0,
      fuelUsed: 4096,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 1,
      nextStateRootCount: 1,
      stateRootOverrideCount: 0,
      emittedMessageCount: 0,
      emittedMessageIdentityTargetCount: 0,
      emittedSealedMessageCount: 0,
      plaintextMessageBytes: 0,
      contributionReceiptCount: 2,
      unpaidResourceUseCount: 0,
      resourceBudgetOverrunCount: 0,
      migrationIntentCount: 0,
      userSignedMigrationCount: 0,
      developerToReleaseMigrationCount: 0,
      hiddenServiceIntentCount: 1,
      rawListenPortCount: 0,
      clearnetIngressCount: 0,
    },
  },
  {
    name: "reject-transition-tls-without-capability",
    wat: path.join(port, "tests/reject-abi-missing-sealed-intent-capability.wat"),
    transition: {
      ptr: 352,
      len: 200,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 1,
      directNetworkWriteCount: 0,
      routeBytesRequested: 24,
      sealedObjectIntentCount: 0,
      sealedPrincipalMask: 0,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 0,
      childMemoryBytes: 0,
      childStorageBytes: 0,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 1,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 2,
      receiptRequiredMask: 5,
      unsignedReceiptCount: 0,
      fuelUsed: 4096,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 1,
      nextStateRootCount: 1,
      stateRootOverrideCount: 0,
      emittedMessageCount: 0,
      emittedMessageIdentityTargetCount: 0,
      emittedSealedMessageCount: 0,
      plaintextMessageBytes: 0,
      contributionReceiptCount: 3,
      unpaidResourceUseCount: 0,
      resourceBudgetOverrunCount: 0,
      migrationIntentCount: 0,
      userSignedMigrationCount: 0,
      developerToReleaseMigrationCount: 0,
      hiddenServiceIntentCount: 0,
      rawListenPortCount: 0,
      clearnetIngressCount: 0,
      tlsIntentCount: 1,
      tlsIdentityRouteCount: 1,
      rawTlsSocketCount: 0,
      tlsPlaintextKeyExportCount: 0,
    },
  },
  {
    name: "reject-transition-tls-without-identity-route",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 352,
      len: 200,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 1,
      directNetworkWriteCount: 0,
      routeBytesRequested: 24,
      sealedObjectIntentCount: 0,
      sealedPrincipalMask: 0,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 1,
      childMemoryBytes: 16384,
      childStorageBytes: 2048,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 1,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 4,
      receiptRequiredMask: 15,
      unsignedReceiptCount: 0,
      fuelUsed: 8192,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 1,
      nextStateRootCount: 1,
      stateRootOverrideCount: 0,
      emittedMessageCount: 1,
      emittedMessageIdentityTargetCount: 1,
      emittedSealedMessageCount: 1,
      plaintextMessageBytes: 0,
      contributionReceiptCount: 4,
      unpaidResourceUseCount: 0,
      resourceBudgetOverrunCount: 0,
      migrationIntentCount: 0,
      userSignedMigrationCount: 0,
      developerToReleaseMigrationCount: 0,
      hiddenServiceIntentCount: 0,
      rawListenPortCount: 0,
      clearnetIngressCount: 0,
      tlsIntentCount: 1,
      tlsIdentityRouteCount: 0,
      rawTlsSocketCount: 0,
      tlsPlaintextKeyExportCount: 0,
    },
  },
  {
    name: "reject-transition-raw-tls-socket",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 352,
      len: 200,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 1,
      directNetworkWriteCount: 0,
      routeBytesRequested: 24,
      sealedObjectIntentCount: 0,
      sealedPrincipalMask: 0,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 1,
      childMemoryBytes: 16384,
      childStorageBytes: 2048,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 1,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 4,
      receiptRequiredMask: 15,
      unsignedReceiptCount: 0,
      fuelUsed: 8192,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 1,
      nextStateRootCount: 1,
      stateRootOverrideCount: 0,
      emittedMessageCount: 1,
      emittedMessageIdentityTargetCount: 1,
      emittedSealedMessageCount: 1,
      plaintextMessageBytes: 0,
      contributionReceiptCount: 4,
      unpaidResourceUseCount: 0,
      resourceBudgetOverrunCount: 0,
      migrationIntentCount: 0,
      userSignedMigrationCount: 0,
      developerToReleaseMigrationCount: 0,
      hiddenServiceIntentCount: 0,
      rawListenPortCount: 0,
      clearnetIngressCount: 0,
      tlsIntentCount: 1,
      tlsIdentityRouteCount: 1,
      rawTlsSocketCount: 1,
      tlsPlaintextKeyExportCount: 0,
    },
  },
  {
    name: "reject-transition-tls-key-export",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: {
      ptr: 352,
      len: 200,
      magic: 0x5452414e,
      kind: 2,
      inputLen: 8,
      clockDelta: 1,
      storageIntentCount: 1,
      directStorageWriteCount: 0,
      storageBytesRequested: 32,
      routeIntentCount: 1,
      directNetworkWriteCount: 0,
      routeBytesRequested: 24,
      sealedObjectIntentCount: 0,
      sealedPrincipalMask: 0,
      plaintextPrivateBytes: 0,
      childSpawnIntentCount: 1,
      childMemoryBytes: 16384,
      childStorageBytes: 2048,
      childInspectHandleCount: 0,
      routeIdentityIntentCount: 1,
      rawPortOpenCount: 0,
      dnsLookupCount: 0,
      eventAppendIntentCount: 1,
      immediateStorageIoCount: 0,
      cacheWriteIntentCount: 0,
      durableResultIntentCount: 1,
      receiptIntentCount: 4,
      receiptRequiredMask: 15,
      unsignedReceiptCount: 0,
      fuelUsed: 8192,
      fuelLimit: 1000000,
      unmeteredLoopCount: 0,
      previousStateRootCount: 1,
      nextStateRootCount: 1,
      stateRootOverrideCount: 0,
      emittedMessageCount: 1,
      emittedMessageIdentityTargetCount: 1,
      emittedSealedMessageCount: 1,
      plaintextMessageBytes: 0,
      contributionReceiptCount: 4,
      unpaidResourceUseCount: 0,
      resourceBudgetOverrunCount: 0,
      migrationIntentCount: 0,
      userSignedMigrationCount: 0,
      developerToReleaseMigrationCount: 0,
      hiddenServiceIntentCount: 0,
      rawListenPortCount: 0,
      clearnetIngressCount: 0,
      tlsIntentCount: 1,
      tlsIdentityRouteCount: 1,
      rawTlsSocketCount: 0,
      tlsPlaintextKeyExportCount: 1,
    },
  },
  {
    name: "reject-transition-dependency-without-pinned-identity",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: validActionTransition({
      dependencyCallIntentCount: 1,
      dependencyPinnedIdentityCount: 0,
      dynamicDependencyCallCount: 0,
      dependencyReceiptCount: 1,
    }),
  },
  {
    name: "reject-transition-dynamic-dependency-call",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: validActionTransition({
      dependencyCallIntentCount: 1,
      dependencyPinnedIdentityCount: 1,
      dynamicDependencyCallCount: 1,
      dependencyReceiptCount: 1,
    }),
  },
  {
    name: "reject-transition-dependency-without-receipt",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: validActionTransition({
      dependencyCallIntentCount: 1,
      dependencyPinnedIdentityCount: 1,
      dynamicDependencyCallCount: 0,
      dependencyReceiptCount: 0,
    }),
  },
  {
    name: "reject-transition-public-object-without-requirements",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: validActionTransition({
      publicObjectIntentCount: 1,
      objectRequirementCount: 0,
    }),
  },
  {
    name: "reject-transition-object-requirement-omitted",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    transition: validActionTransition({
      integrityObjectIntentCount: 1,
      objectRequirementCount: 1,
      objectWithoutRequirementCount: 1,
    }),
  },
];

function validActionTransition(overrides = {}) {
  return {
    ptr: 352,
    len: 232,
    magic: 0x5452414e,
    kind: 2,
    inputLen: 8,
    clockDelta: 1,
    storageIntentCount: 1,
    directStorageWriteCount: 0,
    storageBytesRequested: 32,
    routeIntentCount: 1,
    directNetworkWriteCount: 0,
    routeBytesRequested: 24,
    sealedObjectIntentCount: 0,
    sealedPrincipalMask: 0,
    plaintextPrivateBytes: 0,
    childSpawnIntentCount: 1,
    childMemoryBytes: 16384,
    childStorageBytes: 2048,
    childInspectHandleCount: 0,
    routeIdentityIntentCount: 1,
    rawPortOpenCount: 0,
    dnsLookupCount: 0,
    eventAppendIntentCount: 1,
    immediateStorageIoCount: 0,
    cacheWriteIntentCount: 0,
    durableResultIntentCount: 1,
    receiptIntentCount: 4,
    receiptRequiredMask: 15,
    unsignedReceiptCount: 0,
    fuelUsed: 8192,
    fuelLimit: 1000000,
    unmeteredLoopCount: 0,
    previousStateRootCount: 1,
    nextStateRootCount: 1,
    stateRootOverrideCount: 0,
    emittedMessageCount: 1,
    emittedMessageIdentityTargetCount: 1,
    emittedSealedMessageCount: 1,
    plaintextMessageBytes: 0,
    contributionReceiptCount: 4,
    unpaidResourceUseCount: 0,
    resourceBudgetOverrunCount: 0,
    migrationIntentCount: 0,
    userSignedMigrationCount: 0,
    developerToReleaseMigrationCount: 0,
    hiddenServiceIntentCount: 0,
    rawListenPortCount: 0,
    clearnetIngressCount: 0,
    tlsIntentCount: 1,
    tlsIdentityRouteCount: 1,
    rawTlsSocketCount: 0,
    tlsPlaintextKeyExportCount: 0,
    dependencyCallIntentCount: 0,
    dependencyPinnedIdentityCount: 0,
    dynamicDependencyCallCount: 0,
    dependencyReceiptCount: 0,
    objectRequirementCount: 0,
    publicObjectIntentCount: 0,
    integrityObjectIntentCount: 0,
    objectWithoutRequirementCount: 0,
    ...overrides,
  };
}

const renderPolicyRejectTests = [
  {
    name: "reject-render-plaintext-private",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    render: {
      ptr: 320,
      len: 12,
      magic: 0x55490000,
      sealedRefCount: 1,
      plaintextPrivateBytes: 1,
    },
  },
  {
    name: "reject-render-memory-bounds",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    render: {
      ptr: 65532,
      len: 12,
      magic: 0x55490000,
      sealedRefCount: 1,
      plaintextPrivateBytes: 0,
    },
  },
  {
    name: "reject-render-raw-input-capture",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    render: {
      ptr: 896,
      len: 24,
      magic: 0x55490000,
      sealedRefCount: 1,
      plaintextPrivateBytes: 0,
      sealedInputRequestCount: 1,
      rawInputCaptureCount: 1,
      revealWithoutUserActionCount: 0,
    },
  },
  {
    name: "reject-render-reveal-without-user-action",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    render: {
      ptr: 896,
      len: 40,
      magic: 0x55490000,
      sealedRefCount: 1,
      plaintextPrivateBytes: 0,
      sealedInputRequestCount: 1,
      rawInputCaptureCount: 0,
      revealWithoutUserActionCount: 1,
    },
  },
  {
    name: "reject-render-direct-state-mutation",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    render: {
      ptr: 896,
      len: 40,
      magic: 0x55490000,
      sealedRefCount: 1,
      plaintextPrivateBytes: 0,
      sealedInputRequestCount: 1,
      rawInputCaptureCount: 0,
      revealWithoutUserActionCount: 0,
      uiActionIntentCount: 1,
      renderDirectStateMutationCount: 1,
      renderDirectStorageMutationCount: 0,
      renderDirectNetworkMutationCount: 0,
    },
  },
  {
    name: "reject-render-direct-storage-mutation",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    render: {
      ptr: 896,
      len: 40,
      magic: 0x55490000,
      sealedRefCount: 1,
      plaintextPrivateBytes: 0,
      sealedInputRequestCount: 1,
      rawInputCaptureCount: 0,
      revealWithoutUserActionCount: 0,
      uiActionIntentCount: 1,
      renderDirectStateMutationCount: 0,
      renderDirectStorageMutationCount: 1,
      renderDirectNetworkMutationCount: 0,
    },
  },
  {
    name: "reject-render-direct-network-mutation",
    wat: path.join(port, "tests/app-abi-v0.wat"),
    render: {
      ptr: 896,
      len: 40,
      magic: 0x55490000,
      sealedRefCount: 1,
      plaintextPrivateBytes: 0,
      sealedInputRequestCount: 1,
      rawInputCaptureCount: 0,
      revealWithoutUserActionCount: 0,
      uiActionIntentCount: 1,
      renderDirectStateMutationCount: 0,
      renderDirectStorageMutationCount: 0,
      renderDirectNetworkMutationCount: 1,
    },
  },
];

const watRunErrorTests = [
  {
    name: "reject-recursive-call",
    wat: path.join(port, "tests/reject-recursive-call.wat"),
    exportName: "entry",
    error: 31,
    requiresManifest: true,
  },
  {
    name: "reject-memory-grow",
    wat: path.join(port, "tests/reject-memory-grow.wat"),
    exportName: "entry",
    error: 15,
    requiresManifest: true,
  },
  {
    name: "reject-start-section",
    wat: path.join(port, "tests/reject-start-section.wat"),
    exportName: "entry",
    error: 1,
    requiresManifest: true,
  },
  {
    name: "reject-mutable-global",
    wat: path.join(port, "tests/reject-mutable-global.wat"),
    exportName: "entry",
    error: 1,
    requiresManifest: true,
  },
  {
    name: "reject-call-indirect",
    wat: path.join(port, "tests/reject-call-indirect.wat"),
    exportName: "entry",
    error: 1,
    requiresManifest: true,
  },
  {
    name: "reject-direct-storage-import",
    wat: path.join(port, "tests/reject-direct-storage-import.wat"),
    exportName: "entry",
    error: 18,
    requiresManifest: true,
  },
  {
    name: "reject-raw-network-import",
    wat: path.join(port, "tests/reject-raw-network-import.wat"),
    exportName: "entry",
    error: 18,
    requiresManifest: true,
  },
  {
    name: "reject-dns-import",
    wat: path.join(port, "tests/reject-dns-import.wat"),
    exportName: "entry",
    error: 18,
    requiresManifest: true,
  },
  {
    name: "reject-raw-tls-import",
    wat: path.join(port, "tests/reject-raw-tls-import.wat"),
    exportName: "entry",
    error: 18,
    requiresManifest: true,
  },
  {
    name: "reject-tls-key-export-import",
    wat: path.join(port, "tests/reject-tls-key-export-import.wat"),
    exportName: "entry",
    error: 18,
    requiresManifest: true,
  },
  {
    name: "reject-hostcall-import",
    wat: path.join(port, "tests/reject-hostcall-import.wat"),
    exportName: "entry",
    error: 18,
    requiresManifest: true,
  },
  {
    name: "reject-wall-clock-import",
    wat: path.join(port, "tests/reject-wall-clock-import.wat"),
    exportName: "entry",
    error: 18,
    requiresManifest: true,
  },
  {
    name: "reject-ambient-random-import",
    wat: path.join(port, "tests/reject-ambient-random-import.wat"),
    exportName: "entry",
    error: 18,
    requiresManifest: true,
  },
];

const watPolicyRejectTests = [
  {
    name: "reject-missing-manifest",
    wat: path.join(port, "tests/reject-missing-manifest.wat"),
    reason: "missing er.manifest",
  },
  {
    name: "reject-invalid-manifest-mode",
    wat: path.join(port, "tests/reject-invalid-manifest-mode.wat"),
    reason: "invalid manifest mode",
  },
  {
    name: "reject-invalid-manifest-abi",
    wat: path.join(port, "tests/reject-invalid-manifest-abi.wat"),
    reason: "manifest ABI",
  },
  {
    name: "reject-manifest-recursion-true",
    wat: path.join(port, "tests/reject-manifest-recursion-true.wat"),
    reason: "manifest enables recursion",
  },
  {
    name: "reject-manifest-memory-not-static",
    wat: path.join(port, "tests/reject-manifest-memory-not-static.wat"),
    reason: "manifest memory must be static",
  },
  {
    name: "reject-manifest-direct-storage-access",
    wat: path.join(port, "tests/reject-manifest-direct-storage-access.wat"),
    reason: "manifest storage direct access",
  },
  {
    name: "reject-manifest-missing-storage",
    wat: path.join(port, "tests/reject-manifest-missing-storage.wat"),
    reason: "missing storage.min",
  },
  {
    name: "reject-missing-memory-section",
    wat: path.join(port, "tests/reject-missing-memory-section.wat"),
    reason: "missing wasm memory section",
  },
  {
    name: "reject-manifest-memory-mismatch",
    wat: path.join(port, "tests/reject-manifest-memory-mismatch.wat"),
    reason: "manifest memory.min does not match wasm memory min",
  },
  {
    name: "reject-memory-max",
    wat: path.join(port, "tests/reject-memory-max.wat"),
    reason: "wasm memory declares max",
  },
  {
    name: "reject-memory-export",
    wat: path.join(port, "tests/reject-memory-export.wat"),
    reason: "memory export",
  },
  {
    name: "reject-global-export",
    wat: path.join(port, "tests/reject-global-export.wat"),
    reason: "global export",
  },
  {
    name: "reject-table-export",
    wat: path.join(port, "tests/reject-table-export.wat"),
    reason: "table export",
  },
  {
    name: "reject-manifest-zero-memory",
    wat: path.join(port, "tests/reject-manifest-zero-memory.wat"),
    reason: "zero memory.min",
  },
  {
    name: "reject-manifest-zero-storage",
    wat: path.join(port, "tests/reject-manifest-zero-storage.wat"),
    reason: "zero storage.min",
  },
  {
    name: "reject-manifest-unaligned-storage",
    wat: path.join(port, "tests/reject-manifest-unaligned-storage.wat"),
    reason: "unaligned storage.min",
  },
  {
    name: "reject-duplicate-manifest",
    wat: path.join(port, "tests/reject-duplicate-manifest.wat"),
    reason: "duplicate er.manifest",
  },
  {
    name: "reject-data-section",
    wat: path.join(port, "tests/reject-data-section.wat"),
    reason: "data section",
  },
  {
    name: "reject-unknown-capability",
    wat: path.join(port, "tests/reject-unknown-capability.wat"),
    reason: "unknown capability",
  },
  {
    name: "reject-dynamic-dependency",
    wat: path.join(port, "tests/reject-dynamic-dependency.wat"),
    reason: "dynamic dependency",
  },
  {
    name: "reject-unpinned-dependency",
    wat: path.join(port, "tests/reject-unpinned-dependency.wat"),
    reason: "unpinned dependency",
  },
  {
    name: "reject-dependency-without-capability",
    wat: path.join(port, "tests/reject-dependency-without-capability.wat"),
    reason: "dependency.use capability",
  },
  {
    name: "reject-postinstall-dependency",
    wat: path.join(port, "tests/reject-postinstall-dependency.wat"),
    reason: "postinstall dependency",
  },
];

const watAbiPolicyRejectTests = [
  {
    name: "reject-abi-missing-storage-intent-capability",
    wat: path.join(port, "tests/reject-abi-missing-storage-intent-capability.wat"),
    reason: "missing storage.intent capability",
  },
  {
    name: "reject-abi-missing-render",
    wat: path.join(port, "tests/reject-abi-missing-render.wat"),
    reason: "missing er_render",
  },
  {
    name: "reject-abi-wrong-message-signature",
    wat: path.join(port, "tests/reject-abi-wrong-message-signature.wat"),
    reason: "wrong er_handle_message signature",
  },
  {
    name: "reject-abi-extra-er-export",
    wat: path.join(port, "tests/reject-abi-extra-er-export.wat"),
    reason: "extra er_* export",
  },
  {
    name: "reject-abi-wrong-init-result",
    wat: path.join(port, "tests/reject-abi-wrong-init-result.wat"),
    reason: "wrong er_init result",
  },
];

function run(cmd, args, options = {}) {
  const result = spawnSync(cmd, args, {
    cwd: options.cwd || root,
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 32,
  });
  if (result.status !== 0) {
    const detail = [
      `${cmd} ${args.join(" ")}`,
      result.stdout && `stdout:\n${result.stdout}`,
      result.stderr && `stderr:\n${result.stderr}`,
    ]
      .filter(Boolean)
      .join("\n");
    throw new Error(detail);
  }
  return result;
}

function assemble(input, output) {
  run("yasm", ["-f", "elf64", "-I", source, "-o", output, input]);
}

function link(objects, output) {
  run("ld", [
    "-T",
    path.join(source, "test/test_jit.ld"),
    "-nostdlib",
    "-static",
    "-o",
    output,
    ...objects,
  ]);
}

function compileWat(wat, wasm) {
  run("wat2wasm", ["--enable-annotations", wat, "-o", wasm]);
}

function asmString(value) {
  return value
    .split("")
    .map((ch) => ch.charCodeAt(0))
    .join(",");
}

function asmCString(value) {
  return `${asmString(value)},0`;
}

function asmQwords(values) {
  return values.length === 0 ? "0" : values.map((value) => String(value)).join(",");
}

function asmMemoryChecks(memoryChecks = []) {
  return memoryChecks
    .map(
      ([offset, expected]) => `    cmp     dword [linear_memory + ${offset}], ${expected}
    jne     .fail`,
    )
    .join("\n");
}

function asmMemoryByteChecks(memoryByteChecks = []) {
  return memoryByteChecks
    .map(
      ([offset, expected]) => `    cmp     byte [linear_memory + ${offset}], ${expected}
    jne     .fail`,
    )
    .join("\n");
}

function stageRuntimeConfigAsm() {
  return `    lea     rax, [rel linear_memory]
    mov     [runtime + RUNTIME_MEMORY_PTR_OFF], rax
    mov     qword [runtime + RUNTIME_MEMORY_LEN_OFF], ${harnessMemoryBytes}
    lea     rax, [rel ticks]
    mov     [runtime + RUNTIME_TICKS_PTR_OFF], rax
`;
}

function writeWatHarness(wasmPath, exportName, expected, output) {
  const escapedWasmPath = wasmPath.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
  fs.writeFileSync(
    output,
    `; Generated by edgerun-x86-wasm-runtime-smoke.js
%define HAVE_ER_WASM_RUNTIME_PTR
%include "x86_64/wasm/wasm_interpreter.asm"
%include "test/test_macros.inc"

SECTION .data
wasm_bytes:
    incbin "${escapedWasmPath}"
wasm_bytes_end:
export_name: db ${asmString(exportName)}
EXPORT_NAME_LEN equ ${exportName.length}

SECTION .bss
runtime: resb RUNTIME_SIZE
linear_memory: resb ${harnessMemoryBytes}
ticks: resq 1

global er_wasm_runtime_ptr
er_wasm_runtime_ptr: resq 1

SECTION .text
global _start
_start:
${stageRuntimeConfigAsm()}\
    lea     rdi, [rel runtime]
    lea     rsi, [rel wasm_bytes]
    mov     rdx, wasm_bytes_end - wasm_bytes
    lea     rcx, [rel export_name]
    mov     r8, EXPORT_NAME_LEN
    call    er_fn_run
    cmp     rdx, 1
    jne     .fail
    cmp     rax, ${expected}
    jne     .fail
    TEST_EXIT 0
.fail:
    TEST_EXIT 1
`,
  );
}

function writeWatArgsHarness(
  wasmPath,
  exportName,
  args,
  expected,
  output,
  memoryChecks = [],
  memoryByteChecks = [],
  dumpPath = null,
  trustedLoad = false,
) {
  const escapedWasmPath = wasmPath.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
  const escapedDumpPath = dumpPath ? dumpPath.replace(/\\/g, "\\\\") : null;
  const dumpData = escapedDumpPath ? `dump_path: db ${asmCString(escapedDumpPath)}\n` : "";
  const dumpAsm = escapedDumpPath
    ? `    mov     rax, 85
    lea     rdi, [rel dump_path]
    mov     rsi, 420
    syscall
    cmp     rax, 0
    jl      .fail
    mov     r12, rax
    mov     rax, 1
    mov     rdi, r12
    lea     rsi, [rel linear_memory]
    mov     rdx, ${harnessMemoryBytes}
    syscall
    cmp     rax, ${harnessMemoryBytes}
    jne     .fail
    mov     rax, 3
    mov     rdi, r12
    syscall
`
    : "";
  const callAsm = trustedLoad
    ? `    lea     rdi, [rel runtime]
    lea     rsi, [rel wasm_bytes]
    mov     rdx, wasm_bytes_end - wasm_bytes
    call    er_fn_load_trusted
    cmp     rdx, 0
    jne     .fail
    lea     rdi, [rel runtime]
    lea     rsi, [rel export_name]
    mov     rdx, EXPORT_NAME_LEN
    lea     rcx, [rel args]
    mov     r8, ${args.length}
    call    er_fn_call_args
    cmp     rdx, 0
    jne     .fail`
    : `    lea     rdi, [rel runtime]
    lea     rsi, [rel wasm_bytes]
    mov     rdx, wasm_bytes_end - wasm_bytes
    lea     rcx, [rel export_name]
    mov     r8, EXPORT_NAME_LEN
    lea     r9, [rel args]
    mov     rax, ${args.length}
    push    rax
    call    er_fn_run_args
    add     rsp, 8
    cmp     rdx, 1
    jne     .fail`;
  fs.writeFileSync(
    output,
    `; Generated by edgerun-x86-wasm-runtime-smoke.js
%define HAVE_ER_WASM_RUNTIME_PTR
%include "x86_64/wasm/wasm_interpreter.asm"
%include "test/test_macros.inc"

SECTION .data
wasm_bytes:
    incbin "${escapedWasmPath}"
wasm_bytes_end:
export_name: db ${asmString(exportName)}
EXPORT_NAME_LEN equ ${exportName.length}
args: dq ${asmQwords(args)}
${dumpData}

SECTION .bss
runtime: resb RUNTIME_SIZE
linear_memory: resb ${harnessMemoryBytes}
ticks: resq 1

global er_wasm_runtime_ptr
er_wasm_runtime_ptr: resq 1

SECTION .text
global _start
_start:
${stageRuntimeConfigAsm()}\
${callAsm}
    cmp     eax, ${expected}
    jne     .fail
${asmMemoryChecks(memoryChecks)}
${asmMemoryByteChecks(memoryByteChecks)}
${dumpAsm}
    TEST_EXIT 0
.fail:
    TEST_EXIT 1
`,
  );
}

function writeWatErrorHarness(wasmPath, exportName, expectedError, output) {
  const escapedWasmPath = wasmPath.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
  fs.writeFileSync(
    output,
    `; Generated by edgerun-x86-wasm-runtime-smoke.js
%define HAVE_ER_WASM_RUNTIME_PTR
%include "x86_64/wasm/wasm_interpreter.asm"
%include "test/test_macros.inc"

SECTION .data
wasm_bytes:
    incbin "${escapedWasmPath}"
wasm_bytes_end:
export_name: db ${asmString(exportName)}
EXPORT_NAME_LEN equ ${exportName.length}

SECTION .bss
runtime: resb RUNTIME_SIZE
linear_memory: resb ${harnessMemoryBytes}
ticks: resq 1

global er_wasm_runtime_ptr
er_wasm_runtime_ptr: resq 1

SECTION .text
global _start
_start:
${stageRuntimeConfigAsm()}\
    lea     rdi, [rel runtime]
    lea     rsi, [rel wasm_bytes]
    mov     rdx, wasm_bytes_end - wasm_bytes
    lea     rcx, [rel export_name]
    mov     r8, EXPORT_NAME_LEN
    call    er_fn_run
    cmp     rdx, ${expectedError}
    jne     .fail
    cmp     rax, -1
    jne     .fail
    TEST_EXIT 0
.fail:
    TEST_EXIT 1
`,
  );
}

function runWatArgsCase({
  wasm,
  caseName,
  exportName,
  args,
  expected,
  memoryChecks = [],
  memoryByteChecks = [],
  runtimeMemObject,
  dumpMemory = false,
  trustedLoad = false,
}) {
  const harness = path.join(outDir, `${caseName}.asm`);
  const object = path.join(outDir, `${caseName}.o`);
  const binary = path.join(outDir, caseName);
  const dumpPath = dumpMemory ? path.join(outDir, `${caseName}.linear-memory.bin`) : null;
  writeWatArgsHarness(wasm, exportName, args, expected, harness, memoryChecks, memoryByteChecks, dumpPath, trustedLoad);
  assemble(harness, object);
  link([object, runtimeMemObject], binary);
  const result = spawnSync(binary, [], {
    cwd: outDir,
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 32,
  });
  assert.strictEqual(result.status, 0, `${caseName}\n${result.stdout}\n${result.stderr}`);
  if (!dumpMemory) {
    return null;
  }
  assert(fs.existsSync(dumpPath), `${caseName} did not dump linear memory`);
  return fs.readFileSync(dumpPath);
}

try {
  const runtimeMemObject = path.join(outDir, "runtime_mem_unit.o");
  assemble(path.join(source, "x86_64/rt/runtime_mem_unit.asm"), runtimeMemObject);

  const results = [];
  for (const test of tests) {
    const testObject = path.join(outDir, `${test}.o`);
    const binary = path.join(outDir, test);
    assemble(path.join(source, `test/${test}.asm`), testObject);
    link([testObject, runtimeMemObject], binary);
    const result = spawnSync(binary, [], {
      cwd: outDir,
      encoding: "utf8",
      maxBuffer: 1024 * 1024 * 32,
    });
    assert.strictEqual(result.status, 0, `${test}\n${result.stdout}\n${result.stderr}`);
    results.push(test);
  }

  const watResults = [];
  for (const test of watTests) {
    const wasm = path.join(outDir, `${test.name}.wasm`);
    run("wat2wasm", ["--enable-annotations", test.wat, "-o", wasm]);
    if (test.requiresManifest) {
      policy.assertManifest(wasm, test.name);
    }
    for (const [exportName, expected] of test.cases) {
      const caseName = `${test.name}-${exportName}`;
      const harness = path.join(outDir, `${caseName}.asm`);
      const object = path.join(outDir, `${caseName}.o`);
      const binary = path.join(outDir, caseName);
      writeWatHarness(wasm, exportName, expected, harness);
      assemble(harness, object);
      link([object, runtimeMemObject], binary);
      const result = spawnSync(binary, [], {
        cwd: outDir,
        encoding: "utf8",
        maxBuffer: 1024 * 1024 * 32,
      });
      assert.strictEqual(result.status, 0, `${caseName}\n${result.stdout}\n${result.stderr}`);
      watResults.push(caseName);
    }
  }

  const watRunErrorResults = [];
  for (const test of watRunErrorTests) {
    const wasm = path.join(outDir, `${test.name}.wasm`);
    run("wat2wasm", ["--enable-annotations", test.wat, "-o", wasm]);
    if (test.requiresManifest) {
      policy.assertManifest(wasm, test.name);
    }
    const harness = path.join(outDir, `${test.name}.asm`);
    const object = path.join(outDir, `${test.name}.o`);
    const binary = path.join(outDir, test.name);
    writeWatErrorHarness(wasm, test.exportName, test.error, harness);
    assemble(harness, object);
    link([object, runtimeMemObject], binary);
    const result = spawnSync(binary, [], {
      cwd: outDir,
      encoding: "utf8",
      maxBuffer: 1024 * 1024 * 32,
    });
    assert.strictEqual(result.status, 0, `${test.name}\n${result.stdout}\n${result.stderr}`);
    watRunErrorResults.push(test.name);
  }

  const watArgResults = [];
  for (const test of watArgTests) {
    const wasm = path.join(outDir, `${test.name}.wasm`);
    run("wat2wasm", ["--enable-annotations", test.wat, "-o", wasm]);
    if (test.requiresManifest) {
      policy.assertManifest(wasm, test.name);
    }
    if (test.requiresAppAbi) {
      policy.assertAppAbi(wasm, test.name);
    }
    for (const [testCaseIndex, testCase] of test.cases.entries()) {
      const caseName = `${test.name}-${testCase.exportName}-${testCaseIndex}`;
      runWatArgsCase({
        wasm,
        caseName,
        exportName: testCase.exportName,
        args: testCase.args,
        expected: testCase.expected,
        memoryChecks: testCase.memoryChecks,
        memoryByteChecks: testCase.memoryByteChecks,
        runtimeMemObject,
        trustedLoad: test.trustedLoad || testCase.trustedLoad || false,
      });
      watArgResults.push(caseName);
    }
  }

  const watIdentityResults = [];
  for (const test of watIdentityTests) {
    const releaseA = path.join(outDir, `${test.name}-release-a.wasm`);
    const releaseB = path.join(outDir, `${test.name}-release-b.wasm`);
    const developer = path.join(outDir, `${test.name}-developer.wasm`);
    compileWat(test.releaseWat, releaseA);
    compileWat(test.releaseWat, releaseB);
    compileWat(test.developerWat, developer);

    const releaseManifest = policy.manifestPolicy(releaseA);
    const developerManifest = policy.manifestPolicy(developer);
    policy.assertAppAbi(releaseA, `${test.name}-release`, releaseManifest);
    policy.assertAppAbi(developer, `${test.name}-developer`, developerManifest);

    const releaseHash = policy.wasmHash(releaseA);
    assert.strictEqual(releaseHash, policy.wasmHash(releaseB), `${test.name} release build hash is unstable`);
    assert.notStrictEqual(
      releaseHash,
      policy.wasmHash(developer),
      `${test.name} release and developer identities must differ`,
    );
    watIdentityResults.push(test.name);
  }

	  const transitionLogResults = [];
	  for (const test of transitionLogTests) {
    const wasm = path.join(outDir, `${test.name}.wasm`);
    compileWat(test.appWat, wasm);
    const manifest = policy.manifestPolicy(wasm);
    policy.assertAppAbi(wasm, test.name, manifest);

    const appId = Buffer.from(policy.wasmHash(wasm), "hex");
    const transitions = [];
    for (const call of test.transitionCalls) {
      const caseName = `${test.name}-${call.exportName}-decode`;
      const memory = runWatArgsCase({
        wasm,
        caseName,
        exportName: call.exportName,
        args: call.args,
        expected: call.expectedPtr,
        runtimeMemObject,
        dumpMemory: true,
      });
      transitions.push(policy.decodeTransition(memory, call.expectedPtr));
    }

    const renderCaseName = `${test.name}-${test.renderCall.exportName}-decode`;
    const renderMemory = runWatArgsCase({
      wasm,
      caseName: renderCaseName,
      exportName: test.renderCall.exportName,
      args: test.renderCall.args,
      expected: test.renderCall.expectedPtr,
      runtimeMemObject,
      dumpMemory: true,
    });
    const render = policy.decodeRender(renderMemory, test.renderCall.expectedPtr);

    let clock = 0;
    let previousHash = Buffer.alloc(32, 0);
    for (const transition of transitions) {
      policy.assertTransitionIdentityPolicy(test.name, appId, appId);
      policy.assertTransitionPolicy(test.name, transition, manifest);
      clock += transition.clockDelta;
      const nextHash = policy.eventHash({ appId, clock, previousHash, transition });
      assert.notStrictEqual(nextHash.toString("hex"), previousHash.toString("hex"), `${test.name} event hash did not advance`);
      previousHash = nextHash;
    }

    const renderClock = clock;
    policy.assertRenderPolicy(test.name, render, manifest);
    assert.strictEqual(clock, renderClock, `${test.name} render must not tick app clock`);

    let reorderedClock = 0;
    let reorderedHash = Buffer.alloc(32, 0);
    for (const transition of [...transitions].reverse()) {
      reorderedClock += transition.clockDelta;
      reorderedHash = policy.eventHash({
        appId,
        clock: reorderedClock,
        previousHash: reorderedHash,
        transition,
      });
    }
    assert.notStrictEqual(
      previousHash.toString("hex"),
      reorderedHash.toString("hex"),
      `${test.name} event hash chain must be order-sensitive`,
    );

    const developerWasm = path.join(outDir, `${test.name}-developer-identity.wasm`);
    compileWat(path.join(port, "tests/app-identity-developer.wat"), developerWasm);
    const developerAppId = Buffer.from(policy.wasmHash(developerWasm), "hex");
    let developerClock = 0;
    let developerHash = Buffer.alloc(32, 0);
    for (const transition of transitions) {
      developerClock += transition.clockDelta;
      developerHash = policy.eventHash({
        appId: developerAppId,
        clock: developerClock,
        previousHash: developerHash,
        transition,
      });
    }
    assert.notStrictEqual(
      previousHash.toString("hex"),
      developerHash.toString("hex"),
      `${test.name} event hash chain must be bound to app identity`,
    );
	    transitionLogResults.push(test.name);
	  }

	  const localMemorySimResults = [];
	  for (const test of localMemorySimTests) {
	    const result = simulateWatApp({ watPath: test.wat });
	    assert.strictEqual(result.clock, 2, `${test.name} simulated app clock mismatch`);
	    assert.strictEqual(result.transitions.length, 2, `${test.name} transition count mismatch`);
	    assert.strictEqual(result.render.magic, appAbiV0.magic.render, `${test.name} render magic mismatch`);
	    assert.strictEqual(result.init.marker, 0x494e4954, `${test.name} init marker mismatch`);
	    const manifest = result.manifest;
	    const appId = Buffer.from(result.appId, "hex");
	    let clock = 0;
	    let previousHash = Buffer.alloc(32, 0);
	    for (const item of result.transitions) {
	      policy.assertTransitionIdentityPolicy(test.name, appId, appId);
	      policy.assertTransitionPolicy(test.name, item.transition, manifest);
	      clock += item.transition.clockDelta;
	      previousHash = policy.eventHash({
	        appId,
	        clock,
	        previousHash,
	        transition: item.transition,
	      });
	      assert.strictEqual(
	        item.hash,
	        previousHash.toString("hex"),
	        `${test.name} simulator event hash mismatch`,
	      );
	    }
	    policy.assertRenderPolicy(test.name, result.render, manifest);
	    assert.strictEqual(result.eventHash, previousHash.toString("hex"), `${test.name} final event hash mismatch`);
	    localMemorySimResults.push(test.name);
	  }

	  const localKernelCommitResults = [];
	  for (const test of localKernelCommitTests) {
	    const result = simulateWatAppCommit({ watPath: test.wat });
	    assert.strictEqual(result.commit.appId, result.app.appId, `${test.name} commit app id mismatch`);
	    assert.strictEqual(result.commit.clock, result.app.clock, `${test.name} commit clock mismatch`);
	    assert.strictEqual(result.commit.eventHash, result.app.eventHash, `${test.name} commit event hash mismatch`);
	    assert.strictEqual(result.commit.eventLog.length, result.app.transitions.length, `${test.name} event count mismatch`);
	    assert.strictEqual(result.commit.queuedWrites.length, 4, `${test.name} queued write count mismatch`);
	    assert.strictEqual(result.commit.grants.storageUsedBytes, 96, `${test.name} storage usage mismatch`);
	    assert.strictEqual(result.commit.grants.childMemoryMovedBytes, 16384, `${test.name} child memory move mismatch`);
	    assert.strictEqual(result.commit.grants.childStorageMovedBytes, 2048, `${test.name} child storage move mismatch`);
	    assert.strictEqual(result.app.transitions[0].transition.intentRecords.length, 2, `${test.name} message intent record count mismatch`);
	    assert.strictEqual(result.app.transitions[1].transition.intentRecords.length, 4, `${test.name} action intent record count mismatch`);
	    assert.strictEqual(result.app.transitions[0].transition.intentRecords[0].kind, appAbiV0.intentKinds.storageAppend, `${test.name} message storage intent kind mismatch`);
	    assert.strictEqual(result.app.transitions[0].transition.intentRecords[1].kind, appAbiV0.intentKinds.sealedStore, `${test.name} message sealed intent kind mismatch`);
	    assert.strictEqual(result.app.transitions[1].transition.intentRecords[1].kind, appAbiV0.intentKinds.routeSendIdentity, `${test.name} action route intent kind mismatch`);
	    assert.strictEqual(
	      result.app.transitions[1].transition.intentRecords[1].flags & appAbiV0.intentFlags.routeIdentityTarget,
	      appAbiV0.intentFlags.routeIdentityTarget,
	      `${test.name} route intent must target identity`,
	    );
	    assert.strictEqual(result.app.transitions[1].transition.intentRecords[2].kind, appAbiV0.intentKinds.routeCreateHiddenService, `${test.name} hidden service intent kind mismatch`);
	    assert.strictEqual(
	      result.app.transitions[1].transition.intentRecords[2].flags & appAbiV0.intentFlags.routeIdentityTarget,
	      appAbiV0.intentFlags.routeIdentityTarget,
	      `${test.name} hidden service intent must be identity-routed`,
	    );
	    assert.strictEqual(result.app.transitions[1].transition.intentRecords[3].kind, appAbiV0.intentKinds.childSpawn, `${test.name} child intent kind mismatch`);
	    assert.doesNotThrow(() => assertIntentRecords(result.app.transitions[0].transition), `${test.name} message intent records invalid`);
	    assert.doesNotThrow(() => assertIntentRecords(result.app.transitions[1].transition), `${test.name} action intent records invalid`);
	    assert.deepStrictEqual(
	      assertCommitReplay({
	        appId: result.app.appId,
	        eventLog: result.commit.eventLog,
	        transitions: result.app.transitions,
	      }),
	      {
	        clock: result.commit.clock,
	        eventHash: result.commit.eventHash,
	        stateRoot: result.commit.stateRoot,
	      },
	      `${test.name} commit replay mismatch`,
	    );
	    for (const event of result.commit.eventLog) {
	      assert.strictEqual(event.appId, result.app.appId, `${test.name} event app id mismatch`);
	      assert(event.receipts.length > 0, `${test.name} event missing receipts`);
	      assert(event.queuedWrites.some((write) => write.kind === "event-log"), `${test.name} event missing queued event-log write`);
	      assert(
	        event.queuedWrites.every((write) => write.scheduledBy === "kernel"),
	        `${test.name} queued writes must be kernel scheduled`,
	      );
	    }
	    assert.throws(
	      () =>
	        commitSimulation({
	          appId: result.app.appId,
	          manifest: result.app.manifest,
	          transitions: [
	            result.app.transitions[0],
	            result.app.transitions[1],
	            result.app.transitions[1],
	          ],
	        }),
	      `${test.name} cumulative storage exhaustion unexpectedly passed`,
	    );
	    assert.throws(
	      () =>
	        commitSimulation({
	          appId: result.app.appId,
	          manifest: result.app.manifest,
	          transitions: [
	            {
	              transition: {
	                ...result.app.transitions[1].transition,
	                intentRecords: [],
	              },
	            },
	          ],
	        }),
	      `${test.name} missing intent records unexpectedly passed`,
	    );
	    assert.throws(
	      () =>
	        assertIntentRecords({
	          ...result.app.transitions[1].transition,
	          intentRecords: result.app.transitions[1].transition.intentRecords.map((record, index) =>
	            index === 1 ? { ...record, flags: 0 } : { ...record },
	          ),
	        }),
	      `${test.name} route without identity target unexpectedly passed`,
	    );
	    localKernelCommitResults.push(test.name);
	  }

	  const localIdentityRelayResults = [];
	  for (const test of localIdentityRelayTests) {
	    const result = simulateWatAppIdentityRelay({ watPath: test.wat });
	    assert.strictEqual(result.relay.appId, result.app.appId, `${test.name} relay app id mismatch`);
	    assert.strictEqual(result.relay.sourceEventHash, result.commit.eventHash, `${test.name} relay source hash mismatch`);
	    assert.strictEqual(result.relay.routeCount, 1, `${test.name} relay route count mismatch`);
	    assert.strictEqual(result.relay.hiddenServiceCount, 1, `${test.name} hidden service count mismatch`);
	    assert.strictEqual(result.relay.receipts.length, 3, `${test.name} relay receipt count mismatch`);
	    assert.deepStrictEqual(
	      result.relay.receipts.map((receipt) => receipt.kind),
	      ["transit", "delivery", "hidden_service"],
	      `${test.name} relay receipt kinds mismatch`,
	    );
	    const accepted = result.relay.accepted[0];
	    assert.strictEqual(accepted.targetIdentity, "8888888877777777", `${test.name} target identity mismatch`);
	    assert.strictEqual(accepted.payloadBytes, 24, `${test.name} payload byte mismatch`);
	    assert.strictEqual(accepted.payloadSealed, true, `${test.name} payload must be sealed`);
	    assert.strictEqual(accepted.transport, "identity-memory-relay-v0", `${test.name} transport mismatch`);
	    const service = result.relay.hiddenServices[0];
	    assert.strictEqual(service.serviceIdentity, "ccccccccbbbbbbbb", `${test.name} hidden service identity mismatch`);
	    assert.strictEqual(service.registered, true, `${test.name} hidden service registration mismatch`);
	    assert.strictEqual(service.rawListenPort, null, `${test.name} hidden service must not expose raw listen port`);
	    assert.strictEqual(service.hsdirFetchRequestBytes, 71, `${test.name} hsdir fetch request length mismatch`);
	    assert.strictEqual(service.hsdirPublishHeaderBytes, 75, `${test.name} hsdir publish header length mismatch`);
	    assert.strictEqual(service.descriptorArmorBytes, 286, `${test.name} descriptor armor length mismatch`);
	    assert.strictEqual(service.contactFrameBytes, 54, `${test.name} contact frame length mismatch`);
	    assert.strictEqual(service.messageFrameBytes, 80, `${test.name} message frame length mismatch`);
	    assert.strictEqual(service.messageFrameAccepted, true, `${test.name} WAT hidden-service frame handler rejected message`);
	    assert.strictEqual(service.watContactCount, 1, `${test.name} WAT contact count mismatch`);
	    assert.strictEqual(service.watMessageCount, 1, `${test.name} WAT message count mismatch`);
	    assert.strictEqual(service.watContactStateBytes, 68, `${test.name} WAT contact state size mismatch`);
	    assert.strictEqual(service.watMessageStateBytes, 420, `${test.name} WAT message state size mismatch`);
	    for (const field of [
	      "hsdirFetchRequestHash",
	      "hsdirPublishHeaderHash",
	      "descriptorArmorHash",
	      "contactFrameHash",
	      "messageFrameHash",
	      "watContactStateHash",
	      "watMessageStateHash",
	    ]) {
	      assert.match(service[field], /^[0-9a-f]{64}$/, `${test.name} missing ${field}`);
	    }
	    assert.throws(
	      () =>
	        relayCommittedRoutes({
	          app: result.app,
	          commit: {
	            ...result.commit,
	            eventLog: result.commit.eventLog.map((event) =>
	              event.routeQueuedBytes === 0
	                ? event
	                : {
	                    ...event,
	                    intents: event.intents.map((record) =>
	                      record.kind === appAbiV0.intentKinds.routeSendIdentity
	                        ? { ...record, aux0: 0 }
	                        : record,
	                    ),
	                  },
	            ),
	          },
	        }),
	      `${test.name} unsealed route unexpectedly relayed`,
	    );
	    assert.throws(
	      () =>
	        relayCommittedRoutes({
	          app: result.app,
	          commit: {
	            ...result.commit,
	            eventLog: result.commit.eventLog.map((event) =>
	              event.routeQueuedBytes === 0
	                ? event
	                : {
	                    ...event,
	                    intents: event.intents.map((record) =>
	                      record.kind === appAbiV0.intentKinds.routeSendIdentity
	                        ? { ...record, amount: 16 }
	                        : record,
	                    ),
	                  },
	            ),
	          },
	        }),
	      `${test.name} route byte mismatch unexpectedly relayed`,
	    );
	    assert.throws(
	      () =>
	        assertHiddenServiceRecord({
	          record: {
	            ...result.app.transitions[1].transition.intentRecords[2],
	            amount: 9050,
	          },
	        }),
	      `${test.name} hidden service raw port material unexpectedly passed`,
	    );
	    localIdentityRelayResults.push(test.name);
	  }

	  const localTorCircuitResults = [];
	  for (const test of localTorCircuitTests) {
	    const result = simulateWatAppTorCircuit({ watPath: test.wat });
	    assert.strictEqual(result.tor.delivered, true, `${test.name} Tor circuit did not deliver`);
	    assert.strictEqual(result.tor.payloadSealed, true, `${test.name} Tor payload must remain sealed`);
	    assert.strictEqual(result.tor.routeTargetIdentity, "8888888877777777", `${test.name} route target identity mismatch`);
	    assert.strictEqual(result.tor.hiddenServiceIdentity, "ccccccccbbbbbbbb", `${test.name} hidden service identity mismatch`);
	    assert.strictEqual(result.tor.circuit.rawSocket, false, `${test.name} Tor circuit must not use raw socket`);
	    assert.strictEqual(result.tor.circuit.dnsLookup, false, `${test.name} Tor circuit must not use DNS`);
	    assert.strictEqual(result.tor.circuit.rawListenPort, null, `${test.name} Tor circuit must not expose raw listen port`);
	    assert.deepStrictEqual(
	      result.tor.receipts.map((receipt) => receipt.kind),
	      ["guard_accepted", "middle_transit", "rendezvous_established", "hidden_service_delivered"],
	      `${test.name} Tor receipt kinds mismatch`,
	    );
	    assert.strictEqual(result.tor.cells.length, 7, `${test.name} Tor cell count mismatch`);
	    assert.deepStrictEqual(
	      result.tor.cells.map((cell) => [cell.command, cell.relayCommand]),
	      [
	        [torCommands.CREATE2, 0],
	        [torCommands.CREATED2, 0],
	        [torCommands.RELAY, relayCommands.EXTEND2],
	        [torCommands.RELAY, relayCommands.EXTENDED2],
	        [torCommands.RELAY, relayCommands.BEGIN],
	        [torCommands.RELAY, relayCommands.DATA],
	        [torCommands.RELAY, relayCommands.END],
	      ],
	      `${test.name} Tor cell command sequence mismatch`,
	    );
	    assert.strictEqual(result.tor.cells[2].length, 119, `${test.name} RELAY_EXTEND2 payload length mismatch`);
	    assert.strictEqual(result.tor.cells[5].length, 24, `${test.name} RELAY_DATA payload length mismatch`);
	    assert(result.tor.cells.every((cell) => cell.appId === result.app.appId), `${test.name} Tor cells must bind app id`);
	    assert(result.tor.cells.every((cell) => cell.sourceEventHash === result.relay.accepted[0].sourceEventHash), `${test.name} Tor cells must bind source event`);
	    assert(result.tor.cells.every((cell) => cell.plaintextPrivateBytes === 0), `${test.name} Tor cells must not expose plaintext private bytes`);
	    assert(result.tor.cells.every((cell) => cell.canonicalRecordBytes === 304), `${test.name} Tor cells must carry WAT-canonical record lengths`);
	    assert(result.tor.cells.every((cell) => /^[0-9a-f]{64}$/.test(cell.relayBodyHash)), `${test.name} Tor cells must bind relay body hashes`);
	    assert(result.tor.cells.every((cell) => !Object.hasOwn(cell, "plaintextPayload")), `${test.name} Tor cells must not carry plaintext payload`);
	    assert(result.tor.receipts.every((receipt) => result.tor.cells.some((cell) => cell.cellHash === receipt.cellHash)), `${test.name} Tor receipts must bind cell hashes`);
	    assert.match(result.tor.deliveryProof.proofHash, /^[0-9a-f]{64}$/, `${test.name} Tor delivery proof hash missing`);
	    assert.match(result.tor.deliveryProof.receipt, /^[0-9a-f]{64}$/, `${test.name} Tor delivery proof receipt missing`);
	    assert.strictEqual(result.tor.deliveryProof.canonicalRecordBytes, 728, `${test.name} Tor delivery proof record length mismatch`);
	    assert.strictEqual(
	      result.tor.deliveryProof.watMessageStateHash,
	      result.relay.hiddenServices[0].watMessageStateHash,
	      `${test.name} Tor delivery proof must bind WAT message state`,
	    );
	    assert.strictEqual(
	      result.tor.deliveryProof.dataCellHash,
	      result.tor.cells[5].cellHash,
	      `${test.name} Tor delivery proof must bind RELAY_DATA cell`,
	    );
	    assert.doesNotThrow(
	      () =>
	        assertTorCells({
	          app: result.app,
	          commit: result.commit,
	          route: result.relay.accepted[0],
	          cells: result.tor.cells,
	        }),
	      `${test.name} Tor cell validation failed`,
	    );
	    assert.strictEqual(
	      buildTorCellRecord(result.tor.cells[5]).length,
	      result.tor.cells[5].canonicalRecordBytes,
	      `${test.name} RELAY_DATA cell must be WAT-canonicalized`,
	    );
	    assert.doesNotThrow(
	      () =>
	        assertTorDeliveryProof({
	          app: result.app,
	          commit: result.commit,
	          relay: result.relay,
	          tor: result.tor,
	        }),
	      `${test.name} Tor delivery proof validation failed`,
	    );
	    assert.strictEqual(
	      buildTorDeliveryProofRecord(result.tor.deliveryProof).length,
	      result.tor.deliveryProof.canonicalRecordBytes,
	      `${test.name} Tor delivery proof must be WAT-canonicalized`,
	    );
	    assert.throws(
	      () =>
	        assertTorDeliveryProof({
	          app: result.app,
	          commit: result.commit,
	          relay: {
	            ...result.relay,
	            hiddenServices: result.relay.hiddenServices.map((service) => ({
	              ...service,
	              watMessageStateHash: "0".repeat(64),
	            })),
	          },
	          tor: result.tor,
	        }),
	      `${test.name} tampered WAT message state unexpectedly matched delivery proof`,
	    );
	    assert.throws(
	      () =>
	        assertTorDeliveryProof({
	          app: result.app,
	          commit: result.commit,
	          relay: result.relay,
	          tor: {
	            ...result.tor,
	            cells: result.tor.cells.map((cell, index) =>
	              index === 5 ? { ...cell, cellHash: "0".repeat(64) } : cell,
	            ),
	          },
	        }),
	      `${test.name} tampered Tor cell unexpectedly matched delivery proof`,
	    );
	    assert.throws(
	      () =>
	        buildLocalTorCircuit({
	          app: result.app,
	          commit: result.commit,
	          relay: {
	            ...result.relay,
	            hiddenServiceCount: 0,
	            hiddenServices: [],
	            receipts: result.relay.receipts.filter((receipt) => receipt.kind !== "hidden_service"),
	          },
	        }),
	      `${test.name} missing hidden service unexpectedly delivered`,
	    );
	    assert.throws(
	      () =>
	        buildLocalTorCircuit({
	          app: result.app,
	          commit: result.commit,
	          relay: {
	            ...result.relay,
	            accepted: result.relay.accepted.map((route) => ({ ...route, payloadSealed: false })),
	          },
	        }),
	      `${test.name} unsealed payload unexpectedly delivered`,
	    );
	    assert.throws(
	      () =>
	        assertTorCells({
	          app: result.app,
	          commit: result.commit,
	          route: result.relay.accepted[0],
	          cells: result.tor.cells.map((cell, index) =>
	            index === 5 ? { ...cell, length: 16 } : cell,
	          ),
	        }),
	      `${test.name} malformed RELAY_DATA length unexpectedly passed`,
	    );
	    localTorCircuitResults.push(test.name);
	  }

	  const transitionIdentityRejectResults = [];
  for (const test of transitionIdentityRejectTests) {
    const wasm = path.join(outDir, `${test.name}.wasm`);
    compileWat(test.appWat, wasm);
    const manifest = policy.manifestPolicy(wasm);
    policy.assertAppAbi(wasm, test.name, manifest);
    const canonicalAppId = Buffer.from(policy.wasmHash(wasm), "hex");
    let claimedAppId = test.claimedAppId;
    if (test.claimedAppWat) {
      const claimedWasm = path.join(outDir, `${test.name}-claimed.wasm`);
      compileWat(test.claimedAppWat, claimedWasm);
      claimedAppId = Buffer.from(policy.wasmHash(claimedWasm), "hex");
    }
    assert.throws(
      () => policy.assertTransitionIdentityPolicy(test.name, canonicalAppId, claimedAppId),
      `${test.name} unexpectedly passed transition identity policy`,
    );
    transitionIdentityRejectResults.push(test.name);
  }

  const transitionPolicyRejectResults = [];
  for (const test of transitionPolicyRejectTests) {
    const wasm = path.join(outDir, `${test.name}.wasm`);
    compileWat(test.wat, wasm);
    const manifest = policy.manifestPolicy(wasm);
    policy.assertAppAbi(wasm, test.name, manifest);
    assert.throws(
      () => policy.assertTransitionPolicy(test.name, test.transition, manifest),
      `${test.name} unexpectedly passed transition policy`,
    );
    transitionPolicyRejectResults.push(test.name);
  }

  const renderPolicyRejectResults = [];
  for (const test of renderPolicyRejectTests) {
    const wasm = path.join(outDir, `${test.name}.wasm`);
    compileWat(test.wat, wasm);
    const manifest = policy.manifestPolicy(wasm);
    policy.assertAppAbi(wasm, test.name, manifest);
    assert.throws(
      () => policy.assertRenderPolicy(test.name, test.render, manifest),
      `${test.name} unexpectedly passed render policy`,
    );
    renderPolicyRejectResults.push(test.name);
  }

  const watPolicyRejectResults = [];
  for (const test of watPolicyRejectTests) {
    const wasm = path.join(outDir, `${test.name}.wasm`);
    run("wat2wasm", ["--enable-annotations", test.wat, "-o", wasm]);
    assert.throws(() => policy.manifestPolicy(wasm), `${test.name} unexpectedly passed manifest policy`);
    watPolicyRejectResults.push(`${test.name}:${test.reason}`);
  }

  const watAbiPolicyRejectResults = [];
  for (const test of watAbiPolicyRejectTests) {
    const wasm = path.join(outDir, `${test.name}.wasm`);
    run("wat2wasm", ["--enable-annotations", test.wat, "-o", wasm]);
    policy.assertManifest(wasm, test.name);
    assert.throws(() => policy.assertAppAbi(wasm, test.name), `${test.name} unexpectedly passed ABI policy`);
    watAbiPolicyRejectResults.push(`${test.name}:${test.reason}`);
  }

  console.log(
    JSON.stringify({
      unit: "edgerun-x86-wasm-runtime",
      ok: true,
      tests: results,
      wat_tests: watResults,
      wat_arg_tests: watArgResults,
	      wat_identity_tests: watIdentityResults,
	      transition_log_tests: transitionLogResults,
	      local_memory_sim_tests: localMemorySimResults,
	      local_kernel_commit_tests: localKernelCommitResults,
	      local_identity_relay_tests: localIdentityRelayResults,
	      local_tor_circuit_tests: localTorCircuitResults,
	      transition_identity_reject_tests: transitionIdentityRejectResults,
      transition_policy_reject_tests: transitionPolicyRejectResults,
      render_policy_reject_tests: renderPolicyRejectResults,
      wat_run_error_tests: watRunErrorResults,
      wat_policy_reject_tests: watPolicyRejectResults,
      wat_abi_policy_reject_tests: watAbiPolicyRejectResults,
    }),
  );
} finally {
  fs.rmSync(outDir, { recursive: true, force: true });
}
