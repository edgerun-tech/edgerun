"use client"

import { Activity, Cpu, HardDrive, RadioTower, WalletCards } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { XrayViewport } from "@/features/xray/XrayViewport"
import { XrayInspector } from "@/features/xray/XrayInspector"
import { XrayCommandSurface } from "@/features/xray/XrayCommandSurface"

type XrayDesktopSurfaceProps = {
  nodeCount: number
  activeSessions: number
  ramUsage: { used: number; total: number }
  runningApps: number
  isConnected: boolean
}

function MiniTile({ icon, label, value, sub }: { icon: React.ReactNode; label: string; value: string; sub?: string }) {
  return (
    <div className="rounded-2xl border border-[var(--window-border)] bg-background/70 p-3 shadow-2xl backdrop-blur-md">
      <div className="mb-2 flex items-center gap-2 text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
        <span className="text-primary">{icon}</span>
        {label}
      </div>
      <div className="font-mono text-lg text-foreground">{value}</div>
      {sub && <div className="mt-1 text-[11px] text-muted-foreground">{sub}</div>}
    </div>
  )
}

export function XrayDesktopSurface({
  nodeCount,
  activeSessions,
  ramUsage,
  runningApps,
  isConnected,
}: XrayDesktopSurfaceProps) {
  const ramPercent = ramUsage.total > 0 ? Math.round((ramUsage.used / ramUsage.total) * 100) : 0
  const edgePerHour = (nodeCount * 0.018 + activeSessions * 0.041).toFixed(3)

  return (
    <div className="absolute inset-0 z-0 overflow-hidden bg-zinc-950">
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_50%_35%,rgba(120,119,198,0.16),transparent_36%),radial-gradient(circle_at_80%_10%,rgba(16,185,129,0.08),transparent_32%)]" />

      <div className="absolute inset-y-4 left-4 z-10 grid w-56 grid-rows-2 gap-4 pt-12 pb-20">
        <MiniTile
          icon={<RadioTower className="h-3.5 w-3.5" />}
          label="network"
          value={`${nodeCount} peers`}
          sub={isConnected ? "online · discovery active" : "offline"}
        />
        <MiniTile
          icon={<WalletCards className="h-3.5 w-3.5" />}
          label="earnings"
          value={`${edgePerHour}`}
          sub="EDGE / hour estimated"
        />
      </div>

      <div className="absolute inset-y-4 right-4 z-10 grid w-56 grid-rows-2 gap-4 pt-12 pb-20">
        <MiniTile
          icon={<Cpu className="h-3.5 w-3.5" />}
          label="workloads"
          value={`${activeSessions}`}
          sub={`${runningApps} temporary app${runningApps === 1 ? "" : "s"}`}
        />
        <MiniTile
          icon={<HardDrive className="h-3.5 w-3.5" />}
          label="memory"
          value={`${ramPercent}%`}
          sub={`${ramUsage.used.toFixed(1)} / ${ramUsage.total} GB`}
        />
      </div>

      <div className="absolute inset-4 z-0 rounded-[28px] border border-white/5 bg-black/30 shadow-[0_0_120px_rgba(0,0,0,0.65)]">
        <div className="absolute left-1/2 top-4 z-20 flex -translate-x-1/2 items-center gap-2 rounded-full border border-white/10 bg-background/65 px-3 py-1.5 backdrop-blur-md">
          <Badge variant="secondary" className="bg-primary/10 text-primary">
            <Activity className="h-3 w-3" /> XRAY
          </Badge>
          <span className="font-mono text-[10px] uppercase tracking-[0.24em] text-muted-foreground">
            graph workspace
          </span>
        </div>

        <div className="absolute inset-0 flex overflow-hidden rounded-[28px]">
          <div className="min-w-0 flex-1">
            <XrayViewport />
          </div>
          <div className="hidden shrink-0 lg:block">
            <XrayInspector />
          </div>
        </div>

        <div className="absolute inset-x-0 bottom-0 z-20 overflow-hidden rounded-b-[28px] border-t border-white/5">
          <XrayCommandSurface />
        </div>
      </div>
    </div>
  )
}
