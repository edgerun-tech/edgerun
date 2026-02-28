const EVENTBUS_MAX_EVENTS = 300;

let runtime = {
  engine: "worker-js",
  wasmLoaded: false,
  workerReady: false,
  peers: [],
  lastSyncAt: ""
};

let timeline = [];
let wasmExports = null;
let jsSeq = 1;

function notify(type, payload = {}) {
  postMessage({ type, ...payload });
}

function hashTopic(value) {
  const text = String(value || "");
  let hash = 2166136261 >>> 0;
  for (let i = 0; i < text.length; i += 1) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 16777619) >>> 0;
  }
  return hash >>> 0;
}

function nextEventCode(topic, ts) {
  if (wasmExports?.bus_event_code) {
    return wasmExports.bus_event_code(hashTopic(topic), Number(ts >>> 0)) >>> 0;
  }
  const seq = jsSeq;
  jsSeq = (jsSeq + 1) >>> 0;
  return (hashTopic(topic) ^ Number(ts >>> 0) ^ seq) >>> 0;
}

function publish(topic, payload = {}, meta = {}) {
  const now = Date.now();
  const code = nextEventCode(topic, now);
  const event = {
    id: `evt-${now}-${code.toString(16)}`,
    topic: String(topic || "event.unknown"),
    payload,
    meta,
    createdAt: new Date(now).toISOString()
  };
  timeline = [...timeline, event].slice(-EVENTBUS_MAX_EVENTS);
  notify("event", { event, runtime, timelineSize: timeline.length });
}

async function initWorker(wasmUrl) {
  try {
    const response = await fetch(wasmUrl, { cache: "no-store" });
    if (!response.ok) throw new Error(`wasm load failed (${response.status})`);
    const bytes = await response.arrayBuffer();
    const { instance } = await WebAssembly.instantiate(bytes, {});
    wasmExports = instance?.exports || null;
    if (!wasmExports?.bus_event_code) throw new Error("missing bus_event_code export");
    runtime = { ...runtime, engine: "worker-wasm", wasmLoaded: true, workerReady: true };
  } catch {
    wasmExports = null;
    runtime = { ...runtime, engine: "worker-js", wasmLoaded: false, workerReady: true };
  }
  notify("runtime", { runtime });
  publish("eventbus.runtime", { engine: runtime.engine }, { source: "eventbus.worker" });
}

onmessage = (message) => {
  const data = message?.data || {};
  if (data.type === "init") {
    initWorker(String(data.wasmUrl || "/intent-ui/eventbus.wasm"));
    return;
  }
  if (data.type === "publish") {
    publish(data.topic, data.payload, data.meta);
    return;
  }
  if (data.type === "snapshot") {
    const remoteNodeId = String(data.remoteNodeId || "").trim();
    const events = Array.isArray(data.events) ? data.events : [];
    if (remoteNodeId) {
      runtime = {
        ...runtime,
        peers: Array.from(new Set([...(runtime.peers || []), remoteNodeId])),
        lastSyncAt: new Date().toISOString()
      };
    }
    for (const event of events) {
      if (!event || typeof event !== "object") continue;
      const id = String(event.id || "").trim();
      if (!id) continue;
      if (timeline.some((item) => item.id === id)) continue;
      timeline.push({
        id,
        topic: String(event.topic || "event.remote"),
        payload: event.payload || {},
        meta: { ...(event.meta || {}), remoteNodeId },
        createdAt: String(event.createdAt || new Date().toISOString())
      });
    }
    timeline = timeline.slice(-EVENTBUS_MAX_EVENTS);
    notify("runtime", { runtime });
    notify("timeline", { events: timeline.slice() });
    return;
  }
  if (data.type === "get_timeline") {
    notify("timeline", { events: timeline.slice() });
  }
};
