import { createSignal } from "solid-js";
import { context } from "./context";

const DEVICES_KEY = "intent-ui-devices-v1";
const DEVICE_SCAN_KEY = "intent-ui-device-scan-v1";
const CURRENT_DEVICE_ID = "local-browser-device";
const SCAN_INTERVAL_MS = 20000;
const BLUETOOTH_ACTIVE_SCAN_SECONDS = 6;

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

function readStoredScan() {
  if (typeof window === "undefined") {
    return { wifi: false, bluetooth: false, lan: false };
  }
  const parsed = safeParse(localStorage.getItem(DEVICE_SCAN_KEY) || "");
  if (!parsed || typeof parsed !== "object") {
    return { wifi: false, bluetooth: false, lan: false };
  }
  return {
    wifi: Boolean(parsed.wifi),
    bluetooth: Boolean(parsed.bluetooth),
    lan: Boolean(parsed.lan)
  };
}

function persistDevices(next) {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(DEVICES_KEY, JSON.stringify(next));
  } catch {
    // ignore storage failures
  }
}

function persistScan(next) {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(DEVICE_SCAN_KEY, JSON.stringify(next));
  } catch {
    // ignore storage failures
  }
}

function buildCurrentDevice() {
  if (typeof window === "undefined") {
    return {
      id: CURRENT_DEVICE_ID,
      name: "This Device",
      type: "browser",
      os: "Unknown",
      browser: "Unknown",
      online: true,
      connectedAt: nowIso(),
      lastSeenAt: nowIso(),
      ip: "Unknown",
      metadata: {}
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
  return {
    id: CURRENT_DEVICE_ID,
    name: "This Device",
    type: "browser",
    os: navigator.platform || "Unknown",
    browser,
    online: navigator.onLine,
    connectedAt: nowIso(),
    lastSeenAt: nowIso(),
    ip: "Unknown",
    metadata: {
      language: navigator.language || "",
      userAgent: ua,
      viewport: `${window.innerWidth}x${window.innerHeight}`
    }
  };
}

function buildConnectedHostDevice() {
  const host = String(context.currentHost || "").trim();
  if (!host) return null;
  return {
    id: `host:${host}`,
    name: "Connected Host",
    type: "host",
    os: "Host OS",
    browser: "",
    online: true,
    connectedAt: nowIso(),
    lastSeenAt: nowIso(),
    ip: host,
    metadata: { host }
  };
}

async function refreshConnectedHostStatus() {
  const host = String(context.currentHost || "").trim();
  if (!host || typeof window === "undefined") return;
  try {
    const response = await fetch("/api/host/status");
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || payload?.ok === false) {
      throw new Error(payload?.error || "Host status unavailable.");
    }
    const primaryIp = Array.isArray(payload?.host?.ipAddresses) && payload.host.ipAddresses.length > 0
      ? payload.host.ipAddresses[0]
      : host;
    upsertDevice({
      id: `host:${host}`,
      name: payload?.host?.hostname || "Connected Host",
      type: "host",
      os: [payload?.host?.platform, payload?.host?.release].filter(Boolean).join(" ") || "Host OS",
      browser: "",
      online: true,
      connectedAt: nowIso(),
      lastSeenAt: nowIso(),
      ip: primaryIp,
      metadata: {
        host,
        ipAddresses: payload?.host?.ipAddresses || [],
        resources: payload?.resources || {},
        capabilities: payload?.capabilities || {},
        server: payload?.server || {},
        timestamp: payload?.timestamp || nowIso()
      }
    });
  } catch {
    setDeviceOnline(`host:${host}`, false);
  }
}

const [devices, setDevices] = createSignal(readStoredDevices());
const [scanSettings, setScanSettings] = createSignal(readStoredScan());
const [scanState, setScanState] = createSignal({
  running: false,
  lastScanAt: "",
  error: "",
  lastCounts: { wifi: 0, bluetooth: 0, lan: 0 }
});

function buildScanDevice(entry, source) {
  const ip = String(entry?.ip || "").trim();
  const id = String(entry?.id || `${source}:${ip || "item"}`).trim();
  return {
    id,
    name: String(entry?.name || entry?.metadata?.mac || ip || `${source} device`).trim(),
    type: source,
    os: source.toUpperCase(),
    browser: "",
    online: entry?.online !== false,
    connectedAt: nowIso(),
    lastSeenAt: nowIso(),
    ip: ip || "Unknown",
    metadata: {
      ...(entry?.metadata && typeof entry.metadata === "object" ? entry.metadata : {}),
      source
    }
  };
}

async function runDiscoveryScan({ force = false } = {}) {
  const settings = scanSettings();
  const enabled = settings.wifi || settings.bluetooth || settings.lan;
  if (!enabled && !force) return;
  if (scanState().running && !force) return;
  setScanState((prev) => ({ ...prev, running: true, error: "" }));
  try {
    const response = await fetch("/api/discovery/scan", {
      method: "POST",
      headers: { "content-type": "application/json; charset=utf-8" },
      body: JSON.stringify({
        scan: {
          ...settings,
          bluetoothActiveScanSeconds: settings.bluetooth ? BLUETOOTH_ACTIVE_SCAN_SECONDS : 0
        }
      })
    });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || payload?.ok === false) {
      throw new Error(payload?.error || "Discovery scan failed.");
    }
    const buckets = [
      { key: "wifi", items: Array.isArray(payload?.wifi?.items) ? payload.wifi.items : [] },
      { key: "bluetooth", items: Array.isArray(payload?.bluetooth?.items) ? payload.bluetooth.items : [] },
      { key: "lan", items: Array.isArray(payload?.lan?.items) ? payload.lan.items : [] }
    ];
    for (const bucket of buckets) {
      for (const item of bucket.items) {
        upsertDevice(buildScanDevice(item, bucket.key));
      }
    }
    const warning = String(payload?.bluetooth?.warning || "").trim();
    setScanState((prev) => ({
      ...prev,
      running: false,
      lastScanAt: payload?.timestamp || nowIso(),
      error: warning,
      lastCounts: {
        wifi: buckets[0].items.length,
        bluetooth: buckets[1].items.length,
        lan: buckets[2].items.length
      }
    }));
  } catch (error) {
    setScanState((prev) => ({
      ...prev,
      running: false,
      lastScanAt: prev.lastScanAt || "",
      error: error instanceof Error ? error.message : "Discovery scan failed."
    }));
  }
}

