"use client"

import { useStore } from "@nanostores/react"
import { cn } from "@/lib/utils"
import { xrayState, resetView, setLayout, setRuntimeMode } from "./graph/graph-store"
import { setRuntimeOverlayEnabled } from "./runtime/runtime-store"
import type { LayoutType } from "./graph/types"

type XrayCommandSurfaceProps = {
  className?: string
  surface?: "top" | "bottom" | "inline"
}

export function XrayCommandSurface({ className, surface = "top" }: XrayCommandSurfaceProps) {
  const state = useStore(xrayState)
  const top = surface === "top"

  return (
    <div
      className={cn(
        "pointer-events-auto flex items-center gap-3 text-sm backdrop-blur-md",
        top
          ? "rounded-full border border-white/10 bg-background/70 px-3 py-1.5 shadow-2xl"
          : surface === "bottom"
            ? "h-12 border-t border-zinc-800 bg-zinc-950/90 px-4"
            : "rounded-2xl border border-border bg-card/80 px-3 py-2",
        className,
      )}
    >
      <div className="flex items-center gap-2">
        <span className="text-[10px] font-mono uppercase tracking-[0.18em] text-zinc-500">Layout</span>
        <div className="flex gap-1">
          {(["force", "globe", "layers"] as LayoutType[]).map((layout) => (
            <button
              key={layout}
              type="button"
              onClick={() => setLayout(layout)}
              className={cn(
                "rounded px-2.5 py-1 text-xs font-mono transition-colors",
                state.layout === layout
                  ? "bg-zinc-700 text-zinc-100"
                  : "bg-zinc-800/50 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300",
              )}
            >
              {layout}
            </button>
          ))}
        </div>
      </div>

      <div className="h-6 w-px bg-zinc-800" />

      <div className="flex items-center gap-2">
        <span className="text-[10px] font-mono uppercase tracking-[0.18em] text-zinc-500">Runtime</span>
        <button
          type="button"
          onClick={() => {
            setRuntimeMode(!state.runtimeMode)
            setRuntimeOverlayEnabled(!state.runtimeMode)
          }}
          className={cn(
            "rounded px-2.5 py-1 text-xs font-mono transition-colors",
            state.runtimeMode
              ? "bg-emerald-800/60 text-emerald-300"
              : "bg-zinc-800/50 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300",
          )}
        >
          {state.runtimeMode ? "ON" : "OFF"}
        </button>
      </div>

      <div className="h-6 w-px bg-zinc-800" />

      <button
        type="button"
        onClick={resetView}
        className="rounded bg-zinc-800/50 px-2.5 py-1 text-xs font-mono text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-300"
      >
        Reset
      </button>

      <div className="hidden items-center gap-2 xl:flex">
        <div className="h-6 w-px bg-zinc-800" />
        <span className="text-xs font-mono text-zinc-600">{state.nodes.size} nodes · {state.edges.length} edges</span>
      </div>
    </div>
  )
}
