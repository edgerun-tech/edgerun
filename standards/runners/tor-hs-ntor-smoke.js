#!/usr/bin/env node

const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/tor-hs-ntor/tor-hs-ntor.wat");
const wasm = path.join(os.tmpdir(), `tor-hs-ntor-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });
execFileSync("wasm-validate", [wasm], { stdio: "inherit" });

function hex(s) {
  return Buffer.from(s.replace(/\s+/g, ""), "hex");
}

function put(mem, ptr, bytes) {
  mem.set(bytes, ptr);
}

function take(mem, ptr, len) {
  return Buffer.from(mem.slice(ptr, ptr + len));
}

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_standard_id(), 300214);
  assert.equal(e.proto_abi_version(), 1);
  assert.equal(e.tor_hs_ntor_intro_prefix_len(), 56);
  assert.equal(e.tor_hs_ntor_intro_plaintext_base_len(), 57);
  assert.equal(e.tor_hs_ntor_max_body_len(), 490);

  const authKey = hex("34E171E4358E501BFF21ED907E96AC6BFEF697C779D040BBAF49ACC30FC5D21F");
  const servicePub = hex("8E5127A40E83AABF6493E41F142B6EE3604B85A3961CD7E38D247239AFF71979");
  const serviceSecret = hex("A0ED5DBF94EEB2EDB3B514E4CF6ABFF6022051CC5F103391F1970A3FCD15296A");
  const subcred = hex("0085D26A9DEBA252263BF0231AEAC59B17CA11BAD8A218238AD6487CBAD68B57");
  const clientSecret = hex("60B4D6BF5234DCF87A4E9D7487BDF3F4A69B6729835E825CA29089CFDDA1E341");
  const clientPub = hex("BF04348B46D09AED726F1D66C618FDEA1DE58E8CB8B89738D7356A0C59111D5D");
  const encKey = hex("9B8917BA3D05F3130DACCE5300C3DC27F6D012912F1C733036F822D0ED238706");
  const macKey = hex("FC4058DA59D4DF61E7B40985D122F502FD59336BC21C30CAF5E7F0D4A2C38FD5");
  const plain = hex(`
    6BD364C12638DD5C3BE23D76ACA05B04E6CE932C0101000100200DE6130E4FCA
    C4EDDA24E21220CC3EADAE403EF6B7D11C8273AC71908DE565450300067F0000
    0113890214F823C4F8CC085C792E0AEE0283FE00AD7520B37D0320728D5DF39B
    7B7077A0118A900FF4456C382F0041300ACF9C58E51C392795EF870000000000
    0000000000000000000000000000000000000000000000000000000000000000
    000000000000000000000000000000000000000000000000000000000000`);
  const expectedIntro = hex(`
    000000000000000000000000000000000000000002002034E171E4358E501BFF
    21ED907E96AC6BFEF697C779D040BBAF49ACC30FC5D21F00BF04348B46D09AED
    726F1D66C618FDEA1DE58E8CB8B89738D7356A0C59111D5DADBECCCB38E37830
    4DCC179D3D9E437B452AF5702CED2CCFEC085BC02C4C175FA446525C1B9D5530
    563C362FDFFB802DAB8CD9EBC7A5EE17DA62E37DEEB0EB187FBB48C63298B0E8
    3F391B7566F42ADC97C46BA7588278273A44CE96BC68FFDAE31EF5F0913B9A9C
    7E0F173DBC0BDDCD4ACB4C4600980A7DDD9EAEC6E7F3FA3FC37CD95E5B8BFB3E
    35717012B78B4930569F895CB349A07538E42309C993223AEA77EF8AEA64F25D
    DEE97DA623F1AEC0A47F150002150455845C385E5606E41A9A199E7111D54EF2
    D1A51B7554D8B3692D85AC587FB9E69DF990EFB776D8`);
  const ySecret = hex("68CB5188CA0CD7924250404FAB54EE1392D3D2B9C049A2E446513875952F8F55");
  const handshake = hex("8FBE0DB4D4A9C7FF46701E3E0EE7FD05CD28BE4F302460ADDEEC9E93354EE7004A92E8437B8424D5E5EC279245D5C72B25A0327ACF6DAF902079FCB643D8B208");
  const seed = hex("4D0C72FE8AFF35559D95ECC18EB5A36883402B28CDFD48C8A530A5A3D7D578DB");

  put(mem, 1000, authKey);
  put(mem, 1040, servicePub);
  put(mem, 1080, serviceSecret);
  put(mem, 1120, subcred);
  put(mem, 1160, clientSecret);
  put(mem, 2000, plain);
  assert.equal(e.tor_hs_ntor_derive_intro_keys_client(1160, 1000, 1040, 1120, 3000, 3040), 0);
  assert.deepEqual(take(mem, 3000, 32), clientPub);
  assert.deepEqual(take(mem, 3040, 32), encKey);
  assert.deepEqual(take(mem, 3072, 32), macKey);
  assert.equal(e.tor_hs_ntor_derive_intro_keys_service(1080, 1000, 1040, 3000, 1120, 3120), 0);
  assert.deepEqual(take(mem, 3120, 64), Buffer.concat([encKey, macKey]));

  const encryptedLen = e.tor_hs_ntor_build_introduce1_encrypted(4000, 1000, 1040, 1160, 1120, 2000, plain.length);
  assert.equal(encryptedLen, expectedIntro.length - 56);
  const bodyLen = e.tor_hs_ntor_build_introduce1_prefix(5000, 1000, 4000, encryptedLen);
  assert.equal(bodyLen, expectedIntro.length);
  assert.deepEqual(take(mem, 5000, bodyLen), expectedIntro);

  assert.equal(e.tor_hs_ntor_open_introduce2(6000, 6500, 1000, 1080, 1040, 1120, 4056, encryptedLen), -3);
  assert.equal(e.tor_hs_ntor_open_introduce2(6000, 6500, 1000, 1080, 1040, 1120, 4000, encryptedLen), 0);
  assert.equal(new DataView(e.memory.buffer).getUint32(6500, true), plain.length);
  assert.deepEqual(take(mem, 6000, plain.length), plain);
  mem[4000 + encryptedLen - 1] ^= 0xff;
  assert.equal(e.tor_hs_ntor_open_introduce2(6000, 6500, 1000, 1080, 1040, 1120, 4000, encryptedLen), -3);
  mem[4000 + encryptedLen - 1] ^= 0xff;

  put(mem, 7000, ySecret);
  assert.equal(e.tor_hs_ntor_service_rendezvous(4000, encryptedLen, 1000, 1080, 1040, 7000, 7040, 7120), 0);
  assert.deepEqual(take(mem, 7040, 64), handshake);
  assert.deepEqual(take(mem, 7120, 32), seed);
  assert.equal(e.tor_hs_ntor_client_verify_rendezvous(1160, 1000, 1040, 7040, 7200), 0);
  assert.deepEqual(take(mem, 7200, 32), seed);
  mem[7040 + 63] ^= 0xff;
  assert.equal(e.tor_hs_ntor_client_verify_rendezvous(1160, 1000, 1040, 7040, 7200), -3);
  mem[7040 + 63] ^= 0xff;

  const cookie = plain.subarray(0, 20);
  e.tor_hs_ntor_build_rendezvous1(7300, 2000, 7040);
  assert.deepEqual(take(mem, 7300, 84), Buffer.concat([cookie, handshake]));

  const simpleCookie = Buffer.from(Array.from({ length: 20 }, (_, i) => i));
  const onionKey = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x80 + i));
  const links = Buffer.from([0, 6, 0x7f, 0, 0, 1, 0x13, 0x89]);
  put(mem, 8000, simpleCookie);
  put(mem, 8040, onionKey);
  put(mem, 8080, links);
  const simpleLen = e.tor_hs_ntor_build_intro_plain(8200, 8000, 8040, 1, 8080, links.length, 4);
  assert.equal(simpleLen, 57 + links.length + 4);
  assert.equal(e.tor_hs_ntor_parse_intro_plain(8200, simpleLen, 8400, 8440, 8480, 64, 8600), 0);
  assert.deepEqual(take(mem, 8400, 20), simpleCookie);
  assert.deepEqual(take(mem, 8440, 32), onionKey);
  assert.deepEqual(take(mem, 8480, links.length + 4), Buffer.concat([links, Buffer.alloc(4)]));

  console.log("tor hs ntor smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