function setScannerEnabled(kind, enabled) {
  if (!["wifi", "bluetooth", "lan"].includes(kind)) return;
  let nextSettings = null;
  setScanSettings((prev) => {
    const next = { ...prev, [kind]: Boolean(enabled) };
    persistScan(next);
    nextSettings = next;
    return next;
  });
  if (nextSettings && (nextSettings.wifi || nextSettings.bluetooth || nextSettings.lan)) {
    runDiscoveryScan({ force: true });
  }
}

function upsertDevice(entry) {
  if (!entry?.id) return;
  setDevices((prev) => {
    const index = prev.findIndex((item) => item.id === entry.id);
    const nextEntry = {
      ...entry,
      lastSeenAt: entry.lastSeenAt || nowIso()
    };
    const next = [...prev];
    if (index >= 0) {
      next[index] = {
        ...next[index],
        ...nextEntry
      };
    } else {
      next.unshift(nextEntry);
    }
    persistDevices(next);
    return next;
  });
}

function setDeviceOnline(id, online) {
  if (!id) return;
  setDevices((prev) => {
    const next = prev.map((item) => item.id === id ? {
      ...item,
      online: Boolean(online),
      lastSeenAt: nowIso()
    } : item);
    persistDevices(next);
    return next;
  });
}

function initializeCurrentDevice() {
  upsertDevice(buildCurrentDevice());
  const hostDevice = buildConnectedHostDevice();
  if (hostDevice) upsertDevice(hostDevice);
}

if (typeof window !== "undefined") {
  queueMicrotask(() => {
    initializeCurrentDevice();
    refreshConnectedHostStatus();
    const onOnline = () => setDeviceOnline(CURRENT_DEVICE_ID, true);
    const onOffline = () => setDeviceOnline(CURRENT_DEVICE_ID, false);
    const onResize = () => {
      const current = buildCurrentDevice();
      upsertDevice(current);
    };
    const hostTimer = window.setInterval(() => {
      refreshConnectedHostStatus();
    }, 15000);
    const scanTimer = window.setInterval(() => {
      runDiscoveryScan();
    }, SCAN_INTERVAL_MS);
    runDiscoveryScan();
    window.addEventListener("online", onOnline);
    window.addEventListener("offline", onOffline);
    window.addEventListener("resize", onResize);
    window.addEventListener("beforeunload", () => {
      window.clearInterval(hostTimer);
      window.clearInterval(scanTimer);
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
  runDiscoveryScan,
  scanSettings,
  scanState,
  setScannerEnabled
};
