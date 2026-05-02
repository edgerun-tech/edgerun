"use client"

import { useStore } from "@nanostores/react"
import { resourceStore } from "@/platform/state/resource-store"
import { getDashboardMode } from "@/platform/runtime/dashboard-mode"
import { cn } from "@/lib/utils"
import {
  Cpu,
  MemoryStick,
  Network,
  HardDrive,
  Activity,
  CheckCircle2,
  AlertTriangle,
  Clock,
} from "lucide-react"

export function NodeGrid() {
  const resource = useStore(resourceStore)
  const nodes = resource.nodes ? Array.from(resource.nodes.values()) : []
  const mode = getDashboardMode()

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-foreground">Node Grid</h3>
        {mode === "demo" && (
          <span className="rounded bg-yellow-500/20 px-1.5 py-0.5 text-[9px] font-medium text-yellow-400">
            Demo
          </span>
        )}
      </div>

      {nodes.length === 0 && (
        <div className="rounded-lg border border-border bg-card p-4 text-center">
          <p className="text-sm text-muted-foreground">
            {mode === "demo" ? "No real node data (demo mode)" : "No nodes registered"}
          </p>
        </div>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
        {nodes.map((node) => {
          const isStale = Date.now() - node.lastHeartbeat > 60000
          const cpuPct = node.cpu.cores > 0 ? Math.round(node.cpu.utilization * 100) : 0
          const memPct = node.memory.total > 0 ? Math.round((node.memory.used / node.memory.total) * 100) : 0

          return (
            <div
              key={node.nodeId}
              className={cn(
                "rounded-lg border p-4",
                node.connectionState === "online" && node.health === "healthy"
                  ? "border-green-500/30 bg-green-500/5"
                  : node.connectionState === "online"
                  ? "border-yellow-500/30 bg-yellow-500/5"
                  : "border-red-500/30 bg-red-500/5"
              )}
            >
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2">
                  {node.connectionState === "online" ? (
                    node.health === "healthy" ? (
                      <CheckCircle2 className="h-4 w-4 text-green-500" />
                    ) : (
                      <AlertTriangle className="h-4 w-4 text-yellow-500" />
                    )
                  ) : (
                    <Clock className="h-4 w-4 text-red-500" />
                  )}
                  <span className="text-sm font-medium">{node.label || node.nodeId.slice(0, 12)}</span>
                </div>
                <span className={cn(
                  "rounded px-1.5 py-0.5 text-[9px] font-medium",
                  node.connectionState === "online" && node.health === "healthy"
                    ? "bg-green-500/20 text-green-400"
                    : node.connectionState === "online"
                    ? "bg-yellow-500/20 text-yellow-400"
                    : "bg-red-500/20 text-red-400"
                )}>
                  {node.connectionState === "online" 
                    ? node.health === "healthy" ? "healthy" : node.health || "degraded"
                    : "offline"}
                </span>
              </div>

              <div className="space-y-2 text-xs text-muted-foreground">
                <div className="flex items-center gap-1">
                  <Cpu className="h-3 w-3" />
                  <span>{node.cpu.cores} cores · {cpuPct}% utilized</span>
                </div>
                <div className="flex items-center gap-1">
                  <MemoryStick className="h-3 w-3" />
                  <span>
                    {Math.round(node.memory.used / 1024 / 1024 / 1024 * 10) / 10}/
                    {Math.round(node.memory.total / 1024 / 1024 / 1024 * 10) / 10} GB ({memPct}%)
                  </span>
                </div>
                <div className="flex items-center gap-1">
                  <Activity className="h-3 w-3" />
                  <span>{node.activeJobs} active jobs · {node.activeApps.length} apps</span>
                </div>
                {isStale && (
                  <p className="text-yellow-400">Stale data — last heartbeat {Math.round((Date.now() - node.lastHeartbeat) / 1000)}s ago</p>
                )}
                <p className="text-[9px]">Source: {node.cpu.source}</p>
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
