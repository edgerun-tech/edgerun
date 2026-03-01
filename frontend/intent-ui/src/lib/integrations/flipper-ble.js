const SERIAL_SERVICE_UUID = "8fe5b3d5-2e7f-4a98-2a48-7acc60fe0000";
const SERIAL_TX_UUID = "19ed82ae-ed21-4c9d-4145-228e61fe0000";
const SERIAL_RX_UUID = "19ed82ae-ed21-4c9d-4145-228e62fe0000";
const SERIAL_FLOW_UUID = "19ed82ae-ed21-4c9d-4145-228e63fe0000";
const SERIAL_RPC_STATUS_UUID = "19ed82ae-ed21-4c9d-4145-228e64fe0000";

const BATTERY_SERVICE = "battery_service";
const BATTERY_LEVEL_CHARACTERISTIC = "battery_level";

const MAX_CHAR_WRITE = 243;
const DEFAULT_FLOW_BUDGET = MAX_CHAR_WRITE;
const FLOW_WAIT_MS = 120;
const RPC_TIMEOUT_MS = 8000;

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

function normalizeUuid(input) {
  return String(input || "").trim().toLowerCase();
}

function toUint8Array(view) {
  if (view instanceof Uint8Array) return view;
  if (view instanceof DataView) {
    return new Uint8Array(view.buffer, view.byteOffset, view.byteLength);
  }
  if (ArrayBuffer.isView(view)) {
    return new Uint8Array(view.buffer, view.byteOffset, view.byteLength);
  }
  if (view instanceof ArrayBuffer) return new Uint8Array(view);
  return new Uint8Array();
}

