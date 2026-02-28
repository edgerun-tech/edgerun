import { createSignal } from "solid-js";
import { decodeLocalEventEnvelope, decodeLocalNodeInfoResponse, encodeLocalEventEnvelope } from "../lib/local-bridge-proto";
import { localBridgeHttpUrl, localBridgeWsUrl } from "../lib/local-bridge-origin";
import { setDeviceOnlineState, upsertDevice } from "./devices";

const EVENTBUS_TIMELINE_KEY = "intent-ui-eventbus-timeline-v1";
const EVENTBUS_MAX_EVENTS = 300;
const LOCAL_BRIDGE_WS_PATH = "/v1/local/eventbus/ws";
const LOCAL_BRIDGE_DEVICE_ID = "local-node-manager";
const LOCAL_BRIDGE_ERROR_MESSAGE = "Can't connect to local bridge, is it running?";
const LOCAL_BRIDGE_CONNECT_TIMEOUT_MS = 2500;

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
  engine: "bridge-ws",
  wasmLoaded: false,
  workerReady: false,
  bridgeRequired: true,
  localBridgeConnected: false,
  localBridgeStatus: "connecting",
  localBridgeError: "",
  peers: [],
  lastSyncAt: ""
});

const [eventTimeline, setEventTimeline] = createSignal(readTimeline());

const topicListeners = new Map();
let localBridgeSocket = null;
let initialized = false;
let localBridgeConnectTimer = null;

