"use client"

import { useCallback, useMemo, useState } from "react"
import { useStore } from "@nanostores/react"
import { Ban, Filter, Plus, Tags } from "lucide-react"
import { cn } from "@/lib/utils"
import { nodeFilterKeys, toggleXrayFilter, xrayState } from "@/features/xray/graph/graph-store"
import type { XrayFilterKey } from "@/features/xray/graph/types"

interface WorkspaceStatusPanelProps {
  className?: string
  surface?: "floating" | "embedded"
}

type FilterItem = {
  key: XrayFilterKey
  label: string
  count: number
  color: string
}

const FILTER_ORDER: Array<{ key: XrayFilterKey; label: string; color: string }> = [
  { key: "file", label: "Files", color: "bg-zinc-400" },
  { key: "function", label: "Functions", color: "bg-sky-300" },
  { key: "ui", label: "UI", color: "bg-sky-300" },
  { key: "runtime", label: "Runtime", color: "bg-orange-400" },
  { key: "storage", label: "Storage", color: "bg-yellow-300" },
  { key: "network", label: "Network", color: "bg-cyan-400" },
  { key: "crypto", label: "Crypto", color: "bg-red-400" },
  { key: "agent", label: "Agent", color: "bg-zinc-500" },
  { key: "rust", label: "Rust", color: "bg-orange-500" },
  { key: "typescript", label: "TypeScript", color: "bg-blue-400" },
  { key: "javascript", label: "JavaScript", color: "bg-yellow-300" },
  { key: "go", label: "Go", color: "bg-cyan-300" },
  { key: "python", label: "Python", color: "bg-green-400" },
  { key: "c", label: "C", color: "bg-purple-400" },
]

function buildFilters(state: ReturnType<typeof xrayState.get>): FilterItem[] {
  const counts = new Map<XrayFilterKey, number>()
  for (const node of state.nodes.values()) {
    for (const key of nodeFilterKeys(node)) counts.set(key, (counts.get(key) || 0) + 1)
  }
  return FILTER_ORDER
    .map((item) => ({ ...item, count: counts.get(item.key) || 0 }))
    .filter((item) => item.count > 0)
}

export function WorkspaceStatusPanel({ className, surface = "floating" }: WorkspaceStatusPanelProps) {
  const state = useStore(xrayState)
  const [customTag, setCustomTag] = useState("")
  const [customTags, setCustomTags] = useState<string[]>([])
  const embedded = surface === "embedded"
  const filters = useMemo(() => buildFilters(state), [state])
  const hiddenCount = state.hiddenFilterKeys.size
  const visibleNodeCount = useMemo(() => {
    if (hiddenCount === 0) return state.nodes.size
    let count = 0
    for (const node of state.nodes.values()) {
      const hidden = nodeFilterKeys(node).some((key) => state.hiddenFilterKeys.has(key))
      if (!hidden) count++
    }
    return count
  }, [hiddenCount, state])

  function addCustomTag() {
    const tag = customTag.trim().replace(/^#/, "")
    if (!tag || customTags.includes(tag)) return
    setCustomTags([...customTags, tag])
    setCustomTag("")
  }

  return (
    <div
      className={cn(
        "pointer-events-auto flex flex-col gap-2 rounded-lg bg-transparent text-foreground",
        embedded ? "h-full w-full overflow-hidden" : "fixed bottom-4 left-4 z-40",
        className,
      )}
    >
      <div className="flex shrink-0 items-center justify-between gap-2 px-3 pt-3">
        <div className="min-w-0">
          <div className="flex items-center gap-1.5 text-xs font-semibold">
            <Filter className="h-3.5 w-3.5 text-primary" />
            Filters
          </div>
          <div className="mt-0.5 truncate text-[10px] text-muted-foreground">
            {visibleNodeCount} / {state.nodes.size} nodes visible
          </div>
        </div>
        <div className="flex items-center gap-1 rounded-full bg-background/45 px-2 py-0.5 font-mono text-[10px] text-muted-foreground">
          <Ban className="h-3 w-3" /> {hiddenCount}
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto px-3 pb-3">
        <div className="flex flex-wrap gap-1.5">
          {filters.map((filter) => {
            const hidden = state.hiddenFilterKeys.has(filter.key)
            return (
              <button
                key={filter.key}
                type="button"
                onClick={() => toggleXrayFilter(filter.key)}
                className={cn(
                  "flex max-w-full items-center gap-1.5 rounded-full border border-border/70 bg-background/45 px-2 py-1 text-[10px] transition-all hover:border-primary/30 hover:bg-background/70",
                  hidden && "opacity-60 grayscale",
                )}
                title={hidden ? `Show ${filter.label}` : `Hide ${filter.label}`}
              >
                <span className={cn("h-2 w-2 shrink-0 rounded-full", filter.color)} />
                <span className="min-w-0 truncate text-muted-foreground">{filter.label}</span>
                <span className="shrink-0 font-mono text-foreground">{filter.count}</span>
              </button>
            )
          })}
        </div>

        <div className="mt-3 rounded-xl border border-border/60 bg-background/35 p-2">
          <div className="mb-2 flex items-center gap-1.5 text-[10px] font-medium text-muted-foreground">
            <Tags className="h-3 w-3 text-primary" /> Custom tags
          </div>
          <div className="flex gap-1">
            <input
              value={customTag}
              onChange={(event) => setCustomTag(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") addCustomTag()
              }}
              placeholder="tag"
              className="min-w-0 flex-1 rounded-md border border-border bg-background/45 px-2 py-1 text-[10px] outline-none placeholder:text-muted-foreground/45"
            />
            <button type="button" onClick={addCustomTag} className="rounded-md border border-border bg-background/45 px-2 text-muted-foreground hover:text-foreground">
              <Plus className="h-3 w-3" />
            </button>
          </div>
          {customTags.length > 0 ? (
            <div className="mt-2 flex flex-wrap gap-1">
              {customTags.map((tag) => (
                <button
                  key={tag}
                  type="button"
                  onClick={() => setCustomTags(customTags.filter((candidate) => candidate !== tag))}
                  className="rounded-full bg-primary/10 px-2 py-0.5 text-[9px] text-primary"
                  title="Remove custom tag"
                >
                  #{tag}
                </button>
              ))}
            </div>
          ) : (
            <div className="mt-2 text-[9px] text-muted-foreground/70">
              Tag assignment comes next; this is the control surface.
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

export function useWorkspaceAwareness() {
  const refreshGitStatus = useCallback(() => {
    if (typeof window === "undefined") return
  }, [])

  return { refreshGitStatus }
}
