"use client"

import type { ReactNode } from "react"
import { HardDrive, RadioTower, WalletCards } from "lucide-react"
import { GlowingContainer } from "@/components/layouts/glowing-containers"
import { CodelyzerCodeWidget, NetworkConnectionsWidget } from "@/components/sections/codelyzer-network"
import { FinancesOverviewWidget } from "@/components/sections/finance-overviews"
import { WorkspaceStatusPanel } from "@/components/workspace"
import { XrayViewport } from "@/features/xray/XrayViewport"
import { XrayCommandSurface } from "@/features/xray/XrayCommandSurface"
import { XrayLegendOverlay, XrayMouseHelpOverlay } from "@/features/xray/XrayOverlayWidgets"
import { EdgerunLogo } from "./edgerun-logo"
import type { AppSurfaceDef, AppSurfaceSlot } from "@/stores/desktop-store"

type XrayDesktopSurfaceProps = {
  nodeCount: number
  activeSessions: number
  ramUsage: { used: number; total: number }
  runningApps: number
  isConnected: boolean
  pinnedSurfaces?: AppSurfaceDef[]
}

function SurfaceSlot({ surface, fallback }: { surface?: AppSurfaceDef; fallback: ReactNode }) {
  if (!surface) return <>{fallback}</>

  return (
    <GlowingContainer
      className="h-full min-h-0"
      contentClassName="flex h-full min-h-0 flex-col bg-background/78"
      proximity={52}
      spread={70}
      borderWidth={2}
      data-pinned-surface-id={surface.id}
      data-app-id={surface.appId}
    >
      <div className="flex h-9 shrink-0 items-center gap-2 border-b border-border/60 px-3">
        <span className="text-primary">{surface.icon}</span>
        <span className="min-w-0 truncate text-xs font-medium text-foreground">{surface.title}</span>
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        {surface.component}
      </div>
    </GlowingContainer>
  )
}

function WidgetFrame({ children }: { children: ReactNode }) {
  return (
    <GlowingContainer className="h-full min-h-0" contentClassName="h-full min-h-0 bg-background/78" proximity={52} spread={70} borderWidth={2}>
      {children}
    </GlowingContainer>
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

      <div className="absolute inset-y-4 left-4 z-10 hidden w-56 grid-rows-2 gap-4 pb-20 pt-4 xl:grid">
        <SurfaceSlot
          surface={bySlot(pinnedSurfaces, "left-top")}
          fallback={<WidgetFrame><NetworkConnectionsWidget /></WidgetFrame>}
        />
        <SurfaceSlot
          surface={bySlot(pinnedSurfaces, "left-bottom")}
          fallback={<WidgetFrame><CodelyzerCodeWidget /></WidgetFrame>}
        />
      </div>

      <div className="absolute inset-y-4 right-4 z-10 hidden w-56 grid-rows-2 gap-4 pb-20 pt-4 xl:grid">
        <SurfaceSlot
          surface={bySlot(pinnedSurfaces, "right-top")}
          fallback={<WidgetFrame><FinancesOverviewWidget /></WidgetFrame>}
        />
        <SurfaceSlot
          surface={bySlot(pinnedSurfaces, "right-bottom")}
          fallback={<WidgetFrame><WorkspaceStatusPanel surface="embedded" /></WidgetFrame>}
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

      <GlowingContainer
        className="absolute inset-y-4 left-[260px] right-[260px] z-0 max-xl:left-4 max-xl:right-4 max-xl:top-14"
        contentClassName="h-full bg-black/30 shadow-[0_0_120px_rgba(0,0,0,0.65)]"
        proximity={96}
        spread={110}
        borderWidth={2}
      >
        <div className="pointer-events-none absolute left-1/2 top-4 z-20 hidden -translate-x-1/2 text-primary/85 xl:block">
          <EdgerunLogo variant="full" size="sm" />
        </div>

        <XrayLegendOverlay />
        <XrayMouseHelpOverlay />

        <div className="absolute right-3 top-1/2 z-20 hidden -translate-y-1/2 xl:block">
          <XrayCommandSurface surface="vertical" />
        </div>

        <div className="absolute inset-0 overflow-hidden rounded-xl">
          <XrayViewport />
        </div>
      </GlowingContainer>
    </div>
  )
}
