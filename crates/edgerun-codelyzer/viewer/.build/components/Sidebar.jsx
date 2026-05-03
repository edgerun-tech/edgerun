import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { style as _$style } from "solid-js/web";
import { classList as _$classList } from "solid-js/web";
import { addEventListener as _$addEventListener } from "solid-js/web";
import { setStyleProperty as _$setStyleProperty } from "solid-js/web";
import { memo as _$memo } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div class="panel shrink-0"><div class="p-3 border-b border-border-default relative"><div class=relative><input type=text class="input pl-8"placeholder="Search functions… (Ctrl+K)"></div></div><div class="flex gap-3 px-3 py-2 text-[11px] text-text-muted border-b border-border-default"><span> nodes</span><span> edges</span><span> fps</span><span class=ml-auto><span></span></span></div><div class="border-b border-border-default"><div class="flex items-center justify-between px-3 py-2"><div class="flex items-center gap-1 text-xs text-text-secondary"><span>Tag Groups</span></div></div><div class="flex flex-wrap gap-1 px-3 pb-2"></div></div><div class="flex-1 overflow-y-auto p-3"></div><div class="p-3 border-t border-border-default flex flex-wrap gap-1.5 items-center"><button class=btn title="Browse files">Browse</button><button class=btn title=Repositories>Repos</button><button class=btn title="Code viewer (Ctrl+Shift+C)">Code</button><button class=btn title="Chat (Ctrl+Shift+L)">Chat</button><button class=btn title=Files>Files</button><button class=btn title=Lint>Lint</button><select class="input py-1 px-2"title="Switch view"><option value=functions>Functions</option><option value=files>Files</option><option value=directories>Directories</option></select><button class=btn title="Reset view"></button><button class=btn title="Fit all"></button><label class="flex items-center gap-1 text-[11px] text-text-secondary cursor-pointer"><input type=checkbox class=accent-accent-blue>Labels</label></div><div class="px-3 pb-3 border-t border-border-default"><div class="text-[11px] text-text-muted mb-1.5">Languages</div><div class="flex flex-wrap gap-2 text-[11px]"><span class="flex items-center gap-1"><span class="w-2 h-2 rounded-full bg-lang-c inline-block"></span> C</span><span class="flex items-center gap-1"><span class="w-2 h-2 rounded-full bg-lang-rust inline-block"></span> Rust</span><span class="flex items-center gap-1"><span class="w-2 h-2 rounded-full bg-lang-ts inline-block"></span> TypeScript</span><span class="flex items-center gap-1"><span class="w-2 h-2 rounded-full bg-lang-js inline-block"></span> JavaScript`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div class="absolute top-full left-0 right-0 max-h-72 overflow-y-auto bg-bg-tertiary border border-border-default border-t-0 rounded-b-md z-50">`),
  _tmpl$3 = /*#__PURE__*/_$template(`<button class="w-full px-3 py-2 text-xs text-left text-text-primary hover:bg-bg-hover truncate border-b border-border-default/50 last:border-0"> <span class=text-text-muted>(<!>)`),
  _tmpl$4 = /*#__PURE__*/_$template(`<span class=text-accent-blue>(<!> filtered)`),
  _tmpl$5 = /*#__PURE__*/_$template(`<button class="text-[10px] text-accent-red hover:text-accent-red/80">`),
  _tmpl$6 = /*#__PURE__*/_$template(`<button>`),
  _tmpl$7 = /*#__PURE__*/_$template(`<div><h3 class="text-xs font-semibold text-text-heading mb-2 break-all"></h3><div class="space-y-1 text-xs">`),
  _tmpl$8 = /*#__PURE__*/_$template(`<div class="flex justify-between"><span class=text-text-secondary>File</span><span class="text-text-primary text-right max-w-[60%] break-all">`),
  _tmpl$9 = /*#__PURE__*/_$template(`<div class="flex justify-between"><span class=text-text-secondary>Language</span><span class=text-text-primary>`),
  _tmpl$0 = /*#__PURE__*/_$template(`<div class="flex justify-between"><span class=text-text-secondary>Static</span><span class=text-text-primary>`),
  _tmpl$1 = /*#__PURE__*/_$template(`<div class="flex justify-between"><span class=text-text-secondary>Connections</span><span class=text-text-primary>`),
  _tmpl$10 = /*#__PURE__*/_$template(`<div class="flex justify-between"><span class=text-text-secondary>Last commit</span><span class=text-text-primary>`),
  _tmpl$11 = /*#__PURE__*/_$template(`<div class=mt-2><span class="text-text-secondary text-xs">Tags</span><div class="flex flex-wrap gap-1 mt-1">`),
  _tmpl$12 = /*#__PURE__*/_$template(`<button class="px-1.5 py-0.5 text-[9px] rounded bg-bg-tertiary text-text-muted hover:text-text-primary hover:bg-bg-hover">`),
  _tmpl$13 = /*#__PURE__*/_$template(`<div class=mt-3><div class="flex justify-between text-xs"><span class=text-text-secondary>Neighbors</span><span class=text-text-primary></span></div><div class="mt-1 space-y-0.5">`),
  _tmpl$14 = /*#__PURE__*/_$template(`<button class="w-full text-left px-2 py-0.5 text-xs text-text-secondary rounded-sm hover:bg-bg-hover hover:text-text-primary truncate">`),
  _tmpl$15 = /*#__PURE__*/_$template(`<button class="text-[11px] text-text-muted px-2 py-0.5">+<!> more`),
  _tmpl$16 = /*#__PURE__*/_$template(`<div class="text-xs text-text-muted text-center py-8">Select a node to see details`);
