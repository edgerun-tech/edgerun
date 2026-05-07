/**
 * Application state store using Solid.js signals.
 */
import { createSignal } from "solid-js";

// ─── Graph State ──────────────────────────────────────────────────────

const [graphData, setGraphData] = createSignal({ nodes: [], edges: [], tag_groups: [] });
const [nodesMap, setNodesMap] = createSignal(new Map());
const [edges, setEdges] = createSignal([]);
const [tagGroups, setTagGroups] = createSignal([]);
const [activeTagFilters, setActiveTagFilters] = createSignal(new Set());
const [focusedNodeId, setFocusedNodeId] = createSignal(null);
const [selectedNodeIds, setSelectedNodeIds] = createSignal(new Set());
const [highlightedNodeIds, setHighlightedNodeIds] = createSignal(new Set());
const [hoveredNodeId, setHoveredNodeId] = createSignal(null);
const [currentView, setCurrentView] = createSignal("functions");
export { currentView, setCurrentView };
export { graphData, setGraphData, nodesMap, setNodesMap, edges, setEdges, tagGroups, setTagGroups, activeTagFilters, setActiveTagFilters, focusedNodeId, setFocusedNodeId, selectedNodeIds, setSelectedNodeIds, highlightedNodeIds, setHighlightedNodeIds, hoveredNodeId, setHoveredNodeId };

export function loadGraph(data) {
  const map = new Map();
  for (const node of data.nodes) {
    map.set(node.id, { ...node });
  }
  setNodesMap(map);
  setEdges(data.edges || []);
  setTagGroups(data.tag_groups || []);
  setGraphData(data);
  window.__nodesMap = map;
}

export function updateNodesFromProto(protoNodes) {
  setNodesMap((prev) => {
    const next = new Map(prev);
    for (const n of protoNodes) {
      next.set(n.id, n);
    }
    window.__nodesMap = next;
    return next;
  });
}

export function removeNodesFromProto(nodeIds) {
  setNodesMap((prev) => {
    const next = new Map(prev);
    for (const id of nodeIds) {
      next.delete(id);
    }
    window.__nodesMap = next;
    return next;
  });
}

export function toggleTagFilter(tag) {
  setActiveTagFilters(prev => {
    const next = new Set(prev);
    if (next.has(tag)) {
      next.delete(tag);
    } else {
      next.add(tag);
    }
    return next;
  });
}

export function clearTagFilters() {
  setActiveTagFilters(new Set());
}

let filteredNodesCache = null;
let filteredNodesCacheKey = null;

export function getFilteredNodes() {
  const filters = activeTagFilters();
  const nodes = nodesMap();

  // Generate cache key from filters + node count
  const filterKey = [...filters].sort().join(',');
  const cacheKey = `${filterKey}:${nodes.size}`;

  // Return cached result if key matches
  if (cacheKey === filteredNodesCacheKey && filteredNodesCache !== null) {
    return filteredNodesCache;
  }

  if (filters.size === 0) {
    filteredNodesCache = null;
    filteredNodesCacheKey = null;
    return null;
  }

  const result = new Map();
  for (const [id, node] of nodes) {
    const tags = node.tags || [];
    if ([...filters].every(f => tags.includes(f))) {
      result.set(id, node);
    }
  }

  filteredNodesCache = result;
  filteredNodesCacheKey = cacheKey;
  return result;
}

export function getFilteredNodeCount() {
  const filtered = getFilteredNodes();
  return filtered ? filtered.size : nodesMap().size;
}

export function updateNodePosition(id, x, y) {
  setNodesMap((prev) => {
    const next = new Map(prev);
    const node = next.get(id);
    if (node) {
      next.set(id, { ...node, x, y, vx: 0, vy: 0 });
    }
    return next;
  });
}

export function clearSelection() {
  setSelectedNodeIds(new Set());
  setHighlightedNodeIds(new Set());
  setFocusedNodeId(null);
}

export function setFocusedNodeWithHighlight(nodeId) {
  setFocusedNodeId(nodeId);
  if (nodeId) {
    setSelectedNodeIds(new Set([nodeId]));
    const neighbors = getNeighbors(nodeId, 1);
    neighbors.add(nodeId);
    setHighlightedNodeIds(neighbors);
  } else {
    setSelectedNodeIds(new Set());
    setHighlightedNodeIds(new Set());
  }
}

export function getNeighbors(nodeId, depth) {
  if (depth <= 0) return new Set();
  const result = new Set();
  const allEdges = edges();
  const visited = new Set([nodeId]);
  let frontier = new Set([nodeId]);

  for (let d = 0; d < depth; d++) {
    const next = new Set();
    for (const current of frontier) {
      for (const edge of allEdges) {
        if (edge.source === current && !visited.has(edge.target)) {
          visited.add(edge.target);
          next.add(edge.target);
          result.add(edge.target);
        }
        if (edge.target === current && !visited.has(edge.source)) {
          visited.add(edge.source);
          next.add(edge.source);
          result.add(edge.source);
        }
      }
    }
    frontier = next;
  }

  return result;
}

// ─── Camera State ─────────────────────────────────────────────────────

const [zoom, setZoom] = createSignal(1);
const [panX, setPanX] = createSignal(0);
const [panY, setPanY] = createSignal(0);
const [rotation, setRotation] = createSignal(0);

export { zoom, setZoom, panX, setPanX, panY, setPanY, rotation, setRotation };

// ─── UI State ─────────────────────────────────────────────────────────

