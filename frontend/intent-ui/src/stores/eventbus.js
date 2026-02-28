import { createSignal } from "solid-js";

const EVENTBUS_TIMELINE_KEY = "intent-ui-eventbus-timeline-v1";
const EVENTBUS_MAX_EVENTS = 300;

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
  engine: "js",
  wasmLoaded: false,
  peers: [],
  lastSyncAt: ""
});

const [eventTimeline, setEventTimeline] = createSignal(readTimeline());

const topicListeners = new Map();

function publishEvent(topic, payload = {}, meta = {}) {
  const event = {
    id: `${topic}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    topic,
    payload,
    meta,
    createdAt: new Date().toISOString()
  };
  setEventTimeline((prev) => {
    const next = [...prev, event].slice(-EVENTBUS_MAX_EVENTS);
    persistTimeline(next);
    return next;
  });
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
  return event;
}

function subscribeEvent(topic, handler) {
  if (!topicListeners.has(topic)) topicListeners.set(topic, []);
  topicListeners.get(topic).push(handler);
  return () => {
    const current = topicListeners.get(topic) || [];
    topicListeners.set(topic, current.filter((item) => item !== handler));
  };
}

async function initializeBrowserEventBus() {
  if (typeof window === "undefined") return;
  try {
    const response = await fetch("/intent-ui/eventbus.wasm", { method: "GET" });
    if (!response.ok) throw new Error("No wasm eventbus artifact");
    const bytes = await response.arrayBuffer();
    await WebAssembly.instantiate(bytes, {});
    setEventBusRuntime((prev) => ({ ...prev, engine: "wasm", wasmLoaded: true }));
    publishEvent("eventbus.runtime", { engine: "wasm" }, { source: "browser" });
  } catch {
    setEventBusRuntime((prev) => ({ ...prev, engine: "js", wasmLoaded: false }));
    publishEvent("eventbus.runtime", { engine: "js" }, { source: "browser" });
  }
}

function syncRemoteEventBusSnapshot(remoteNodeId, events = []) {
  if (!remoteNodeId || !Array.isArray(events) || events.length === 0) return;
  const normalized = events
    .filter((event) => event && typeof event === "object" && typeof event.topic === "string")
    .map((event) => ({
      id: String(event.id || `${event.topic}-${Math.random().toString(36).slice(2, 8)}`),
      topic: String(event.topic),
      payload: event.payload || {},
      meta: { ...(event.meta || {}), remoteNodeId },
      createdAt: String(event.createdAt || new Date().toISOString())
    }));
  if (normalized.length === 0) return;
  setEventTimeline((prev) => {
    const existing = new Set(prev.map((event) => event.id));
    const merged = [...prev];
    for (const event of normalized) {
      if (!existing.has(event.id)) merged.push(event);
    }
    const next = merged.slice(-EVENTBUS_MAX_EVENTS);
    persistTimeline(next);
    return next;
  });
  setEventBusRuntime((prev) => {
    const peers = Array.from(new Set([...(prev.peers || []), remoteNodeId]));
    return {
      ...prev,
      peers,
      lastSyncAt: new Date().toISOString()
    };
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
