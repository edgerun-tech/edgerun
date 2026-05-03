"use client"

import { useStore } from "@nanostores/react"
import { xrayState } from "./graph/graph-store"

export function XrayInspector() {
  const state = useStore(xrayState)
  const selectedNode = state.selectedId ? state.nodes.get(state.selectedId) : null
  const selectedRuntime = state.selectedId ? state.runtimeStats.get(state.selectedId) : null

  if (!selectedNode) {
    return (
      <div className="w-64 border-l border-zinc-800 bg-zinc-950/80 p-4 flex flex-col items-center justify-center text-zinc-500 text-sm">
        <div className="text-center">
          <div className="text-2xl mb-2">○</div>
          <div>No node selected</div>
          <div className="text-xs mt-2 text-zinc-600">Click a node in the graph to inspect</div>
        </div>
      </div>
    )
  }

  return (
    <div className="w-64 border-l border-zinc-800 bg-zinc-950/80 overflow-y-auto">
      <div className="p-4">
        <div className="text-xs font-mono text-zinc-500 mb-1">ID</div>
        <div className="text-sm text-zinc-200 font-mono break-all mb-3">{selectedNode.id}</div>

        <div className="text-xs font-mono text-zinc-500 mb-1">Label</div>
        <div className="text-sm text-zinc-200 mb-3">{selectedNode.label}</div>

        <div className="text-xs font-mono text-zinc-500 mb-1">Kind</div>
        <div className="text-sm text-zinc-200 mb-3">
          <span className="px-2 py-0.5 rounded bg-zinc-800 text-xs font-mono">{selectedNode.kind}</span>
        </div>

        {selectedNode.layer && (
          <>
            <div className="text-xs font-mono text-zinc-500 mb-1">Layer</div>
            <div className="text-sm text-zinc-200 mb-3">
              <span className="px-2 py-0.5 rounded bg-zinc-800 text-xs font-mono">{selectedNode.layer}</span>
            </div>
          </>
        )}

        {selectedNode.language && (
          <>
            <div className="text-xs font-mono text-zinc-500 mb-1">Language</div>
            <div className="text-sm text-zinc-200 mb-3">
              <span className="px-2 py-0.5 rounded bg-zinc-800 text-xs font-mono">{selectedNode.language}</span>
            </div>
          </>
        )}

        {selectedNode.tags.length > 0 && (
          <>
            <div className="text-xs font-mono text-zinc-500 mb-1">Tags</div>
            <div className="flex flex-wrap gap-1 mb-3">
              {selectedNode.tags.map((tag) => (
                <span key={tag} className="px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-400 text-xs font-mono">
                  {tag}
                </span>
              ))}
            </div>
          </>
        )}

        {selectedNode.source && (
          <>
            <div className="text-xs font-mono text-zinc-500 mb-1">Source</div>
            <div className="text-xs text-zinc-400 font-mono break-all mb-3">
              {selectedNode.source.file && <div>{selectedNode.source.file}</div>}
              {selectedNode.source.line && <div className="text-zinc-500">line {selectedNode.source.line}</div>}
              {selectedNode.source.symbol && <div className="text-zinc-500">{selectedNode.source.symbol}</div>}
            </div>
          </>
        )}

        {selectedRuntime && (
          <>
            <div className="border-t border-zinc-800 pt-3 mt-3">
              <div className="text-xs font-mono text-zinc-500 mb-2">Runtime Stats</div>
              <div className="grid grid-cols-2 gap-2 text-xs font-mono">
                <div className="bg-zinc-900 p-2 rounded">
                  <div className="text-zinc-500">hits</div>
                  <div className="text-zinc-200">{selectedRuntime.hits.toLocaleString()}</div>
                </div>
                <div className="bg-zinc-900 p-2 rounded">
                  <div className="text-zinc-500">errors</div>
                  <div className={selectedRuntime.errors > 0 ? "text-red-400" : "text-zinc-200"}>
                    {selectedRuntime.errors}
                  </div>
                </div>
                <div className="bg-zinc-900 p-2 rounded">
                  <div className="text-zinc-500">openSpans</div>
                  <div className={selectedRuntime.openSpans > 0 ? "text-amber-400" : "text-zinc-200"}>
                    {selectedRuntime.openSpans}
                  </div>
                </div>
                <div className="bg-zinc-900 p-2 rounded">
                  <div className="text-zinc-500">maxUs</div>
                  <div className="text-zinc-200">{(selectedRuntime.maxUs / 1000).toFixed(1)}ms</div>
                </div>
                <div className="bg-zinc-900 p-2 rounded col-span-2">
                  <div className="text-zinc-500">totalUs</div>
                  <div className="text-zinc-200">{(selectedRuntime.totalUs / 1000).toFixed(1)}ms</div>
                </div>
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  )
}
