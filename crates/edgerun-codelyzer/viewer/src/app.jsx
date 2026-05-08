/** @jsxImportSource solid-js **/
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
  return <div class="flex w-full h-full overflow-hidden bg-bg-primary">
      {/* File Explorer overlay */}
      <FileExplorer />

      {/* Main canvas area */}
        <GraphCanvas />

        {/* Loading overlay */}
        <div class="absolute inset-0 flex items-center justify-center bg-bg-primary/80 backdrop-blur-sm z-50 transition-opacity" classList={{
        "opacity-0 pointer-events-none": !loading()
      }}>
          <div class="flex flex-col items-center gap-3">
            <div class="w-8 h-8 border-2 border-accent-blue border-t-transparent rounded-full animate-spin" />
            <span class="text-sm text-text-secondary">{loadingMessage() || "Loading…"}</span>
          </div>
        </div>

      {/* Sidebar */}
      {setSidebarOpen() && <>
          <Sidebar />
          <Resizer width={sidebarWidth()} onResize={w => setSidebarWidth(w)} min={200} max={600} />
        </>}

      {/* Code Panel */}
      {codePanelOpen() && <>
          <CodePanel />
          <Resizer width={codePanelWidth()} onResize={w => setCodePanelWidth(w)} min={300} max={900} />
        </>}

      {/* Chat Panel */}
      {chatPanelOpen() && <>
          <ChatPanel />
          <Resizer width={chatPanelWidth()} onResize={w => setChatPanelWidth(w)} min={280} max={700} />
        </>}

      {/* Diagnostics Panel */}
      {diagnosticsPanelOpen() && <>
          <DiagnosticsPanel />
          <Resizer width={diagnosticsPanelWidth()} onResize={w => setDiagnosticsPanelWidth(w)} min={300} max={800} />
        </>}

      {/* Repo Panel */}
      {repoPanelOpen() && <RepoPanel />}

      {/* Keyboard shortcuts */}
      <KeyboardShortcuts />
    </div>;
}