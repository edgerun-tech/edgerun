/** @jsxImportSource solid-js **/
/**
 * Sidebar component — search, status, node details, controls.
 */
import { createSignal, createMemo, createEffect, For } from "solid-js";
import {
  nodesMap,
  edges,
  focusedNodeId,
  setFocusedNodeWithHighlight,
  sidebarWidth,
  wsConnected,
  fps,
  searchQuery,
  setSearchQuery,
  searchResults,
  setSearchResults,
  searchOpen,
  setSearchOpen,
  showLabels,
  setShowLabels,
  setCodePanelOpen,
  setChatPanelOpen,
  setDiagnosticsPanelOpen,
  setFileExplorerOpen,
  setRepoPanelOpenLocal,
  setLoading,
  setLoadingMessage,
  setCurrentView,
  currentView,
  setPanX,
  setPanY,
  tagGroups,
  activeTagFilters,
  toggleTagFilter,
  clearTagFilters,
} from "../store.js";
import { Search, Code2, MessageSquare, FileText, GitBranch, RotateCcw, Maximize2, FolderOpen, ScanSearch, Bug, Tags, X, Filter } from "lucide-solid";
import { loadView } from "../ws.js";

export function Sidebar() {
  let searchTimeout;
  const [neighborLimit, setNeighborLimit] = createSignal(10);

  function doSearch() {
    const query = searchQuery().trim().toLowerCase();
    if (query.length < 2) {
      setSearchResults([]);
      setSearchOpen(false);
      return;
    }
    const nodes = nodesMap();
    const results = [];
    for (const [, node] of nodes) {
      if (node._hidden) continue;
      const name = (node.name ?? "").toLowerCase();
      const file = (node.file ?? "").toLowerCase();
      if (name.includes(query) || file.includes(query) || node.id.toLowerCase().includes(query)) {
        results.push(node);
      }
      if (results.length >= 20) break;
    }
    setSearchResults(results);
    setSearchOpen(true);
  }

  function onSearchInput() {
    clearTimeout(searchTimeout);
    searchTimeout = setTimeout(doSearch, 150);
  }

  function onSearchKeydown(e) {
    if (e.key === "Enter") {
      const results = searchResults();
      if (results.length > 0) {
        selectNode(results[0].id);
      }
    }
  }

  function selectNode(id) {
    setFocusedNodeWithHighlight(id);
    setSearchQuery("");
    setSearchResults([]);
    setSearchOpen(false);
  }

  const neighbors = createMemo(() => {
    const nodeId = focusedNodeId();
    if (!nodeId) return [];
    const result = new Set();
    const edgeList = edges();
    for (const edge of edgeList) {
      if (edge.source === nodeId) result.add(edge.target);
      if (edge.target === nodeId) result.add(edge.source);
    }
    return Array.from(result);
  });

  const displayedNeighbors = createMemo(() => neighbors().slice(0, neighborLimit()));

  function fitAll() {
    const nodes = nodesMap();
    if (nodes.size === 0) return;
    let minX = Infinity;
    let minY = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;
    for (const [, node] of nodes) {
      if (node._hidden) continue;
      const x = node.x ?? 0;
      const y = node.y ?? 0;
      if (x < minX) minX = x;
      if (y < minY) minY = y;
      if (x > maxX) maxX = x;
      if (y > maxY) maxY = y;
    }
    const container = document.getElementById("canvas-container");
    if (!container) return;
    const vw = container.clientWidth;
    const vh = container.clientHeight;
    const rangeX = maxX - minX || 1;
    const rangeY = maxY - minY || 1;
    setZoom(Math.min(vw / (rangeX + 100), vh / (rangeY + 100), 3));
    setPanX(-((minX + maxX) / 2));
    setPanY(-((minY + maxY) / 2));
  }

  async function loadView(viewType) {
    if (currentView() === viewType) return;
    setCurrentView(viewType);
    setLoading(true);
    setLoadingMessage(`Loading ${viewType} view…`);
    try {
      await wsLoadView(viewType);
      setTimeout(() => {
        fitAll();
        setLoading(false);
      }, 500);
    } catch (e) {
      console.error("Failed to load view:", e);
      setLoading(false);
    }
  }

  const focusedNode = () => {
    const id = focusedNodeId();
    if (!id) return null;
    return nodesMap().get(id) ?? null;
  };

  return (
    <div class="panel shrink-0" style={{ width: `${sidebarWidth()}px` }}>
      {/* Search */}
      <div class="p-3 border-b border-border-default relative">
        <div class="relative">
          <Search class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-muted" />
          <input
            type="text"
            class="input pl-8"
            placeholder="Search functions… (Ctrl+K)"
            value={searchQuery()}
            onInput={(e) => {
              setSearchQuery(e.currentTarget.value);
              onSearchInput();
            }}
            onKeyDown={onSearchKeydown}
          />
        </div>
        {searchOpen() && searchResults().length > 0 && (
          <div class="absolute top-full left-0 right-0 max-h-72 overflow-y-auto bg-bg-tertiary border border-border-default border-t-0 rounded-b-md z-50">
            <For each={searchResults()}>
              {(node) => (
                <button
                  class="w-full px-3 py-2 text-xs text-left text-text-primary hover:bg-bg-hover truncate border-b border-border-default/50 last:border-0"
                  onClick={() => selectNode(node.id)}
                >
                  {node.name}{" "}
                  <span class="text-text-muted">({node.file ?? ""})</span>
                </button>
              )}
            </For>
          </div>
        )}
      </div>

      {/* Status bar */}
      <div class="flex gap-3 px-3 py-2 text-[11px] text-text-muted border-b border-border-default">
        <span>{nodesMap().size} nodes</span>
        <span>{edges().length} edges</span>
        <span>{fps()} fps</span>
        {activeTagFilters().size > 0 && <span class="text-accent-blue">({getFilteredNodeCount()} filtered)</span>}
        <span class="ml-auto">
          <span classList={{ "text-accent-green": wsConnected(), "text-text-muted": !wsConnected() }}>
            {wsConnected() ? "●" : "○"}
          </span>
        </span>
      </div>

      {/* Tag filters */}
      <div class="border-b border-border-default">
        <div class="flex items-center justify-between px-3 py-2">
          <div class="flex items-center gap-1 text-xs text-text-secondary">
            <Filter size={12} />
            <span>Tag Groups</span>
          </div>
          {activeTagFilters().size > 0 && (
            <button onClick={clearTagFilters} class="text-[10px] text-accent-red hover:text-accent-red/80">
              <X size={12} />
            </button>
          )}
        </div>
        <div class="flex flex-wrap gap-1 px-3 pb-2">
          <For each={tagGroups()}>
            {(group) => {
              const isActive = () => {
                const filters = activeTagFilters();
                return group.tags.some(t => filters.has(t));
              };
              return (
                <button
                  onClick={() => group.tags.forEach(t => toggleTagFilter(t))}
                  classList={{
                    "px-2 py-0.5 text-[10px] rounded-full border transition-colors": true,
                    "bg-opacity-20 text-white": isActive(),
                    "bg-bg-tertiary text-text-muted border-border-default hover:border-text-muted": !isActive()
                  }}
                  style={isActive() ? { "background-color": group.color + "33", "border-color": group.color, "color": group.color } : {}}
                >
                  {group.name}
                </button>
              );
            }}
          </For>
        </div>
      </div>

      {/* Node details */}
      <div class="flex-1 overflow-y-auto p-3">
        {focusedNode() ? (
          <div>
            <h3 class="text-xs font-semibold text-text-heading mb-2 break-all">{focusedNode().name}</h3>
            <div class="space-y-1 text-xs">
              {focusedNode().file && (
                <div class="flex justify-between">
                  <span class="text-text-secondary">File</span>
                  <span class="text-text-primary text-right max-w-[60%] break-all">{focusedNode().file}</span>
                </div>
              )}
              {focusedNode().language && (
                <div class="flex justify-between">
                  <span class="text-text-secondary">Language</span>
                  <span class="text-text-primary">{focusedNode().language}</span>
                </div>
              )}
              {focusedNode().is_static !== undefined && (
                <div class="flex justify-between">
                  <span class="text-text-secondary">Static</span>
                  <span class="text-text-primary">{focusedNode().is_static ? "yes" : "no"}</span>
                </div>
              )}
              {focusedNode().connections !== undefined && (
                <div class="flex justify-between">
                  <span class="text-text-secondary">Connections</span>
                  <span class="text-text-primary">{focusedNode().connections}</span>
                </div>
              )}
              {focusedNode().commit && (
                <div class="flex justify-between">
                  <span class="text-text-secondary">Last commit</span>
                  <span class="text-text-primary">{focusedNode().commit}</span>
                </div>
              )}
              {focusedNode().tags && focusedNode().tags.length > 0 && (
                <div class="mt-2">
                  <span class="text-text-secondary text-xs">Tags</span>
                  <div class="flex flex-wrap gap-1 mt-1">
                    <For each={focusedNode().tags}>
                      {(tag) => (
                        <button
                          onClick={() => toggleTagFilter(tag)}
                          class="px-1.5 py-0.5 text-[9px] rounded bg-bg-tertiary text-text-muted hover:text-text-primary hover:bg-bg-hover"
                        >
                          {tag}
                        </button>
                      )}
                    </For>
                  </div>
                </div>
              )}
            </div>

            {/* Neighbors */}
            {neighbors().length > 0 && (
              <div class="mt-3">
                <div class="flex justify-between text-xs">
                  <span class="text-text-secondary">Neighbors</span>
                  <span class="text-text-primary">{neighbors().length}</span>
                </div>
                <div class="mt-1 space-y-0.5">
                  <For each={displayedNeighbors()}>
                    {(nid) => {
                      const n = nodesMap().get(nid);
                      return (
                        <button
                          class="w-full text-left px-2 py-0.5 text-xs text-text-secondary rounded-sm hover:bg-bg-hover hover:text-text-primary truncate"
                          onClick={() => setFocusedNodeWithHighlight(nid)}
                        >
                          {n?.name ?? nid}
                        </button>
                      );
                    }}
                  </For>
                  {neighbors().length > neighborLimit() && (
                    <button
                      class="text-[11px] text-text-muted px-2 py-0.5"
                      onClick={() => setNeighborLimit(neighborLimit() + 10)}
                    >
                      +{neighbors().length - neighborLimit()} more
                    </button>
                  )}
                </div>
              </div>
            )}
          </div>
        ) : (
          <div class="text-xs text-text-muted text-center py-8">Select a node to see details</div>
        )}
      </div>

      {/* Controls */}
      <div class="p-3 border-t border-border-default flex flex-wrap gap-1.5 items-center">
        <button class="btn" title="Browse files" onClick={() => setFileExplorerOpen(true)}>
          <FolderOpen class="w-3 h-3 inline mr-1" />
          Browse
        </button>
        <button class="btn" title="Repositories" onClick={() => setRepoPanelOpenLocal(true)}>
          Repos
        </button>
        <button class="btn" title="Code viewer (Ctrl+Shift+C)" onClick={() => setCodePanelOpen((v) => !v)}>
          <Code2 class="w-3 h-3 inline mr-1" />
          Code
        </button>
        <button class="btn" title="Chat (Ctrl+Shift+L)" onClick={() => setChatPanelOpen((v) => !v)}>
          <MessageSquare class="w-3 h-3 inline mr-1" />
          Chat
        </button>
        <button class="btn" title="Files" onClick={() => { /* toggle files panel */ }}>
          <FileText class="w-3 h-3 inline mr-1" />
          Files
        </button>
        <button class="btn" title="Lint" onClick={() => setDiagnosticsPanelOpen((v) => !v)}>
          <ScanSearch class="w-3 h-3 inline mr-1" />
          Lint
        </button>
        <select
          class="input py-1 px-2"
          value={currentView()}
          onChange={(e) => loadView(e.currentTarget.value)}
          title="Switch view"
        >
          <option value="functions">Functions</option>
          <option value="files">Files</option>
          <option value="directories">Directories</option>
        </select>
        <button class="btn" title="Reset view" onClick={() => { setZoom(1); setPanX(0); setPanY(0); }}>
          <RotateCcw class="w-3 h-3" />
        </button>
        <button class="btn" title="Fit all" onClick={fitAll}>
          <Maximize2 class="w-3 h-3" />
        </button>
        <label class="flex items-center gap-1 text-[11px] text-text-secondary cursor-pointer">
          <Tags class="w-3 h-3" />
          <input type="checkbox" class="accent-accent-blue" checked={showLabels()} onChange={(e) => setShowLabels(e.currentTarget.checked)} />
          Labels
        </label>
      </div>

      {/* Legend */}
      <div class="px-3 pb-3 border-t border-border-default">
        <div class="text-[11px] text-text-muted mb-1.5">Languages</div>
        <div class="flex flex-wrap gap-2 text-[11px]">
          <span class="flex items-center gap-1"><span class="w-2 h-2 rounded-full bg-lang-c inline-block" /> C</span>
          <span class="flex items-center gap-1"><span class="w-2 h-2 rounded-full bg-lang-rust inline-block" /> Rust</span>
          <span class="flex items-center gap-1"><span class="w-2 h-2 rounded-full bg-lang-ts inline-block" /> TypeScript</span>
          <span class="flex items-center gap-1"><span class="w-2 h-2 rounded-full bg-lang-js inline-block" /> JavaScript</span>
        </div>
      </div>
    </div>
  );
}
