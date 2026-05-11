"use client"

import { useStore } from "@nanostores/react"
import { cn } from "@/lib/utils"
import { DashboardPanel } from "@/components/ui/dashboard-panel"
import {
  GitBranch,
  AlertTriangle,
  Package,
  Ban,
} from "lucide-react"
import { atom, computed } from "nanostores"
import { computedMapToList } from "@/platform/utils/computed"

export interface DepNode {
  name: string
  version: string
  source: "crates_io" | "workspace" | "external" | "demo"
  isIllegal: boolean
  usedBy: string[]
  usesIllegalDeps: string[]
}

export interface DependencyGraph {
  nodes: Map<string, DepNode>
  illegalEdges: Array<{ from: string; to: string; reason: string }>
  source: "real" | "demo"
}

const initialGraph: DependencyGraph = {
  nodes: new Map(),
  illegalEdges: [],
  source: "real",
}

export const dependencyStore = atom<DependencyGraph>(initialGraph)

export const allDepNodes = computed(dependencyStore, (g) =>
  Array.from(g.nodes.values()),
)

export const illegalDeps = computed(dependencyStore, (g) =>
  g.illegalEdges,
)

export function updateDependencyGraph(graph: DependencyGraph): void {
  dependencyStore.set(graph)
}

function DepNodeCard({ node }: { node: DepNode }) {
  return (
    <div className={cn(
      "rounded border p-2 text-xs space-y-1",
      node.isIllegal ? "border-red-500/30 bg-red-500/5" : "border-border bg-card",
    )}>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1">
          <Package className="h-3 w-3 text-muted-foreground" />
          <span className="font-medium">{node.name}</span>
          <span className="text-muted-foreground">v{node.version}</span>
        </div>
        {node.isIllegal && <Ban className="h-3 w-3 text-red-500" />}
        {node.source === "demo" && (
          <span className="rounded bg-yellow-500/20 px-1 py-0.5 text-[9px] text-yellow-400">
            Demo
          </span>
        )}
      </div>
      {node.usedBy.length > 0 && (
        <p className="text-[10px] text-muted-foreground">
          Used by: {node.usedBy.slice(0, 3).join(", ")}
          {node.usedBy.length > 3 && ` +${node.usedBy.length - 3}`}
        </p>
      )}
      {node.usesIllegalDeps.length > 0 && (
        <p className="text-[10px] text-red-400">
          Illegal deps: {node.usesIllegalDeps.join(", ")}
        </p>
      )}
    </div>
  )
}

export function DependencyGraph() {
  const nodes = useStore(allDepNodes)
  const illegalEdges = useStore(illegalDeps)
  const graph = useStore(dependencyStore)

  const workspaceNodes = nodes.filter(n => n.source === "workspace")
  const externalNodes = nodes.filter(n => n.source === "external" || n.isIllegal)

  return (
    <DashboardPanel
      title="Dependency Graph"
      summary={`${nodes.length} deps · ${illegalEdges.length} illegal`}
      empty={nodes.length === 0}
      emptyMessage="No dependency data"
      demoMessage="No real dependency data (demo mode)"
    >
      {illegalEdges.length > 0 && (
        <div className="rounded border border-red-500/30 bg-red-500/5 p-3">
          <div className="flex items-center gap-2 mb-2">
            <AlertTriangle className="h-4 w-4 text-red-500" />
            <span className="text-xs font-medium text-red-400">Illegal Dependencies Detected</span>
          </div>
          <div className="space-y-1">
            {illegalEdges.map((edge, i) => (
              <p key={i} className="text-[10px] text-red-400">
                {edge.from} → {edge.to}: {edge.reason}
              </p>
            ))}
          </div>
        </div>
      )}

      {workspaceNodes.length > 0 && (
        <>
          <h4 className="text-xs font-medium text-muted-foreground">Workspace Crates</h4>
          <div className="grid grid-cols-2 lg:grid-cols-3 gap-2">
            {workspaceNodes.map(node => (
              <DepNodeCard key={node.name} node={node} />
            ))}
          </div>
        </>
      )}

      {externalNodes.length > 0 && (
        <>
          <h4 className="text-xs font-medium text-muted-foreground flex items-center gap-1">
            {illegalEdges.length > 0 ? (
              <Ban className="h-3 w-3 text-red-500" />
            ) : (
              <GitBranch className="h-3 w-3 text-muted-foreground" />
            )}
            {illegalEdges.length > 0 ? "External/Illegal Dependencies" : "External Dependencies"}
          </h4>
          <div className="grid grid-cols-2 lg:grid-cols-3 gap-2">
            {externalNodes.map(node => (
              <DepNodeCard key={node.name} node={node} />
            ))}
          </div>
        </>
      )}
    </DashboardPanel>
  )
}
