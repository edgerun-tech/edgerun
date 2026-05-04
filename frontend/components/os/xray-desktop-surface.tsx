"use client"

import type { ReactNode } from "react"
import { Activity, Cpu, HardDrive, RadioTower, WalletCards } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { FinancesOverviewWidget } from "@/components/sections/finance-overviews"
import { WorkspaceStatusPanel } from "@/components/workspace"
import { XrayViewport } from "@/features/xray/XrayViewport"
import { XrayInspector } from "@/features/xray/XrayInspector"
import { XrayCommandSurface } from "@/features/xray/XrayCommandSurface"
import type { AppSurfaceDef, AppSurfaceSlot } from "@/stores/desktop-store"

type XrayDesktopSurfaceProps = {
  nodeCount: number
  activeSessions: number
  ramUsage: { used: number; total: number }
  runningApps: number
  isConnected: boolean
  pinnedSurfaces?: AppSurfaceDef[]
}

function MiniTile({ icon, label, value, sub }: { icon: ReactNode; label: string; value: string; sub?: string }) {
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

function SurfaceSlot({ surface, fallback }: { surface?: AppSurfaceDef; fallback: ReactNode }) {
  if (!surface) return <>{fallback}</>

  return (
    <div
      data-pinned-surface-id={surface.id}
      data-app-id={surface.appId}
      className="min-h-0 overflow-hidden rounded-2xl border border-[var(--window-border)] bg-background/78 shadow-2xl backdrop-blur-md"
    >
      <div className="flex h-9 items-center gap-2 border-b border-border/60 px-3">
        <span className="text-primary">{surface.icon}</span>
        <span className="min-w-0 truncate text-xs font-medium text-foreground">{surface.title}</span>
      </div>
      <div className="h-[calc(100%-2.25rem)] min-h-0 overflow-auto">
        {surface.component}
      </div>
    </div>
  )
}

function CompactMetric({ icon, label, value }: { icon: ReactNode; label: string; value: string }) {
  return (
    <div className="flex min-w-0 items-center gap-2 rounded-full border border-white/10 bg-background/70 px-3 py-1.5 shadow-xl backdrop-blur-md">
      <span className="shrink-0 text-primary">{icon}</span>
      <span className="hidden text-[10px] uppercase tracking-[0.18em] text-muted-foreground sm:inline">{label}</span>
      <span className="font-mono text-xs text-foreground">{value}</span>
    </div>
  )
}

function MiniFinanceSparkline() {
  return (
    <div className="flex h-5 w-20 items-end gap-0.5 overflow-hidden rounded-sm opacity-80">
      {[28, 42, 34, 55, 47, 64, 58, 76, 69, 86].map((height, index) => (
        <div
          key={index}
          className="w-1 rounded-t bg-primary/65"
          style={{ height: `${height}%` }}
        />
      ))}
    </div>
  )
}

function bySlot(surfaces: AppSurfaceDef[] | undefined, slot: AppSurfaceSlot) {
  return surfaces?.find((surface) => surface.preferredSlot === slot)
}

export function XrayDesktopSurface({
  nodeCount,
  activeSessions,
  ramUsage,
  runningApps,
  isConnected,
  pinnedSurfaces = [],
}: XrayDesktopSurfaceProps) {
  const ramPercent = ramUsage.total > 0 ? Math.round((ramUsage.used / ramUsage.total) * 100) : 0
  const edgePerHour = (nodeCount * 0.018 + activeSessions * 0.041).toFixed(3)

  return (
    <div className="absolute inset-0 z-0 overflow-hidden bg-zinc-950">
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_50%_35%,rgba(120,119,198,0.16),transparent_36%),radial-gradient(circle_at_80%_10%,rgba(16,185,129,0.08),transparent_32%)]" />

      <div className="absolute inset-y-4 left-4 z-10 hidden w-56 grid-rows-2 gap-4 pt-12 pb-20 xl:grid">
        <SurfaceSlot
          surface={bySlot(pinnedSurfaces, "left-top")}
          fallback={
            <MiniTile
              icon={<RadioTower className="h-3.5 w-3.5" />}
              label="network"
              value={`${nodeCount} peers`}
              sub={isConnected ? "online · discovery active" : "offline"}
            />
          }
        />
        <SurfaceSlot
          surface={bySlot(pinnedSurfaces, "left-bottom")}
          fallback={
            <div className="min-h-0 overflow-hidden rounded-2xl border border-[var(--window-border)] bg-background/78 shadow-2xl backdrop-blur-md">
              <FinancesOverviewWidget />
            </div>
          }
        />
      </div>

      <div className="absolute inset-y-4 right-4 z-10 hidden w-56 grid-rows-2 gap-4 pt-12 pb-20 xl:grid">
        <SurfaceSlot
          surface={bySlot(pinnedSurfaces, "right-top")}
          fallback={
            <MiniTile
              icon={<Cpu className="h-3.5 w-3.5" />}
              label="workloads"
              value={`${activeSessions}`}
              sub={`${runningApps} active surface${runningApps === 1 ? "" : "s"}`}
            />
          }
        />
        <SurfaceSlot
          surface={bySlot(pinnedSurfaces, "right-bottom")}
          fallback={
            <div className="min-h-0 overflow-hidden rounded-2xl border border-[var(--window-border)] bg-background/78 shadow-2xl backdrop-blur-md">
              <WorkspaceStatusPanel surface="embedded" />
            </div>
          }
        />
      </div>

      <div className="absolute left-4 right-4 top-4 z-20 flex items-center gap-2 overflow-x-auto xl:hidden">
        <CompactMetric
          icon={<RadioTower className="h-3.5 w-3.5" />}
          label="nodes"
          value={`${nodeCount}`}
        />
        <CompactMetric
          icon={<WalletCards className="h-3.5 w-3.5" />}
          label="EDGE/h"
          value={edgePerHour}
        />
        <div className="flex min-w-0 items-center gap-2 rounded-full border border-white/10 bg-background/70 px-3 py-1.5 shadow-xl backdrop-blur-md">
          <MiniFinanceSparkline />
          <span className="font-mono text-xs text-primary">market</span>
        </div>
        <CompactMetric
          icon={<HardDrive className="h-3.5 w-3.5" />}
          label="mem"
          value={`${ramPercent}%`}
        />
      </div>

      <div className="absolute inset-y-4 left-[260px] right-[260px] z-0 rounded-[28px] border border-white/5 bg-black/30 shadow-[0_0_120px_rgba(0,0,0,0.65)] max-xl:left-4 max-xl:right-4 max-xl:top-14">
        <div className="absolute left-1/2 top-4 z-20 hidden -translate-x-1/2 items-center gap-2 xl:flex">
          <Badge variant="secondary" className="border-white/10 bg-background/70 text-primary backdrop-blur-md">
            <Activity className="h-3 w-3" /> XRAY
          </Badge>
          <XrayCommandSurface surface="top" />
        </div>

        <div className="absolute inset-0 flex overflow-hidden rounded-[28px]">
          <div className="min-w-0 flex-1">
            <XrayViewport />
          </div>
          <div className="hidden shrink-0 lg:block">
            <XrayInspector />
          </div>
        </div>
      </div>
    </div>
  )
}
