import { createSignal } from "solid-js";

const DEVICES_KEY = "intent-ui-devices-v1";
const CURRENT_DEVICE_ID = "local-browser-device";

function safeParse(raw) {
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function nowIso() {
  return new Date().toISOString();
}

function readStoredDevices() {
  if (typeof window === "undefined") return [];
  const parsed = safeParse(localStorage.getItem(DEVICES_KEY) || "");
  return Array.isArray(parsed) ? parsed : [];
}

function persistDevices(next) {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(DEVICES_KEY, JSON.stringify(next));
  } catch {
    // ignore storage failures
  }
}

function browserCapabilities(online) {
  return {
    networkUse: Boolean(online),
    storageRead: true,
    storageWrite: true,
    display: true,
    graphics: true,
    audioOutput: true,
    usb: false,
    camera: false,
    microphone: false,
    shell: false,
    fileSystem: false,
    virtualization: false,
    hostControl: false,
    tpm: false
  };
}

function buildCurrentDevice() {
  if (typeof window === "undefined") {
    return {
      id: CURRENT_DEVICE_ID,
      name: "This Browser",
      type: "browser",
      os: "Unknown",
      browser: "Unknown",
      online: true,
      connectedAt: nowIso(),
      lastSeenAt: nowIso(),
      ip: "Unknown",
      metadata: {
        capabilities: browserCapabilities(true)
      }
    };
  }
  const ua = navigator.userAgent || "";
  const browser = ua.includes("Firefox")
    ? "Firefox"
    : ua.includes("Edg/")
      ? "Edge"
      : ua.includes("Chrome")
        ? "Chrome"
        : ua.includes("Safari")
          ? "Safari"
          : "Unknown";
  const online = navigator.onLine;
  return {
    id: CURRENT_DEVICE_ID,
    name: "This Browser",
    type: "browser",
    os: navigator.platform || "Unknown",
    browser,
    online,
    connectedAt: nowIso(),
    lastSeenAt: nowIso(),
    ip: "Unknown",
    metadata: {
      language: navigator.language || "",
      userAgent: ua,
      viewport: `${window.innerWidth}x${window.innerHeight}`,
      capabilities: browserCapabilities(online)
    }
  };
}

const [devices, setDevices] = createSignal(readStoredDevices());
const [scanSettings, setScanSettings] = createSignal({ wifi: false, bluetooth: false, lan: false });
const [scanState, setScanState] = createSignal({
  running: false,
  lastScanAt: "",
  error: "",
  lastCounts: { wifi: 0, bluetooth: 0, lan: 0 }
});

function setSingleBrowserDevice(entry) {
  const next = [{ ...entry, lastSeenAt: entry.lastSeenAt || nowIso() }];
  setDevices(next);
  persistDevices(next);
}

function upsertDevice(entry) {
  if (!entry?.id) return;
  if (entry.id === CURRENT_DEVICE_ID) {
    setSingleBrowserDevice(entry);
    return;
  }
  setDevices((prev) => {
    const current = Array.isArray(prev) ? [...prev] : [];
    const index = current.findIndex((item) => item?.id === entry.id);
    const nextEntry = {
      ...entry,
      connectedAt: entry.connectedAt || nowIso(),
      lastSeenAt: entry.lastSeenAt || nowIso()
    };
    if (index >= 0) {
      const existing = current[index] || {};
      current[index] = {
        ...existing,
        ...nextEntry,
        metadata: {
          ...(existing.metadata || {}),
          ...(nextEntry.metadata || {})
        }
      };
    } else {
      current.push(nextEntry);
    }
    persistDevices(current);
    return current;
  });
}

function setDeviceOnlineState(id, online) {
  if (!id || id === CURRENT_DEVICE_ID) return;
  setDevices((prev) => {
    const current = Array.isArray(prev) ? [...prev] : [];
    const index = current.findIndex((item) => item?.id === id);
    if (index < 0) return current;
    current[index] = {
      ...current[index],
      online: Boolean(online),
      lastSeenAt: nowIso()
    };
    persistDevices(current);
    return current;
  });
}

async function runDiscoveryScan() {
  setScanState((prev) => ({
    ...prev,
    running: false,
    error: ""
  }));
}

function setScannerEnabled(kind, enabled) {
  if (!["wifi", "bluetooth", "lan"].includes(kind)) return;
  setScanSettings((prev) => ({
    ...prev,
    [kind]: Boolean(enabled) && false
  }));
}

function initializeCurrentDevice() {
  setSingleBrowserDevice(buildCurrentDevice());
}

if (typeof window !== "undefined") {
  queueMicrotask(() => {
    initializeCurrentDevice();
    const onOnline = () => setSingleBrowserDevice(buildCurrentDevice());
    const onOffline = () => setSingleBrowserDevice(buildCurrentDevice());
    const onResize = () => setSingleBrowserDevice(buildCurrentDevice());
    window.addEventListener("online", onOnline);
    window.addEventListener("offline", onOffline);
    window.addEventListener("resize", onResize);
    window.addEventListener("beforeunload", () => {
      window.removeEventListener("online", onOnline);
      window.removeEventListener("offline", onOffline);
      window.removeEventListener("resize", onResize);
    });
  });
}

function knownDevices() {
  return [...devices()].sort((a, b) => {
    if (a.online !== b.online) return a.online ? -1 : 1;
    return new Date(b.lastSeenAt || 0).getTime() - new Date(a.lastSeenAt || 0).getTime();
  });
}

export {
  CURRENT_DEVICE_ID,
  devices,
  knownDevices,
  upsertDevice,
  setDeviceOnlineState,
  runDiscoveryScan,
  scanSettings,
  scanState,
  setScannerEnabled
};
