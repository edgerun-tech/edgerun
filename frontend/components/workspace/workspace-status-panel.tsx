"use client"

import { useCallback, useMemo, useState } from "react"
import { useStore } from "@nanostores/react"
import { Ban, Check, Filter, Plus, Tags, X } from "lucide-react"
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
}

const FILTER_ORDER: Array<{ key: XrayFilterKey; label: string }> = [
  { key: "file", label: "Files" },
  { key: "function", label: "Functions" },
  { key: "ui", label: "UI" },
  { key: "runtime", label: "Runtime" },
  { key: "storage", label: "Storage" },
  { key: "network", label: "Network" },
  { key: "crypto", label: "Crypto" },
  { key: "agent", label: "Agent" },
  { key: "rust", label: "Rust" },
  { key: "typescript", label: "TypeScript" },
  { key: "javascript", label: "JavaScript" },
  { key: "go", label: "Go" },
  { key: "python", label: "Python" },
  { key: "c", label: "C" },
]

function buildFilters(state: ReturnType<typeof xrayState.get>): FilterItem[] {
  const counts = new Map<XrayFilterKey, number>()
  for (const node of state.nodes.values()) {
    for (const key of nodeFilterKeys(node)) {
      counts.set(key, (counts.get(key) || 0) + 1)
    }
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
        <div className="space-y-1">
          {filters.map((filter) => {
            const hidden = state.hiddenFilterKeys.has(filter.key)
            return (
              <button
                key={filter.key}
                type="button"
                onClick={() => toggleXrayFilter(filter.key)}
                className={cn(
                  "flex w-full min-w-0 items-center gap-2 rounded-lg bg-background/45 px-2 py-1.5 text-left text-[10px] transition-colors hover:bg-background/70",
                  hidden && "opacity-45",
                )}
                title={hidden ? `Show ${filter.label}` : `Hide ${filter.label}`}
              >
                <span className={cn("flex h-4 w-4 shrink-0 items-center justify-center rounded-full", hidden ? "text-muted-foreground" : "text-[var(--status-online)]")}>
                  {hidden ? <X className="h-3 w-3" /> : <Check className="h-3 w-3" />}
                </span>
                <span className="min-w-0 flex-1 truncate text-muted-foreground">{filter.label}</span>
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
