"use client"

import { useCallback, useEffect, useMemo, useState } from "react"
import { Activity, GitBranch, Network, RefreshCw, Server, Unplug } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { cn } from "@/lib/utils"

type BridgeNode = {
  id: string
  name?: string
  label?: string
  file?: string
  language?: string
  tags?: string[]
}

type BridgeEdge = {
  id?: string
  source: string
  target: string
  kind?: string
  tags?: string[]
}

type BridgeGraph = {
  nodes?: BridgeNode[]
  edges?: BridgeEdge[]
  node_count?: number
  edge_count?: number
  total_bytes?: number
  elapsed_ms?: number
  source?: string
}

type BridgeStatus = "checking" | "online" | "offline"

const BRIDGE_URL = "http://localhost:13337/graph"

function displayNode(node: BridgeNode | undefined, id: string) {
  if (!node) return id
  return node.name || node.label || node.id
}

function shortFile(file?: string) {
  if (!file) return "unknown"
  const parts = file.split("/")
  return parts.length > 3 ? parts.slice(-3).join("/") : file
}

function formatBytes(bytes?: number) {
  if (!bytes) return "0 B"
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MiB`
}

export function CodelyzerNetworkPanel() {
  const [status, setStatus] = useState<BridgeStatus>("checking")
  const [graph, setGraph] = useState<BridgeGraph | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [lastRefresh, setLastRefresh] = useState<Date | null>(null)

  const refresh = useCallback(async () => {
    setStatus("checking")
    setError(null)
    try {
      const response = await fetch(BRIDGE_URL, { cache: "no-store" })
      if (!response.ok) throw new Error(`HTTP ${response.status}`)
      const json = await response.json()
      const nextGraph = json?.type === "graph_update" ? json.data : json
      setGraph(nextGraph)
      setStatus("online")
      setLastRefresh(new Date())
    } catch (err) {
      setStatus("offline")
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [])

  useEffect(() => {
    void refresh()
  }, [refresh])

  const nodeMap = useMemo(() => {
    const map = new Map<string, BridgeNode>()
    for (const node of graph?.nodes || []) map.set(node.id, node)
    return map
  }, [graph])

  const activeConnections = useMemo(() => {
    return [...(graph?.edges || [])]
      .filter((edge) => edge.source && edge.target)
      .slice(0, 12)
  }, [graph])

  const languageSummary = useMemo(() => {
    const counts = new Map<string, number>()
    for (const node of graph?.nodes || []) {
      const key = node.language || "unknown"
      counts.set(key, (counts.get(key) || 0) + 1)
    }
    return [...counts.entries()].sort((a, b) => b[1] - a[1]).slice(0, 6)
  }, [graph])

  const statusTone = status === "online"
    ? "border-[var(--status-online)]/30 bg-[var(--status-online)]/10 text-[var(--status-online)]"
    : status === "checking"
      ? "border-primary/30 bg-primary/10 text-primary"
      : "border-[var(--status-error)]/30 bg-[var(--status-error)]/10 text-[var(--status-error)]"

  return (
    <div className="space-y-4">
      <div className="grid gap-3 md:grid-cols-4">
        <Card className="border-[var(--window-border)] bg-card/55 py-4 shadow-none">
          <CardContent className="px-3">
            <div className="mb-1 flex items-center gap-2 text-[10px] uppercase tracking-widest text-muted-foreground">
              <Server className="h-3 w-3" /> Bridge
            </div>
            <Badge variant="outline" className={cn("text-[10px]", statusTone)}>
              {status}
            </Badge>
          </CardContent>
        </Card>
        <Card className="border-[var(--window-border)] bg-card/55 py-4 shadow-none">
          <CardContent className="px-3">
            <p className="font-mono text-lg font-semibold text-primary">{graph?.node_count ?? graph?.nodes?.length ?? 0}</p>
            <p className="text-[10px] uppercase tracking-widest text-muted-foreground">nodes</p>
          </CardContent>
        </Card>
        <Card className="border-[var(--window-border)] bg-card/55 py-4 shadow-none">
          <CardContent className="px-3">
            <p className="font-mono text-lg font-semibold text-primary">{graph?.edge_count ?? graph?.edges?.length ?? 0}</p>
            <p className="text-[10px] uppercase tracking-widest text-muted-foreground">connections</p>
          </CardContent>
        </Card>
        <Card className="border-[var(--window-border)] bg-card/55 py-4 shadow-none">
          <CardContent className="px-3">
            <p className="font-mono text-lg font-semibold text-primary">{graph?.elapsed_ms ?? 0}ms</p>
            <p className="text-[10px] uppercase tracking-widest text-muted-foreground">scan time</p>
          </CardContent>
        </Card>
      </div>

      <Card className="border-[var(--window-border)] bg-card/55 shadow-none">
        <CardHeader className="flex flex-row items-start justify-between gap-4">
          <div>
            <CardTitle className="flex items-center gap-2 text-sm">
              <Network className="h-4 w-4 text-primary" />
              Active codelyzer connections
            </CardTitle>
            <CardDescription className="text-xs">
              Function and module relationships from the local codelyzer bridge.
            </CardDescription>
          </div>
          <Button size="xs" variant="outline" onClick={() => void refresh()}>
            <RefreshCw className={cn("h-3.5 w-3.5", status === "checking" && "animate-spin")} />
            Refresh
          </Button>
        </CardHeader>
        <CardContent>
          {error ? (
            <div className="rounded-xl border border-[var(--status-error)]/20 bg-[var(--status-error)]/10 p-3 text-xs text-[var(--status-error)]">
              <div className="mb-1 flex items-center gap-2 font-medium"><Unplug className="h-3.5 w-3.5" /> Bridge unavailable</div>
              <div className="font-mono">{error}</div>
              <div className="mt-2 text-muted-foreground">Expected endpoint: {BRIDGE_URL}</div>
            </div>
          ) : activeConnections.length === 0 ? (
            <div className="rounded-xl border border-dashed border-border p-4 text-xs text-muted-foreground">
              No codelyzer connections loaded yet.
            </div>
          ) : (
            <div className="max-h-[360px] space-y-1 overflow-auto pr-1">
              {activeConnections.map((edge, index) => {
                const source = nodeMap.get(edge.source)
                const target = nodeMap.get(edge.target)
                return (
                  <div key={edge.id || `${edge.source}-${edge.target}-${index}`} className="rounded-xl border border-border/70 bg-background/45 p-3">
                    <div className="flex items-center gap-2 text-xs">
                      <span className="min-w-0 flex-1 truncate font-medium text-foreground">{displayNode(source, edge.source)}</span>
                      <span className="rounded-full border border-primary/20 bg-primary/10 px-2 py-0.5 font-mono text-[10px] text-primary">{edge.kind || "calls"}</span>
                      <span className="min-w-0 flex-1 truncate text-right font-medium text-foreground">{displayNode(target, edge.target)}</span>
                    </div>
                    <div className="mt-1 flex items-center justify-between gap-3 text-[10px] text-muted-foreground">
                      <span className="truncate">{shortFile(source?.file)}</span>
                      <span className="truncate text-right">{shortFile(target?.file)}</span>
                    </div>
                  </div>
                )
              })}
            </div>
          )}
        </CardContent>
      </Card>

      <Card className="border-[var(--window-border)] bg-card/55 shadow-none">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-sm">
            <GitBranch className="h-4 w-4 text-primary" />
            Source summary
          </CardTitle>
          <CardDescription className="text-xs">
            {graph?.source || "No repository source loaded"} · {formatBytes(graph?.total_bytes)} indexed
            {lastRefresh ? ` · refreshed ${lastRefresh.toLocaleTimeString()}` : ""}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap gap-2">
            {languageSummary.length === 0 ? (
              <span className="text-xs text-muted-foreground">No language data yet.</span>
            ) : languageSummary.map(([language, count]) => (
              <span key={language} className="rounded-full border border-border bg-background/50 px-2.5 py-1 font-mono text-[11px] text-muted-foreground">
                {language}: <span className="text-foreground">{count}</span>
              </span>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
