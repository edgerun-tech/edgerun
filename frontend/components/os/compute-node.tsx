"use client"

import { useRef, useEffect, useState } from "react"
import { Cpu, Activity, CheckCircle2, Clock, AlertCircle, TrendingUp } from "lucide-react"
import { systemStatsStore } from "@/stores/desktop-store"
import { useStore } from "@nanostores/react"

interface WorkloadItem {
  id: string
  name: string
  status: "running" | "idle" | "error" | "pending"
  duration: string
  cpuUsage: number
}

export function ComputeNode() {
  const stats = useStore(systemStatsStore)
  const [workloads, setWorkloads] = useState<WorkloadItem[]>([
    { id: "wl-1", name: "xray-graph-render", status: "running", duration: "2h 14m", cpuUsage: 34 },
    { id: "wl-2", name: "trust-verify", status: "running", duration: "45m", cpuUsage: 18 },
    { id: "wl-3", name: "mesh-sync", status: "idle", duration: "--", cpuUsage: 0 },
    { id: "wl-4", name: "storage-compact", status: "pending", duration: "--", cpuUsage: 0 },
  ])

  const [sparkData, setSparkData] = useState<number[]>([28, 42, 35, 58, 47, 62, 55, 71, 64, 78, 69, 82])

  useEffect(() => {
    const interval = setInterval(() => {
      setSparkData((prev) => {
        const next = [...prev.slice(1), Math.floor(Math.random() * 40) + 40]
        return next
      })
    }, 2000)
    return () => clearInterval(interval)
  }, [])

  const statusColor = {
    running: "text-emerald-500",
    idle: "text-amber-500",
    error: "text-red-500",
    pending: "text-muted-foreground",
  }

  const statusIcon = {
    running: <Activity className="h-3 w-3 animate-pulse" />,
    idle: <Clock className="h-3 w-3" />,
    error: <AlertCircle className="h-3 w-3" />,
    pending: <Clock className="h-3 w-3" />,
  }

  const runningCount = workloads.filter((w) => w.status === "running").length
  const totalCpu = workloads.reduce((acc, w) => acc + w.cpuUsage, 0)

  return (
    <div className="flex h-full flex-col gap-2.5 p-3 overflow-auto">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <Cpu className="h-3.5 w-3.5 text-primary" />
          <span className="text-xs font-medium text-foreground">Compute</span>
        </div>
        <div className="flex items-center gap-1 rounded-full bg-emerald-500/10 px-2 py-0.5">
          <CheckCircle2 className="h-2.5 w-2.5 text-emerald-500" />
          <span className="text-[10px] font-medium text-emerald-500">{stats.isConnected ? "Online" : "Offline"}</span>
        </div>
      </div>

      {/* Primary Metric */}
      <div className="rounded-lg bg-background/50 px-2.5 py-2">
        <div className="flex items-end justify-between">
          <div>
            <div className="text-[10px] uppercase tracking-[0.15em] text-muted-foreground">Active Workloads</div>
            <div className="mt-0.5 flex items-baseline gap-1">
              <span className="font-mono text-xl font-semibold text-foreground">{runningCount}</span>
              <span className="text-[10px] text-muted-foreground">/ {workloads.length} total</span>
            </div>
          </div>
          <div className="text-right">
            <div className="text-[10px] uppercase tracking-[0.15em] text-muted-foreground">CPU</div>
            <div className="mt-0.5 font-mono text-sm text-foreground">{totalCpu}%</div>
          </div>
        </div>

        {/* Sparkline */}
        <div className="mt-2 flex h-6 items-end gap-0.5">
          {sparkData.map((val, i) => (
            <div
              key={i}
              className="flex-1 rounded-sm bg-primary/50 transition-all duration-300"
              style={{ height: `${val}%` }}
            />
          ))}
        </div>
      </div>

      {/* Workload List */}
      <div className="flex-1 space-y-1 overflow-auto">
        <div className="text-[10px] uppercase tracking-[0.15em] text-muted-foreground">Workloads</div>
        {workloads.map((workload) => (
          <div
            key={workload.id}
            className="flex items-center gap-2 rounded-md bg-background/30 px-2 py-1.5"
          >
            <div className={statusColor[workload.status]}>
              {statusIcon[workload.status]}
            </div>
            <div className="min-w-0 flex-1">
              <div className="truncate text-[11px] font-medium text-foreground">{workload.name}</div>
              <div className="flex items-center gap-2 text-[10px] text-muted-foreground">
                <span className="capitalize">{workload.status}</span>
                {workload.duration !== "--" && <span>· {workload.duration}</span>}
              </div>
            </div>
            {workload.cpuUsage > 0 && (
              <div className="font-mono text-[10px] text-muted-foreground">{workload.cpuUsage}%</div>
            )}
          </div>
        ))}
      </div>

      {/* Footer Stats */}
      <div className="flex items-center gap-3 border-t border-border/40 pt-2">
        <div className="flex items-center gap-1">
          <TrendingUp className="h-3 w-3 text-muted-foreground" />
          <span className="text-[10px] text-muted-foreground">
            {stats.nodeCount} nodes
          </span>
        </div>
        <div className="flex items-center gap-1">
          <Activity className="h-3 w-3 text-muted-foreground" />
          <span className="text-[10px] text-muted-foreground">
            {stats.activeSessions} sessions
          </span>
        </div>
      </div>
    </div>
  )
}
