import { createSignal } from "solid-js";
import { setDeviceOnlineState, upsertDevice } from "./devices";

const EVENTBUS_TIMELINE_KEY = "intent-ui-eventbus-timeline-v1";
const EVENTBUS_MAX_EVENTS = 300;
const LOCAL_BRIDGE_WS_URL = "ws://127.0.0.1:7777/v1/local/eventbus/ws";
const LOCAL_BRIDGE_DEVICE_ID = "local-node-manager";

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
  if (eventBusWorker) {
    eventBusWorker.postMessage({
      type: "publish",
      topic: normalizedTopic,
      payload,
      meta
    });
    return;
  }
  const fallbackEvent = {
    id: `evt-fallback-${Date.now()}`,
    topic: normalizedTopic,
    payload,
    meta,
    createdAt: new Date().toISOString()
  };
  setEventTimeline((prev) => {
    const next = [...prev, fallbackEvent].slice(-EVENTBUS_MAX_EVENTS);
    persistTimeline(next);
    return next;
  });
  notifyListeners(fallbackEvent);
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
  try {
    const socket = new WebSocket(LOCAL_BRIDGE_WS_URL);
    localBridgeSocket = socket;
    socket.binaryType = "arraybuffer";
    socket.onopen = () => {
      setEventBusRuntime((prev) => ({ ...prev, localBridgeConnected: true }));
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
      socket.send("ping");
    };
    socket.onclose = () => {
      setEventBusRuntime((prev) => ({ ...prev, localBridgeConnected: false }));
      setDeviceOnlineState(LOCAL_BRIDGE_DEVICE_ID, false);
      localBridgeSocket = null;
      window.setTimeout(() => {
        if (initialized && !localBridgeSocket) initializeLocalBridgeSocket();
      }, 2500);
    };
    socket.onerror = () => {
      setEventBusRuntime((prev) => ({ ...prev, localBridgeConnected: false }));
    };
    socket.onmessage = (message) => {
      if (typeof message?.data === "string" && message.data.trim() === "pong") {
        setDeviceOnlineState(LOCAL_BRIDGE_DEVICE_ID, true);
      }
    };
  } catch {
    setEventBusRuntime((prev) => ({ ...prev, localBridgeConnected: false }));
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
