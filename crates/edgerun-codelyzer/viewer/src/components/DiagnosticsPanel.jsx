/** @jsxImportSource solid-js **/
/** @jsxImportSource solid-js **/
/**
 * Diagnostics panel with clickable navigation to code.
 */
import { For, createEffect } from "solid-js";
import { setDiagnosticsPanelOpen, diagnosticsFile, setDiagnosticsFile, diagnosticsItems, setDiagnosticsItems, diagnosticsLoading, setDiagnosticsLoading, setCodePanelOpen, setCodeFilePath, focusedNodeId, nodesMap } from "../store.js";
import { X, RefreshCw } from "lucide-solid";
import { getDiagnostics } from "../ws.js";
export function DiagnosticsPanel() {
  async function runDiagnostics() {
    const file = diagnosticsFile();
    if (!file) return;
    setDiagnosticsLoading(true);
    try {
      const items = await getDiagnostics(file);
      setDiagnosticsItems(items);
    } catch (e) {
      console.error("Diagnostics error:", e);
    } finally {
      setDiagnosticsLoading(false);
    }
  }

  // Auto-load when file changes
  createEffect(() => {
    const file = diagnosticsFile();
    if (file) {
      void runDiagnostics();
    }
  });

  // Track focused node's file
  createEffect(() => {
    const nodeId = focusedNodeId();
    if (nodeId) {
      const node = nodesMap().get(nodeId);
      if (node?.file && node.file !== diagnosticsFile()) {
        setDiagnosticsFile(node.file);
      }
    }
  });
  function navigateToFile(file, line) {
    setCodeFilePath(file);
    setCodePanelOpen(true);
    // Monaco will navigate to line after file loads — simplified
    console.log(`Navigate to ${file}:${line}`);
  }
  const severityColors = {
    error: "text-red-400 bg-red-900/20",
    warning: "text-yellow-400 bg-yellow-900/20",
    info: "text-blue-400 bg-blue-900/20"
  };
  const severityIcons = {
    error: "❌",
    warning: "⚠️",
    info: "ℹ️"
  };
  return <div class="panel shrink-0" style={{
    width: "480px"
  }}>
      {/* Header */}
      <div class="panel-header">
        <span class="panel-title">Diagnostics</span>
        <div class="flex gap-1.5">
          <button class="btn px-2 py-0.5 text-xs" onClick={runDiagnostics} disabled={diagnosticsLoading()}>
            <RefreshCw classList={{
            "w-3.5 h-3.5 animate-spin": diagnosticsLoading(),
            "w-3.5 h-3.5": !diagnosticsLoading()
          }} />
          </button>
          <button class="btn px-2 py-0.5 text-xs" onClick={() => setDiagnosticsPanelOpen(false)}>
            <X class="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* File info */}
      {diagnosticsFile() && <div class="px-3 py-2 text-xs text-text-muted border-b border-border-default truncate">
          {diagnosticsFile()}
        </div>}

      {/* List */}
      <div class="flex-1 overflow-y-auto p-2">
        {diagnosticsLoading() ? <div class="text-center py-8 text-text-muted text-xs">Running linters…</div> : diagnosticsItems().length === 0 ? <div class="text-center py-8 text-text-muted text-xs">No linting issues found.</div> : <div class="space-y-1.5">
            <For each={diagnosticsItems()}>
              {item => <button class={`w-full text-left px-2.5 py-2 rounded-md text-xs border border-border-default hover:border-border-focus transition-colors ${severityColors[item.severity] ?? ""}`} onClick={() => navigateToFile(item.file, item.line)}>
                  <div class="flex items-start gap-2">
                    <span class="shrink-0 mt-0.5">{severityIcons[item.severity] ?? "•"}</span>
                    <div class="flex-1 min-w-0">
                      <div class="font-medium truncate">{item.message}</div>
                      <div class="text-[11px] text-text-muted mt-0.5">
                        {item.file}:{item.line}:{item.column}
                        {item.code && ` [${item.code}]`}
                      </div>
                    </div>
                    <span class="badge shrink-0 mt-0.5 capitalize" classList={{
                "bg-red-900/30 text-red-300": item.severity === "error",
                "bg-yellow-900/30 text-yellow-300": item.severity === "warning",
                "bg-blue-900/30 text-blue-300": item.severity === "info"
              }}>
                      {item.severity}
                    </span>
                  </div>
                </button>}
            </For>
          </div>}
      </div>
    </div>;
}