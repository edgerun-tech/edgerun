import { BluetoothIntegration } from "./BluetoothIntegration";

const DALY_NUS_SERVICE_UUID = "6e400001-b5a3-f393-e0a9-e50e24dcca9e";
const DALY_NUS_TX_UUID = "6e400003-b5a3-f393-e0a9-e50e24dcca9e";
const DALY_NUS_RX_UUID = "6e400002-b5a3-f393-e0a9-e50e24dcca9e";
const DALY_FFF0_SERVICE_UUID = "0000fff0-0000-1000-8000-00805f9b34fb";
const DALY_FFF1_CHAR_UUID = "0000fff1-0000-1000-8000-00805f9b34fb";
const DALY_FFF2_CHAR_UUID = "0000fff2-0000-1000-8000-00805f9b34fb";
const DALY_FFE0_SERVICE_UUID = "0000ffe0-0000-1000-8000-00805f9b34fb";
const DALY_FFE1_CHAR_UUID = "0000ffe1-0000-1000-8000-00805f9b34fb";
const BATTERY_SERVICE = "battery_service";
const BATTERY_LEVEL_CHARACTERISTIC = "battery_level";

const DALY_PROFILES = [
  { serviceUuid: DALY_NUS_SERVICE_UUID, txUuid: DALY_NUS_TX_UUID, rxUuid: DALY_NUS_RX_UUID, label: "NUS" },
  { serviceUuid: DALY_FFF0_SERVICE_UUID, txUuid: DALY_FFF1_CHAR_UUID, rxUuid: DALY_FFF2_CHAR_UUID, label: "FFF0/FFF1/FFF2" },
  { serviceUuid: DALY_FFF0_SERVICE_UUID, txUuid: DALY_FFF2_CHAR_UUID, rxUuid: DALY_FFF1_CHAR_UUID, label: "FFF0/FFF2/FFF1" },
  { serviceUuid: DALY_FFE0_SERVICE_UUID, txUuid: DALY_FFE1_CHAR_UUID, rxUuid: DALY_FFE1_CHAR_UUID, label: "FFE0/FFE1" }
];

const DALY_OPTIONAL_SERVICES = Array.from(new Set(DALY_PROFILES.map((profile) => profile.serviceUuid)));

