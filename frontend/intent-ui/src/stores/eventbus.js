import { createSignal } from "solid-js";
import { decodeLocalEventEnvelope, decodeLocalNodeInfoResponse, encodeLocalEventEnvelope } from "../lib/local-bridge-proto";
import { setDeviceOnlineState, upsertDevice } from "./devices";

const EVENTBUS_TIMELINE_KEY = "intent-ui-eventbus-timeline-v1";
const EVENTBUS_MAX_EVENTS = 300;
const LOCAL_BRIDGE_WS_URL = "ws://127.0.0.1:7777/v1/local/eventbus/ws";
const LOCAL_BRIDGE_DEVICE_ID = "local-node-manager";
const LOCAL_BRIDGE_RETRY_INITIAL_MS = 2000;
const LOCAL_BRIDGE_RETRY_MAX_MS = 120000;
const LOCAL_BRIDGE_RETRY_PAUSE_AFTER_ATTEMPTS = 6;
const LOCAL_BRIDGE_RETRY_PAUSE_MS = 5 * 60 * 1000;

function safeParse(raw) {
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function readTimeline() {
  if (typeof window === "undefined") return [];
  const parsed = safeParse(localStorage.getItem(EVENTBUS_TIMELINE_KEY) || "");
  return Array.isArray(parsed) ? parsed : [];
}

function persistTimeline(entries) {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(EVENTBUS_TIMELINE_KEY, JSON.stringify(entries.slice(-EVENTBUS_MAX_EVENTS)));
  } catch {
    // ignore storage failures
  }
}

const [eventBusRuntime, setEventBusRuntime] = createSignal({
  engine: "worker-js",
  wasmLoaded: false,
  workerReady: false,
  localBridgeConnected: false,
  peers: [],
  lastSyncAt: ""
});

const [eventTimeline, setEventTimeline] = createSignal(readTimeline());

const topicListeners = new Map();
let eventBusWorker = null;
let localBridgeSocket = null;
let initialized = false;
let localBridgeRetryTimer = null;
let localBridgeRetryDelayMs = LOCAL_BRIDGE_RETRY_INITIAL_MS;
let localBridgeRetryAttempts = 0;

function notifyListeners(event) {
  const topic = String(event?.topic || "");
  const listeners = topicListeners.get(topic) || [];
  for (const listener of listeners) {
    try {
      listener(event);
    } catch {
      // ignore listener failures
    }
  }
  const wildcard = topicListeners.get("*") || [];
  for (const listener of wildcard) {
    try {
      listener(event);
    } catch {
      // ignore listener failures
    }
  }
}

function publishEvent(topic, payload = {}, meta = {}) {
  const normalizedTopic = String(topic || "").trim() || "event.unknown";
  if (localBridgeSocket && localBridgeSocket.readyState === WebSocket.OPEN && meta?.localBridgeForward !== false) {
    try {
      const envelope = encodeLocalEventEnvelope({
        eventId: `local-${Date.now()}-${Math.random().toString(16).slice(2, 10)}`,
        topic: normalizedTopic,
        source: String(meta?.source || "browser"),
        tsUnixMs: Date.now(),
        payloadBytes: new Uint8Array()
      });
      localBridgeSocket.send(envelope);
    } catch {
      // ignore local bridge send failures
    }
  }
  const localEvent = {
    id: `evt-local-${Date.now()}-${Math.random().toString(16).slice(2, 10)}`,
    topic: normalizedTopic,
    payload,
    meta,
    createdAt: new Date().toISOString()
  };
  setEventTimeline((prev) => {
    const next = [...prev, localEvent].slice(-EVENTBUS_MAX_EVENTS);
    persistTimeline(next);
    return next;
  });
  notifyListeners(localEvent);

  if (eventBusWorker && meta?.domain !== "ui") {
    eventBusWorker.postMessage({
      type: "publish",
      topic: normalizedTopic,
      payload,
      meta
    });
    return;
  }
}