function concatUint8(chunks) {
  const total = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
  const out = new Uint8Array(total);
  let cursor = 0;
  for (const chunk of chunks) {
    out.set(chunk, cursor);
    cursor += chunk.length;
  }
  return out;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function encodeVarint(value) {
  let n = Number(value || 0);
  if (!Number.isFinite(n) || n < 0) n = 0;
  const out = [];
  while (n >= 0x80) {
    out.push((n & 0x7f) | 0x80);
    n = Math.floor(n / 128);
  }
  out.push(n & 0x7f);
  return Uint8Array.from(out);
}

function decodeVarint(bytes, offset = 0) {
  let value = 0;
  let shift = 0;
  let cursor = offset;
  while (cursor < bytes.length) {
    const byte = bytes[cursor];
    value += (byte & 0x7f) * 2 ** shift;
    cursor += 1;
    if ((byte & 0x80) === 0) {
      return { value, offset: cursor };
    }
    shift += 7;
    if (shift > 56) throw new Error("varint too large");
  }
  throw new Error("incomplete varint");
}

function encodeField(tag, wireType, payload) {
  return concatUint8([encodeVarint((tag << 3) | wireType), payload]);
}

function encodeBytesField(tag, bytes) {
  const payload = toUint8Array(bytes);
  return encodeField(tag, 2, concatUint8([encodeVarint(payload.length), payload]));
}

function encodeStringField(tag, value) {
  return encodeBytesField(tag, textEncoder.encode(String(value || "")));
}

function encodeBoolField(tag, value) {
  return encodeField(tag, 0, Uint8Array.from([value ? 1 : 0]));
}

function encodeUintField(tag, value) {
  return encodeField(tag, 0, encodeVarint(value));
}

function decodeLengthDelimited(bytes, offset) {
  const len = decodeVarint(bytes, offset);
  const end = len.offset + len.value;
  if (end > bytes.length) throw new Error("length-delimited overrun");
  return { bytes: bytes.slice(len.offset, end), offset: end };
}

function encodeDelimited(payload) {
  const bytes = toUint8Array(payload);
  return concatUint8([encodeVarint(bytes.length), bytes]);
}

function tryDecodeDelimitedFromBuffer(buffer) {
  const frames = [];
  let cursor = 0;
  try {
    while (cursor < buffer.length) {
      const size = decodeVarint(buffer, cursor);
      const start = size.offset;
      const end = start + size.value;
      if (end > buffer.length) break;
      frames.push(buffer.slice(start, end));
      cursor = end;
    }
  } catch {
    return { frames: [], remainder: buffer };
  }
  return { frames, remainder: buffer.slice(cursor) };
}

function decodePbSystemPingResponse(bytes) {
  const out = { data: new Uint8Array() };
  let cursor = 0;
  while (cursor < bytes.length) {
    const key = decodeVarint(bytes, cursor);
    cursor = key.offset;
    const tag = key.value >> 3;
    const wireType = key.value & 0x07;
    if (wireType === 2) {
      const field = decodeLengthDelimited(bytes, cursor);
      cursor = field.offset;
      if (tag === 1) out.data = field.bytes;
      continue;
    }
    if (wireType === 0) {
      const field = decodeVarint(bytes, cursor);
      cursor = field.offset;
      continue;
    }
    break;
  }
  return out;
}

function decodePbSystemDeviceInfoResponse(bytes) {
  const out = { key: "", value: "" };
  let cursor = 0;
  while (cursor < bytes.length) {
    const key = decodeVarint(bytes, cursor);
    cursor = key.offset;
    const tag = key.value >> 3;
    const wireType = key.value & 0x07;
    if (wireType === 2) {
      const field = decodeLengthDelimited(bytes, cursor);
      cursor = field.offset;
      const value = textDecoder.decode(field.bytes).trim();
      if (tag === 1) out.key = value;
      if (tag === 2) out.value = value;
      continue;
    }
    if (wireType === 0) {
      const field = decodeVarint(bytes, cursor);
      cursor = field.offset;
      continue;
    }
    break;
  }
  return out;
}

function decodePbMain(bytes) {
  const out = {
    commandId: 0,
    commandStatus: 0,
    hasNext: false,
    contentTag: 0,
    contentBytes: new Uint8Array(),
    pingResponse: null,
    deviceInfoResponse: null
  };

  let cursor = 0;
  while (cursor < bytes.length) {
    const key = decodeVarint(bytes, cursor);
    cursor = key.offset;
    const tag = key.value >> 3;
    const wireType = key.value & 0x07;

    if (wireType === 0) {
      const field = decodeVarint(bytes, cursor);
      cursor = field.offset;
      if (tag === 1) out.commandId = field.value;
      if (tag === 2) out.commandStatus = field.value;
      if (tag === 3) out.hasNext = field.value !== 0;
      continue;
    }

    if (wireType === 2) {
      const field = decodeLengthDelimited(bytes, cursor);
      cursor = field.offset;
      out.contentTag = tag;
      out.contentBytes = field.bytes;
      if (tag === 6) out.pingResponse = decodePbSystemPingResponse(field.bytes);
      if (tag === 33) out.deviceInfoResponse = decodePbSystemDeviceInfoResponse(field.bytes);
      continue;
    }

    break;
  }

  return out;
}

function encodePbSystemPingRequest(dataBytes) {
  return encodeBytesField(1, dataBytes);
}

function encodePbMain(commandId, contentTag, contentBytes) {
  const chunks = [];
  chunks.push(encodeUintField(1, commandId));
  chunks.push(encodeBoolField(3, false));
  chunks.push(encodeField(contentTag, 2, concatUint8([encodeVarint(contentBytes.length), contentBytes])));
  return concatUint8(chunks);
}

function encodePbMainPingRequest(commandId, payload) {
  const body = encodePbSystemPingRequest(payload);
  return encodePbMain(commandId, 5, body);
}

function encodePbMainDeviceInfoRequest(commandId) {
  return encodePbMain(commandId, 32, new Uint8Array());
}

function decodeFlowBudget(dataView) {
  if (!(dataView instanceof DataView) || dataView.byteLength < 4) return DEFAULT_FLOW_BUDGET;
  const be = dataView.getUint32(0, false);
  const le = dataView.getUint32(0, true);
  const candidates = [be, le].filter((value) => Number.isFinite(value) && value > 0 && value <= 1024 * 1024);
  if (candidates.length === 0) return DEFAULT_FLOW_BUDGET;
  return Math.max(...candidates);
}

function requireWebBluetooth() {
  if (typeof window === "undefined" || !window.isSecureContext) {
    throw new Error("Web Bluetooth requires a secure browser context (HTTPS).");
  }
  if (!navigator?.bluetooth?.requestDevice) {
    throw new Error("Web Bluetooth API is unavailable in this browser.");
  }
  return navigator.bluetooth;
}

async function resolveKnownDevice(bluetooth, preferredId = "") {
  const normalizedId = String(preferredId || "").trim();
  if (!normalizedId || typeof bluetooth.getDevices !== "function") return null;
  const devices = await bluetooth.getDevices();
  return devices.find((device) => String(device?.id || "").trim() === normalizedId) || null;
}

async function requestFlipperDevice(bluetooth) {
  return bluetooth.requestDevice({
    acceptAllDevices: true,
    optionalServices: [SERIAL_SERVICE_UUID, BATTERY_SERVICE]
  });
}

async function resolveFlipperDevice(preferredId = "") {
  const bluetooth = requireWebBluetooth();
  const known = await resolveKnownDevice(bluetooth, preferredId);
  return known || requestFlipperDevice(bluetooth);
}

async function ensureGattServer(device) {
  if (!device?.gatt) throw new Error("Selected device does not expose GATT.");
  if (device.gatt.connected) return device.gatt;
  return device.gatt.connect();
}

async function readBatteryLevel(server) {
  try {
    const batteryService = await server.getPrimaryService(BATTERY_SERVICE);
    const batteryCharacteristic = await batteryService.getCharacteristic(BATTERY_LEVEL_CHARACTERISTIC);
    const value = await batteryCharacteristic.readValue();
    return value.getUint8(0);
  } catch {
    return null;
  }
}

async function readPrimaryServiceUuids(server) {
  try {
    if (typeof server.getPrimaryServices !== "function") return [];
    const services = await server.getPrimaryServices();
    return (Array.isArray(services) ? services : [])
      .map((service) => normalizeUuid(service?.uuid))
      .filter(Boolean);
  } catch {
    return [];
  }
}

function readCharacteristicPropertySummary(characteristic) {
  const p = characteristic?.properties || {};
  return {
    read: Boolean(p.read),
    write: Boolean(p.write),
    writeWithoutResponse: Boolean(p.writeWithoutResponse),
    notify: Boolean(p.notify),
    indicate: Boolean(p.indicate)
  };
}

class FlipperSerialSession {
  constructor({ device, server, serialService, txChar, rxChar, flowChar, rpcStatusChar }) {
    this.device = device;
    this.server = server;
    this.serialService = serialService;
    this.txChar = txChar;
    this.rxChar = rxChar;
    this.flowChar = flowChar;
    this.rpcStatusChar = rpcStatusChar;

    this.commandId = 1;
    this.flowBudget = DEFAULT_FLOW_BUDGET;
    this.rxBuffer = new Uint8Array();
    this.mainFrames = [];

    this.onTxNotification = this.onTxNotification.bind(this);
    this.onFlowNotification = this.onFlowNotification.bind(this);
  }

  async start() {
    if (typeof this.txChar.startNotifications === "function") {
      await this.txChar.startNotifications();
      this.txChar.addEventListener("characteristicvaluechanged", this.onTxNotification);
    }
    if (typeof this.flowChar.startNotifications === "function") {
      await this.flowChar.startNotifications();
      this.flowChar.addEventListener("characteristicvaluechanged", this.onFlowNotification);
    }
    if (typeof this.flowChar.readValue === "function") {
      try {
        const value = await this.flowChar.readValue();
        this.flowBudget = decodeFlowBudget(value);
      } catch {
        this.flowBudget = DEFAULT_FLOW_BUDGET;
      }
    }
  }

  async stop() {
    try {
      this.txChar.removeEventListener("characteristicvaluechanged", this.onTxNotification);
      this.flowChar.removeEventListener("characteristicvaluechanged", this.onFlowNotification);
    } catch {
      // best effort
    }
    try {
      if (typeof this.txChar.stopNotifications === "function") {
        await this.txChar.stopNotifications();
      }
    } catch {
      // best effort
    }
    try {
      if (typeof this.flowChar.stopNotifications === "function") {
        await this.flowChar.stopNotifications();
      }
    } catch {
      // best effort
    }
  }

  nextCommandId() {
    this.commandId = (this.commandId % 0x7fffffff) + 1;
    return this.commandId;
  }

  onFlowNotification(event) {
    const value = event?.target?.value;
    if (!(value instanceof DataView)) return;
    const budget = decodeFlowBudget(value);
    if (Number.isFinite(budget) && budget > 0) {
      this.flowBudget = budget;
    }
  }

  onTxNotification(event) {
    const value = event?.target?.value;
    const bytes = toUint8Array(value);
    if (bytes.length === 0) return;
    this.rxBuffer = concatUint8([this.rxBuffer, bytes]);
    const decoded = tryDecodeDelimitedFromBuffer(this.rxBuffer);
    this.rxBuffer = decoded.remainder;
    for (const frame of decoded.frames) {
      try {
        this.mainFrames.push(decodePbMain(frame));
      } catch {
        // Ignore malformed frame and continue stream.
      }
    }
  }

  async writeBytes(bytes) {
    const payload = toUint8Array(bytes);
    let offset = 0;
    while (offset < payload.length) {
      if (this.flowBudget <= 0) {
        await sleep(FLOW_WAIT_MS);
        continue;
      }
      const remaining = payload.length - offset;
      const allowed = Math.max(1, Math.min(this.flowBudget, MAX_CHAR_WRITE, remaining));
      const chunk = payload.slice(offset, offset + allowed);
      if (typeof this.rxChar.writeValueWithoutResponse === "function") {
        await this.rxChar.writeValueWithoutResponse(chunk);
      } else if (typeof this.rxChar.writeValue === "function") {
        await this.rxChar.writeValue(chunk);
      } else {
        throw new Error("Flipper RX characteristic is not writable.");
      }
      offset += chunk.length;
      this.flowBudget = Math.max(0, this.flowBudget - chunk.length);
    }
  }

  async sendMain(payload) {
    const delimited = encodeDelimited(payload);
    await this.writeBytes(delimited);
  }

  async waitForFrame(match, timeoutMs = RPC_TIMEOUT_MS) {
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
      const index = this.mainFrames.findIndex(match);
      if (index >= 0) {
        return this.mainFrames.splice(index, 1)[0];
      }
      await sleep(30);
    }
    return null;
  }

  async ping(payloadText = "edgerun-flipper-ping") {
    const data = textEncoder.encode(payloadText);
    const commandId = this.nextCommandId();
    const request = encodePbMainPingRequest(commandId, data);
    await this.sendMain(request);
    const response = await this.waitForFrame((frame) => frame.commandId === commandId && frame.contentTag === 6, RPC_TIMEOUT_MS);
    if (!response) {
      throw new Error("Timed out waiting for Flipper ping RPC response.");
    }
    if (response.commandStatus !== 0) {
      throw new Error(`Flipper ping RPC failed with status ${response.commandStatus}.`);
    }
    const echoed = response.pingResponse?.data || new Uint8Array();
    const matched = echoed.length === data.length && echoed.every((byte, index) => byte === data[index]);
    if (!matched) {
      throw new Error("Flipper ping RPC returned unexpected payload.");
    }
    return {
      ok: true,
      commandId,
      echoedBytes: echoed.length
    };
  }

  async readDeviceInfo(maxEntries = 16) {
    const commandId = this.nextCommandId();
    const request = encodePbMainDeviceInfoRequest(commandId);
    await this.sendMain(request);

    const entries = [];
    while (entries.length < maxEntries) {
      const frame = await this.waitForFrame((candidate) => candidate.commandId === commandId, RPC_TIMEOUT_MS);
      if (!frame) break;
      if (frame.commandStatus !== 0) {
        throw new Error(`Flipper device info RPC failed with status ${frame.commandStatus}.`);
      }
      if (frame.contentTag === 33 && frame.deviceInfoResponse) {
        const key = String(frame.deviceInfoResponse.key || "").trim();
        const value = String(frame.deviceInfoResponse.value || "").trim();
        if (key) entries.push({ key, value });
      }
      if (!frame.hasNext) break;
    }

    return entries;
  }
}

