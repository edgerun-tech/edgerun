
/**
 * WebSocket communication layer.
 *
 * Protocol is rkyv-only. Request/control handling is intentionally disabled
 * until a rkyv JS bridge is installed.
 */
import {
  setWsConnected,
  loadGraph,
  setChatMessages,
  setChatBusy,
  setChatSessionId,
} from "./store.js";


let ws = null;

function unavailableRkyvRequest(requestVariant) {
  throw new Error(`Cannot send ${requestVariant}: browser rkyv encoding is not implemented`);
}

function handleBinaryMessage(data) {
  throw new Error(`Received ${data.byteLength} byte binary frame, but browser rkyv decoding is not implemented`);
}


function sendEvalResult(id, result, error) {
  throw new Error(`Cannot send EvalResult ${id}: browser rkyv encoding is not implemented`);
}

export function connectWS(backendUrl) {
  const wsScheme = location.protocol === "https:" ? "wss:" : "ws:";
  const url = backendUrl ?? `${wsScheme}//localhost:13337/ws`;

  const MIN_RECONNECT_DELAY = 1000;
  const MAX_RECONNECT_DELAY = 30000;
  let reconnectDelay = MIN_RECONNECT_DELAY;
  let reconnectTimeout = null;

  function doConnect() {
    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout);
      reconnectTimeout = null;
    }

    try {
      ws = new WebSocket(url);
      ws.binaryType = "arraybuffer";
    } catch {
      setWsConnected(false);
      reconnectTimeout = setTimeout(doConnect, reconnectDelay);
      reconnectDelay = Math.min(reconnectDelay * 1.5, MAX_RECONNECT_DELAY);
      return;
    }

    ws.onopen = () => {
      setWsConnected(true);
      reconnectDelay = MIN_RECONNECT_DELAY;
      // ClientReady must be sent as rkyv once browser encoding exists.
    };

    ws.onmessage = async (ev) => {
      if (ev.data instanceof ArrayBuffer) {
        handleBinaryMessage(new Uint8Array(ev.data));
        return;
      }
      throw new Error("Received non-rkyv WebSocket text frame");
    };

    ws.onclose = () => {
      setWsConnected(false);
      reconnectTimeout = setTimeout(doConnect, reconnectDelay);
      reconnectDelay = Math.min(reconnectDelay * 1.5, MAX_RECONNECT_DELAY);
    };

    window.__ws = ws;
  }

  doConnect();
}


export async function loadGraphFromServer() {
  const data = unavailableRkyvRequest("GetGraph");
  if (data && data.nodes) {
    loadGraph(data);
  }
  return data;
}

export async function getFileContent(path) {
  const result = unavailableRkyvRequest("GetFile");
  return result.content;
}

export async function getDirectoryListing(path) {
  const result = unavailableRkyvRequest("GetFs");
  return result.entries || [];
}

export async function loadView(viewType) {
  const result = unavailableRkyvRequest("GetView");
  return result.data || { nodes: [], edges: [] };
}

export async function getDiagnostics(filePath) {
  const result = unavailableRkyvRequest("GetDiagnostics");
  return result.diagnostics || [];
}

export async function getFilesWithDependencies() {
  const result = unavailableRkyvRequest("GetFiles");
  return result.files || [];
}

export async function analyzePath(path) {
  const result = unavailableRkyvRequest("Analyze");
  return result;
}

export async function applyEdit(path, content) {
  const result = unavailableRkyvRequest("ApplyEdit");
  return result;
}

export async function getRepos() {
  const result = unavailableRkyvRequest("GetRepos");
  return result.repos || { repos: [], active: null };
}

export async function discoverRepos() {
  const result = unavailableRkyvRequest("DiscoverRepos");
  return result;
}

export async function addRepo(path) {
  const result = unavailableRkyvRequest("AddRepo");
  return result;
}

export async function removeRepo(path) {
  const result = unavailableRkyvRequest("RemoveRepo");
  return result;
}

export async function switchRepo(path) {
  const result = unavailableRkyvRequest("SwitchRepo");
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
    const result = unavailableRkyvRequest("Chat");

    const reply = {
      id: crypto.randomUUID(),
      role: "assistant",
      text: result.reply ?? "",
      tools: Array.isArray(result.tools)
        ? result.tools.map((t) => ({
            name: t.name ?? "unknown",
            args: t.args ?? "",
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
    throw new Error("Cannot send DebugLog: browser rkyv encoding is not implemented");
  }
}

export function isConnected() {
  return ws?.readyState === WebSocket.OPEN;
}