/**
 * Sidebar component — search, status, node details, controls.
 */
import { createSignal, createMemo, createEffect, For } from "solid-js";
import { nodesMap, edges, focusedNodeId, setFocusedNodeWithHighlight, sidebarWidth, wsConnected, fps, searchQuery, setSearchQuery, searchResults, setSearchResults, searchOpen, setSearchOpen, showLabels, setShowLabels, setCodePanelOpen, setChatPanelOpen, setDiagnosticsPanelOpen, setFileExplorerOpen, setRepoPanelOpenLocal, setLoading, setLoadingMessage, setCurrentView, currentView, setPanX, setPanY, tagGroups, activeTagFilters, toggleTagFilter, clearTagFilters } from "../store.js";
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
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.firstChild,
      _el$4 = _el$3.firstChild,
      _el$5 = _el$2.nextSibling,
      _el$6 = _el$5.firstChild,
      _el$7 = _el$6.firstChild,
      _el$8 = _el$6.nextSibling,
      _el$9 = _el$8.firstChild,
      _el$0 = _el$8.nextSibling,
      _el$1 = _el$0.firstChild,
      _el$10 = _el$0.nextSibling,
      _el$11 = _el$10.firstChild,
      _el$12 = _el$5.nextSibling,
      _el$13 = _el$12.firstChild,
      _el$14 = _el$13.firstChild,
      _el$15 = _el$14.firstChild,
      _el$16 = _el$13.nextSibling,
      _el$17 = _el$12.nextSibling,
      _el$18 = _el$17.nextSibling,
      _el$19 = _el$18.firstChild,
      _el$20 = _el$19.firstChild,
      _el$21 = _el$19.nextSibling,
      _el$22 = _el$21.nextSibling,
      _el$23 = _el$22.firstChild,
      _el$24 = _el$22.nextSibling,
      _el$25 = _el$24.firstChild,
      _el$26 = _el$24.nextSibling,
      _el$27 = _el$26.firstChild,
      _el$28 = _el$26.nextSibling,
      _el$29 = _el$28.firstChild,
      _el$30 = _el$28.nextSibling,
      _el$31 = _el$30.nextSibling,
      _el$32 = _el$31.nextSibling,
      _el$33 = _el$32.nextSibling,
      _el$34 = _el$33.firstChild;
    _$insert(_el$3, _$createComponent(Search, {
      "class": "absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-muted"
    }), _el$4);
    _el$4.$$keydown = onSearchKeydown;
    _el$4.$$input = e => {
      setSearchQuery(e.currentTarget.value);
      onSearchInput();
    };
    _$insert(_el$2, (() => {
      var _c$ = _$memo(() => !!(searchOpen() && searchResults().length > 0));
      return () => _c$() && (() => {
        var _el$35 = _tmpl$2();
        _$insert(_el$35, _$createComponent(For, {
          get each() {
            return searchResults();
          },
          children: node => (() => {
            var _el$36 = _tmpl$3(),
              _el$37 = _el$36.firstChild,
              _el$38 = _el$37.nextSibling,
              _el$39 = _el$38.firstChild,
              _el$41 = _el$39.nextSibling,
              _el$40 = _el$41.nextSibling;
            _el$36.$$click = () => selectNode(node.id);
            _$insert(_el$36, () => node.name, _el$37);
            _$insert(_el$38, () => node.file ?? "", _el$41);
            return _el$36;
          })()
        }));
        return _el$35;
      })();
    })(), null);
    _$insert(_el$6, () => nodesMap().size, _el$7);
    _$insert(_el$8, () => edges().length, _el$9);
    _$insert(_el$0, fps, _el$1);
    _$insert(_el$5, (() => {
      var _c$2 = _$memo(() => activeTagFilters().size > 0);
      return () => _c$2() && (() => {
        var _el$42 = _tmpl$4(),
          _el$43 = _el$42.firstChild,
          _el$45 = _el$43.nextSibling,
          _el$44 = _el$45.nextSibling;
        _$insert(_el$42, getFilteredNodeCount, _el$45);
        return _el$42;
      })();
    })(), _el$10);
    _$insert(_el$11, () => wsConnected() ? "●" : "○");
    _$insert(_el$14, _$createComponent(Filter, {
      size: 12
    }), _el$15);
    _$insert(_el$13, (() => {
      var _c$3 = _$memo(() => activeTagFilters().size > 0);
      return () => _c$3() && (() => {
        var _el$46 = _tmpl$5();
        _$addEventListener(_el$46, "click", clearTagFilters, true);
        _$insert(_el$46, _$createComponent(X, {
          size: 12
        }));
        return _el$46;
      })();
    })(), null);
    _$insert(_el$16, _$createComponent(For, {
      get each() {
        return tagGroups();
      },
      children: group => {
        const isActive = () => {
          const filters = activeTagFilters();
          return group.tags.some(t => filters.has(t));
        };
        return (() => {
          var _el$47 = _tmpl$6();
          _el$47.$$click = () => group.tags.forEach(t => toggleTagFilter(t));
          _$insert(_el$47, () => group.name);
          _$effect(_p$ => {
            var _v$4 = {
                "px-2 py-0.5 text-[10px] rounded-full border transition-colors": true,
                "bg-opacity-20 text-white": isActive(),
                "bg-bg-tertiary text-text-muted border-border-default hover:border-text-muted": !isActive()
              },
              _v$5 = isActive() ? {
                "background-color": group.color + "33",
                "border-color": group.color,
                "color": group.color
              } : {};
            _p$.e = _$classList(_el$47, _v$4, _p$.e);
            _p$.t = _$style(_el$47, _v$5, _p$.t);
            return _p$;
          }, {
            e: undefined,
            t: undefined
          });
          return _el$47;
        })();
      }
    }));
    _$insert(_el$17, (() => {
      var _c$4 = _$memo(() => !!focusedNode());
      return () => _c$4() ? (() => {
        var _el$48 = _tmpl$7(),
          _el$49 = _el$48.firstChild,
          _el$50 = _el$49.nextSibling;
        _$insert(_el$49, () => focusedNode().name);
        _$insert(_el$50, (() => {
          var _c$5 = _$memo(() => !!focusedNode().file);
          return () => _c$5() && (() => {
            var _el$51 = _tmpl$8(),
              _el$52 = _el$51.firstChild,
              _el$53 = _el$52.nextSibling;
            _$insert(_el$53, () => focusedNode().file);
            return _el$51;
          })();
        })(), null);
        _$insert(_el$50, (() => {
          var _c$6 = _$memo(() => !!focusedNode().language);
          return () => _c$6() && (() => {
            var _el$54 = _tmpl$9(),
              _el$55 = _el$54.firstChild,
              _el$56 = _el$55.nextSibling;
            _$insert(_el$56, () => focusedNode().language);
            return _el$54;
          })();
        })(), null);
        _$insert(_el$50, (() => {
          var _c$7 = _$memo(() => focusedNode().is_static !== undefined);
          return () => _c$7() && (() => {
            var _el$57 = _tmpl$0(),
              _el$58 = _el$57.firstChild,
              _el$59 = _el$58.nextSibling;
            _$insert(_el$59, () => focusedNode().is_static ? "yes" : "no");
            return _el$57;
          })();
        })(), null);
        _$insert(_el$50, (() => {
          var _c$8 = _$memo(() => focusedNode().connections !== undefined);
          return () => _c$8() && (() => {
            var _el$60 = _tmpl$1(),
              _el$61 = _el$60.firstChild,
              _el$62 = _el$61.nextSibling;
            _$insert(_el$62, () => focusedNode().connections);
            return _el$60;
          })();
        })(), null);
        _$insert(_el$50, (() => {
          var _c$9 = _$memo(() => !!focusedNode().commit);
          return () => _c$9() && (() => {
            var _el$63 = _tmpl$10(),
              _el$64 = _el$63.firstChild,
              _el$65 = _el$64.nextSibling;
            _$insert(_el$65, () => focusedNode().commit);
            return _el$63;
          })();
        })(), null);
        _$insert(_el$50, (() => {
          var _c$0 = _$memo(() => !!(focusedNode().tags && focusedNode().tags.length > 0));
          return () => _c$0() && (() => {
            var _el$66 = _tmpl$11(),
              _el$67 = _el$66.firstChild,
              _el$68 = _el$67.nextSibling;
            _$insert(_el$68, _$createComponent(For, {
              get each() {
                return focusedNode().tags;
              },
              children: tag => (() => {
                var _el$69 = _tmpl$12();
                _el$69.$$click = () => toggleTagFilter(tag);
                _$insert(_el$69, tag);
                return _el$69;
              })()
            }));
            return _el$66;
          })();
        })(), null);
        _$insert(_el$48, (() => {
          var _c$1 = _$memo(() => neighbors().length > 0);
          return () => _c$1() && (() => {
            var _el$70 = _tmpl$13(),
              _el$71 = _el$70.firstChild,
              _el$72 = _el$71.firstChild,
              _el$73 = _el$72.nextSibling,
              _el$74 = _el$71.nextSibling;
            _$insert(_el$73, () => neighbors().length);
            _$insert(_el$74, _$createComponent(For, {
              get each() {
                return displayedNeighbors();
              },
              children: nid => {
                const n = nodesMap().get(nid);
                return (() => {
                  var _el$75 = _tmpl$14();
                  _el$75.$$click = () => setFocusedNodeWithHighlight(nid);
                  _$insert(_el$75, () => n?.name ?? nid);
                  return _el$75;
                })();
              }
            }), null);
            _$insert(_el$74, (() => {
              var _c$10 = _$memo(() => neighbors().length > neighborLimit());
              return () => _c$10() && (() => {
                var _el$76 = _tmpl$15(),
                  _el$77 = _el$76.firstChild,
                  _el$79 = _el$77.nextSibling,
                  _el$78 = _el$79.nextSibling;
                _el$76.$$click = () => setNeighborLimit(neighborLimit() + 10);
                _$insert(_el$76, () => neighbors().length - neighborLimit(), _el$79);
                return _el$76;
              })();
            })(), null);
            return _el$70;
          })();
        })(), null);
        return _el$48;
      })() : _tmpl$16();
    })());
    _el$19.$$click = () => setFileExplorerOpen(true);
    _$insert(_el$19, _$createComponent(FolderOpen, {
      "class": "w-3 h-3 inline mr-1"
    }), _el$20);
    _el$21.$$click = () => setRepoPanelOpenLocal(true);
    _el$22.$$click = () => setCodePanelOpen(v => !v);
    _$insert(_el$22, _$createComponent(Code2, {
      "class": "w-3 h-3 inline mr-1"
    }), _el$23);
    _el$24.$$click = () => setChatPanelOpen(v => !v);
    _$insert(_el$24, _$createComponent(MessageSquare, {
      "class": "w-3 h-3 inline mr-1"
    }), _el$25);
    _el$26.$$click = () => {/* toggle files panel */};
    _$insert(_el$26, _$createComponent(FileText, {
      "class": "w-3 h-3 inline mr-1"
    }), _el$27);
    _el$28.$$click = () => setDiagnosticsPanelOpen(v => !v);
    _$insert(_el$28, _$createComponent(ScanSearch, {
      "class": "w-3 h-3 inline mr-1"
    }), _el$29);
    _el$30.addEventListener("change", e => loadView(e.currentTarget.value));
    _el$31.$$click = () => {
      setZoom(1);
      setPanX(0);
      setPanY(0);
    };
    _$insert(_el$31, _$createComponent(RotateCcw, {
      "class": "w-3 h-3"
    }));
    _el$32.$$click = fitAll;
    _$insert(_el$32, _$createComponent(Maximize2, {
      "class": "w-3 h-3"
    }));
    _$insert(_el$33, _$createComponent(Tags, {
      "class": "w-3 h-3"
    }), _el$34);
    _el$34.addEventListener("change", e => setShowLabels(e.currentTarget.checked));
    _$effect(_p$ => {
      var _v$ = `${sidebarWidth()}px`,
        _v$2 = !!wsConnected(),
        _v$3 = !wsConnected();
      _v$ !== _p$.e && _$setStyleProperty(_el$, "width", _p$.e = _v$);
      _v$2 !== _p$.t && _el$11.classList.toggle("text-accent-green", _p$.t = _v$2);
      _v$3 !== _p$.a && _el$11.classList.toggle("text-text-muted", _p$.a = _v$3);
      return _p$;
    }, {
      e: undefined,
      t: undefined,
      a: undefined
    });
    _$effect(() => _el$4.value = searchQuery());
    _$effect(() => _el$30.value = currentView());
    _$effect(() => _el$34.checked = showLabels());
    return _el$;
  })();
}
_$delegateEvents(["input", "keydown", "click"]);