async function openFlipperSerialSession(details = {}) {
  const device = await resolveFlipperDevice(String(details?.flipperDeviceId || "").trim());
  if (!device) throw new Error("No Flipper device selected.");

  const server = await ensureGattServer(device);
  const serialService = await server.getPrimaryService(SERIAL_SERVICE_UUID);
  if (!serialService) throw new Error("Flipper serial service unavailable.");

  const txChar = await serialService.getCharacteristic(SERIAL_TX_UUID);
  const rxChar = await serialService.getCharacteristic(SERIAL_RX_UUID);
  const flowChar = await serialService.getCharacteristic(SERIAL_FLOW_UUID);
  const rpcStatusChar = await serialService.getCharacteristic(SERIAL_RPC_STATUS_UUID);

  if (!txChar || !rxChar || !flowChar || !rpcStatusChar) {
    throw new Error("Flipper serial service is missing required characteristics.");
  }

  const session = new FlipperSerialSession({
    device,
    server,
    serialService,
    txChar,
    rxChar,
    flowChar,
    rpcStatusChar
  });
  await session.start();

  return {
    device,
    server,
    session,
    characteristics: {
      tx: readCharacteristicPropertySummary(txChar),
      rx: readCharacteristicPropertySummary(rxChar),
      flow: readCharacteristicPropertySummary(flowChar),
      rpcStatus: readCharacteristicPropertySummary(rpcStatusChar)
    }
  };
}

