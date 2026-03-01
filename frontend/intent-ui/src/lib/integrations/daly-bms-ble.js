import { BluetoothIntegration } from "./BluetoothIntegration";

const DALY_NUS_SERVICE_UUID = "6e400001-b5a3-f393-e0a9-e50e24dcca9e";
const DALY_NUS_TX_UUID = "6e400003-b5a3-f393-e0a9-e50e24dcca9e";
const DALY_NUS_RX_UUID = "6e400002-b5a3-f393-e0a9-e50e24dcca9e";
const BATTERY_SERVICE = "battery_service";
const BATTERY_LEVEL_CHARACTERISTIC = "battery_level";

const dalyBluetooth = new BluetoothIntegration({
  integrationId: "daly_bms",
  optionalServices: [DALY_NUS_SERVICE_UUID, BATTERY_SERVICE, "device_information"]
});

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function toHex(bytes) {
  if (!(bytes instanceof Uint8Array)) return "";
  return Array.from(bytes).map((value) => value.toString(16).padStart(2, "0")).join("");
}

function toUint8Array(view) {
  if (view instanceof Uint8Array) return view;
  if (view instanceof DataView) return new Uint8Array(view.buffer, view.byteOffset, view.byteLength);
  if (ArrayBuffer.isView(view)) return new Uint8Array(view.buffer, view.byteOffset, view.byteLength);
  if (view instanceof ArrayBuffer) return new Uint8Array(view);
  return new Uint8Array();
}

function detectProtocolFromSamples(samples) {
  for (const sample of samples) {
    if (!(sample instanceof Uint8Array) || sample.length === 0) continue;
    if (sample[0] === 0xD2) return "D2";
    if (sample[0] === 0xA5) return "A5";
  }
  return "unknown";
}

async function openDalySession(details = {}) {
  const preferredId = String(details?.dalyDeviceId || "").trim();
  const namePrefix = String(details?.namePrefix || "DL-").trim();
  const requestOptions = namePrefix
    ? { filters: [{ namePrefix }], optionalServices: [DALY_NUS_SERVICE_UUID] }
    : { acceptAllDevices: true, optionalServices: [DALY_NUS_SERVICE_UUID] };
  let device = await dalyBluetooth.resolveDevice(preferredId, requestOptions);
  if (!device) throw new Error("No Daly device selected.");

  let server = null;
  let service = null;
  try {
    const resolved = await dalyBluetooth.getService(device, DALY_NUS_SERVICE_UUID);
    device = resolved.device;
    server = resolved.server;
    service = resolved.service;
  } catch (error) {
    if (dalyBluetooth.isPermissionError(error)) {
      throw new Error("Daly BLE service not granted for this origin. Re-select the device and allow BLE service access.");
    }
    throw error;
  }

  const txResolved = await dalyBluetooth.getCharacteristic(device, DALY_NUS_SERVICE_UUID, DALY_NUS_TX_UUID);
  const rxResolved = await dalyBluetooth.getCharacteristic(txResolved.device, DALY_NUS_SERVICE_UUID, DALY_NUS_RX_UUID);

  return {
    device: rxResolved.device,
    server: rxResolved.server,
    service,
    txChar: txResolved.characteristic,
    rxChar: rxResolved.characteristic
  };
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
      .map((service) => String(service?.uuid || "").trim().toLowerCase())
      .filter(Boolean);
  } catch {
    return [];
  }
}

async function verifyDalyBmsBluetooth(details = {}) {
  const { device, txChar, rxChar } = await openDalySession(details);
  const txProps = txChar?.properties || {};
  const rxProps = rxChar?.properties || {};
  if (!txProps.notify && !txProps.indicate && typeof txChar.startNotifications !== "function") {
    throw new Error("Daly TX characteristic does not support notifications.");
  }
  if (!rxProps.write && !rxProps.writeWithoutResponse && typeof rxChar.writeValueWithoutResponse !== "function" && typeof rxChar.writeValue !== "function") {
    throw new Error("Daly RX characteristic is not writable.");
  }
  return {
    deviceId: String(device.id || "").trim(),
    deviceName: String(device.name || details?.dalyDeviceName || "Daly BMS").trim()
  };
}

async function probeDalyBms(details = {}) {
  let device = null;
  let server = null;
  let txChar = null;
  let rxChar = null;
  const samples = [];
  const diagnostics = [];
  let batteryLevel = null;
  let services = [];
  let listener = null;
  try {
    const opened = await openDalySession(details);
    device = opened.device;
    server = opened.server;
    txChar = opened.txChar;
    rxChar = opened.rxChar;

    [batteryLevel, services] = await Promise.all([
      readBatteryLevel(server),
      readPrimaryServiceUuids(server)
    ]);

    listener = (event) => {
      const value = toUint8Array(event?.target?.value);
      if (value.length === 0) return;
      if (samples.length < 10) samples.push(value);
    };
    if (typeof txChar.startNotifications === "function") {
      await txChar.startNotifications();
      txChar.addEventListener("characteristicvaluechanged", listener);
    }

    // Conservative probe writes: common Daly poll frame candidates.
    const candidates = [
      Uint8Array.from([0xD2, 0x03, 0x00, 0x00, 0x00, 0x3D]),
      Uint8Array.from([0xA5, 0x40, 0x90, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
    ];
    for (const frame of candidates) {
      try {
        if (typeof rxChar.writeValueWithoutResponse === "function") {
          await rxChar.writeValueWithoutResponse(frame);
        } else if (typeof rxChar.writeValue === "function") {
          await rxChar.writeValue(frame);
        }
      } catch {
        // continue with other candidates
      }
      await sleep(220);
    }
    await sleep(450);

    if (!Number.isFinite(batteryLevel)) diagnostics.push("battery characteristic unavailable");
    if (!services.includes(DALY_NUS_SERVICE_UUID)) diagnostics.push("daly nus service not visible in primary services");
    if (samples.length === 0) diagnostics.push("no notify packets captured during probe");
    const protocol = detectProtocolFromSamples(samples);

    return {
      deviceId: String(device.id || "").trim(),
      deviceName: String(device.name || details?.dalyDeviceName || "Daly BMS").trim(),
      batteryLevel,
      services,
      protocol,
      packetSamplesHex: samples.slice(0, 5).map((sample) => toHex(sample)),
      diagnostics,
      probeOk: diagnostics.length === 0,
      serial: {
        serviceUuid: DALY_NUS_SERVICE_UUID,
        txUuid: DALY_NUS_TX_UUID,
        rxUuid: DALY_NUS_RX_UUID
      },
      probedAt: new Date().toISOString()
    };
  } catch (error) {
    return {
      deviceId: String(details?.dalyDeviceId || "").trim(),
      deviceName: String(details?.dalyDeviceName || "Daly BMS").trim(),
      batteryLevel: null,
      services: [],
      protocol: "unknown",
      packetSamplesHex: [],
      diagnostics: [error instanceof Error ? error.message : String(error || "probe failed")],
      probeOk: false,
      serial: {
        serviceUuid: DALY_NUS_SERVICE_UUID,
        txUuid: DALY_NUS_TX_UUID,
        rxUuid: DALY_NUS_RX_UUID
      },
      probedAt: new Date().toISOString()
    };
  } finally {
    if (txChar && listener) {
      try {
        txChar.removeEventListener("characteristicvaluechanged", listener);
      } catch {}
      try {
        if (typeof txChar.stopNotifications === "function") {
          await txChar.stopNotifications();
        }
      } catch {}
    }
  }
}

export {
  verifyDalyBmsBluetooth,
  probeDalyBms
};
