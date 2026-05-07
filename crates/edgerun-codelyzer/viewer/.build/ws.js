
/**
 * WebSocket communication layer — ALL frontend↔backend traffic goes through WS.
 *
 * Protocol (Binary-first):
 *   Client → Server (JSON text frame):
 *     { type: "request", id: number, action: string, payload: object }
 *     { type: "client_ready" }
 *     { type: "debug_log", level: string, message: string }
 *     { type: "eval_result", id: number, result: string, error: string }
 *
 *   Client → Server (Binary frame):
 *     WsRequest proto message
 *
 *   Server → Client (Binary frame):
 *     WsMessage proto with one of:
 *       - graph_data: GraphData proto (initial/full graph)
 *       - graph_update: GraphUpdate proto (incremental updates)
 *       - response: WsResponse proto with result data
 *       - raw_json: string (for debugging/backwards compat)
 *
 *   Server → Client (Text frame - backwards compat):
 *     JSON messages for debugging
 */
import {
  setWsConnected,
  loadGraph,
  setChatMessages,
  setChatBusy,
  setChatSessionId,
} from "./store.js";

import {
  GraphData,
  GraphUpdate,
  WsMessage,
  WsResponse,
  GetFileResponse,
  DirListing,
  GetViewResponse,
  GetDiagnosticsResponse,
  GetFilesResponse,
  AnalyzeResponse,
  ApplyEditResponse,
  GetReposResponse,
  DiscoverReposResponse,
  AddRepoResponse,
  RemoveRepoResponse,
  SwitchRepoResponse,
  ToolUseResponse,
  QueryResult,
  StatsResponse,
  XrefResponse,
  FilesResponse,
  GetConfigResponse,
} from "./generated/codeanalyzer_pb.js";

let ws = null;
let nextId = 1;
const pendingRequests = new Map();

const REQUEST_TIMEOUT = 30_000;

let wsReadyResolve = null;
const wsReady = new Promise((resolve) => {
  wsReadyResolve = resolve;
});