const dalyBluetooth = new BluetoothIntegration({
  integrationId: "daly_bms",
  optionalServices: [...DALY_OPTIONAL_SERVICES, BATTERY_SERVICE, "device_information"]
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

function buildA5ReadFrame(command) {
  const frame = Uint8Array.from([0xA5, 0x40, command & 0xff, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
  let checksum = 0;
  for (let index = 0; index < 12; index += 1) checksum = (checksum + frame[index]) & 0xff;
  frame[12] = checksum;
  return frame;
}

function normalizeUuid(input) {
  return String(input || "").trim().toLowerCase();
}

async function resolveProfileSession(device, profile) {
  const serviceResolved = await dalyBluetooth.getService(device, profile.serviceUuid);
  const txResolved = await dalyBluetooth.getCharacteristic(serviceResolved.device, profile.serviceUuid, profile.txUuid);
  const rxResolved = await dalyBluetooth.getCharacteristic(txResolved.device, profile.serviceUuid, profile.rxUuid);
  return {
    device: rxResolved.device,
    server: rxResolved.server,
    service: serviceResolved.service,
    txChar: txResolved.characteristic,
    rxChar: rxResolved.characteristic,
    serviceUuid: profile.serviceUuid,
    txUuid: profile.txUuid,
    rxUuid: profile.rxUuid,
    profileLabel: profile.label
  };
}

async function resolveAdaptiveSession(device) {
  const server = await dalyBluetooth.connectGatt(device);
  if (typeof server.getPrimaryServices !== "function") {
    throw new Error("Primary service enumeration is unavailable for Daly adaptive probe.");
  }
  const services = await server.getPrimaryServices();
  for (const service of Array.isArray(services) ? services : []) {
    const serviceUuid = normalizeUuid(service?.uuid);
    if (!serviceUuid) continue;
    let characteristics = [];
    try {
      characteristics = await service.getCharacteristics();
    } catch {
      continue;
    }
    const list = Array.isArray(characteristics) ? characteristics : [];
    const txChar = list.find((char) => {
      const p = char?.properties || {};
      return Boolean(p.notify || p.indicate || p.read);
    });
    const rxChar = list.find((char) => {
      const p = char?.properties || {};
      return Boolean(p.write || p.writeWithoutResponse);
    });
    if (!txChar || !rxChar) continue;
    return {
      device,
      server,
      service,
      txChar,
      rxChar,
      serviceUuid,
      txUuid: normalizeUuid(txChar?.uuid),
      rxUuid: normalizeUuid(rxChar?.uuid),
      profileLabel: "adaptive"
    };
  }
  throw new Error("No compatible notify/write characteristic pair found for Daly device.");
}

async function openDalySession(details = {}) {
  const preferredId = String(details?.dalyDeviceId || "").trim();
  const namePrefix = String(details?.namePrefix || "DL-").trim();
  const requestOptions = namePrefix
    ? { filters: [{ namePrefix }], optionalServices: DALY_OPTIONAL_SERVICES }
    : { acceptAllDevices: true, optionalServices: DALY_OPTIONAL_SERVICES };
  let device = await dalyBluetooth.resolveDevice(preferredId, requestOptions);
  if (!device) throw new Error("No Daly device selected.");

  for (const profile of DALY_PROFILES) {
    try {
      return await resolveProfileSession(device, profile);
    } catch (error) {
      if (dalyBluetooth.isPermissionError(error)) {
        throw new Error("Daly BLE service not granted for this origin. Re-select the device and allow BLE service access.");
      }
      // continue profile probing
    }
  }
  try {
    return await resolveAdaptiveSession(device);
  } catch (error) {
    if (dalyBluetooth.isPermissionError(error)) {
      throw new Error("Daly BLE service not granted for this origin. Re-select the device and allow BLE service access.");
    }
    throw error;
  }
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
  const { device, txChar, rxChar, profileLabel } = await openDalySession(details);
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
    deviceName: String(device.name || details?.dalyDeviceName || "Daly BMS").trim(),
    profileLabel
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
  let listeners = [];
  let notifiedChars = [];
  let writableChars = [];
  let session = null;
  try {
    session = await openDalySession(details);
    device = session.device;
    server = session.server;
    txChar = session.txChar;
    rxChar = session.rxChar;

    [batteryLevel, services] = await Promise.all([
      readBatteryLevel(server),
      readPrimaryServiceUuids(server)
    ]);

    const notifyCandidates = [txChar, rxChar].filter((characteristic, index, list) =>
      characteristic && list.indexOf(characteristic) === index
    );
    for (const characteristic of notifyCandidates) {
      const props = characteristic?.properties || {};
      if (!props.notify && !props.indicate && typeof characteristic.startNotifications !== "function") continue;
      const listener = (event) => {
        const value = toUint8Array(event?.target?.value);
        if (value.length === 0) return;
        if (samples.length < 10) samples.push(value);
      };
      try {
        if (typeof characteristic.startNotifications === "function") {
          await characteristic.startNotifications();
        }
        characteristic.addEventListener("characteristicvaluechanged", listener);
        listeners.push({ characteristic, listener });
        notifiedChars.push(normalizeUuid(characteristic?.uuid));
      } catch {
        // continue probing other characteristics
      }
    }

    writableChars = [txChar, rxChar].filter((characteristic, index, list) => {
      if (!characteristic || list.indexOf(characteristic) !== index) return false;
      const props = characteristic?.properties || {};
      return Boolean(props.write || props.writeWithoutResponse || typeof characteristic.writeValueWithoutResponse === "function" || typeof characteristic.writeValue === "function");
    });

    // Conservative poll frames: D2 query + checksum-correct A5 reads.
    const candidates = [
      Uint8Array.from([0xD2, 0x03, 0x00, 0x00, 0x00, 0x3D]),
      buildA5ReadFrame(0x90),
      buildA5ReadFrame(0x91),
      buildA5ReadFrame(0x93),
      buildA5ReadFrame(0x94)
    ];
    for (const frame of candidates) {
      for (const characteristic of writableChars) {
        try {
          if (typeof characteristic.writeValueWithoutResponse === "function") {
            await characteristic.writeValueWithoutResponse(frame);
          } else if (typeof characteristic.writeValue === "function") {
            await characteristic.writeValue(frame);
          }
        } catch {
          // continue
        }
      }
      await sleep(220);
    }
    await sleep(600);

    if (!Number.isFinite(batteryLevel)) diagnostics.push("battery characteristic unavailable");
    if (writableChars.length === 0) diagnostics.push("no writable Daly characteristic detected");
    if (notifiedChars.length === 0) diagnostics.push("no notify Daly characteristic detected");
    const expectedServiceUuid = normalizeUuid(session?.serviceUuid || "");
    if (expectedServiceUuid && !services.includes(expectedServiceUuid)) {
      diagnostics.push(`selected service ${expectedServiceUuid} not visible in primary services`);
    }
    if (samples.length === 0) {
      for (const characteristic of [txChar, rxChar]) {
        if (!characteristic || typeof characteristic.readValue !== "function") continue;
        try {
          const snapshot = toUint8Array(await characteristic.readValue());
          if (snapshot.length > 0 && samples.length < 10) {
            samples.push(snapshot);
          }
        } catch {
          // ignore read fallback failures
        }
      }
    }
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
        serviceUuid: session.serviceUuid || "",
        txUuid: session.txUuid || "",
        rxUuid: session.rxUuid || "",
        profileLabel: session.profileLabel || "unknown",
        notifyUuids: notifiedChars.filter(Boolean),
        writeUuids: writableChars.map((char) => normalizeUuid(char?.uuid)).filter(Boolean)
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
        serviceUuid: "",
        txUuid: "",
        rxUuid: "",
        profileLabel: "unknown"
      },
      probedAt: new Date().toISOString()
    };
  } finally {
    for (const entry of listeners) {
      const characteristic = entry?.characteristic;
      const listener = entry?.listener;
      if (!characteristic || !listener) continue;
      try {
        characteristic.removeEventListener("characteristicvaluechanged", listener);
      } catch {}
      try {
        if (typeof characteristic.stopNotifications === "function") {
          await characteristic.stopNotifications();
        }
      } catch {}
    }
  }
}

export {
  verifyDalyBmsBluetooth,
  probeDalyBms
};
