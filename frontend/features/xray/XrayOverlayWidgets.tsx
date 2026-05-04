"use client"

import { useMemo } from "react"
import { useStore } from "@nanostores/react"
import { CircleHelp, FileCode2, FileText, Network, Shield, Tags } from "lucide-react"
import { cn } from "@/lib/utils"
import { nodeFilterKeys, toggleXrayFilter, xrayState } from "./graph/graph-store"
import type { XrayFilterKey, XrayNode } from "./graph/types"

type LegendItem = {
  key: XrayFilterKey
  label: string
  colorClass: string
  icon: React.ReactNode
  count: number
}

const COLOR_CLASSES: Record<string, string> = {
  file: "bg-zinc-400",
  function: "bg-sky-300",
  ui: "bg-sky-300",
  runtime: "bg-orange-400",
  storage: "bg-yellow-300",
  network: "bg-cyan-400",
  crypto: "bg-red-400",
  agent: "bg-zinc-500",
  rust: "bg-orange-500",
  typescript: "bg-blue-400",
  javascript: "bg-yellow-300",
  go: "bg-cyan-300",
  python: "bg-green-400",
  c: "bg-purple-400",
  unknown: "bg-zinc-500",
}

function makeLegend(nodes: Map<string, XrayNode>): LegendItem[] {
  const counts = new Map<XrayFilterKey, number>()
  for (const node of nodes.values()) {
    for (const key of nodeFilterKeys(node)) counts.set(key, (counts.get(key) || 0) + 1)
  }

  const order: Array<{ key: XrayFilterKey; label: string; icon: React.ReactNode }> = [
    { key: "file", label: "files", icon: <FileText className="h-3 w-3" /> },
    { key: "function", label: "functions", icon: <FileCode2 className="h-3 w-3" /> },
    { key: "ui", label: "ui", icon: <Tags className="h-3 w-3" /> },
    { key: "runtime", label: "runtime", icon: <Tags className="h-3 w-3" /> },
    { key: "storage", label: "storage", icon: <Tags className="h-3 w-3" /> },
    { key: "network", label: "network", icon: <Network className="h-3 w-3" /> },
    { key: "crypto", label: "crypto", icon: <Shield className="h-3 w-3" /> },
    { key: "rust", label: "rust", icon: <Tags className="h-3 w-3" /> },
    { key: "typescript", label: "ts", icon: <Tags className="h-3 w-3" /> },
    { key: "javascript", label: "js", icon: <Tags className="h-3 w-3" /> },
    { key: "go", label: "go", icon: <Tags className="h-3 w-3" /> },
    { key: "python", label: "py", icon: <Tags className="h-3 w-3" /> },
  ]

  return order
    .map((item) => ({ ...item, count: counts.get(item.key) || 0, colorClass: COLOR_CLASSES[item.key] || COLOR_CLASSES.unknown }))
    .filter((item) => item.count > 0)
}

export function XrayLegendOverlay() {
  const state = useStore(xrayState)
  const items = useMemo(() => makeLegend(state.nodes), [state.nodes])

  return (
    <div className="pointer-events-auto absolute left-4 top-4 z-30 hidden max-w-[220px] rounded-2xl border border-white/10 bg-black/35 p-2 backdrop-blur-md xl:block">
      <div className="mb-1 px-1 font-mono text-[9px] uppercase tracking-[0.22em] text-muted-foreground">Legend</div>
      <div className="grid grid-cols-2 gap-1">
        {items.map((item) => {
          const hidden = state.hiddenFilterKeys.has(item.key)
          return (
            <button
              key={item.key}
              type="button"
              onClick={() => toggleXrayFilter(item.key)}
              className={cn(
                "flex min-w-0 items-center gap-1.5 rounded-lg px-2 py-1 text-left text-[10px] transition-colors",
                hidden ? "text-muted-foreground/35 opacity-50" : "text-foreground hover:bg-white/5",
              )}
              title={`${hidden ? "Show" : "Hide"} ${item.label}`}
            >
              <span className={cn("h-2 w-2 shrink-0 rounded-full", item.colorClass)} />
              <span className="min-w-0 flex-1 truncate">{item.label}</span>
              <span className="shrink-0 font-mono text-[9px] text-muted-foreground">{item.count}</span>
            </button>
          )
        })}
      </div>
    </div>
  )
}

export function XrayMouseHelpOverlay() {
  return (
    <div className="pointer-events-none absolute bottom-4 right-4 z-30 hidden max-w-[240px] rounded-2xl border border-white/10 bg-black/30 p-3 text-[10px] text-muted-foreground backdrop-blur-md xl:block">
      <div className="mb-2 flex items-center gap-1.5 font-mono uppercase tracking-[0.2em] text-foreground/80">
        <CircleHelp className="h-3.5 w-3.5 text-primary" /> Mouse
      </div>
      <div className="space-y-1 font-mono">
        <div className="flex justify-between gap-4"><span>drag empty</span><span className="text-foreground/80">rotate</span></div>
        <div className="flex justify-between gap-4"><span>ctrl + drag</span><span className="text-foreground/80">pan</span></div>
        <div className="flex justify-between gap-4"><span>scroll</span><span className="text-foreground/80">zoom</span></div>
        <div className="flex justify-between gap-4"><span>hover node</span><span className="text-foreground/80">details</span></div>
        <div className="flex justify-between gap-4"><span>click node</span><span className="text-foreground/80">select</span></div>
      </div>
    </div>
  )
}
