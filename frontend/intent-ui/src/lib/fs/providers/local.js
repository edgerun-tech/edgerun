import { localBridgeHttpUrl } from "../../local-bridge-origin";
import { getFsNodeTargetId } from "../../../stores/fs-node-target";

function buildQuery(path) {
  const params = new URLSearchParams();
  params.set("path", path);
  const nodeId = getFsNodeTargetId();
  if (nodeId) params.set("node_id", nodeId);
  return params.toString();
}

function withNode(payload = {}) {
  const nodeId = getFsNodeTargetId();
  if (!nodeId) return payload;
  return { ...payload, node_id: nodeId };
}

const localProvider = {
  id: "local",
  label: "Local FS (Forwarded)",
  authState() {
    return "ready";
  },
  async list(path = "/") {
    const response = await fetch(localBridgeHttpUrl(`/v1/local/fs/list?${buildQuery(path)}`), { cache: "no-store" });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || !payload?.ok) {
      throw new Error(payload?.error || "Failed to list local filesystem.");
    }
    return Array.isArray(payload.entries) ? payload.entries : [];
  },
  async read(path = "/") {
    const response = await fetch(localBridgeHttpUrl(`/v1/local/fs/read?${buildQuery(path)}`), { cache: "no-store" });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || !payload?.ok) {
      throw new Error(payload?.error || "Failed to read local file.");
    }
    return typeof payload.content === "string" ? payload.content : "";
  },
  async write(path, content) {
    const response = await fetch(localBridgeHttpUrl("/v1/local/fs/write"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(withNode({ path, content: typeof content === "string" ? content : String(content) }))
    });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || !payload?.ok) {
      throw new Error(payload?.error || "Failed to write local file.");
    }
  },
  async mkdir(path) {
    const response = await fetch(localBridgeHttpUrl("/v1/local/fs/mkdir"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(withNode({ path }))
    });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || !payload?.ok) {
      throw new Error(payload?.error || "Failed to create local directory.");
    }
  },
  async delete(path) {
    const response = await fetch(localBridgeHttpUrl("/v1/local/fs/delete"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(withNode({ path }))
    });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || !payload?.ok) {
      throw new Error(payload?.error || "Failed to delete local path.");
    }
  },
  async move(from, to) {
    const response = await fetch(localBridgeHttpUrl("/v1/local/fs/move"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(withNode({ from, to }))
    });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || !payload?.ok) {
      throw new Error(payload?.error || "Failed to move local path.");
    }
  },
  async copy(from, to) {
    const response = await fetch(localBridgeHttpUrl("/v1/local/fs/copy"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(withNode({ from, to }))
    });
    const payload = await response.json().catch(() => ({}));
    if (!response.ok || !payload?.ok) {
      throw new Error(payload?.error || "Failed to copy local path.");
    }
  }
};

export { localProvider };
