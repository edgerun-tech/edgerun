"use client"

import { useStore } from "@nanostores/react"
import { resourceStore, formatResourceSummary } from "@/platform/state/resource-store"
import { getDashboardMode } from "@/platform/runtime/dashboard-mode"
import { cn } from "@/lib/utils"
import { Progress } from "@/components/ui/progress"
import {
  Cpu,
  MemoryStick,
  Network,
  HardDrive,
  Activity,
} from "lucide-react"

export function ResourceOverview() {
  const resource = useStore(resourceStore)
  const agg = resource.aggregate
  const mode = getDashboardMode()

  if (!agg) {
    return (
      <div className="rounded-lg border border-border bg-card p-4">
        <h3 className="text-sm font-medium text-foreground mb-2">Resource Overview</h3>
        <p className="text-xs text-muted-foreground">
          {mode === "demo" ? "No real resource data (demo mode)" : "No resource data available"}
        </p>
      </div>
    )
  }

  const cpuPct = agg.totalCores > 0 ? Math.round(agg.weightedCpuUtilization * 100) : 0
  const memPct = agg.totalMemory > 0 ? Math.round((agg.usedMemory / agg.totalMemory) * 100) : 0
  const diskPct = agg.totalDisk > 0 ? Math.round((agg.usedDisk / agg.totalDisk) * 100) : 0

  return (
    <div className="space-y-4">
      <div className="rounded-lg border border-border bg-card p-4">
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-sm font-medium text-foreground">Resource Summary</h3>
          {mode === "demo" && (
            <span className="rounded bg-yellow-500/20 px-1.5 py-0.5 text-[9px] font-medium text-yellow-400">
              Demo
            </span>
          )}
        </div>
        <p className="text-sm text-muted-foreground">{formatResourceSummary()}</p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
        {/* CPU */}
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="flex items-center gap-2 mb-2">
            <Cpu className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm font-medium">CPU</span>
          </div>
          <div className="text-2xl font-bold">{cpuPct}%</div>
          <p className="text-xs text-muted-foreground">
            {agg.totalCores} cores · weighted
          </p>
          <Progress value={cpuPct} className="mt-2" />
        </div>

        {/* Memory */}
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="flex items-center gap-2 mb-2">
            <MemoryStick className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm font-medium">Memory</span>
          </div>
          <div className="text-2xl font-bold">
            {Math.round(agg.usedMemory / 1024 / 1024 / 1024 * 10) / 10} GB
          </div>
          <p className="text-xs text-muted-foreground">
            of {Math.round(agg.totalMemory / 1024 / 1024 / 1024 * 10) / 10} GB ({memPct}%)
          </p>
          <Progress value={memPct} className="mt-2" />
        </div>

        {/* Disk */}
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="flex items-center gap-2 mb-2">
            <HardDrive className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm font-medium">Disk</span>
          </div>
          <div className="text-2xl font-bold">
            {Math.round(agg.usedDisk / 1024 / 1024 / 1024 * 10) / 10} GB
          </div>
          <p className="text-xs text-muted-foreground">
            of {Math.round(agg.totalDisk / 1024 / 1024 / 1024 * 10) / 10} GB ({diskPct}%)
          </p>
          <Progress value={diskPct} className="mt-2" />
        </div>
      </div>

      {/* Network & Jobs */}
      <div className="grid grid-cols-2 gap-3">
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="flex items-center gap-2 mb-2">
            <Network className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm font-medium">Nodes</span>
          </div>
          <div className="text-2xl font-bold">{agg.connectedNodes}/{agg.totalNodes}</div>
          <p className="text-xs text-muted-foreground">
            {agg.unhealthyNodes > 0 && `${agg.unhealthyNodes} unhealthy · `}
            {agg.staleNodes > 0 && `${agg.staleNodes} stale · `}
            {agg.availableCapacity} capacity
          </p>
        </div>

        <div className="rounded-lg border border-border bg-card p-4">
          <div className="flex items-center gap-2 mb-2">
            <Activity className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm font-medium">Active Jobs</span>
          </div>
          <div className="text-2xl font-bold">{agg.activeJobs}</div>
          <p className="text-xs text-muted-foreground">
            across {agg.totalNodes} nodes
          </p>
        </div>
      </div>
    </div>
  )
}
