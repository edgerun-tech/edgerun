"use client"

import { useStore } from "@nanostores/react"
import { xrayState, resetView, setLayout, setRuntimeMode } from "./graph/graph-store"
import { setRuntimeOverlayEnabled } from "./runtime/runtime-store"
import type { LayoutType } from "./graph/types"

export function XrayCommandSurface() {
  const state = useStore(xrayState)

  return (
    <div className="h-12 border-t border-zinc-800 bg-zinc-950/90 flex items-center px-4 gap-4 text-sm">
      <div className="flex items-center gap-2">
        <span className="text-zinc-500 text-xs font-mono">Layout</span>
        <div className="flex gap-1">
          {(["force", "globe", "layers"] as LayoutType[]).map((layout) => (
            <button
              key={layout}
              onClick={() => setLayout(layout)}
              className={`px-2.5 py-1 rounded text-xs font-mono transition-colors ${
                state.layout === layout
                  ? "bg-zinc-700 text-zinc-100"
                  : "bg-zinc-800/50 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300"
              }`}
            >
              {layout}
            </button>
          ))}
        </div>
      </div>

      <div className="h-6 w-px bg-zinc-800" />

      <div className="flex items-center gap-2">
        <span className="text-zinc-500 text-xs font-mono">Runtime</span>
        <button
          onClick={() => {
            setRuntimeMode(!state.runtimeMode)
            setRuntimeOverlayEnabled(!state.runtimeMode)
          }}
          className={`px-2.5 py-1 rounded text-xs font-mono transition-colors ${
            state.runtimeMode
              ? "bg-emerald-800/60 text-emerald-300"
              : "bg-zinc-800/50 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300"
          }`}
        >
          {state.runtimeMode ? "ON" : "OFF"}
        </button>
      </div>

      <div className="h-6 w-px bg-zinc-800" />

      <button
        onClick={resetView}
        className="px-2.5 py-1 rounded text-xs font-mono bg-zinc-800/50 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300 transition-colors"
      >
        Reset View
      </button>

      <div className="flex-1" />

      <div className="flex items-center gap-2">
        <span className="text-zinc-600 text-xs font-mono">Timeline</span>
        <div className="w-32 h-1.5 bg-zinc-800 rounded-full overflow-hidden">
          <div className="w-1/3 h-full bg-zinc-600 rounded-full" />
        </div>
        <span className="text-zinc-600 text-xs font-mono">placeholder</span>
      </div>

      <div className="h-6 w-px bg-zinc-800" />

      <div className="text-zinc-600 text-xs font-mono">
        {state.nodes.size} nodes · {state.edges.length} edges
      </div>
    </div>
  )
}
