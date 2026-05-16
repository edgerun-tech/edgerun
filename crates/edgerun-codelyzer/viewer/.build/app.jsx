import { template as _$template } from "solid-js/web";
import { classList as _$classList } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { memo as _$memo } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div class="flex w-full h-full overflow-hidden bg-bg-primary"><div class="absolute inset-0 flex items-center justify-center bg-bg-primary/80 backdrop-blur-sm z-50 transition-opacity"><div class="flex flex-col items-center gap-3"><div class="w-8 h-8 border-2 border-accent-blue border-t-transparent rounded-full animate-spin"></div><span class="text-sm text-text-secondary">`);
/**
 * Root App component — layout and panel orchestration.
 */
import { onMount, onCleanup } from "solid-js";
import { setSidebarOpen, sidebarWidth, codePanelWidth, chatPanelWidth, diagnosticsPanelWidth, setSidebarWidth, setCodePanelWidth, setChatPanelWidth, setDiagnosticsPanelWidth, codePanelOpen, chatPanelOpen, diagnosticsPanelOpen, repoPanelOpen, loading, loadingMessage, setLoading, setLoadingMessage } from "./store.js";
import { GraphCanvas } from "./components/GraphCanvas.jsx";
import { Sidebar } from "./components/Sidebar.jsx";
import { CodePanel } from "./components/CodePanel.jsx";
import { ChatPanel } from "./components/ChatPanel.jsx";
import { DiagnosticsPanel } from "./components/DiagnosticsPanel.jsx";
import { FileExplorer } from "./components/FileExplorer.jsx";
import { RepoPanel } from "./components/RepoPanel.jsx";
import { Resizer } from "./components/Resizer.jsx";
import { KeyboardShortcuts } from "./components/KeyboardShortcuts.jsx";
import { connectWS, loadGraphFromServer } from "./ws.js";
export function App() {
  onMount(() => {
    // Connect WebSocket first
    connectWS();

    // Register cleanup synchronously
    onCleanup(() => {
      const ws = window.__ws;
      ws?.close();
    });
  });

  // Load graph after mount (non-blocking, waits for WS)
  onMount(async () => {
    try {
      setLoading(true);
      setLoadingMessage("Loading graph…");
      await loadGraphFromServer();
    } catch (e) {
      console.error("Failed to load graph:", e);
    } finally {
      setLoading(false);
    }
  });
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.firstChild,
      _el$4 = _el$3.firstChild,
      _el$5 = _el$4.nextSibling;
    _$insert(_el$, _$createComponent(FileExplorer, {}), _el$2);
    _$insert(_el$, _$createComponent(GraphCanvas, {}), _el$2);
    _$insert(_el$5, () => loadingMessage() || "Loading…");
    _$insert(_el$, (() => {
      var _c$ = _$memo(() => !!setSidebarOpen());
      return () => _c$() && [_$createComponent(Sidebar, {}), _$createComponent(Resizer, {
        get width() {
          return sidebarWidth();
        },
        onResize: w => setSidebarWidth(w),
        min: 200,
        max: 600
      })];
    })(), null);
    _$insert(_el$, (() => {
      var _c$2 = _$memo(() => !!codePanelOpen());
      return () => _c$2() && [_$createComponent(CodePanel, {}), _$createComponent(Resizer, {
        get width() {
          return codePanelWidth();
        },
        onResize: w => setCodePanelWidth(w),
        min: 300,
        max: 900
      })];
    })(), null);
    _$insert(_el$, (() => {
      var _c$3 = _$memo(() => !!chatPanelOpen());
      return () => _c$3() && [_$createComponent(ChatPanel, {}), _$createComponent(Resizer, {
        get width() {
          return chatPanelWidth();
        },
        onResize: w => setChatPanelWidth(w),
        min: 280,
        max: 700
      })];
    })(), null);
    _$insert(_el$, (() => {
      var _c$4 = _$memo(() => !!diagnosticsPanelOpen());
      return () => _c$4() && [_$createComponent(DiagnosticsPanel, {}), _$createComponent(Resizer, {
        get width() {
          return diagnosticsPanelWidth();
        },
        onResize: w => setDiagnosticsPanelWidth(w),
        min: 300,
        max: 800
      })];
    })(), null);
    _$insert(_el$, (() => {
      var _c$5 = _$memo(() => !!repoPanelOpen());
      return () => _c$5() && _$createComponent(RepoPanel, {});
    })(), null);
    _$insert(_el$, _$createComponent(KeyboardShortcuts, {}), null);
    _$effect(_$p => _$classList(_el$2, {
      "opacity-0 pointer-events-none": !loading()
    }, _$p));
    return _el$;
  })();
}