function mapDeviceInfoEntries(entries) {
  const out = {};
  for (const entry of entries) {
    const key = String(entry?.key || "").trim();
    if (!key) continue;
    out[key] = String(entry?.value || "").trim();
  }
  return out;
}

async function verifyFlipperBluetooth(details = {}) {
  const { device, session, characteristics } = await openFlipperSerialSession(details);
  try {
    const ping = await session.ping();
    return {
      deviceId: String(device.id || "").trim(),
      deviceName: String(device.name || details?.flipperDeviceName || "Flipper").trim(),
      ping,
      characteristics
    };
  } finally {
    await session.stop();
  }
}

async function probeFlipper(details = {}) {
  let device = null;
  let server = null;
  let session = null;
  let characteristics = null;
  try {
    const opened = await openFlipperSerialSession(details);
    device = opened.device;
    server = opened.server;
    session = opened.session;
    characteristics = opened.characteristics;

    const [batteryLevel, services] = await Promise.all([
      readBatteryLevel(server),
      readPrimaryServiceUuids(server)
    ]);

    const diagnostics = [];
    let ping = null;
    let deviceInfoEntries = [];
    let deviceInfo = {};

    try {
      ping = await session.ping(`edgerun-probe-${Date.now()}`);
    } catch (error) {
      diagnostics.push(error instanceof Error ? `ping rpc failed: ${error.message}` : "ping rpc failed");
    }

    try {
      deviceInfoEntries = await session.readDeviceInfo(24);
      deviceInfo = mapDeviceInfoEntries(deviceInfoEntries);
    } catch (error) {
      diagnostics.push(error instanceof Error ? `device info rpc failed: ${error.message}` : "device info rpc failed");
    }

    if (!Number.isFinite(batteryLevel)) diagnostics.push("battery characteristic unavailable");
    if (!normalizeUuid(services.join(",")).includes(normalizeUuid(SERIAL_SERVICE_UUID))) {
      diagnostics.push("serial service missing from primary service enumeration");
    }
    if (deviceInfoEntries.length === 0) diagnostics.push("device info rpc returned no key/value entries");

    return {
      deviceId: String(device.id || "").trim(),
      deviceName: String(device.name || details?.flipperDeviceName || "Flipper").trim(),
      batteryLevel,
      deviceInfo,
      deviceInfoEntries,
      services,
      diagnostics,
      serial: {
        serviceUuid: SERIAL_SERVICE_UUID,
        txUuid: SERIAL_TX_UUID,
        rxUuid: SERIAL_RX_UUID,
        flowUuid: SERIAL_FLOW_UUID,
        rpcStatusUuid: SERIAL_RPC_STATUS_UUID,
        flowBudget: session.flowBudget,
        characteristics
      },
      rpc: {
        ping,
        protobufDelimited: true
      },
      probedAt: new Date().toISOString()
    };
  } catch (error) {
    return {
      deviceId: String(details?.flipperDeviceId || "").trim(),
      deviceName: String(details?.flipperDeviceName || "Flipper").trim(),
      batteryLevel: null,
      deviceInfo: {},
      deviceInfoEntries: [],
      services: [],
      diagnostics: [
        error instanceof Error ? error.message : String(error || "probe setup failed")
      ],
      serial: {
        serviceUuid: SERIAL_SERVICE_UUID,
        txUuid: SERIAL_TX_UUID,
        rxUuid: SERIAL_RX_UUID,
        flowUuid: SERIAL_FLOW_UUID,
        rpcStatusUuid: SERIAL_RPC_STATUS_UUID,
        flowBudget: DEFAULT_FLOW_BUDGET,
        characteristics: characteristics || {}
      },
      rpc: {
        ping: null,
        protobufDelimited: true
      },
      probedAt: new Date().toISOString()
    };
  } finally {
    if (session) {
      await session.stop();
    }
  }
}

export {
  verifyFlipperBluetooth,
  probeFlipper
};
