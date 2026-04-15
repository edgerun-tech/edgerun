/** @jsxImportSource solid-js **/
/** @jsxImportSource solid-js **/
/**
 * File explorer with tree view, lazy loading, breadcrumbs, and search.
 */
import { createSignal, For, Show } from "solid-js";
import { fileExplorerOpen, setFileExplorerOpen, selectedFilePath, setSelectedFilePath, setLoading, setLoadingMessage } from "../store.js";
import { X, Search, FolderOpen, FileText, ChevronRight, Play } from "lucide-solid";
import { getDirectoryListing, analyzePath } from "../ws.js";
const dirCache = {};
const [expandedPaths, setExpandedPaths] = createSignal(new Set(["/"]));
const [searchFilter, setSearchFilter] = createSignal("");
const [analyzing, setAnalyzing] = createSignal(false);
const [statusMsg, setStatusMsg] = createSignal("");
export function FileExplorer() {
  async function loadDir(path) {
    if (dirCache[path]) return dirCache[path];
    try {
      const entries = await getDirectoryListing(path);
      dirCache[path] = entries;
      return entries;
    } catch (e) {
      console.error(`Failed to fetch directory ${path}:`, e);
      return [];
    }
  }
  async function toggleDir(path) {
    const expanded = new Set(expandedPaths());
    if (expanded.has(path)) {
      expanded.delete(path);
    } else {
      expanded.add(path);
      await loadDir(path);
    }
    setExpandedPaths(expanded);
  }
  function selectFile(path) {
    setSelectedFilePath(path);
  }
  async function analyze() {
    const path = selectedFilePath();
    if (!path || analyzing()) return;
    setAnalyzing(true);
    setStatusMsg("Analyzing…");
    setLoading(true);
    setLoadingMessage("Analyzing repository…");
    try {
      const result = await analyzePath(path);
      setStatusMsg(`Done: ${result.functions} functions, ${result.edges} edges`);
    } catch (e) {
      setStatusMsg(`Error: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setAnalyzing(false);
      setLoading(false);
    }
  }
  const filter = searchFilter().toLowerCase();
  const matchesFilter = name => !filter || name.toLowerCase().includes(filter);
  function renderTree(entries, parentPath, depth) {
    const expanded = expandedPaths();
    const sorted = [...entries].sort((a, b) => {
      if (a.type !== b.type) return a.type === "dir" ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
    const filtered = filter ? sorted.filter(e => {
      if (e.type === "dir") return true; // Always show dirs for search expansion
      return matchesFilter(e.name);
    }) : sorted;
    return <div style={{
      "margin-left": depth > 0 ? "12px" : undefined,
      "border-left": depth > 0 ? "1px solid var(--color-border-default)" : undefined
    }}>
        <For each={filtered}>
          {entry => {
          const fullPath = parentPath === "/" ? `/${entry.name}` : `${parentPath}/${entry.name}`;
          const isExpanded = expanded.has(fullPath);
          if (entry.type === "dir") {
            return <div>
                  <button class="flex items-center gap-1 px-1.5 py-0.5 w-full text-left text-xs rounded-sm hover:bg-bg-hover text-text-secondary" onClick={() => toggleDir(fullPath)}>
                    <ChevronRight class="w-3 h-3 shrink-0 transition-transform" classList={{
                  "rotate-90": isExpanded
                }} />
                    <FolderOpen class="w-3.5 h-3.5 shrink-0 text-yellow-600" />
                    <span class="truncate">{entry.name}</span>
                  </button>
                  <Show when={isExpanded}>
                    <DirContents path={fullPath} />
                  </Show>
                </div>;
          }
          return <button class="flex items-center gap-1 px-1.5 py-0.5 w-full text-left text-xs rounded-sm hover:bg-bg-hover text-text-secondary" classList={{
            "bg-accent-blue-bg text-text-primary": selectedFilePath() === fullPath
          }} onClick={() => selectFile(fullPath)}>
                <span class="w-3 h-3 shrink-0" />
                <FileText class="w-3.5 h-3.5 shrink-0 text-blue-500" />
                <span class="truncate">{entry.name}</span>
              </button>;
        }}
        </For>
      </div>;
  }
  return <Show when={fileExplorerOpen()}>
      <div class="fixed top-0 left-0 h-full bg-bg-secondary border-r border-border-default flex flex-col z-50 transition-transform duration-200" style={{
      width: "350px"
    }} classList={{
      "-translate-x-full": !fileExplorerOpen()
    }}>
        {/* Header */}
        <div class="p-2.5 border-b border-border-default space-y-2">
          <div class="flex items-center justify-between">
            <h4 class="text-xs font-semibold text-text-heading">File Explorer</h4>
            <button class="btn px-1.5 py-0.5 text-sm" onClick={() => setFileExplorerOpen(false)}>
              <X class="w-4 h-4" />
            </button>
          </div>
          <div class="relative">
            <Search class="absolute left-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-muted" />
            <input type="text" class="input pl-8 text-xs" placeholder="Filter files…" value={searchFilter()} onInput={e => setSearchFilter(e.currentTarget.value)} />
          </div>
        </div>

        {/* Tree */}
        <div class="flex-1 overflow-y-auto p-2">
          <DirContents path="/" renderTree={renderTree} />
        </div>

        {/* Actions */}
        <div class="flex items-center gap-2 p-2.5 border-t border-border-default">
          <button class="btn btn-primary text-xs" disabled={analyzing() || !selectedFilePath()} onClick={analyze}>
            <Play class="w-3 h-3 inline mr-1" />
            Analyze
          </button>
          <span class="text-[11px] text-text-muted truncate">{statusMsg()}</span>
        </div>
      </div>
    </Show>;
}
function DirContents(props) {
  const [entries, setEntries] = createSignal([]);
  const [loaded, setLoaded] = createSignal(false);
  async function load() {
    if (loaded()) return;
    try {
      const result = await getDirectoryListing(props.path);
      setEntries(result);
      setLoaded(true);
    } catch (e) {
      console.error(e);
    }
  }

  // Load on mount
  void load();
  if (props.renderTree) {
    return props.renderTree(entries(), props.path, 1);
  }
  return <div>{/* Fallback, shouldn't be used */}</div>;
}
// Benchmark comment