function decodeEnvelopePayload(payloadBytes) {
  if (!(payloadBytes instanceof Uint8Array) || payloadBytes.length === 0) return {};
  try {
    const text = new TextDecoder().decode(payloadBytes);
    const parsed = JSON.parse(text);
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

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
  const canUseBridge = Boolean(localBridgeSocket) && localBridgeSocket.readyState === WebSocket.OPEN;
  if (!canUseBridge) {
    return false;
  }
  try {
    const payloadBytes = new TextEncoder().encode(JSON.stringify({
      payload: payload && typeof payload === "object" ? payload : {},
      meta: meta && typeof meta === "object" ? meta : {}
    }));
    const envelope = encodeLocalEventEnvelope({
      eventId: `local-${Date.now()}-${Math.random().toString(16).slice(2, 10)}`,
      topic: normalizedTopic,
      source: String(meta?.source || "browser"),
      tsUnixMs: Date.now(),
      payloadBytes
    });
    localBridgeSocket.send(envelope);
  } catch {
    return false;
  }
  return true;
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
  initializeLocalBridgeSocket();
}

function initializeLocalBridgeSocket() {
  if (typeof window === "undefined") return;
  if (localBridgeSocket) return;
  setEventBusRuntime((prev) => ({
    ...prev,
    localBridgeConnected: false,
    localBridgeStatus: "connecting",
    localBridgeError: ""
  }));
  try {
    const socket = new WebSocket(localBridgeWsUrl(LOCAL_BRIDGE_WS_PATH));
    localBridgeSocket = socket;
    socket.binaryType = "arraybuffer";
    localBridgeConnectTimer = window.setTimeout(() => {
      if (!localBridgeSocket || localBridgeSocket !== socket) return;
      setEventBusRuntime((prev) => ({
        ...prev,
        localBridgeConnected: false,
        localBridgeStatus: "error",
        localBridgeError: LOCAL_BRIDGE_ERROR_MESSAGE
      }));
      try {
        socket.close();
      } catch {
        // ignore socket close failures
      }
    }, LOCAL_BRIDGE_CONNECT_TIMEOUT_MS);
    socket.onopen = () => {
      if (localBridgeConnectTimer) {
        window.clearTimeout(localBridgeConnectTimer);
        localBridgeConnectTimer = null;
      }
      setEventBusRuntime((prev) => ({
        ...prev,
        localBridgeConnected: true,
        localBridgeStatus: "connected",
        localBridgeError: ""
      }));
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
      if (localBridgeConnectTimer) {
        window.clearTimeout(localBridgeConnectTimer);
        localBridgeConnectTimer = null;
      }
      setEventBusRuntime((prev) => ({
        ...prev,
        localBridgeConnected: false,
        localBridgeStatus: "error",
        localBridgeError: LOCAL_BRIDGE_ERROR_MESSAGE
      }));
      setDeviceOnlineState(LOCAL_BRIDGE_DEVICE_ID, false);
      localBridgeSocket = null;
    };
    socket.onerror = () => {
      if (localBridgeConnectTimer) {
        window.clearTimeout(localBridgeConnectTimer);
        localBridgeConnectTimer = null;
      }
      setEventBusRuntime((prev) => ({
        ...prev,
        localBridgeConnected: false,
        localBridgeStatus: "error",
        localBridgeError: LOCAL_BRIDGE_ERROR_MESSAGE
      }));
      if (localBridgeSocket && localBridgeSocket.readyState !== WebSocket.CLOSED) {
        try {
          localBridgeSocket.close();
        } catch {
          // ignore socket close failures
        }
      }
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
          const envelopeData = decodeEnvelopePayload(envelope.payloadBytes);
          const bridgeEvent = {
            id: envelope.eventId || `evt-bridge-${Date.now()}-${Math.random().toString(16).slice(2, 10)}`,
            topic: envelope.topic,
            payload: envelopeData.payload && typeof envelopeData.payload === "object" ? envelopeData.payload : {},
            meta: {
              ...(envelopeData.meta && typeof envelopeData.meta === "object" ? envelopeData.meta : {}),
              source: `local-bridge:${envelope.source || "node-manager"}`
            },
            createdAt: envelope.tsUnixMs ? new Date(envelope.tsUnixMs).toISOString() : new Date().toISOString()
          };
          setEventTimeline((prev) => {
            const next = [...prev, bridgeEvent].slice(-EVENTBUS_MAX_EVENTS);
            persistTimeline(next);
            return next;
          });
          notifyListeners(bridgeEvent);
        }
        return;
      }
      if (typeof message?.data === "string" && message.data.trim() === "pong") {
        setDeviceOnlineState(LOCAL_BRIDGE_DEVICE_ID, true);
      }
    };
  } catch {
    setEventBusRuntime((prev) => ({
      ...prev,
      localBridgeConnected: false,
      localBridgeStatus: "error",
      localBridgeError: LOCAL_BRIDGE_ERROR_MESSAGE
    }));
  }
}

async function hydrateLocalBridgeNodeInfo() {
  try {
    const response = await fetch(localBridgeHttpUrl("/v1/local/node/info.pb"), { cache: "no-store" });
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
        bridgeUrl: localBridgeWsUrl(info.eventbusWsPath || "/v1/local/eventbus/ws"),
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
    setEventBusRuntime((prev) => ({
      ...prev,
      localBridgeConnected: false,
      localBridgeStatus: "error",
      localBridgeError: LOCAL_BRIDGE_ERROR_MESSAGE
    }));
  }
}

function retryLocalBridgeConnection() {
  if (typeof window !== "undefined" && localBridgeConnectTimer) {
    window.clearTimeout(localBridgeConnectTimer);
    localBridgeConnectTimer = null;
  }
  if (localBridgeSocket) {
    try {
      localBridgeSocket.close();
    } catch {
      // ignore close failures
    }
    localBridgeSocket = null;
  }
  initializeLocalBridgeSocket();
}

function syncRemoteEventBusSnapshot(remoteNodeId, events = []) {
  const _remoteNodeId = remoteNodeId;
  const _events = events;
  // Bridge-only mode: snapshot sync is disabled in browser runtime.
}

export {
  eventBusRuntime,
  eventTimeline,
  initializeBrowserEventBus,
  publishEvent,
  subscribeEvent,
  syncRemoteEventBusSnapshot,
  retryLocalBridgeConnection
};
