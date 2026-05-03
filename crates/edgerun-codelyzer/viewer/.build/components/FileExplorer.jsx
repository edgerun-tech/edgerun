import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { classList as _$classList } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { setStyleProperty as _$setStyleProperty } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div><button class="flex items-center gap-1 px-1.5 py-0.5 w-full text-left text-xs rounded-sm hover:bg-bg-hover text-text-secondary"><span class=truncate>`),
  _tmpl$3 = /*#__PURE__*/_$template(`<button class="flex items-center gap-1 px-1.5 py-0.5 w-full text-left text-xs rounded-sm hover:bg-bg-hover text-text-secondary"><span class="w-3 h-3 shrink-0"></span><span class=truncate>`),
  _tmpl$4 = /*#__PURE__*/_$template(`<div class="fixed top-0 left-0 h-full bg-bg-secondary border-r border-border-default flex flex-col z-50 transition-transform duration-200"style=width:350px><div class="p-2.5 border-b border-border-default space-y-2"><div class="flex items-center justify-between"><h4 class="text-xs font-semibold text-text-heading">File Explorer</h4><button class="btn px-1.5 py-0.5 text-sm"></button></div><div class=relative><input type=text class="input pl-8 text-xs"placeholder="Filter files…"></div></div><div class="flex-1 overflow-y-auto p-2"></div><div class="flex items-center gap-2 p-2.5 border-t border-border-default"><button class="btn btn-primary text-xs">Analyze</button><span class="text-[11px] text-text-muted truncate">`);
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
    return (() => {
      var _el$ = _tmpl$();
      _$setStyleProperty(_el$, "margin-left", depth > 0 ? "12px" : undefined);
      _$setStyleProperty(_el$, "border-left", depth > 0 ? "1px solid var(--color-border-default)" : undefined);
      _$insert(_el$, _$createComponent(For, {
        each: filtered,
        children: entry => {
          const fullPath = parentPath === "/" ? `/${entry.name}` : `${parentPath}/${entry.name}`;
          const isExpanded = expanded.has(fullPath);
          if (entry.type === "dir") {
            return (() => {
              var _el$2 = _tmpl$2(),
                _el$3 = _el$2.firstChild,
                _el$4 = _el$3.firstChild;
              _el$3.$$click = () => toggleDir(fullPath);
              _$insert(_el$3, _$createComponent(ChevronRight, {
                "class": "w-3 h-3 shrink-0 transition-transform",
                classList: {
                  "rotate-90": isExpanded
                }
              }), _el$4);
              _$insert(_el$3, _$createComponent(FolderOpen, {
                "class": "w-3.5 h-3.5 shrink-0 text-yellow-600"
              }), _el$4);
              _$insert(_el$4, () => entry.name);
              _$insert(_el$2, _$createComponent(Show, {
                when: isExpanded,
                get children() {
                  return _$createComponent(DirContents, {
                    path: fullPath
                  });
                }
              }), null);
              return _el$2;
            })();
          }
          return (() => {
            var _el$5 = _tmpl$3(),
              _el$6 = _el$5.firstChild,
              _el$7 = _el$6.nextSibling;
            _el$5.$$click = () => selectFile(fullPath);
            _$insert(_el$5, _$createComponent(FileText, {
              "class": "w-3.5 h-3.5 shrink-0 text-blue-500"
            }), _el$7);
            _$insert(_el$7, () => entry.name);
            _$effect(_$p => _$classList(_el$5, {
              "bg-accent-blue-bg text-text-primary": selectedFilePath() === fullPath
            }, _$p));
            return _el$5;
          })();
        }
      }));
      return _el$;
    })();
  }
  return _$createComponent(Show, {
    get when() {
      return fileExplorerOpen();
    },
    get children() {
      var _el$8 = _tmpl$4(),
        _el$9 = _el$8.firstChild,
        _el$0 = _el$9.firstChild,
        _el$1 = _el$0.firstChild,
        _el$10 = _el$1.nextSibling,
        _el$11 = _el$0.nextSibling,
        _el$12 = _el$11.firstChild,
        _el$13 = _el$9.nextSibling,
        _el$14 = _el$13.nextSibling,
        _el$15 = _el$14.firstChild,
        _el$16 = _el$15.firstChild,
        _el$17 = _el$15.nextSibling;
      _el$10.$$click = () => setFileExplorerOpen(false);
      _$insert(_el$10, _$createComponent(X, {
        "class": "w-4 h-4"
      }));
      _$insert(_el$11, _$createComponent(Search, {
        "class": "absolute left-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-muted"
      }), _el$12);
      _el$12.$$input = e => setSearchFilter(e.currentTarget.value);
      _$insert(_el$13, _$createComponent(DirContents, {
        path: "/",
        renderTree: renderTree
      }));
      _el$15.$$click = analyze;
      _$insert(_el$15, _$createComponent(Play, {
        "class": "w-3 h-3 inline mr-1"
      }), _el$16);
      _$insert(_el$17, statusMsg);
      _$effect(_p$ => {
        var _v$ = !fileExplorerOpen(),
          _v$2 = analyzing() || !selectedFilePath();
        _v$ !== _p$.e && _el$8.classList.toggle("-translate-x-full", _p$.e = _v$);
        _v$2 !== _p$.t && (_el$15.disabled = _p$.t = _v$2);
        return _p$;
      }, {
        e: undefined,
        t: undefined
      });
      _$effect(() => _el$12.value = searchFilter());
      return _el$8;
    }
  });
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
  return _tmpl$();
}
_$delegateEvents(["click", "input"]);