const [sidebarOpen, setSidebarOpen] = createSignal(true);
const [codePanelOpen, setCodePanelOpen] = createSignal(false);
const [chatPanelOpen, setChatPanelOpen] = createSignal(false);
const [diagnosticsPanelOpen, setDiagnosticsPanelOpen] = createSignal(false);
const [fileExplorerOpen, setFileExplorerOpen] = createSignal(false);
const [repoPanelOpen, setRepoPanelOpenLocal] = createSignal(false);
const [showLabels, setShowLabels] = createSignal(true);
const [glDebugOpen, setGlDebugOpen] = createSignal(false);
const [glDebugText, setGlDebugText] = createSignal("");

export {
  sidebarOpen,
  setSidebarOpen,
  codePanelOpen,
  setCodePanelOpen,
  chatPanelOpen,
  setChatPanelOpen,
  diagnosticsPanelOpen,
  setDiagnosticsPanelOpen,
  fileExplorerOpen,
  setFileExplorerOpen,
  repoPanelOpen,
  setRepoPanelOpenLocal,
  showLabels,
  setShowLabels,
  glDebugOpen,
  setGlDebugOpen,
  glDebugText,
  setGlDebugText,
};

// ─── Panel widths (resizable) ─────────────────────────────────────────

const [sidebarWidth, setSidebarWidth] = createSignal(300);
const [codePanelWidth, setCodePanelWidth] = createSignal(500);
const [chatPanelWidth, setChatPanelWidth] = createSignal(420);
const [diagnosticsPanelWidth, setDiagnosticsPanelWidth] = createSignal(480);
const [fileExplorerWidth, setFileExplorerWidth] = createSignal(350);

export {
  sidebarWidth,
  setSidebarWidth,
  codePanelWidth,
  setCodePanelWidth,
  chatPanelWidth,
  setChatPanelWidth,
  diagnosticsPanelWidth,
  setDiagnosticsPanelWidth,
  fileExplorerWidth,
  setFileExplorerWidth,
};

// ─── Connection State ─────────────────────────────────────────────────

const [wsConnected, setWsConnected] = createSignal(false);
export { wsConnected, setWsConnected };

// ─── Performance State ────────────────────────────────────────────────

const [fps, setFps] = createSignal(0);
const [loading, setLoading] = createSignal(false);
const [loadingMessage, setLoadingMessage] = createSignal("");

export { fps, setFps, loading, setLoading, loadingMessage, setLoadingMessage };

// ─── Search State ─────────────────────────────────────────────────────

const [searchQuery, setSearchQuery] = createSignal("");
const [searchResults, setSearchResults] = createSignal([]);
const [searchOpen, setSearchOpen] = createSignal(false);

export {
  searchQuery,
  setSearchQuery,
  searchResults,
  setSearchResults,
  searchOpen,
  setSearchOpen,
};

// ─── Chat State ───────────────────────────────────────────────────────

const [chatMessages, setChatMessages] = createSignal([]);
const [chatBusy, setChatBusy] = createSignal(false);
const [chatContextNode, setChatContextNode] = createSignal(null);
const [chatSessionId, setChatSessionId] = createSignal(crypto.randomUUID());

export {
  chatMessages,
  setChatMessages,
  chatBusy,
  setChatBusy,
  chatContextNode,
  setChatContextNode,
  chatSessionId,
  setChatSessionId,
};

// ─── Code Viewer State ────────────────────────────────────────────────

const [codeFilePath, setCodeFilePath] = createSignal("");
const [codeContent, setCodeContent] = createSignal("");
const [diffCode, setDiffCode] = createSignal("");
const [codeTab, setCodeTab] = createSignal("current");
const [pendingEdit, setPendingEdit] = createSignal(null);

export {
  codeFilePath,
  setCodeFilePath,
  codeContent,
  setCodeContent,
  diffCode,
  setDiffCode,
  codeTab,
  setCodeTab,
  pendingEdit,
  setPendingEdit,
};

// ─── File Explorer State ──────────────────────────────────────────────

const [fsEntries, setFsEntries] = createSignal(new Map());
const [selectedFilePath, setSelectedFilePath] = createSignal(null);

export { fsEntries, setFsEntries, selectedFilePath, setSelectedFilePath };

// ─── Files Panel State ────────────────────────────────────────────────

const [fileDependencies, setFileDependencies] = createSignal([]);

export { fileDependencies, setFileDependencies };

// ─── Diagnostics State ────────────────────────────────────────────────

const [diagnosticsFile, setDiagnosticsFile] = createSignal(null);
const [diagnosticsItems, setDiagnosticsItems] = createSignal([]);
const [diagnosticsLoading, setDiagnosticsLoading] = createSignal(false);

export {
  diagnosticsFile,
  setDiagnosticsFile,
  diagnosticsItems,
  setDiagnosticsItems,
  diagnosticsLoading,
  setDiagnosticsLoading,
};

// ─── Repo State ───────────────────────────────────────────────────────

const [repos, setRepos] = createSignal([]);
const [activeRepoPath, setActiveRepoPath] = createSignal(null);

export { repos, setRepos, activeRepoPath, setActiveRepoPath };

// ─── Overlay State ────────────────────────────────────────────────────

const [boxSelectRect, setBoxSelectRect] = createSignal(null);

const [rotationIndicator, setRotationIndicator] = createSignal(null);

export { boxSelectRect, setBoxSelectRect, rotationIndicator, setRotationIndicator };
