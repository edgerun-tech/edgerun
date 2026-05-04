"use client"

import { Activity, Coins, HardDrive, RadioTower, Server, Wifi, WifiOff, Zap } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { cn } from "@/lib/utils"

type DesktopTelemetryProps = {
  nodeCount: number
  activeSessions: number
  ramUsage: { used: number; total: number }
  isConnected: boolean
  runningApps: number
}

function MetricLine({ label, value, muted = false }: { label: string; value: string; muted?: boolean }) {
  return (
    <div className="grid grid-cols-[72px_1fr] gap-3 font-mono text-[10px] leading-5">
      <span className="uppercase tracking-[0.22em] text-muted-foreground/45">{label}</span>
      <span className={cn("text-foreground/55", muted && "text-muted-foreground/35")}>{value}</span>
    </div>
  )
}

function Bar({ value }: { value: number }) {
  const clamped = Math.max(0, Math.min(100, value))
  const filled = Math.round(clamped / 10)
  return (
    <span className="tracking-[0.16em] text-primary/45">
      {"█".repeat(filled)}<span className="text-muted-foreground/20">{"░".repeat(10 - filled)}</span>
    </span>
  )
}

export function DesktopTelemetry({
  nodeCount,
  activeSessions,
  ramUsage,
  isConnected,
  runningApps,
}: DesktopTelemetryProps) {
  const ramPercent = ramUsage.total > 0 ? Math.round((ramUsage.used / ramUsage.total) * 100) : 0
  const utilization = Math.max(8, Math.min(92, Math.round(activeSessions * 12 + runningApps * 4)))
  const estimatedEdgePerHour = (nodeCount * 0.018 + activeSessions * 0.041).toFixed(3)
  const dailyEstimate = (Number(estimatedEdgePerHour) * 24).toFixed(2)

  return (
    <div className="pointer-events-none absolute inset-0 z-0 overflow-hidden">
      <div className="absolute left-6 top-6 w-[300px] select-none rounded-2xl border border-primary/10 bg-background/10 p-4 shadow-[0_0_80px_rgba(0,0,0,0.18)] backdrop-blur-[1px]">
        <div className="mb-4 flex items-center justify-between">
          <div className="flex items-center gap-2 text-[10px] uppercase tracking-[0.24em] text-primary/55">
            <RadioTower className="h-3.5 w-3.5" />
            node telemetry
          </div>
          <Badge variant={isConnected ? "secondary" : "outline"} className="h-5 border-primary/10 bg-primary/5 px-2 font-mono text-[9px] text-primary/70">
            {isConnected ? <Wifi className="h-3 w-3" /> : <WifiOff className="h-3 w-3" />}
            {isConnected ? "online" : "offline"}
          </Badge>
        </div>

        <div className="space-y-1.5">
          <MetricLine label="nodes" value={`${nodeCount} peers visible`} />
          <MetricLine label="apps" value={`${runningApps} active windows`} />
          <MetricLine label="sessions" value={`${activeSessions} workloads`} />
          <MetricLine label="memory" value={`${ramUsage.used.toFixed(1)} / ${ramUsage.total} GB  ${ramPercent}%`} />
          <MetricLine label="mem bar" value="" muted />
          <div className="pl-[84px] font-mono text-[10px]"><Bar value={ramPercent} /></div>
        </div>
      </div>

      <div className="absolute bottom-24 right-8 w-[330px] select-none rounded-2xl border border-primary/10 bg-background/10 p-4 backdrop-blur-[1px]">
        <div className="mb-4 flex items-center gap-2 text-[10px] uppercase tracking-[0.24em] text-primary/55">
          <Coins className="h-3.5 w-3.5" />
          compute market
        </div>
        <div className="grid grid-cols-2 gap-3">
          <div className="rounded-xl border border-primary/10 bg-primary/[0.03] p-3">
            <div className="mb-2 flex items-center gap-1.5 text-[10px] text-muted-foreground/55">
              <Zap className="h-3 w-3" /> utilization
            </div>
            <div className="font-mono text-xl text-foreground/65">{utilization}%</div>
            <Bar value={utilization} />
          </div>
          <div className="rounded-xl border border-primary/10 bg-primary/[0.03] p-3">
            <div className="mb-2 flex items-center gap-1.5 text-[10px] text-muted-foreground/55">
              <Activity className="h-3 w-3" /> estimate
            </div>
            <div className="font-mono text-xl text-foreground/65">{estimatedEdgePerHour}</div>
            <div className="font-mono text-[10px] text-muted-foreground/40">EDGE / hour</div>
          </div>
        </div>
        <div className="mt-3 grid grid-cols-[16px_1fr_auto] items-center gap-2 font-mono text-[10px] text-muted-foreground/40">
          <Server className="h-3.5 w-3.5" />
          <span>projected daily gross</span>
          <span className="text-primary/55">{dailyEstimate} EDGE</span>
          <HardDrive className="h-3.5 w-3.5" />
          <span>local resource policy</span>
          <span className="text-foreground/45">balanced</span>
        </div>
      </div>
    </div>
  )
}
