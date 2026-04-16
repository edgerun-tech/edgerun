/**
 * WebSocket communication layer — ALL frontend↔backend traffic goes through WS.
 *
 * Protocol:
 *   Client → Server:
 *     { type: "request", id: number, action: string, payload: object }
 *     { type: "client_ready" }
 *     { type: "debug_log", level: string, message: string }
 *     { type: "eval_result", id: number, result: string, error: string }
 *
 *   Server → Client:
 *     { type: "response", id: number, result: object }
 *     { type: "response_error", id: number, error: string }
 *     { type: "graph_data", nodes: [], edges: [] }
 *     { type: "ai_command", command: object }
 *     { type: "eval", id: number, code: string }
 */
import {
  setWsConnected,
  loadGraph,
  setChatMessages,
  setChatBusy,
  setChatSessionId,
} from "./store.js";

// ─── Request/Response Manager ─────────────────────────────────────────

let ws = null;
let nextId = 1;
const pendingRequests = new Map();

const REQUEST_TIMEOUT = 30_000;

// Wait for WS to be connected
let wsReadyResolve = null;
const wsReady = new Promise((resolve) => {
  wsReadyResolve = resolve;
});

/**
 * Send a request and wait for a response.
 */
function wsRequest(action, payload) {
  return new Promise((resolve, reject) => {
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      // Wait for connection
      wsReady.then(() => {
        if (!ws || ws.readyState !== WebSocket.OPEN) {
          reject(new Error("WebSocket not connected"));
          return;
        }
        doSend(action, payload, resolve, reject);
      }).catch(reject);
      return;
    }
    doSend(action, payload, resolve, reject);
  });
}

function doSend(action, payload, resolve, reject) {
  const id = nextId++;
  const timeout = setTimeout(() => {
    pendingRequests.delete(id);
    reject(new Error(`Request "${action}" timed out after ${REQUEST_TIMEOUT}ms`));
  }, REQUEST_TIMEOUT);
  pendingRequests.set(id, { resolve, reject, timeout });
  const req = { type: "request", id, action, payload };
  ws.send(JSON.stringify(req));
}

function handleResponse(msg) {
  const pending = pendingRequests.get(msg.id);
  if (!pending) return;
  clearTimeout(pending.timeout);
  pendingRequests.delete(msg.id);
  if (msg.type === "response_error") {
    pending.reject(new Error(msg.error));
  } else {
    pending.resolve(msg.result);
  }
}

// ─── Connection ───────────────────────────────────────────────────────

export function connectWS(backendUrl) {
  // Always connect directly to the Rust backend on port 8080.
  // In production, the Rust server serves the frontend on the same port.
  // In dev, the Bun dev server serves the frontend but WS goes to port 8080.
  const protocol = location.protocol === "https:" ? "wss:" : "ws:";
  const url = backendUrl ?? `${protocol}//localhost:8080/ws`;

  function doConnect() {
    try {
      ws = new WebSocket(url);
    } catch {
      setWsConnected(false);
      setTimeout(doConnect, 1000);
      return;
    }

    ws.onopen = () => {
      setWsConnected(true);
      ws?.send(JSON.stringify({ type: "client_ready" }));
      wsReadyResolve?.();
      wsReadyResolve = null;
    };

    ws.onmessage = (ev) => {
      try {
        const msg = JSON.parse(ev.data);
        switch (msg.type) {
          case "response":
          case "response_error":
            handleResponse(msg);
            break;
          case "graph_data":
            loadGraph(msg);
            break;
          case "eval":
            handleEval(msg);
            break;
          default:
            break;
        }
      } catch {
        // ignore parse errors
      }
    };

    ws.onclose = () => {
      setWsConnected(false);
      // Reject all pending requests
      for (const [id, pending] of pendingRequests) {
        clearTimeout(pending.timeout);
        pending.reject(new Error("WebSocket disconnected"));
        pendingRequests.delete(id);
      }
      setTimeout(doConnect, 1500);
    };

    window.__ws = ws;
  }

  doConnect();
}

async function handleEval(msg) {
  const id = msg.id ?? 0;
  const code = msg.code ?? "";
  try {
    // eslint-disable-next-line no-eval
    const result = eval(code);
    let serialized;
    try {
      serialized = JSON.stringify(result);
    } catch {
      serialized = String(result);
    }
    ws?.send(JSON.stringify({ type: "eval_result", id, result: serialized, error: null }));
  } catch (e) {
    ws?.send(JSON.stringify({
      type: "eval_result",
      id,
      result: null,
      error: e instanceof Error ? e.message : String(e),
    }));
  }
}

// ─── API Methods (all via WS) ─────────────────────────────────────────

export async function loadGraphFromServer() {
  const data = await wsRequest("get_graph");
  loadGraph(data);
  return data;
}

export async function getFileContent(path) {
  const result = await wsRequest("get_file", { path });
  return result.content;
}

export async function getDirectoryListing(path) {
  const result = await wsRequest("get_fs", { path });
  return result.entries;
}

export async function loadView(viewType) {
  const result = await wsRequest("get_view", { view: viewType });
  return result;
}

export async function getDiagnostics(filePath) {
  const result = await wsRequest("get_diagnostics", { path: filePath });
  return result;
}

export async function getFilesWithDependencies() {
  const result = await wsRequest("get_files");
  return result;
}

export async function analyzePath(path) {
  const result = await wsRequest("analyze", { path });
  return result;
}

export async function applyEdit(path, content) {
  const result = await wsRequest("apply_edit", { path, content });
  return result;
}

export async function getRepos() {
  const result = await wsRequest("get_repos");
  return result;
}

export async function discoverRepos() {
  const result = await wsRequest("discover_repos");
  return result;
}

export async function addRepo(path) {
  const result = await wsRequest("add_repo", { path });
  return result;
}

export async function removeRepo(path) {
  const result = await wsRequest("remove_repo", { path });
  return result;
}

export async function switchRepo(path) {
  const result = await wsRequest("switch_repo", { path });
  return result;
}

export async function sendChatMessage(
  text,
  contextNode,
  sessionId,
  onReply,
) {
  const context = contextNode
    ? `Function: ${contextNode.name}\nFile: ${contextNode.file}\nLanguage: ${contextNode.language}`
    : null;

  setChatBusy(true);

  const userMsg = {
    id: crypto.randomUUID(),
    role: "user",
    text,
    timestamp: Date.now(),
  };
  setChatMessages((prev) => [...prev, userMsg]);

  try {
    const result = await wsRequest("chat", {
      message: text,
      context,
      session_id: sessionId,
    });

    const reply = {
      id: crypto.randomUUID(),
      role: "assistant",
      text: result.reply ?? "",
      tools: Array.isArray(result.tools)
        ? result.tools.map((t) => ({
            name: t.name ?? "unknown",
            args: typeof t.args === "string" ? t.args : JSON.stringify(t.args),
            output: t.output ?? undefined,
            content: t.content ?? undefined,
          }))
        : undefined,
      timestamp: Date.now(),
    };
    onReply(reply);
  } catch (e) {
    const errMsg = {
      id: crypto.randomUUID(),
      role: "error",
      text: e instanceof Error ? e.message : String(e),
      timestamp: Date.now(),
    };
    onReply(errMsg);
  } finally {
    setChatBusy(false);
  }
}

export function sendLog(level, message) {
  if (ws?.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({ type: "debug_log", level, message }));
  }
}

/**
 * Check if WS is connected.
 */
export function isConnected() {
  return ws?.readyState === WebSocket.OPEN;
}

// Benchmark comment