function subscribeEvent(topic, handler) {
  if (!topicListeners.has(topic)) topicListeners.set(topic, []);
  topicListeners.get(topic).push(handler);
  return () => {
    const current = topicListeners.get(topic) || [];
    topicListeners.set(topic, current.filter((item) => item !== handler));
  };
}

function initializeBrowserEventBus() {
  if (typeof window === "undefined" || initialized) return;
  initialized = true;
  const worker = new Worker(new URL("../workers/eventbus.worker.js", import.meta.url), { type: "module" });
  eventBusWorker = worker;

  worker.onmessage = (message) => {
    const data = message?.data || {};
    if (data.type === "runtime" && data.runtime) {
      setEventBusRuntime((prev) => ({ ...prev, ...data.runtime }));
      return;
    }
    if (data.type === "timeline") {
      const events = Array.isArray(data.events) ? data.events : [];
      setEventTimeline(events.slice(-EVENTBUS_MAX_EVENTS));
      persistTimeline(events);
      return;
    }
    if (data.type === "event" && data.event) {
      const event = data.event;
      setEventTimeline((prev) => {
        const next = [...prev, event].slice(-EVENTBUS_MAX_EVENTS);
        persistTimeline(next);
        return next;
      });
      notifyListeners(event);
    }
  };

  worker.onerror = () => {
    setEventBusRuntime((prev) => ({ ...prev, engine: "worker-js", wasmLoaded: false, workerReady: false }));
    eventBusWorker = null;
  };

  worker.postMessage({ type: "init", wasmUrl: "/intent-ui/eventbus.wasm" });
  initializeLocalBridgeSocket();
}

function initializeLocalBridgeSocket() {
  if (typeof window === "undefined") return;
  if (localBridgeSocket) return;
  if (localBridgeRetryTimer) {
    window.clearTimeout(localBridgeRetryTimer);
    localBridgeRetryTimer = null;
  }
  try {
    const socket = new WebSocket(LOCAL_BRIDGE_WS_URL);
    localBridgeSocket = socket;
    socket.binaryType = "arraybuffer";
    socket.onopen = () => {
      setEventBusRuntime((prev) => ({ ...prev, localBridgeConnected: true }));
      localBridgeRetryAttempts = 0;
      localBridgeRetryDelayMs = LOCAL_BRIDGE_RETRY_INITIAL_MS;
      void hydrateLocalBridgeNodeInfo();
      socket.send(encodeLocalEventEnvelope({
        eventId: `bridge-hello-${Date.now()}`,
        topic: "browser.bridge.hello",
        source: "intent-ui",
        tsUnixMs: Date.now(),
        payloadBytes: new Uint8Array()
      }));
    };
    socket.onclose = () => {
      setEventBusRuntime((prev) => ({ ...prev, localBridgeConnected: false }));
      setDeviceOnlineState(LOCAL_BRIDGE_DEVICE_ID, false);
      localBridgeSocket = null;
      scheduleLocalBridgeReconnect();
    };
    socket.onerror = () => {
      setEventBusRuntime((prev) => ({ ...prev, localBridgeConnected: false }));
    };
    socket.onmessage = (message) => {
      if (message?.data instanceof ArrayBuffer) {
        let envelope = null;
        try {
          envelope = decodeLocalEventEnvelope(message.data);
        } catch {
          envelope = null;
        }
        if (!envelope) return;
        if (String(envelope.topic || "").trim() === "local.bridge.pong") {
          setDeviceOnlineState(LOCAL_BRIDGE_DEVICE_ID, true);
          return;
        }
        if (String(envelope.topic || "").trim()) {
          publishEvent(
            envelope.topic,
            {},
            { source: `local-bridge:${envelope.source || "node-manager"}`, localBridgeForward: false }
          );
        }
        return;
      }
      if (typeof message?.data === "string" && message.data.trim() === "pong") {
        setDeviceOnlineState(LOCAL_BRIDGE_DEVICE_ID, true);
      }
    };
  } catch {
    setEventBusRuntime((prev) => ({ ...prev, localBridgeConnected: false }));
    scheduleLocalBridgeReconnect();
  }
}