function wsRequest(action, payload) {
  return new Promise((resolve, reject) => {
    if (!ws || ws.readyState !== WebSocket.OPEN) {
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
  pendingRequests.set(id, { resolve, reject, timeout, action });

  const req = { type: "request", id, action, payload };
  ws.send(JSON.stringify(req));
}

function handleBinaryMessage(data) {
  try {
    const msg = WsMessage.deserializeBinary(data);
    const msgCase = msg.getMsgCase();

    switch (msgCase) {
      case WsMessage.MsgCase.GRAPH_DATA: {
        const graph = msg.getGraphData();
        loadGraphFromProto(graph);
        break;
      }
      case WsMessage.MsgCase.GRAPH_UPDATE: {
        const update = msg.getGraphUpdate();
        applyGraphUpdate(update);
        break;
      }
      case WsMessage.MsgCase.RAW_JSON: {
        const json = msg.getRawJson();
        if (json) {
          try {
            const parsed = JSON.parse(json);
            if (parsed.type === "graph_data") {
              loadGraph(parsed);
            }
          } catch {
          }
        }
        break;
      }
      case WsMessage.MsgCase.RESPONSE: {
        const resp = msg.getResponse();
        handleWsResponse(resp);
        break;
      }
      case WsMessage.MsgCase.EVAL_COMMAND: {
        const cmd = msg.getEvalCommand();
        if (cmd) {
          handleEvalProto(cmd);
        }
        break;
      }
      default:
        break;
    }
  } catch (e) {
    console.error("Failed to decode binary message:", e);
  }
}

function handleWsResponse(resp) {
  const id = resp.getId();
  const pending = pendingRequests.get(id);
  if (!pending) return;

  clearTimeout(pending.timeout);
  pendingRequests.delete(id);

  const resultCase = resp.getResultCase();
  if (resultCase === WsResponse.ResultCase.ERROR) {
    pending.reject(new Error(resp.getError()));
  } else if (resultCase === WsResponse.ResultCase.DATA) {
    const data = resp.getData();
    if (data && data.length > 0) {
      try {
        const decoded = decodeResponseData(data, pending.action);
        pending.resolve(decoded);
      } catch (e) {
        pending.reject(e);
      }
    } else {
      pending.resolve(null);
    }
  }
}

function decodeResponseData(data, action) {
  const bytes = new Uint8Array(data);

  switch (action) {
    case "get_file":
      return GetFileResponse.deserializeBinary(bytes).toObject();
    case "get_fs":
      return DirListing.deserializeBinary(bytes).toObject();
    case "get_view":
      return GetViewResponse.deserializeBinary(bytes).toObject();
    case "get_diagnostics":
      return GetDiagnosticsResponse.deserializeBinary(bytes).toObject();
    case "get_files":
      return GetFilesResponse.deserializeBinary(bytes).toObject();
    case "analyze":
      return AnalyzeResponse.deserializeBinary(bytes).toObject();
    case "apply_edit":
      return ApplyEditResponse.deserializeBinary(bytes).toObject();
    case "get_repos":
      return GetReposResponse.deserializeBinary(bytes).toObject();
    case "discover_repos":
      return DiscoverReposResponse.deserializeBinary(bytes).toObject();
    case "add_repo":
      return AddRepoResponse.deserializeBinary(bytes).toObject();
    case "remove_repo":
      return RemoveRepoResponse.deserializeBinary(bytes).toObject();
    case "switch_repo":
      return SwitchRepoResponse.deserializeBinary(bytes).toObject();
    case "chat":
      return ToolUseResponse.deserializeBinary(bytes).toObject();
    case "get_graph":
      return GraphData.deserializeBinary(bytes).toObject();
    case "query":
      return QueryResult.deserializeBinary(bytes).toObject();
    case "stats":
      return StatsResponse.deserializeBinary(bytes).toObject();
    case "xref":
      return XrefResponse.deserializeBinary(bytes).toObject();
    case "files":
      return FilesResponse.deserializeBinary(bytes).toObject();
    case "get_config":
      return GetConfigResponse.deserializeBinary(bytes).toObject();
    default:
      try {
        return JSON.parse(new TextDecoder().decode(bytes));
      } catch {
        return new TextDecoder().decode(bytes);
      }
  }
}

function loadGraphFromProto(graphProto) {
  const nodes = graphProto.getNodesList().map(n => ({
    id: n.getId(),
    name: n.getName(),
    file: n.getFile(),
    language: n.getLanguage(),
    is_static: n.getIsStatic(),
    connections: n.getConnections(),
    tags: n.getTagsList(),
    commit: n.getCommit(),
  }));

  const edges = graphProto.getEdgesList().map(e => ({
    source: e.getSource(),
    target: e.getTarget(),
    kind: e.getKind(),
  }));

  const tag_groups = graphProto.getTagGroupsList().map(t => ({
    name: t.getName(),
    tags: t.getTagsList(),
    color: t.getColor(),
  }));

  loadGraph({ nodes, edges, tag_groups });
}

function applyGraphUpdate(update) {
  const nodesMap = window.__nodesMap;
  if (!nodesMap) return;

  if (update.getNodes()) {
    const nodeUpdate = update.getNodes();

    const added = nodeUpdate.getAddedList();
    for (const n of added) {
      const node = {
        id: n.getId(),
        name: n.getName(),
        file: n.getFile(),
        language: n.getLanguage(),
        is_static: n.getIsStatic(),
        connections: n.getConnections(),
        tags: n.getTagsList(),
        commit: n.getCommit(),
      };
      nodesMap.set(node.id, node);
    }

    const modified = nodeUpdate.getModifiedList();
    for (const n of modified) {
      const node = nodesMap.get(n.getId());
      if (node) {
        node.name = n.getName();
        node.file = n.getFile();
        node.language = n.getLanguage();
        node.is_static = n.getIsStatic();
        node.connections = n.getConnections();
        node.tags = n.getTagsList();
        node.commit = n.getCommit();
      }
    }

    const removed = nodeUpdate.getRemovedList();
    for (const id of removed) {
      nodesMap.delete(id);
    }
  }

  if (update.getEdges()) {
    const edgeUpdate = update.getEdges();
  }
}

async function handleEvalProto(cmd) {
  const id = cmd.getId();
  const code = cmd.getCode();
  try {
    const result = eval(code);
    let serialized;
    try {
      serialized = JSON.stringify(result);
    } catch {
      serialized = String(result);
    }
    sendEvalResult(id, serialized, null);
  } catch (e) {
    sendEvalResult(id, null, e instanceof Error ? e.message : String(e));
  }
}

function sendEvalResult(id, result, error) {
  if (ws?.readyState === WebSocket.OPEN) {
    const msg = {
      type: "eval_result",
      id,
      result,
      error,
    };
    ws.send(JSON.stringify(msg));
  }
}

export function connectWS(backendUrl) {
  const protocol = location.protocol === "https:" ? "wss:" : "ws:";
  const url = backendUrl ?? `${protocol}//localhost:13337/ws`;

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
      ws.send(JSON.stringify({ type: "client_ready" }));
      wsReadyResolve?.();
      wsReadyResolve = null;
    };

    ws.onmessage = async (ev) => {
      try {
        if (ev.data instanceof ArrayBuffer) {
          handleBinaryMessage(new Uint8Array(ev.data));
        } else {
          const text = await ev.data.text();
          const msg = JSON.parse(text);
          switch (msg.type) {
            case "response":
            case "response_error":
              handleJsonResponse(msg);
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
        }
      } catch (e) {
      }
    };

    ws.onclose = () => {
      setWsConnected(false);
      for (const [id, pending] of pendingRequests) {
        clearTimeout(pending.timeout);
        pending.reject(new Error("WebSocket disconnected"));
        pendingRequests.delete(id);
      }
      reconnectTimeout = setTimeout(doConnect, reconnectDelay);
      reconnectDelay = Math.min(reconnectDelay * 1.5, MAX_RECONNECT_DELAY);
    };

    window.__ws = ws;
  }

  doConnect();
}

function handleJsonResponse(msg) {
  if (msg.type === "response_error") {
    for (const [id, pending] of pendingRequests) {
      if (pending.action) {
        pending.reject(new Error(msg.error));
        pendingRequests.delete(id);
        return;
      }
    }
  } else if (msg.result) {
    for (const [id, pending] of pendingRequests) {
      if (pending.action === "get_graph" || pending.action === "get_view") {
        pending.resolve(msg.result);
        pendingRequests.delete(id);
        return;
      }
    }
  }
}

async function handleEval(msg) {
  const id = msg.id ?? 0;
  const code = msg.code ?? "";
  try {
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

export async function loadGraphFromServer() {
  const data = await wsRequest("get_graph", {});
  if (data && data.nodes) {
    loadGraph(data);
  }
  return data;
}

export async function getFileContent(path) {
  const result = await wsRequest("get_file", { path });
  return result.content;
}

export async function getDirectoryListing(path) {
  const result = await wsRequest("get_fs", { path });
  return result.entries || [];
}

export async function loadView(viewType) {
  const result = await wsRequest("get_view", { view: viewType });
  return result.data || { nodes: [], edges: [] };
}

export async function getDiagnostics(filePath) {
  const result = await wsRequest("get_diagnostics", { path: filePath });
  return result.diagnostics || [];
}

export async function getFilesWithDependencies() {
  const result = await wsRequest("get_files", {});
  return result.files || [];
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
  const result = await wsRequest("get_repos", {});
  return result.repos || { repos: [], active: null };
}

export async function discoverRepos() {
  const result = await wsRequest("discover_repos", {});
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

export function isConnected() {
  return ws?.readyState === WebSocket.OPEN;
}
