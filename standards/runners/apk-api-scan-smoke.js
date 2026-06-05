#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/apk-api-scan.wat";

const OK = 0;
const NO_FINDING = 2;
const INVALID = 3;

const KIND = {
  PACKAGE: 1,
  PERMISSION: 2,
  PLATFORM_API: 3,
  IMPORT_API: 4,
  URI: 5,
  NETWORK_WEB: 6,
  CRYPTO: 7,
  LOCATION: 8,
  CAMERA: 9,
  BLUETOOTH: 10,
  SMS: 11,
  CONTACTS_CALENDAR: 12,
  DYNAMIC_PROCESS: 13,
  JAVASCRIPT_BRIDGE: 14,
  ENDPOINT_STRING: 15,
};

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function put(memory, ptr, value) {
  const bytes = Buffer.from(value, "utf8");
  memory.fill(0, ptr, ptr + bytes.length + 128);
  memory.set(bytes, ptr);
  return bytes.length;
}

function packedStatus(value) {
  return Number(value & 0xffffffffn);
}

function packedCount(value) {
  return Number(value >> 32n);
}

(async () => {
  const { instance } = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const view = new DataView(e.memory.buffer);
  const srcPtr = 1024;
  const outPtr = 50000;

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300091);

  const src = [
    "// import android.location.Location; android.permission.CAMERA",
    "/* Landroid/bluetooth/BluetoothAdapter; */",
    '<manifest package="com.example.app">',
    '<uses-permission android:name="android.permission.INTERNET"/>',
    "import android.net.Uri;",
    "import javax.crypto.Cipher;",
    'const fakeApi = "Landroid/location/LocationManager;";',
    'const endpoint = "https://api.example.test/v1/login?token=x";',
    "invoke-static {}, Lokhttp3/OkHttpClient;->newCall()V",
    "invoke-static {}, Ljavax/crypto/Cipher;->getInstance()V",
    "invoke-static {}, Landroid/hardware/camera2/CameraManager;->openCamera()V",
    "invoke-static {}, Landroid/bluetooth/BluetoothAdapter;->getDefaultAdapter()V",
    "invoke-static {}, Landroid/telephony/SmsManager;->sendTextMessage()V",
    "invoke-static {}, Landroid/provider/ContactsContract;->AUTHORITY()V",
    "invoke-static {}, Ldalvik/system/DexClassLoader;-><init>()V",
    "webView.addJavascriptInterface(bridge, \"EdgeRun\");",
  ].join("\n");

  const len = put(memory, srcPtr, src);
  const packed = e.apk_api_scan_count(srcPtr, len);
  assert.equal(packedStatus(packed), OK);
  assert.equal(packedCount(packed), 22);

  const findings = [];
  let pos = 0;
  for (;;) {
    const status = e.apk_api_scan_next(srcPtr, len, pos, outPtr);
    if (status === NO_FINDING) break;
    assert.equal(status, OK);
    const kind = view.getUint32(outPtr, true);
    const start = view.getUint32(outPtr + 4, true);
    const end = view.getUint32(outPtr + 8, true);
    const flags = view.getUint32(outPtr + 12, true);
    assert.equal(view.getUint32(outPtr + 16, true), OK);
    findings.push({ kind, start, end, flags, text: src.slice(start, end) });
    pos = end > pos ? end : pos + 1;
  }

  assert.deepEqual(
    findings.map((finding) => finding.kind),
    [
      KIND.PACKAGE,
      KIND.PERMISSION,
      KIND.PERMISSION,
      KIND.IMPORT_API,
      KIND.IMPORT_API,
      KIND.CRYPTO,
      KIND.URI,
      KIND.ENDPOINT_STRING,
      KIND.ENDPOINT_STRING,
      KIND.NETWORK_WEB,
      KIND.NETWORK_WEB,
      KIND.CRYPTO,
      KIND.CRYPTO,
      KIND.CAMERA,
      KIND.CAMERA,
      KIND.CAMERA,
      KIND.BLUETOOTH,
      KIND.BLUETOOTH,
      KIND.SMS,
      KIND.CONTACTS_CALENDAR,
      KIND.DYNAMIC_PROCESS,
      KIND.JAVASCRIPT_BRIDGE,
    ],
  );

  assert.equal(findings[0].text, "package=");
  assert.equal(findings[1].text, "uses-permission");
  assert.equal(findings[2].text, "android.permission.");
  assert.equal(findings[6].text, "https://");
  assert.equal(findings[6].flags & 1, 1, "URI was scanned inside quoted endpoint string");
  assert.equal(findings[6].flags & 4, 4, "URI has network flag");
  assert.equal(findings[7].text, "login");
  assert.equal(findings[8].text, "token");
  assert.equal(findings[9].text, "Lokhttp3/");
  assert.equal(findings[11].text, "Ljavax/crypto/");
  assert.equal(findings[11].flags & 8, 8, "crypto has sensitive capability flag");
  assert.equal(findings[21].text, "addJavascriptInterface");

  const commentOnly = put(
    memory,
    srcPtr,
    "// android.permission.ACCESS_FINE_LOCATION\n/* Lokhttp3/OkHttpClient */\n",
  );
  assert.equal(e.apk_api_scan_next(srcPtr, commentOnly, 0, outPtr), NO_FINDING);
  const emptyPacked = e.apk_api_scan_count(srcPtr, commentOnly);
  assert.equal(packedStatus(emptyPacked), OK);
  assert.equal(packedCount(emptyPacked), 0);

  assert.equal(e.apk_api_scan_next(srcPtr, len, len + 1, outPtr), INVALID);

  console.log(
    JSON.stringify({
      unit: "apk-api-scan",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      finding_count: findings.length,
      cases: [
        "manifest_package",
        "permission",
        "imports",
        "comments_ignored",
        "quoted_uri_and_endpoint_strings",
        "network_api",
        "crypto_security",
        "capability_categories",
        "invalid_start",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
