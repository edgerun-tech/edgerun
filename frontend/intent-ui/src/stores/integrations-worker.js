let worker = null;
let sequence = 1;
const pending = new Map();

function ensureWorker() {
  if (typeof window === "undefined") return null;
  if (worker) return worker;
  worker = new Worker("/intent-ui/workers/integrations.worker.js", { type: "module" });
  worker.onmessage = (event) => {
    const data = event?.data || {};
    const id = String(data.id || "").trim();
    if (!id || !pending.has(id)) return;
    const request = pending.get(id);
    pending.delete(id);
    if (data.ok) {
      request.resolve(data.result);
      return;
    }
    request.reject(new Error(String(data.error || "integration worker request failed")));
  };
  worker.onerror = () => {
    for (const request of pending.values()) {
      request.reject(new Error("integration worker crashed"));
    }
    pending.clear();
    worker = null;
  };
  return worker;
}

function initializeIntegrationWorker() {
  ensureWorker();
}

function callIntegrationWorker(type, payload = {}) {
  const activeWorker = ensureWorker();
  if (!activeWorker) {
    return Promise.reject(new Error("integration worker unavailable"));
  }
  const id = `integration-worker-${sequence++}`;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    activeWorker.postMessage({
      id,
      type,
      payload
    });
  });
}

export {
  initializeIntegrationWorker,
  callIntegrationWorker
};
