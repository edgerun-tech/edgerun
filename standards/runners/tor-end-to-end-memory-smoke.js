#!/usr/bin/env node

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const tmp = [];

function watPath(name) {
  return path.join(root, `standards/build/wasm/app-primitives/${name}/${name}.wat`);
}

function compile(name) {
  const wasm = path.join(os.tmpdir(), `${name}-e2e-${process.pid}.wasm`);
  execFileSync("wat2wasm", [watPath(name), "-o", wasm], { stdio: "inherit" });
  execFileSync("wasm-validate", [wasm], { stdio: "inherit" });
  tmp.push(wasm);
  return fs.readFileSync(wasm);
}

function put(mem, ptr, bytes) { mem.set(bytes, ptr); }
function take(mem, ptr, len) { return Buffer.from(mem.slice(ptr, ptr + len)); }
function u32(mem, ptr) { return new DataView(mem.buffer).getUint32(ptr, true); }
function putU16BE(mem, ptr, v) { mem[ptr] = (v >>> 8) & 0xff; mem[ptr + 1] = v & 0xff; }
function putU32LE(mem, ptr, v) { new DataView(mem.buffer).setUint32(ptr, v >>> 0, true); }

async function instantiateSimple(name) {
  const { instance } = await WebAssembly.instantiate(compile(name), {});
  return instance;
}

