"use client"

import { useCallback, useEffect, useMemo, useState } from "react"
import { Activity, FolderGit2, GitBranch, Network, RefreshCw, Server, Settings2, ShieldAlert, Unplug } from "lucide-react"
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

type BridgeFile = {
  path: string
  language: string
  size: number
  modified_ts: number
}

type BridgeGraph = {
  nodes?: BridgeNode[]
  edges?: BridgeEdge[]
  files?: BridgeFile[]
  node_count?: number
  edge_count?: number
  file_count?: number
  total_bytes?: number
  elapsed_ms?: number
  source?: string
}

type LocalConnection = {
  protocol: string
  local_address: string
  local_port: number
  remote_address: string
  remote_port: number
  state: string
}

type BridgeStatus = "checking" | "online" | "offline"
type NetworkWidgetPage = "connections" | "policy"

const BRIDGE_GRAPH_URL = "http://localhost:13337/graph"
const BRIDGE_CONNECTIONS_URL = "http://localhost:13337/connections"

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

function useCodelyzerBridge() {
  const [status, setStatus] = useState<BridgeStatus>("checking")
  const [graph, setGraph] = useState<BridgeGraph | null>(null)
  const [connections, setConnections] = useState<LocalConnection[]>([])
  const [error, setError] = useState<string | null>(null)
  const [lastRefresh, setLastRefresh] = useState<Date | null>(null)

  const refresh = useCallback(async () => {
    setStatus("checking")
    setError(null)
    try {
      const [graphResponse, connectionResponse] = await Promise.all([
        fetch(BRIDGE_GRAPH_URL, { cache: "no-store" }),
        fetch(BRIDGE_CONNECTIONS_URL, { cache: "no-store" }),
      ])
      if (!graphResponse.ok) throw new Error(`graph HTTP ${graphResponse.status}`)
      if (!connectionResponse.ok) throw new Error(`connections HTTP ${connectionResponse.status}`)
      const graphJson = await graphResponse.json()
      const connectionJson = await connectionResponse.json()
      const nextGraph = graphJson?.type === "graph_update" ? graphJson.data : graphJson
      setGraph(nextGraph)
      setConnections(Array.isArray(connectionJson?.connections) ? connectionJson.connections : [])
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

  const activeCodeConnections = useMemo(() => {
    return [...(graph?.edges || [])]
      .filter((edge) => edge.source && edge.target)
      .slice(0, 12)
  }, [graph])

  const suspiciousConnections = useMemo(() => {
    return connections.filter((connection) => {
      const ip = connection.remote_address
      return !(ip.startsWith("127.") || ip === "::1" || ip.startsWith("10.") || ip.startsWith("192.168.") || ip.startsWith("172.16."))
    })
  }, [connections])

  const languageSummary = useMemo(() => {
    const counts = new Map<string, number>()
    for (const node of graph?.nodes || []) {
      const key = node.language || "unknown"
      counts.set(key, (counts.get(key) || 0) + 1)
    }
    return [...counts.entries()].sort((a, b) => b[1] - a[1]).slice(0, 6)
  }, [graph])

  const topFiles = useMemo(() => {
    return [...(graph?.files || [])].sort((a, b) => b.size - a.size).slice(0, 7)
  }, [graph])

  return { status, graph, connections, suspiciousConnections, error, lastRefresh, refresh, nodeMap, activeCodeConnections, languageSummary, topFiles }
}

function statusClass(status: BridgeStatus) {
  return status === "online"
    ? "border-[var(--status-online)]/30 bg-[var(--status-online)]/10 text-[var(--status-online)]"
    : status === "checking"
      ? "border-primary/30 bg-primary/10 text-primary"
      : "border-[var(--status-error)]/30 bg-[var(--status-error)]/10 text-[var(--status-error)]"
}

function MiniStat({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="rounded-xl border border-border/70 bg-background/55 p-2">
      <div className="text-[9px] uppercase tracking-[0.18em] text-muted-foreground">{label}</div>
      <div className="mt-1 truncate font-mono text-xs text-foreground">{value}</div>
    </div>
  )
}

export function NetworkConnectionsWidget() {
  const { status, connections, suspiciousConnections, error, refresh } = useCodelyzerBridge()
  const [page, setPage] = useState<NetworkWidgetPage>("connections")
  const visibleConnections = suspiciousConnections.length > 0 ? suspiciousConnections : connections

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden bg-background/5 p-3 text-foreground">
      <div className="mb-2 flex items-center justify-between gap-2">
        <div className="min-w-0">
          <div className="flex items-center gap-1.5 text-xs font-semibold">
            <Network className="h-3.5 w-3.5 text-primary" />
            Network
          </div>
          <div className="mt-0.5 text-[10px] text-muted-foreground">Active external IPs</div>
        </div>
        <button type="button" onClick={() => void refresh()} className={cn("rounded-full border px-2 py-0.5 font-mono text-[10px]", statusClass(status))}>{status}</button>
      </div>

      <div className="grid grid-cols-2 gap-2">
        <MiniStat label="active" value={connections.length} />
        <MiniStat label="watch" value={suspiciousConnections.length} />
      </div>

      <div className="mt-2 min-h-0 flex-1 space-y-1 overflow-hidden">
        {page === "policy" ? (
          <div className="space-y-1 text-[10px]">
            {[
              ["VPN", "off"],
              ["Relay", "off"],
              ["Discovery", "on"],
              ["Region", "auto"],
            ].map(([label, value]) => (
              <div key={label} className="flex items-center justify-between rounded-lg bg-background/45 px-2 py-1.5">
                <span className="text-muted-foreground">{label}</span>
                <span className="font-mono text-foreground">{value}</span>
              </div>
            ))}
          </div>
        ) : error ? (
          <div className="rounded-lg bg-[var(--status-error)]/10 px-2 py-1.5 text-[10px] text-[var(--status-error)]">Bridge offline · {error}</div>
        ) : visibleConnections.length === 0 ? (
          <div className="rounded-lg bg-background/45 px-2 py-1.5 text-[10px] text-muted-foreground">No external connections.</div>
        ) : visibleConnections.slice(0, 6).map((connection) => (
          <div key={`${connection.protocol}-${connection.remote_address}-${connection.remote_port}`} className="rounded-lg bg-background/45 px-2 py-1.5 text-[10px]">
            <div className="flex items-center gap-1.5">
              <ShieldAlert className="h-3 w-3 shrink-0 text-primary" />
              <span className="min-w-0 flex-1 truncate font-mono text-foreground">{connection.remote_address}</span>
              <span className="font-mono text-muted-foreground">:{connection.remote_port}</span>
            </div>
            <div className="mt-0.5 flex justify-between text-[9px] text-muted-foreground">
              <span>{connection.protocol}</span>
              <span>{connection.state}</span>
            </div>
          </div>
        ))}
      </div>

      <div className="mt-2 flex items-center justify-center gap-2">
        <button type="button" onClick={() => setPage("connections")} className={cn("rounded-full p-1.5 transition-colors", page === "connections" ? "text-[var(--status-online)]" : "text-muted-foreground hover:text-foreground")} title="Connections">
          <Network className="h-3.5 w-3.5" />
        </button>
        <button type="button" onClick={() => setPage("policy")} className={cn("rounded-full p-1.5 transition-colors", page === "policy" ? "text-[var(--status-online)]" : "text-muted-foreground hover:text-foreground")} title="Network settings">
          <Settings2 className="h-3.5 w-3.5" />
        </button>
      </div>
    </div>
  )
}

export function CodelyzerCodeWidget() {
  const { status, graph, error, refresh, topFiles } = useCodelyzerBridge()

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden bg-background/5 p-3 text-foreground">
      <div className="mb-2 flex items-center justify-between gap-2">
        <div className="min-w-0">
          <div className="flex items-center gap-1.5 text-xs font-semibold">
            <FolderGit2 className="h-3.5 w-3.5 text-primary" />
            Code
          </div>
          <div className="mt-0.5 text-[10px] text-muted-foreground">Opened git repo</div>
        </div>
        <button type="button" onClick={() => void refresh()} className={cn("rounded-full border px-2 py-0.5 font-mono text-[10px]", statusClass(status))}>{status}</button>
      </div>

      <div className="grid grid-cols-2 gap-2">
        <MiniStat label="nodes" value={graph?.node_count ?? graph?.nodes?.length ?? 0} />
        <MiniStat label="edges" value={graph?.edge_count ?? graph?.edges?.length ?? 0} />
      </div>

      <div className="mt-2 min-h-0 flex-1 space-y-1 overflow-hidden">
        {error ? (
          <div className="rounded-lg bg-[var(--status-error)]/10 px-2 py-1.5 text-[10px] text-[var(--status-error)]">Bridge offline · {error}</div>
        ) : topFiles.length === 0 ? (
          <div className="rounded-lg bg-background/45 px-2 py-1.5 text-[10px] text-muted-foreground">No repo files indexed.</div>
        ) : topFiles.map((file) => (
          <div key={file.path} className="rounded-lg bg-background/45 px-2 py-1.5 text-[10px]">
            <div className="flex items-center gap-1.5">
              <FolderGit2 className="h-3 w-3 shrink-0 text-primary" />
              <span className="min-w-0 flex-1 truncate text-foreground">{shortFile(file.path)}</span>
              <span className="font-mono text-muted-foreground">{formatBytes(file.size)}</span>
            </div>
            <div className="mt-0.5 text-[9px] text-muted-foreground">{file.language}</div>
          </div>
        ))}
      </div>

      <div className="mt-2 truncate text-[10px] text-muted-foreground">{graph?.source ? shortFile(graph.source) : BRIDGE_GRAPH_URL}</div>
    </div>
  )
}

export function CodelyzerNetworkPanel() {
  const { status, graph, error, lastRefresh, refresh, nodeMap, activeCodeConnections, languageSummary } = useCodelyzerBridge()
  const statusTone = statusClass(status)

  return (
    <div className="space-y-4">
      <div className="grid gap-3 md:grid-cols-4">
        <Card className="border-[var(--window-border)] bg-card/55 py-4 shadow-none">
          <CardContent className="px-3">
            <div className="mb-1 flex items-center gap-2 text-[10px] uppercase tracking-widest text-muted-foreground"><Server className="h-3 w-3" /> Bridge</div>
            <Badge variant="outline" className={cn("text-[10px]", statusTone)}>{status}</Badge>
          </CardContent>
        </Card>
        <Card className="border-[var(--window-border)] bg-card/55 py-4 shadow-none"><CardContent className="px-3"><p className="font-mono text-lg font-semibold text-primary">{graph?.node_count ?? graph?.nodes?.length ?? 0}</p><p className="text-[10px] uppercase tracking-widest text-muted-foreground">nodes</p></CardContent></Card>
        <Card className="border-[var(--window-border)] bg-card/55 py-4 shadow-none"><CardContent className="px-3"><p className="font-mono text-lg font-semibold text-primary">{graph?.edge_count ?? graph?.edges?.length ?? 0}</p><p className="text-[10px] uppercase tracking-widest text-muted-foreground">connections</p></CardContent></Card>
        <Card className="border-[var(--window-border)] bg-card/55 py-4 shadow-none"><CardContent className="px-3"><p className="font-mono text-lg font-semibold text-primary">{graph?.elapsed_ms ?? 0}ms</p><p className="text-[10px] uppercase tracking-widest text-muted-foreground">scan time</p></CardContent></Card>
      </div>

      <Card className="border-[var(--window-border)] bg-card/55 shadow-none">
        <CardHeader className="flex flex-row items-start justify-between gap-4">
          <div>
            <CardTitle className="flex items-center gap-2 text-sm"><GitBranch className="h-4 w-4 text-primary" /> Active codelyzer graph links</CardTitle>
            <CardDescription className="text-xs">Function and module relationships from the local codelyzer bridge.</CardDescription>
          </div>
          <Button size="xs" variant="outline" onClick={() => void refresh()}><RefreshCw className={cn("h-3.5 w-3.5", status === "checking" && "animate-spin")} />Refresh</Button>
        </CardHeader>
        <CardContent>
          {error ? <div className="rounded-xl border border-[var(--status-error)]/20 bg-[var(--status-error)]/10 p-3 text-xs text-[var(--status-error)]"><div className="mb-1 flex items-center gap-2 font-medium"><Unplug className="h-3.5 w-3.5" /> Bridge unavailable</div><div className="font-mono">{error}</div></div> : activeCodeConnections.length === 0 ? <div className="rounded-xl border border-dashed border-border p-4 text-xs text-muted-foreground">No codelyzer connections loaded yet.</div> : (
            <div className="max-h-[360px] space-y-1 overflow-auto pr-1">
              {activeCodeConnections.map((edge, index) => {
                const source = nodeMap.get(edge.source)
                const target = nodeMap.get(edge.target)
                return (
                  <div key={edge.id || `${edge.source}-${edge.target}-${index}`} className="rounded-xl border border-border/70 bg-background/45 p-3">
                    <div className="flex items-center gap-2 text-xs"><span className="min-w-0 flex-1 truncate font-medium text-foreground">{displayNode(source, edge.source)}</span><span className="rounded-full border border-primary/20 bg-primary/10 px-2 py-0.5 font-mono text-[10px] text-primary">{edge.kind || "calls"}</span><span className="min-w-0 flex-1 truncate text-right font-medium text-foreground">{displayNode(target, edge.target)}</span></div>
                    <div className="mt-1 flex items-center justify-between gap-3 text-[10px] text-muted-foreground"><span className="truncate">{shortFile(source?.file)}</span><span className="truncate text-right">{shortFile(target?.file)}</span></div>
                  </div>
                )
              })}
            </div>
          )}
        </CardContent>
      </Card>

      <Card className="border-[var(--window-border)] bg-card/55 shadow-none">
        <CardHeader><CardTitle className="flex items-center gap-2 text-sm"><GitBranch className="h-4 w-4 text-primary" /> Source summary</CardTitle><CardDescription className="text-xs">{graph?.source || "No repository source loaded"} · {formatBytes(graph?.total_bytes)} indexed{lastRefresh ? ` · refreshed ${lastRefresh.toLocaleTimeString()}` : ""}</CardDescription></CardHeader>
        <CardContent><div className="flex flex-wrap gap-2">{languageSummary.length === 0 ? <span className="text-xs text-muted-foreground">No language data yet.</span> : languageSummary.map(([language, count]) => <span key={language} className="rounded-full border border-border bg-background/50 px-2.5 py-1 font-mono text-[11px] text-muted-foreground">{language}: <span className="text-foreground">{count}</span></span>)}</div></CardContent>
      </Card>
    </div>
  )
}