function scheduleLocalBridgeReconnect() {
  if (typeof window === "undefined") return;
  if (!initialized || localBridgeSocket || localBridgeRetryTimer) return;
  localBridgeRetryAttempts += 1;
  const shouldPause = localBridgeRetryAttempts >= LOCAL_BRIDGE_RETRY_PAUSE_AFTER_ATTEMPTS;
  const delay = shouldPause ? LOCAL_BRIDGE_RETRY_PAUSE_MS : localBridgeRetryDelayMs;
  if (shouldPause) {
    localBridgeRetryAttempts = 0;
    localBridgeRetryDelayMs = LOCAL_BRIDGE_RETRY_INITIAL_MS;
  } else {
    localBridgeRetryDelayMs = Math.min(localBridgeRetryDelayMs * 2, LOCAL_BRIDGE_RETRY_MAX_MS);
  }
  localBridgeRetryTimer = window.setTimeout(() => {
    localBridgeRetryTimer = null;
    if (initialized && !localBridgeSocket) initializeLocalBridgeSocket();
  }, delay);
}

async function hydrateLocalBridgeNodeInfo() {
  try {
    const response = await fetch("http://127.0.0.1:7777/v1/local/node/info.pb", { cache: "no-store" });
    if (!response.ok) throw new Error(`node info failed (${response.status})`);
    const bytes = new Uint8Array(await response.arrayBuffer());
    const info = decodeLocalNodeInfoResponse(bytes);
    if (!info.ok) throw new Error(info.error || "node info rejected");
    upsertDevice({
      id: info.nodeId || LOCAL_BRIDGE_DEVICE_ID,
      name: "Linux Node Manager",
      type: "host",
      os: "Linux",
      browser: "",
      online: true,
      connectedAt: new Date().toISOString(),
      lastSeenAt: new Date().toISOString(),
      ip: "127.0.0.1",
      metadata: {
        host: "localhost",
        bridgeUrl: `ws://127.0.0.1:7777${info.eventbusWsPath || "/v1/local/eventbus/ws"}`,
        bridgeVersion: info.bridgeVersion || "v1",
        devicePubkeyB64url: info.devicePubkeyB64url || "",
        capabilities: {
          networkUse: true,
          storageRead: true,
          storageWrite: true,
          display: false,
          graphics: false,
          audioOutput: false,
          usb: true,
          camera: false,
          microphone: false,
          shell: true,
          fileSystem: true,
          virtualization: true,
          hostControl: true,
          tpm: true
        }
      }
    });
    if (info.nodeId && info.nodeId !== LOCAL_BRIDGE_DEVICE_ID) {
      setDeviceOnlineState(LOCAL_BRIDGE_DEVICE_ID, false);
    }
  } catch {
    upsertDevice({
      id: LOCAL_BRIDGE_DEVICE_ID,
      name: "Linux Node Manager",
      type: "host",
      os: "Linux",
      browser: "",
      online: true,
      connectedAt: new Date().toISOString(),
      lastSeenAt: new Date().toISOString(),
      ip: "127.0.0.1",
      metadata: {
        host: "localhost",
        bridgeUrl: LOCAL_BRIDGE_WS_URL,
        bridgeVersion: "v1",
        capabilities: {
          networkUse: true,
          storageRead: true,
          storageWrite: true,
          display: false,
          graphics: false,
          audioOutput: false,
          usb: true,
          camera: false,
          microphone: false,
          shell: true,
          fileSystem: true,
          virtualization: true,
          hostControl: true,
          tpm: true
        }
      }
    });
  }
}

function syncRemoteEventBusSnapshot(remoteNodeId, events = []) {
  if (!eventBusWorker) return;
  eventBusWorker.postMessage({
    type: "snapshot",
    remoteNodeId: String(remoteNodeId || ""),
    events: Array.isArray(events) ? events : []
  });
}

export {
  eventBusRuntime,
  eventTimeline,
  initializeBrowserEventBus,
  publishEvent,
  subscribeEvent,
  syncRemoteEventBusSnapshot
};