(async () => {
  const modules = {
    cell: compile("tor-cell-codec"),
    da: compile("tor-directory-authority"),
    channel: compile("tor-channel-handshake"),
    hs: compile("tor-hidden-service-full"),
    identitySeal: compile("tor-identity-seal"),
    ntor: compile("tor-ntor-crypto"),
    relayCrypto: compile("tor-relay-crypto"),
    relayFull: compile("tor-relay-full"),
  };

  const cell = (await WebAssembly.instantiate(modules.cell, {})).instance.exports;
  const cellMem = new Uint8Array(cell.memory.buffer);
  const da = (await WebAssembly.instantiate(modules.da, {})).instance.exports;
  const daMem = new Uint8Array(da.memory.buffer);
  const channel = (await WebAssembly.instantiate(modules.channel, {})).instance.exports;
  const channelMem = new Uint8Array(channel.memory.buffer);
  const hs = (await WebAssembly.instantiate(modules.hs, {})).instance.exports;
  const hsMem = new Uint8Array(hs.memory.buffer);
  const seal = (await WebAssembly.instantiate(modules.identitySeal, {})).instance.exports;
  const sealMem = new Uint8Array(seal.memory.buffer);
  const ntor = (await WebAssembly.instantiate(modules.ntor, {})).instance.exports;
  const ntorMem = new Uint8Array(ntor.memory.buffer);
  const relayCrypto = (await WebAssembly.instantiate(modules.relayCrypto, {})).instance.exports;
  const relayCryptoMem = new Uint8Array(relayCrypto.memory.buffer);

  let relayMem;
  const hostFabric = {
    connects: [],
    cells: [],
    appDns: 0,
    appIp: 0,
    appPorts: 0,
  };

  const relayImports = {
    "tor.crypto": {
      tor_relay_digest_update20_sha1(statePtr, payloadPtr, len, outPtr) {
        const state = 1024;
        const payload = 2048;
        const out = 4096;
        relayCryptoMem.set(relayMem.slice(statePtr, statePtr + 20), state);
        relayCryptoMem.set(relayMem.slice(payloadPtr, payloadPtr + len), payload);
        const rc = relayCrypto.tor_relay_digest_update20_sha1(state, payload, len, out);
        if (rc !== 0) return rc;
        relayMem.set(relayCryptoMem.slice(out, out + 20), outPtr);
        return 0;
      },
      tor_aes_ctr_crypt(keyPtr, ivPtr, inputPtr, len, outputPtr) {
        const key = 4096;
        const iv = 4128;
        const input = 4160;
        const output = 8192;
        relayCryptoMem.set(relayMem.slice(keyPtr, keyPtr + 16), key);
        relayCryptoMem.set(relayMem.slice(ivPtr, ivPtr + 16), iv);
        relayCryptoMem.set(relayMem.slice(inputPtr, inputPtr + len), input);
        const rc = relayCrypto.tor_relay_aes128_ctr_crypt(key, iv, input, len, output);
        if (rc !== 0) return rc;
        relayMem.set(relayCryptoMem.slice(output, output + len), outputPtr);
        relayMem.set(relayCryptoMem.slice(iv, iv + 16), ivPtr);
        return 0;
      },
      tor_ntor_server_handshake_seeded(handshakePtr, nodePtr, onionPublicPtr, onionSecretPtr, ySecretPtr, replyPtr, keyMaterialPtr) {
        const handshake = 1024;
        const node = 1120;
        const onionPublic = 1160;
        const onionSecret = 1200;
        const ySecret = 1240;
        const reply = 1300;
        const keys = 1400;
        ntorMem.set(relayMem.slice(handshakePtr, handshakePtr + 84), handshake);
        ntorMem.set(relayMem.slice(nodePtr, nodePtr + 20), node);
        ntorMem.set(relayMem.slice(onionPublicPtr, onionPublicPtr + 32), onionPublic);
        ntorMem.set(relayMem.slice(onionSecretPtr, onionSecretPtr + 32), onionSecret);
        ntorMem.set(relayMem.slice(ySecretPtr, ySecretPtr + 32), ySecret);
        const rc = ntor.tor_ntor_server_handshake_seeded(handshake, node, onionPublic, onionSecret, ySecret, reply, keys);
        if (rc !== 0) return rc;
        relayMem.set(ntorMem.slice(reply, reply + 64), replyPtr);
        relayMem.set(ntorMem.slice(keys, keys + 92), keyMaterialPtr);
        return 0;
      },
      tor_ntor_v3_server_handshake_seeded() {
        return -47;
      },
    },
    "edgerun.io": {
      tor_relay_connect(nextHop, port, nodePtr, handshakePtr) {
        hostFabric.connects.push({
          nextHop,
          port,
          node: Buffer.from(relayMem.slice(nodePtr, nodePtr + 20)),
          handshake: Buffer.from(relayMem.slice(handshakePtr, handshakePtr + 84)),
        });
        return 7000 + hostFabric.connects.length;
      },
      tor_relay_send_cell(conn, cellPtr, len) {
        hostFabric.cells.push({ conn, cell: Buffer.from(relayMem.slice(cellPtr, cellPtr + len)) });
        return 0;
      },
    },
  };
  const relay = (await WebAssembly.instantiate(modules.relayFull, relayImports)).instance.exports;
  relayMem = new Uint8Array(relay.memory.buffer);

  assert.equal(cell.proto_standard_id(), 300224);
  assert.equal(da.proto_standard_id(), 300218);
  assert.equal(channel.proto_standard_id(), 300221);
  assert.equal(hs.proto_standard_id(), 300207);
  assert.equal(seal.proto_standard_id(), 300220);
  assert.equal(ntor.proto_standard_id(), 300206);
  assert.equal(relay.proto_standard_id(), 300205);

  const localVersions = Buffer.from([0, 3, 0, 4, 0, 5]);
  const remoteVersions = Buffer.from([0, 4, 0, 5]);
  put(cellMem, 1000, localVersions);
  put(cellMem, 1020, remoteVersions);
  assert.equal(cell.tor_cell_versions_negotiate(1000, localVersions.length, 1020, remoteVersions.length), 5);
  assert.equal(cell.tor_cell_build_var(4, 1100, 0, 7, 1000, localVersions.length), 13);
  assert.equal(cell.tor_cell_parse_var(4, 1100, 13, 1200), 0);
  assert.deepEqual([u32(cellMem, 1200), u32(cellMem, 1204), u32(cellMem, 1208), u32(cellMem, 1212)], [0, 7, 7, 6]);

  const relaySeed = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x20 + i));
  put(channelMem, 1000, relaySeed);
  assert.equal(channel.tor_channel_ed25519_public(1000, 1040), 0);
  const relayEdId = take(channelMem, 1040, 32);

  assert.equal(da.tor_da_init(), 0);
  const guardId = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x10 + i));
  const middleId = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x40 + i));
  const hsDirId = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x80 + i));
  put(daMem, 2000, guardId); put(daMem, 2040, middleId); put(daMem, 2080, hsDirId);
  put(daMem, 3000, Buffer.from("guardrelay"));
  put(daMem, 3040, Buffer.from("middlerelay"));
  put(daMem, 3080, Buffer.from("hsdirrelay"));
  assert.equal(da.tor_da_accept_descriptor(2000, 3000, 10, 4096, 240, 100, 9001, 9030, 0x01, 0xaaa1, 1000, 0), 0);
  assert.equal(da.tor_da_accept_descriptor(2040, 3040, 11, 2048, 120, 101, 9002, 0, 0x01, 0xaaa2, 1000, 0), 0);
  assert.equal(da.tor_da_accept_descriptor(2080, 3080, 10, 2048, 120, 102, 9003, 9031, 0x01, 0xaaa3, 1000, 0), 0);
  assert.equal(da.tor_da_build_consensus(120, 180, 240), 3);
  assert.equal(da.tor_da_route_lookup(2000, 4000), 0);
  assert.equal(u32(daMem, 4000), 0xaaa1);

  const node = 5000;
  const onionSecret = 5040;
  const onionPublic = 5080;
  const ySecret = 5120;
  const clientSecret = 5160;
  const clientPublic = 5200;
  const clientHandshake = 5300;
  const createCell = 5400;
  const createdCell = 6000;
  const clientKeys = 6700;
  put(ntorMem, node, guardId.subarray(0, 20));
  for (let i = 0; i < 32; i += 1) {
    ntorMem[onionSecret + i] = 0x11 + i;
    ntorMem[ySecret + i] = 0x41 + i;
    ntorMem[clientSecret + i] = 0x81 + i;
  }
  assert.equal(ntor.tor_x25519_public(onionSecret, onionPublic), 0);
  assert.equal(ntor.tor_ntor_client_handshake_seeded(node, onionPublic, clientSecret, clientHandshake, clientPublic), 0);
  put(cellMem, 1000, take(ntorMem, clientHandshake, 84));
  assert.equal(cell.tor_cell_build_create2_body(1300, 2, 1000, 84), 88);
  assert.equal(cell.tor_cell_build_fixed(4, 2000, 0x1001, 10, 1300, 88), 514);
  relayMem.set(cellMem.slice(2000, 2514), createCell);
  putU32LE(relayMem, createCell, 0x1001);
  relayMem.set(ntorMem.slice(node, node + 20), 7000);
  relayMem.set(ntorMem.slice(onionPublic, onionPublic + 32), 7040);
  relayMem.set(ntorMem.slice(onionSecret, onionSecret + 32), 7080);
  relayMem.set(ntorMem.slice(ySecret, ySecret + 32), 7120);
  assert.equal(relay.er_tor_relay_init(), 0);
  assert.equal(relay.er_tor_relay_handle_create2(createCell, 514, 7000, 7040, 7080, 7120, createdCell), 0);
  assert.equal(relayMem[createdCell + 4], 11);
  ntorMem.set(relayMem.slice(createdCell + 9, createdCell + 73), 7200);
  assert.equal(ntor.tor_ntor_client_process(7200, node, onionPublic, clientSecret, clientPublic, clientKeys), 0);
  assert.notEqual(take(ntorMem, clientKeys, 92).toString("hex"), "00".repeat(92));

  assert.equal(hs.tor_hs_full_init(), 0);
  for (let i = 0; i < 32; i += 1) {
    hsMem[4000 + i] = relayEdId[i];
    hsMem[4040 + i] = ntorMem[onionPublic + i];
  }
  put(hsMem, 4080, Buffer.from([2, 20, ...Array.from({ length: 20 }, (_, i) => 0x30 + i)]));
  assert.equal(hs.tor_hs_register_intro_point(0x1001, 4000, 4040, 4080, 22), 0);
  assert.equal(hs.tor_hs_intro_count(), 1);
  assert.equal(hs.tor_hs_build_descriptor_record(5000, 9, 180, 1), 16);
  const onion = "pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd.onion";
  put(hsMem, 5200, Buffer.from(onion));
  assert.equal(hs.tor_hs_validate_onion_address(5200, onion.length), 0);
  for (let i = 0; i < 20; i += 1) hsMem[5400 + i] = 0x55 + i;
  assert.equal(hs.tor_hs_build_establish_rendezvous(5500, 5400), 20);
  assert.equal(hs.tor_hs_parse_establish_rendezvous(5500, 20, 5600), 0);

  const senderId = Buffer.from(Array.from({ length: 32 }, (_, i) => 0xa0 + i));
  const recipientSecret = Buffer.from(Array.from({ length: 32 }, (_, i) => 0xb0 + i));
  const ephSecret = Buffer.from(Array.from({ length: 32 }, (_, i) => 0xc0 + i));
  const iv = Buffer.from(Array.from({ length: 16 }, (_, i) => 0xd0 + i));
  const appPayload = Buffer.from("app ipc over memory host tor relay hidden service", "utf8");
  put(sealMem, 1000, senderId);
  put(sealMem, 1040, recipientSecret);
  put(sealMem, 1080, ephSecret);
  put(sealMem, 1120, iv);
  put(sealMem, 1200, appPayload);
  assert.equal(seal.tor_identity_seal_public(1040, 1160), 0);
  const sealedLen = seal.tor_identity_seal(1000, 1160, 1080, 1120, 1200, appPayload.length, 2000);
  assert.equal(sealedLen, appPayload.length + seal.tor_identity_seal_overhead());
  assert.notDeepEqual(take(sealMem, 2000 + 48, appPayload.length), appPayload);

  relayMem.set(sealMem.slice(2000, 2000 + sealedLen), 9000);
  assert.equal(relay.er_tor_relay_alloc_circuit(0x1001, 0, 0, 7), 0);
  assert.equal(relay.er_tor_relay_build_relay_cell(10000, 0x1001, 2, 77, 9000, sealedLen), 0);
  hostFabric.cells.push({ conn: "memory-hidden-service", cell: take(relayMem, 10000, 514) });
  assert.equal(hostFabric.cells.length, 1);

  const delivered = hostFabric.cells.shift().cell;
  assert.equal(delivered[4], 3);
  const relayPayload = delivered.subarray(5, 514);
  put(cellMem, 3000, relayPayload);
  assert.equal(cell.tor_cell_parse_relay_payload(3000, 509, 3600), 0);
  assert.deepEqual([u32(cellMem, 3600), u32(cellMem, 3604), u32(cellMem, 3608), u32(cellMem, 3616)], [2, 0, 77, sealedLen]);
  sealMem.set(relayPayload.subarray(11, 11 + sealedLen), 4000);
  const openedLen = seal.tor_identity_open(1000, 1040, 1160, 4000, sealedLen, 6000);
  assert.equal(openedLen, appPayload.length);
  assert.deepEqual(take(sealMem, 6000, openedLen), appPayload);

  assert.equal(hostFabric.appDns, 0);
  assert.equal(hostFabric.appIp, 0);
  assert.equal(hostFabric.appPorts, 0);

  console.log("tor end-to-end memory smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
}).finally(() => {
  for (const f of tmp) {
    try { fs.unlinkSync(f); } catch {}
  }
});
